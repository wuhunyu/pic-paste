use shot_core::{CoreError, RectPx, RgbaFrame};
use std::fs::{self, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Crop an RGBA frame using image-local, half-open physical-pixel coordinates.
/// This is intentionally independent of desktop logical-point geometry.
pub fn crop_rgba(frame: &RgbaFrame, rect: RectPx) -> Result<RgbaFrame, CoreError> {
    if rect.width == 0 || rect.height == 0 {
        return Err(CoreError::EmptyRegion);
    }
    if rect.x < 0 || rect.y < 0 {
        return Err(CoreError::OutOfBounds);
    }
    let source_bytes = u64::from(frame.width)
        .checked_mul(u64::from(frame.height))
        .and_then(|n| n.checked_mul(4))
        .ok_or(CoreError::Overflow)?;
    if usize::try_from(source_bytes).map_err(|_| CoreError::Overflow)? != frame.rgba.len() {
        return Err(CoreError::PixelSizeMismatch);
    }
    let x = u64::try_from(rect.x).map_err(|_| CoreError::OutOfBounds)?;
    let y = u64::try_from(rect.y).map_err(|_| CoreError::OutOfBounds)?;
    let right = x
        .checked_add(u64::from(rect.width))
        .ok_or(CoreError::Overflow)?;
    let bottom = y
        .checked_add(u64::from(rect.height))
        .ok_or(CoreError::Overflow)?;
    if right > u64::from(frame.width) || bottom > u64::from(frame.height) {
        return Err(CoreError::OutOfBounds);
    }
    let row_len = usize::try_from(u64::from(rect.width) * 4).map_err(|_| CoreError::Overflow)?;
    let total_len = row_len
        .checked_mul(rect.height as usize)
        .ok_or(CoreError::Overflow)?;
    let mut pixels = Vec::with_capacity(total_len);
    for row in y..bottom {
        let offset = row
            .checked_mul(u64::from(frame.width))
            .and_then(|n| n.checked_add(x))
            .and_then(|n| n.checked_mul(4))
            .ok_or(CoreError::Overflow)?;
        let start = usize::try_from(offset).map_err(|_| CoreError::Overflow)?;
        let end = start.checked_add(row_len).ok_or(CoreError::Overflow)?;
        pixels.extend_from_slice(frame.rgba.get(start..end).ok_or(CoreError::OutOfBounds)?);
    }
    RgbaFrame::new(rect.width, rect.height, pixels)
}

/// Publish a completed PNG without ever replacing another file.
/// A unique 0600 .part is created next to the destination and synchronized.
/// A hard link makes the complete data visible in one atomic filesystem step,
/// failing with AlreadyExists if another process won the destination name.
pub fn atomic_save_png(frame: &RgbaFrame, path: &Path) -> io::Result<()> {
    if path.extension().and_then(|s| s.to_str()) != Some("png") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "PNG files only",
        ));
    }
    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid filename"))?;
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp = path.with_file_name(format!(
        ".{filename}.{}.{unique}.{sequence}.part",
        std::process::id()
    ));

    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let result = (|| {
        let file = options.open(&temp)?;
        let mut writer = BufWriter::new(file);
        crate::encode_png(frame, &mut writer).map_err(io::Error::other)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
        drop(writer);
        // Same-volume, no-clobber atomic publication. Works on APFS.
        fs::hard_link(&temp, path)?;
        Ok(())
    })();
    let _ = fs::remove_file(&temp);
    result
}
