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
/// A physical-pixel selection resize or move operation locked at MouseDown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    Move,
}

/// Prefer the corners over edges and the interior when hit zones overlap.
pub fn hit_selection_handle(
    rect: RectPx,
    point: (i32, i32),
    tolerance: i32,
) -> Option<ResizeHandle> {
    if rect.width == 0 || rect.height == 0 {
        return None;
    }
    let left = i64::from(rect.x);
    let top = i64::from(rect.y);
    let right = left + i64::from(rect.width) - 1;
    let bottom = top + i64::from(rect.height) - 1;
    let mid_x = (left + right) / 2;
    let mid_y = (top + bottom) / 2;
    let zone = i64::from(tolerance.max(0));
    let px = i64::from(point.0);
    let py = i64::from(point.1);
    let handles = [
        (ResizeHandle::TopLeft, left, top),
        (ResizeHandle::TopRight, right, top),
        (ResizeHandle::BottomRight, right, bottom),
        (ResizeHandle::BottomLeft, left, bottom),
        (ResizeHandle::Top, mid_x, top),
        (ResizeHandle::Right, right, mid_y),
        (ResizeHandle::Bottom, mid_x, bottom),
        (ResizeHandle::Left, left, mid_y),
    ];
    for (handle, x, y) in handles {
        if (px - x).abs() <= zone && (py - y).abs() <= zone {
            return Some(handle);
        }
    }
    if rect.contains(point.0, point.1) {
        Some(ResizeHandle::Move)
    } else {
        None
    }
}

/// Update a selection using the original MouseDown rectangle and delta. Each
/// edge stays in bounds and the selected region never becomes smaller than
/// 1x1 physical pixel, even when dragging backwards.
pub fn adjust_selection(
    rect: RectPx,
    handle: ResizeHandle,
    delta: (i32, i32),
    canvas: (u32, u32),
) -> Result<RectPx, CoreError> {
    let max_x = i64::from(canvas.0);
    let max_y = i64::from(canvas.1);
    let mut left = i64::from(rect.x);
    let mut top = i64::from(rect.y);
    let mut right = left + i64::from(rect.width);
    let mut bottom = top + i64::from(rect.height);
    if max_x == 0
        || max_y == 0
        || rect.width == 0
        || rect.height == 0
        || left < 0
        || top < 0
        || right > max_x
        || bottom > max_y
    {
        return Err(CoreError::OutOfBounds);
    }
    let dx = i64::from(delta.0);
    let dy = i64::from(delta.1);
    if handle == ResizeHandle::Move {
        let width = right - left;
        let height = bottom - top;
        left = (left + dx).clamp(0, max_x - width);
        top = (top + dy).clamp(0, max_y - height);
        right = left + width;
        bottom = top + height;
    } else {
        if matches!(
            handle,
            ResizeHandle::TopLeft | ResizeHandle::Left | ResizeHandle::BottomLeft
        ) {
            left = (left + dx).clamp(0, right - 1);
        }
        if matches!(
            handle,
            ResizeHandle::TopRight | ResizeHandle::Right | ResizeHandle::BottomRight
        ) {
            right = (right + dx).clamp(left + 1, max_x);
        }
        if matches!(
            handle,
            ResizeHandle::TopLeft | ResizeHandle::Top | ResizeHandle::TopRight
        ) {
            top = (top + dy).clamp(0, bottom - 1);
        }
        if matches!(
            handle,
            ResizeHandle::BottomLeft | ResizeHandle::Bottom | ResizeHandle::BottomRight
        ) {
            bottom = (bottom + dy).clamp(top + 1, max_y);
        }
    }
    Ok(RectPx {
        x: i32::try_from(left).map_err(|_| CoreError::Overflow)?,
        y: i32::try_from(top).map_err(|_| CoreError::Overflow)?,
        width: u32::try_from(right - left).map_err(|_| CoreError::Overflow)?,
        height: u32::try_from(bottom - top).map_err(|_| CoreError::Overflow)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn includes_single_pixel_and_reversed_negative_drag() {
        assert_eq!(
            from_drag((-8, 20), (-8, 20)).unwrap(),
            RectPx {
                x: -8,
                y: 20,
                width: 1,
                height: 1
            }
        );
        assert_eq!(
            from_drag((100, 80), (-10, -20)).unwrap(),
            RectPx {
                x: -10,
                y: -20,
                width: 111,
                height: 101
            }
        );
    }
}
