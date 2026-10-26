//! Cheap PNG-output preview, computed directly from a bounded source crop.
//! This intentionally never allocates a full selected-area RGBA copy.
use shot_core::{CoreError, RectPx, RgbaFrame};

pub fn png_selection_preview(
    source: &RgbaFrame,
    selected: RectPx,
    radius_px: u32,
    max_side: u32,
) -> Result<RgbaFrame, CoreError> {
    if selected.width == 0 || selected.height == 0 || max_side == 0 {
        return Err(CoreError::EmptyRegion);
    }
    if selected.x < 0 || selected.y < 0 {
        return Err(CoreError::OutOfBounds);
    }
    let x = selected.x as u64;
    let y = selected.y as u64;
    if x + u64::from(selected.width) > u64::from(source.width)
        || y + u64::from(selected.height) > u64::from(source.height)
    {
        return Err(CoreError::OutOfBounds);
    }
    let source_len = u64::from(source.width)
        .checked_mul(u64::from(source.height))
        .and_then(|size| size.checked_mul(4))
        .ok_or(CoreError::Overflow)?;
    if usize::try_from(source_len).map_err(|_| CoreError::Overflow)? != source.rgba.len() {
        return Err(CoreError::PixelSizeMismatch);
    }

    let limit = max_side.min(512);
    let longest = selected.width.max(selected.height);
    let output_width = ((u64::from(selected.width) * u64::from(limit) + u64::from(longest / 2))
        / u64::from(longest))
    .clamp(1, u64::from(selected.width)) as u32;
    let output_height = ((u64::from(selected.height) * u64::from(limit) + u64::from(longest / 2))
        / u64::from(longest))
    .clamp(1, u64::from(selected.height)) as u32;
    let mut pixels = Vec::with_capacity(output_width as usize * output_height as usize * 4);
    for preview_y in 0..output_height {
        let source_y =
            y + u64::from(preview_y) * u64::from(selected.height) / u64::from(output_height);
        for preview_x in 0..output_width {
            let source_x =
                x + u64::from(preview_x) * u64::from(selected.width) / u64::from(output_width);
            let offset = ((source_y * u64::from(source.width) + source_x) * 4) as usize;
            pixels.extend_from_slice(&source.rgba[offset..offset + 4]);
        }
    }
    let mut preview = RgbaFrame::new(output_width, output_height, pixels)?;
    let preview_radius = ((f64::from(radius_px) * f64::from(output_width))
        / f64::from(selected.width))
    .round()
    .clamp(0.0, f64::from(output_width.min(output_height) / 2)) as u32;
    crate::rounded_alpha(&mut preview, preview_radius);
    Ok(preview)
}
