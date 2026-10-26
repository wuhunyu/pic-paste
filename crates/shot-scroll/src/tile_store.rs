use crate::limits::validate_canvas;
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
    if frame.width == 0 || frame.height == 0
        || usize::try_from(expected).map_err(|_| invalid("image too large"))?
            != frame.rgba.len()
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
        let first_path = match write_strip(
            &directory, 0, first, 0, 0, first.width, first.height,
        ) {
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
                    let canvas_h = self.height.checked_add(displacement)
                        .ok_or_else(|| invalid("canvas height overflow"))?;
                    (
                        self.width, canvas_h, 0, next.height - displacement,
                        next.width, displacement, 0, self.height,
                    )
                }
                ScrollAxis::Horizontal => {
                    if displacement < 2 || displacement >= next.width {
                        return Err(invalid("invalid horizontal overlap"));
                    }
                    let canvas_w = self.width.checked_add(displacement)
                        .ok_or_else(|| invalid("canvas width overflow"))?;
                    (
                        canvas_w, self.height, next.width - displacement, 0,
                        displacement, next.height, self.width, 0,
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
            crop_x, crop_y, strip_w, strip_h,
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

    pub fn fill_row(&self, y: u32, out: &mut [u8]) -> io::Result<()> {
        if y >= self.height || out.len() != self.width as usize * 4 {
            return Err(invalid("invalid tile output row"));
        }
        out.fill(0);
        for strip in &self.strips {
            if y >= strip.start_y && y < strip.start_y + strip.height {
                let mut file = File::open(&strip.path)?;
                read_strip_row(strip, &mut file, y, out)?;
            }
        }
        Ok(())
    }

    pub fn encode_png<W: Write>(&self, output: W) -> io::Result<()> {
        // Reuse open file handles across all output rows.
        let mut readers: Vec<File> = self.strips.iter()
            .map(|strip| File::open(&strip.path))
            .collect::<io::Result<_>>()?;
        shot_image::stream_png::encode_png_rows(
            output,
            self.width,
            self.height,
            |y, out| {
                out.fill(0);
                for (strip, file) in self.strips.iter().zip(readers.iter_mut()) {
                    if y >= strip.start_y && y < strip.start_y + strip.height {
                        read_strip_row(strip, file, y, out)?;
                    }
                }
                Ok(())
            },
        )
    }
}

fn read_strip_row(
    strip: &Strip,
    file: &mut File,
    row: u32,
    output: &mut [u8],
) -> io::Result<()> {
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
