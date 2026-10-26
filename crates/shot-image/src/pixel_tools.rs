//! Bounded physical-pixel sampling for the on-demand screenshot magnifier.
use shot_core::RgbaFrame;

/// Read the exact raw RGBA pixel at an image-local physical coordinate.
pub fn sample_rgba(image: &RgbaFrame, point: (i32, i32)) -> Option<[u8; 4]> {
    if point.0 < 0 || point.1 < 0 || image.width == 0 || image.height == 0 {
        return None;
    }
    let x = u32::try_from(point.0).ok()?;
    let y = u32::try_from(point.1).ok()?;
    if x >= image.width || y >= image.height {
        return None;
    }
    let pixels = u64::from(image.width).checked_mul(u64::from(image.height))?;
    let expected = usize::try_from(pixels.checked_mul(4)?).ok()?;
    if expected != image.rgba.len() {
        return None;
    }
    let idx = usize::try_from((u64::from(y) * u64::from(image.width) + u64::from(x)) * 4).ok()?;
    image.rgba.get(idx..idx + 4)?.try_into().ok()
}

/// Extract a small RGBA square, repeating edge pixels rather than showing
/// uninitialized/outside memory. No full-image allocations or timers.
pub fn magnifier_patch(
    image: &RgbaFrame,
    center: (i32, i32),
    side: u32,
) -> Result<RgbaFrame, &'static str> {
    if side == 0 || side > 21 || side.is_multiple_of(2) {
        return Err("magnifier size must be odd and at most 21");
    }
    if sample_rgba(image, center).is_none() {
        return Err("magnifier center outside valid RGBA image");
    }
    let radius = i64::from(side / 2);
    let right = i64::from(image.width) - 1;
    let bottom = i64::from(image.height) - 1;
    let mut rgba = Vec::with_capacity(side as usize * side as usize * 4);
    for y in 0..side {
        for x in 0..side {
            let source_x = (i64::from(center.0) + i64::from(x) - radius).clamp(0, right);
            let source_y = (i64::from(center.1) + i64::from(y) - radius).clamp(0, bottom);
            let idx = (source_y as usize * image.width as usize + source_x as usize) * 4;
            rgba.extend_from_slice(&image.rgba[idx..idx + 4]);
        }
    }
    RgbaFrame::new(side, side, rgba).map_err(|_| "invalid magnifier RGBA")
}
