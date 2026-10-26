use crate::{CoreError, RectPx};
/// Convert a drag in physical display coordinates to an inclusive-pixel rectangle.
///
/// The returned dimensions are pixel counts. Endpoints are allowed to be dragged backwards.
pub fn from_drag(start: (i32, i32), end: (i32, i32)) -> Result<RectPx, CoreError> {
    let left = start.0.min(end.0);
    let top = start.1.min(end.1);
    let width = (i64::from(start.0) - i64::from(end.0)).unsigned_abs() + 1;
    let height = (i64::from(start.1) - i64::from(end.1)).unsigned_abs() + 1;
    Ok(RectPx {
        x: left,
        y: top,
        width: u32::try_from(width).map_err(|_| CoreError::Overflow)?,
        height: u32::try_from(height).map_err(|_| CoreError::Overflow)?,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn includes_single_pixel_and_reversed_negative_drag() {
        assert_eq!(from_drag((-8, 20), (-8, 20)).unwrap(),
            RectPx { x: -8, y: 20, width: 1, height: 1 });
        assert_eq!(from_drag((100, 80), (-10, -20)).unwrap(),
            RectPx { x: -10, y: -20, width: 111, height: 101 });
    }
}
