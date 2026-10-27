//! Bounded native-pixel transfer of a trusted long screenshot region.
use shot_core::RectPx;
use std::io;

pub const MAX_TRANSFER_PIXELS: u64 = 4_000_000;
pub const MAX_TRANSFER_SIDE: u32 = 4096;

/// The default is the whole canvas when safe, otherwise its exact top-left crop.
pub fn default_transfer_rect(canvas: (u32, u32)) -> RectPx {
    let width = canvas.0.min(MAX_TRANSFER_SIDE);
    let height = if width == 0 {
        0
    } else {
        canvas
            .1
            .min(MAX_TRANSFER_SIDE)
            .min((MAX_TRANSFER_PIXELS / u64::from(width)) as u32)
    };
    RectPx {
        x: 0,
        y: 0,
        width,
        height,
    }
}

pub fn validate_transfer_rect(rect: RectPx, canvas: (u32, u32)) -> io::Result<()> {
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid native pixel transfer region (4MP / 4096 px per edge)",
        )
    };
    if rect.x < 0
        || rect.y < 0
        || rect.width == 0
        || rect.height == 0
        || rect.width > MAX_TRANSFER_SIDE
        || rect.height > MAX_TRANSFER_SIDE
        || u64::from(rect.width) * u64::from(rect.height) > MAX_TRANSFER_PIXELS
        || u64::from(rect.x as u32) + u64::from(rect.width) > u64::from(canvas.0)
        || u64::from(rect.y as u32) + u64::from(rect.height) > u64::from(canvas.1)
    {
        return Err(invalid());
    }
    Ok(())
}

/// Map the bounded egui image's local logical position to original source pixels.
/// Explicit clamping permits finishing a drag just outside the preview.
pub fn map_preview_point(
    local: (f32, f32),
    displayed: (f32, f32),
    canvas: (u32, u32),
) -> Option<(i32, i32)> {
    if ![local.0, local.1, displayed.0, displayed.1]
        .iter()
        .all(|value| value.is_finite())
        || displayed.0 <= 0.0
        || displayed.1 <= 0.0
        || canvas.0 == 0
        || canvas.1 == 0
    {
        return None;
    }
    let x = (f64::from(local.0) / f64::from(displayed.0) * f64::from(canvas.0))
        .floor()
        .clamp(0.0, f64::from(canvas.0 - 1));
    let y = (f64::from(local.1) / f64::from(displayed.1) * f64::from(canvas.1))
        .floor()
        .clamp(0.0, f64::from(canvas.1 - 1));
    Some((x as i32, y as i32))
}
