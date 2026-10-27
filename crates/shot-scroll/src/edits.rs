//! Disk-backed non-destructive annotations for trusted stitched screenshot strips.
//! Each patch replaces only its clipped source-pixel rectangle; later patches
//! are rendered on earlier results, matching the existing screenshot compositor.
use crate::tile_store::TileStore;
use shot_core::RectPx;
use shot_image::history::Annotation;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

pub const MAX_EDIT_PIXELS: u64 = 4_000_000;
static NEXT_EDIT: AtomicU64 = AtomicU64::new(0);

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

#[derive(Debug)]
struct Patch {
    rect: RectPx,
    path: PathBuf,
}

#[derive(Debug, Default)]
pub struct EditStack {
    patches: Vec<Patch>,
    cursor: usize,
}

fn bounds(op: &Annotation, canvas: (u32, u32)) -> io::Result<RectPx> {
    let pair = |p: (f32, f32), q: (f32, f32), extra: f64| {
        (
            f64::from(p.0.min(q.0)) - extra,
            f64::from(p.1.min(q.1)) - extra,
            f64::from(p.0.max(q.0)) + extra,
            f64::from(p.1.max(q.1)) + extra,
        )
    };
    let (left, top, right, bottom) = match op {
        Annotation::Line { from, to, width, .. }
        | Annotation::Rectangle {
            top_left: from,
            bottom_right: to,
            width,
            ..
        } => pair(*from, *to, f64::from(*width) + 3.0),
        Annotation::Arrow {
            from, to, width, ..
        } => pair(*from, *to, f64::from(*width) + 24.0),
        Annotation::Brush { points, width, .. } => {
            if points.is_empty() {
                return Err(invalid("empty brush path"));
            }
            let mut x0 = f32::INFINITY;
            let mut y0 = f32::INFINITY;
            let mut x1 = f32::NEG_INFINITY;
            let mut y1 = f32::NEG_INFINITY;
            for &(x, y) in points {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
            pair((x0, y0), (x1, y1), f64::from(*width) + 3.0)
        }
        Annotation::Ellipse {
            center, radius, width, ..
        } => pair(
            (center.0 - radius.0, center.1 - radius.1),
            (center.0 + radius.0, center.1 + radius.1),
            f64::from(*width) + 3.0,
        ),
        Annotation::Text {
            at, content, size, ..
        } => {
            let mut longest = 0usize;
            let mut lines = 0usize;
            for line in content.lines() {
                lines += 1;
                longest = longest.max(line.chars().count());
            }
            if lines == 0 {
                lines = 1;
            }
            let size = f64::from(*size);
            (
                f64::from(at.0) - size * 2.0,
                f64::from(at.1) - size * 2.0,
                f64::from(at.0) + longest as f64 * size * 2.0 + size * 2.0,
                f64::from(at.1) + lines as f64 * size * 1.3 + size * 3.0,
            )
        }
        Annotation::Pixelate {
            left, top, width, height, ..
        }
        | Annotation::Blur {
            left, top, width, height, ..
        } => (
            f64::from(*left),
            f64::from(*top),
            f64::from(*left) + f64::from(*width),
            f64::from(*top) + f64::from(*height),
        ),
    };
    if ![left, top, right, bottom].iter().all(|v| v.is_finite()) {
        return Err(invalid("non-finite annotation bounds"));
    }
    let x0 = left.floor().clamp(0.0, f64::from(canvas.0)) as u32;
    let y0 = top.floor().clamp(0.0, f64::from(canvas.1)) as u32;
    let x1 = right.ceil().clamp(0.0, f64::from(canvas.0)) as u32;
    let y1 = bottom.ceil().clamp(0.0, f64::from(canvas.1)) as u32;
    let width = x1.saturating_sub(x0);
    let height = y1.saturating_sub(y0);
    if width == 0 || height == 0 {
        return Err(invalid("annotation is outside the stitched image"));
    }
    if u64::from(width) * u64::from(height) > MAX_EDIT_PIXELS {
        return Err(invalid("annotation exceeds 4MP edit patch limit; draw smaller marks"));
    }
    Ok(RectPx {
        x: x0 as i32,
        y: y0 as i32,
        width,
        height,
    })
}

fn shift_annotation(op: &Annotation, rect: RectPx) -> Annotation {
    let dx = rect.x as f32;
    let dy = rect.y as f32;
    let point = |(x, y): (f32, f32)| (x - dx, y - dy);
    match op {
        Annotation::Line { from, to, rgba, width } => Annotation::Line {
            from: point(*from), to: point(*to), rgba: *rgba, width: *width,
        },
        Annotation::Arrow { from, to, rgba, width } => Annotation::Arrow {
            from: point(*from), to: point(*to), rgba: *rgba, width: *width,
        },
        Annotation::Brush { points, rgba, width } => Annotation::Brush {
            points: points.iter().copied().map(point).collect(),
            rgba: *rgba, width: *width,
        },
        Annotation::Rectangle { top_left, bottom_right, rgba, width } => Annotation::Rectangle {
            top_left: point(*top_left), bottom_right: point(*bottom_right),
            rgba: *rgba, width: *width,
        },
        Annotation::Ellipse { center, radius, rgba, width } => Annotation::Ellipse {
            center: point(*center), radius: *radius, rgba: *rgba, width: *width,
        },
        Annotation::Text { at, content, size, rgba } => Annotation::Text {
            at: point(*at), content: content.clone(), size: *size, rgba: *rgba,
        },
        Annotation::Pixelate { left, top, width, height, block_size } => Annotation::Pixelate {
            left: left.saturating_sub(rect.x as u32),
            top: top.saturating_sub(rect.y as u32),
            width: *width, height: *height, block_size: *block_size,
        },
        Annotation::Blur { left, top, width, height, radius } => Annotation::Blur {
            left: left.saturating_sub(rect.x as u32),
            top: top.saturating_sub(rect.y as u32),
            width: *width, height: *height, radius: *radius,
        },
    }
}

impl EditStack {
    pub fn applied_len(&self) -> usize {
        self.cursor
    }

    pub fn undo(&mut self) -> bool {
        if self.cursor == 0 { return false; }
        self.cursor -= 1;
        true
    }

    pub fn redo(&mut self) -> bool {
        if self.cursor == self.patches.len() { return false; }
        self.cursor += 1;
        true
    }

    pub fn clear(&mut self) {
        for patch in self.patches.drain(..) {
            let _ = fs::remove_file(patch.path);
        }
        self.cursor = 0;
    }

    pub fn apply(&mut self, store: &TileStore, command: Annotation) -> io::Result<()> {
        let rect = bounds(&command, store.canvas())?;
        let source = store.render_edit_region(self, rect)?;
        let translated = shift_annotation(&command, rect);
        let after = shot_image::composite::composite(&source, &[translated])
            .map_err(invalid)?;
        let path = store.working_dir().join(format!(
            ".edit-{}-{}.rgba",
            std::process::id(),
            NEXT_EDIT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let written = (|| {
            let mut file = options.open(&path)?;
            file.write_all(&after.rgba)?;
            file.sync_all()
        })();
        if let Err(error) = written {
            let _ = fs::remove_file(path);
            return Err(error);
        }

        for old in self.patches.drain(self.cursor..) {
            let _ = fs::remove_file(old.path);
        }
        self.patches.push(Patch { rect, path });
        self.cursor += 1;
        Ok(())
    }

    pub(crate) fn open_readers(&self) -> io::Result<Vec<File>> {
        self.patches[..self.cursor]
            .iter()
            .map(|patch| File::open(&patch.path))
            .collect()
    }

    /// Replace target pixels with the final rendered patch, never alpha-blend
    /// twice over earlier annotations. Existing file readers are reused.
    pub(crate) fn apply_to_row(
        &self,
        y: u32,
        row: &mut [u8],
        readers: &mut [File],
    ) -> io::Result<()> {
        if readers.len() != self.cursor {
            return Err(invalid("invalid edit reader count"));
        }
        for (patch, reader) in self.patches[..self.cursor].iter().zip(readers.iter_mut()) {
            let y0 = patch.rect.y as u32;
            if y < y0 || y >= y0 + patch.rect.height {
                continue;
            }
            let offset = u64::from(y - y0) * u64::from(patch.rect.width) * 4;
            let start = patch.rect.x as usize * 4;
            let end = start + patch.rect.width as usize * 4;
            reader.seek(SeekFrom::Start(offset))?;
            reader.read_exact(&mut row[start..end])?;
        }
        Ok(())
    }
}
