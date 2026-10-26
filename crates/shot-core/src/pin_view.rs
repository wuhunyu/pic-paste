//! Convert pointer coordinates within a scaled egui pin image into native
//! source pixels for offline OCR hit testing. No texture resampling required.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PinImageView {
    pub image_width: u32,
    pub image_height: u32,
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

/// Return physical source-image coordinates only when a click is strictly
/// inside the image, excluding pin toolbar/borders and NaN/Inf input.
pub fn map_pin_click(view: PinImageView, point: (f32, f32)) -> Option<(f32, f32)> {
    let values = [
        view.left,
        view.top,
        view.width,
        view.height,
        point.0,
        point.1,
    ];
    if view.image_width == 0
        || view.image_height == 0
        || view.width <= 0.0
        || view.height <= 0.0
        || !values.iter().all(|value| value.is_finite())
    {
        return None;
    }
    let u = (point.0 - view.left) / view.width;
    let v = (point.1 - view.top) / view.height;
    if !u.is_finite() || !v.is_finite() || !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
        return None;
    }
    Some((
        (u * view.image_width as f32).min(view.image_width.saturating_sub(1) as f32),
        (v * view.image_height as f32).min(view.image_height.saturating_sub(1) as f32),
    ))
}
