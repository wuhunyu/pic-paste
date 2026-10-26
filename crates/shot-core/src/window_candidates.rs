//! Defensive window-frame mapping for the snapshot captured from one display.
//! All exported rectangles are physical source pixels, not desktop coordinates.
use crate::RectPx;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowBounds {
    pub id: u64,
    pub origin_pt: (f64, f64),
    pub size_pt: (f64, f64),
    pub layer: i32,
    pub on_screen: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowCandidate {
    pub id: u64,
    pub rect: RectPx,
}

/// Keep input ordering (ScreenCaptureKit usually enumerates front-to-back),
/// so the first containing candidate is preferred when windows overlap.
pub fn candidates_for_capture(
    display_origin_pt: (f64, f64),
    display_size_pt: (f64, f64),
    capture_size_px: (u32, u32),
    windows: &[WindowBounds],
) -> Vec<WindowCandidate> {
    let (origin_x, origin_y) = display_origin_pt;
    let (width_pt, height_pt) = display_size_pt;
    let (width_px, height_px) = capture_size_px;
    let valid = [origin_x, origin_y, width_pt, height_pt]
        .iter()
        .all(|v| v.is_finite())
        && width_pt > 0.0
        && height_pt > 0.0
        && width_px > 0
        && height_px > 0
        && width_px <= i32::MAX as u32
        && height_px <= i32::MAX as u32
        && (origin_x + width_pt).is_finite()
        && (origin_y + height_pt).is_finite();
    if !valid {
        return Vec::new();
    }
    let right_pt = origin_x + width_pt;
    let bottom_pt = origin_y + height_pt;
    let sx = f64::from(width_px) / width_pt;
    let sy = f64::from(height_px) / height_pt;
    if !sx.is_finite() || !sy.is_finite() {
        return Vec::new();
    }
    let mut output: Vec<WindowCandidate> = Vec::new();
    for w in windows {
        if !w.on_screen || w.layer != 0 || w.id == 0 || output.iter().any(|c| c.id == w.id) {
            continue;
        }
        let (x, y) = w.origin_pt;
        let (width, height) = w.size_pt;
        if ![x, y, width, height].iter().all(|v| v.is_finite())
            || width <= 0.0
            || height <= 0.0
            || !(x + width).is_finite()
            || !(y + height).is_finite()
        {
            continue;
        }
        let left = x.max(origin_x);
        let top = y.max(origin_y);
        let right = (x + width).min(right_pt);
        let bottom = (y + height).min(bottom_pt);
        if left >= right || top >= bottom {
            continue;
        }
        let x0 = ((left - origin_x) * sx).floor().clamp(0.0, f64::from(width_px)) as u32;
        let y0 = ((top - origin_y) * sy).floor().clamp(0.0, f64::from(height_px)) as u32;
        let x1 = ((right - origin_x) * sx).ceil().clamp(0.0, f64::from(width_px)) as u32;
        let y1 = ((bottom - origin_y) * sy).ceil().clamp(0.0, f64::from(height_px)) as u32;
        let w_px = x1.saturating_sub(x0);
        let h_px = y1.saturating_sub(y0);
        if w_px < 24 || h_px < 24 {
            continue;
        }
        output.push(WindowCandidate {
            id: w.id,
            rect: RectPx {
                x: x0 as i32,
                y: y0 as i32,
                width: w_px,
                height: h_px,
            },
        });
    }
    output
}

pub fn candidate_at(list: &[WindowCandidate], point: (i32, i32)) -> Option<WindowCandidate> {
    list.iter()
        .copied()
        .find(|candidate| candidate.rect.contains(point.0, point.1))
}
