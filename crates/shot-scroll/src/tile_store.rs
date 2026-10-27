use crate::edits::{EditStack, MAX_EDIT_PIXELS};
use crate::limits::validate_canvas;
use crate::review_transfer::validate_transfer_rect;
use shot_core::{RgbaFrame, platform::ScrollAxis};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_SESSION: AtomicU64 = AtomicU64::new(0);

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn validate_frame(frame: &RgbaFrame) -> io::Result<()> {
    let expected = u64::from(frame.width)
        .checked_mul(u64::from(frame.height))
        .and_then(|value| value.checked_mul(4))
        .ok_or_else(|| invalid("invalid image dimensions"))?;
    if frame.width == 0
        || frame.height == 0
        || usize::try_from(expected).map_err(|_| invalid("image too large"))? != frame.rgba.len()
    {
        return Err(invalid("invalid RGBA buffer"));
    }
    Ok(())
}

fn temporary_directory(root: &Path) -> io::Result<PathBuf> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let serial = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    let directory = root.join(format!(
        ".picpaste-scroll-{}-{nonce}-{serial}",
        std::process::id(),
    ));
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(&directory)?;
    Ok(directory)
}

#[derive(Clone, Debug)]
struct Strip {
    path: PathBuf,
    start_x: u32,
    start_y: u32,
    width: u32,
    height: u32,
}

/// A scroll canvas stored as trusted RGBA strips on disk.
/// Large images never allocate a contiguous width × height × 4 buffer.
pub struct TileStore {
    directory: PathBuf,
    axis: ScrollAxis,
    source_width: u32,
    source_height: u32,
    width: u32,
    height: u32,
    strips: Vec<Strip>,
}

fn write_strip(
    directory: &Path,
    index: usize,
    source: &RgbaFrame,
    crop_x: u32,
    crop_y: u32,
    crop_width: u32,
    crop_height: u32,
) -> io::Result<PathBuf> {
    let path = directory.join(format!("{index:06}.rgba"));
    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&path)?;
        let pitch = source.width as usize * 4;
        let row_bytes = crop_width as usize * 4;
        for y in crop_y..crop_y + crop_height {
            let start = y as usize * pitch + crop_x as usize * 4;
            file.write_all(&source.rgba[start..start + row_bytes])?;
        }
        file.sync_all()?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&path);
        return Err(error);
    }
    Ok(path)
}

impl TileStore {
    pub fn new(first: &RgbaFrame, axis: ScrollAxis, temp_root: &Path) -> io::Result<Self> {
        validate_frame(first)?;
        validate_canvas(u64::from(first.width), u64::from(first.height))
            .map_err(|_| invalid("scroll image exceeds canvas limit"))?;
        let directory = temporary_directory(temp_root)?;
        let first_path = match write_strip(&directory, 0, first, 0, 0, first.width, first.height) {
            Ok(path) => path,
            Err(error) => {
                let _ = fs::remove_dir_all(&directory);
                return Err(error);
            }
        };
        Ok(Self {
            directory,
            axis,
            source_width: first.width,
            source_height: first.height,
            width: first.width,
            height: first.height,
            strips: vec![Strip {
                path: first_path,
                start_x: 0,
                start_y: 0,
                width: first.width,
                height: first.height,
            }],
        })
    }

    pub fn canvas(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn strip_count(&self) -> usize {
        self.strips.len()
    }

    /// One-based index into the trusted timeline. Return half-open source-pixel
    /// span along the manually scrolled axis to help locate a suspect segment.
    pub fn segment_span(&self, segment_one_based: usize) -> Option<(u32, u32)> {
        let strip = self.strips.get(segment_one_based.checked_sub(1)?)?;
        let (start, length) = match self.axis {
            ScrollAxis::Vertical => (strip.start_y, strip.height),
            ScrollAxis::Horizontal => (strip.start_x, strip.width),
        };
        Some((start, start.checked_add(length)?))
    }

    /// Reconstruct ONLY the last original capture-sized viewport from trusted
    /// disk-backed strips after tail trim, without materializing the long canvas.
    /// It is the verified physical-pixel anchor for manual middle-segment retake.
    pub fn last_viewport(&self) -> io::Result<RgbaFrame> {
        let left = self
            .width
            .checked_sub(self.source_width)
            .ok_or_else(|| invalid("trusted scroll viewport width is missing"))?;
        let top = self
            .height
            .checked_sub(self.source_height)
            .ok_or_else(|| invalid("trusted scroll viewport height is missing"))?;
        let capacity = u64::from(self.source_width)
            .checked_mul(u64::from(self.source_height))
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| invalid("invalid native viewport allocation"))?;
        let mut bytes = Vec::with_capacity(capacity);
        let mut full_row = vec![0u8; self.width as usize * 4];
        let mut readers = self.open_strip_readers()?;
        let left_byte = left as usize * 4;
        let right_byte = left_byte + self.source_width as usize * 4;
        for y in top..top + self.source_height {
            self.fill_row_with_readers(y, &mut full_row, &mut readers)?;
            bytes.extend_from_slice(&full_row[left_byte..right_byte]);
        }
        RgbaFrame::new(self.source_width, self.source_height, bytes)
            .map_err(|_| invalid("reconstructed native viewport has invalid dimensions"))
    }

    pub fn working_dir(&self) -> &Path {
        &self.directory
    }

    /// Add only the newly visible tail of a matching ROI frame.
    /// Callers must establish overlap confidence before accepting a displacement.
    pub fn append(&mut self, next: &RgbaFrame, displacement: u32) -> io::Result<()> {
        validate_frame(next)?;
        if next.width != self.source_width || next.height != self.source_height {
            return Err(invalid("scroll ROI dimensions changed"));
        }
        let (canvas_w, canvas_h, crop_x, crop_y, strip_w, strip_h, start_x, start_y) =
            match self.axis {
                ScrollAxis::Vertical => {
                    if displacement < 2 || displacement >= next.height {
                        return Err(invalid("invalid vertical overlap"));
                    }
                    let canvas_h = self
                        .height
                        .checked_add(displacement)
                        .ok_or_else(|| invalid("canvas height overflow"))?;
                    (
                        self.width,
                        canvas_h,
                        0,
                        next.height - displacement,
                        next.width,
                        displacement,
                        0,
                        self.height,
                    )
                }
                ScrollAxis::Horizontal => {
                    if displacement < 2 || displacement >= next.width {
                        return Err(invalid("invalid horizontal overlap"));
                    }
                    let canvas_w = self
                        .width
                        .checked_add(displacement)
                        .ok_or_else(|| invalid("canvas width overflow"))?;
                    (
                        canvas_w,
                        self.height,
                        next.width - displacement,
                        0,
                        displacement,
                        next.height,
                        self.width,
                        0,
                    )
                }
            };
        validate_canvas(u64::from(canvas_w), u64::from(canvas_h))
            .map_err(|_| invalid("scroll canvas capacity exceeded"))?;
        if self.strips.len() >= 4096 {
            return Err(invalid("too many scroll segments"));
        }
        let path = write_strip(
            &self.directory,
            self.strips.len(),
            next,
            crop_x,
            crop_y,
            strip_w,
            strip_h,
        )?;
        self.strips.push(Strip {
            path,
            start_x,
            start_y,
            width: strip_w,
            height: strip_h,
        });
        self.width = canvas_w;
        self.height = canvas_h;
        Ok(())
    }

    pub fn undo_last(&mut self) -> io::Result<bool> {
        if self.strips.len() <= 1 {
            return Ok(false);
        }
        let strip = self.strips.last().expect("non-empty strips");
        fs::remove_file(&strip.path)?;
        match self.axis {
            ScrollAxis::Vertical => self.height -= strip.height,
            ScrollAxis::Horizontal => self.width -= strip.width,
        }
        self.strips.pop();
        Ok(true)
    }

    /// Retain exactly the first `keep_count` trusted strips and delete the tail.
    ///
    /// Removing a middle strip while retaining later strips would break the
    /// overlap geometry, so this operation discards only a contiguous suffix.
    pub fn keep_first(&mut self, keep_count: usize) -> io::Result<usize> {
        if keep_count == 0 || keep_count > self.strips.len() {
            return Err(invalid("trusted strip prefix is out of bounds"));
        }
        let mut removed = 0;
        while self.strips.len() > keep_count {
            self.undo_last()?;
            removed += 1;
        }
        Ok(removed)
    }

    /// Build a bounded, nearest-neighbor preview from trusted disk strips.
    ///
    /// The full scroll image remains on disk; scratch memory is one source
    /// scanline plus at most a 1024x1024 RGBA preview. No upscaling is done.
    pub fn preview(&self, max_width: u32, max_height: u32) -> io::Result<RgbaFrame> {
        self.preview_with_edits(&EditStack::default(), max_width, max_height)
    }

    /// Sample visible edits while keeping the review preview at <= 1024 px.
    pub fn preview_with_edits(
        &self,
        edits: &EditStack,
        max_width: u32,
        max_height: u32,
    ) -> io::Result<RgbaFrame> {
        if max_width == 0 || max_height == 0 {
            return Err(invalid("empty preview bounds"));
        }
        let cap_width = max_width.min(1024);
        let cap_height = max_height.min(1024);
        let scale = (f64::from(cap_width) / f64::from(self.width))
            .min(f64::from(cap_height) / f64::from(self.height))
            .min(1.0);
        let out_width = (f64::from(self.width) * scale).floor().max(1.0) as u32;
        let out_height = (f64::from(self.height) * scale).floor().max(1.0) as u32;
        let mut source_row = vec![0u8; self.width as usize * 4];
        let mut strip_readers = self.open_strip_readers()?;
        let mut edit_readers = edits.open_readers()?;
        let capacity = out_width as usize * out_height as usize * 4;
        let mut rgba = Vec::with_capacity(capacity);
        for y in 0..out_height {
            let source_y = (u64::from(y) * u64::from(self.height) / u64::from(out_height)) as u32;
            self.fill_row_with_readers(source_y, &mut source_row, &mut strip_readers)?;
            edits.apply_to_row(source_y, &mut source_row, &mut edit_readers)?;
            for x in 0..out_width {
                let source_x =
                    (u64::from(x) * u64::from(self.width) / u64::from(out_width)) as usize;
                let start = source_x * 4;
                rgba.extend_from_slice(&source_row[start..start + 4]);
            }
        }
        RgbaFrame::new(out_width, out_height, rgba)
            .map_err(|_| invalid("invalid generated preview"))
    }

    pub fn fill_row(&self, y: u32, out: &mut [u8]) -> io::Result<()> {
        let mut readers = self.open_strip_readers()?;
        self.fill_row_with_readers(y, out, &mut readers)
    }

    fn open_strip_readers(&self) -> io::Result<Vec<File>> {
        self.strips
            .iter()
            .map(|strip| File::open(&strip.path))
            .collect()
    }

    fn fill_row_with_readers(
        &self,
        y: u32,
        out: &mut [u8],
        readers: &mut [File],
    ) -> io::Result<()> {
        if y >= self.height
            || out.len() != self.width as usize * 4
            || readers.len() != self.strips.len()
        {
            return Err(invalid("invalid tile output row"));
        }
        out.fill(0);
        for (strip, file) in self.strips.iter().zip(readers.iter_mut()) {
            if y >= strip.start_y && y < strip.start_y + strip.height {
                read_strip_row(strip, file, y, out)?;
            }
        }
        Ok(())
    }

    /// Only a bounded native RGBA crop may leave disk storage for clipboard/pins.
    pub fn render_region_edited(
        &self,
        edits: &EditStack,
        region: shot_core::RectPx,
        radius: u32,
    ) -> io::Result<RgbaFrame> {
        validate_transfer_rect(region, self.canvas())?;
        let mut frame = self.render_edit_region(edits, region)?;
        shot_image::rounded_alpha(&mut frame, radius);
        Ok(frame)
    }

    /// Read a patch-sized region of the stitched canvas with active edits.
    /// Unlike the clipboard limit, a thin annotation may span over 4096 px.
    pub(crate) fn render_edit_region(
        &self,
        edits: &EditStack,
        region: shot_core::RectPx,
    ) -> io::Result<RgbaFrame> {
        if region.x < 0
            || region.y < 0
            || region.width == 0
            || region.height == 0
            || u64::from(region.x as u32) + u64::from(region.width) > u64::from(self.width)
            || u64::from(region.y as u32) + u64::from(region.height) > u64::from(self.height)
            || u64::from(region.width) * u64::from(region.height) > MAX_EDIT_PIXELS
        {
            return Err(invalid("invalid bounded annotation region"));
        }
        let mut rgba = Vec::with_capacity(region.width as usize * region.height as usize * 4);
        let mut row = vec![0; self.width as usize * 4];
        let mut strip_readers = self.open_strip_readers()?;
        let mut edit_readers = edits.open_readers()?;
        let left = region.x as usize * 4;
        let right = left + region.width as usize * 4;
        for y in region.y as u32..(region.y as u32 + region.height) {
            self.fill_row_with_readers(y, &mut row, &mut strip_readers)?;
            edits.apply_to_row(y, &mut row, &mut edit_readers)?;
            rgba.extend_from_slice(&row[left..right]);
        }
        RgbaFrame::new(region.width, region.height, rgba)
            .map_err(|_| invalid("invalid rendered annotation crop"))
    }

    /// Stream this canvas into a private .part file, sync it, then atomically
    /// publish an immutable PNG hard link without overwriting existing output.
    pub fn save_png_no_clobber(&self, destination: &Path) -> io::Result<()> {
        self.save_png_rounded_no_clobber(destination, 0)
    }

    /// Atomic PNG export with physical-pixel rounded alpha; still streams one row.
    pub fn save_png_rounded_no_clobber(&self, destination: &Path, radius: u32) -> io::Result<()> {
        self.save_png_edited_no_clobber(destination, radius, &EditStack::default())
    }

    pub fn save_png_edited_no_clobber(
        &self,
        destination: &Path,
        radius: u32,
        edits: &EditStack,
    ) -> io::Result<()> {
        if destination.extension().and_then(|value| value.to_str()) != Some("png") {
            return Err(invalid("PNG export only"));
        }
        let filename = destination
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| invalid("invalid output filename"))?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let serial = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
        let part = destination.with_file_name(format!(
            ".{filename}.{}-{nonce}-{serial}.part",
            std::process::id(),
        ));
        let result = (|| {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let file = options.open(&part)?;
            let mut writer = io::BufWriter::new(file);
            self.encode_png_with_edits(&mut writer, radius, edits)?;
            writer.flush()?;
            writer.get_ref().sync_all()?;
            drop(writer);
            fs::hard_link(&part, destination)
        })();
        let _ = fs::remove_file(&part);
        result
    }

    pub fn encode_png<W: Write>(&self, output: W) -> io::Result<()> {
        self.encode_png_rounded(output, 0)
    }

    pub fn encode_png_rounded<W: Write>(&self, output: W, radius: u32) -> io::Result<()> {
        self.encode_png_with_edits(output, radius, &EditStack::default())
    }

    /// Reuse strip and patch file handles through the entire streaming PNG.
    pub fn encode_png_with_edits<W: Write>(
        &self,
        output: W,
        radius: u32,
        edits: &EditStack,
    ) -> io::Result<()> {
        let mut readers = self.open_strip_readers()?;
        let mut edit_readers = edits.open_readers()?;
        shot_image::stream_png::encode_png_rows(output, self.width, self.height, |y, out| {
            self.fill_row_with_readers(y, out, &mut readers)?;
            edits.apply_to_row(y, out, &mut edit_readers)?;
            shot_image::rounded_alpha_row(out, self.width, self.height, y, radius);
            Ok(())
        })
    }
}

fn read_strip_row(strip: &Strip, file: &mut File, row: u32, output: &mut [u8]) -> io::Result<()> {
    let relative_y = u64::from(row - strip.start_y);
    let offset = relative_y * u64::from(strip.width) * 4;
    let start = strip.start_x as usize * 4;
    let end = start + strip.width as usize * 4;
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut output[start..end])
}

impl Drop for TileStore {
    fn drop(&mut self) {
        // Each session owns only the uniquely created directory, never its parent.
        let _ = fs::remove_dir_all(&self.directory);
    }
}
