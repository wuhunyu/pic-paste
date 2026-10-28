//! Edit a native NSPanel selection without changing captured RGBA pixels.
//! Pointer positions are panel-local logical points. Rectangles are exact
//! target-display-local physical pixels and never global desktop points.

use crate::RectPx;
use crate::display::OverlayPlacement;
use crate::native_snapshot::NativeSnapshot;

const GRIP_RADIUS_PT: f32 = 9.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionGrip {
    Create,
    Move,
    NorthWest,
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
}

#[derive(Clone, Copy, Debug)]
struct Gesture {
    grip: SelectionGrip,
    anchor: (f32, f32),
    current: (f32, f32),
    original: Option<RectPx>,
}

/// A local selection may be re-created, moved and resized many times.
/// Only `confirmed` returns a committed region; an active drag cannot be
/// published to the parent editor before the user explicitly confirms.
#[derive(Clone, Debug)]
pub struct NativeSelectionEdit {
    points: (f32, f32),
    pixels: (u32, u32),
    selected: Option<RectPx>,
    gesture: Option<Gesture>,
}

impl NativeSelectionEdit {
    pub fn new(placement: OverlayPlacement, pixels: (u32, u32)) -> Result<Self, String> {
        NativeSnapshot::validate_dimensions(pixels.0, pixels.1)?;
        Ok(Self {
            points: placement.size_pt(),
            pixels,
            selected: None,
            gesture: None,
        })
    }

    fn valid_point(&self, point: (f32, f32)) -> bool {
        point.0.is_finite()
            && point.1.is_finite()
            && point.0 >= 0.0
            && point.1 >= 0.0
            && point.0 <= self.points.0
            && point.1 <= self.points.1
    }

    fn clamp_point(&self, point: (f32, f32)) -> Option<(f32, f32)> {
        if !point.0.is_finite() || !point.1.is_finite() {
            return None;
        }
        Some((
            point.0.clamp(0.0, self.points.0),
            point.1.clamp(0.0, self.points.1),
        ))
    }

    fn scales(&self) -> (f64, f64) {
        (
            f64::from(self.pixels.0) / f64::from(self.points.0),
            f64::from(self.pixels.1) / f64::from(self.points.1),
        )
    }

    fn rect_points(&self, rect: RectPx) -> (f32, f32, f32, f32) {
        let (sx, sy) = self.scales();
        (
            (f64::from(rect.x) / sx) as f32,
            (f64::from(rect.y) / sy) as f32,
            (f64::from(rect.width) / sx) as f32,
            (f64::from(rect.height) / sy) as f32,
        )
    }

    /// The live rect includes in-progress movement/resizing. This is not a
    /// confirmation and does not trigger the PPSEL001 parent response.
    pub fn current_rect(&self) -> Option<RectPx> {
        self.gesture
            .and_then(|gesture| self.candidate(gesture))
            .or(self.selected)
    }

    pub fn preview_rect_points(&self) -> Option<(f32, f32, f32, f32)> {
        self.current_rect().map(|rect| self.rect_points(rect))
    }

    pub fn grip_centers_points(&self) -> Option<[(SelectionGrip, (f32, f32)); 8]> {
        let (x, y, width, height) = self.preview_rect_points()?;
        let mid_x = x + width * 0.5;
        let mid_y = y + height * 0.5;
        let right = x + width;
        let bottom = y + height;
        Some([
            (SelectionGrip::NorthWest, (x, y)),
            (SelectionGrip::North, (mid_x, y)),
            (SelectionGrip::NorthEast, (right, y)),
            (SelectionGrip::East, (right, mid_y)),
            (SelectionGrip::SouthEast, (right, bottom)),
            (SelectionGrip::South, (mid_x, bottom)),
            (SelectionGrip::SouthWest, (x, bottom)),
            (SelectionGrip::West, (x, mid_y)),
        ])
    }

    pub fn hit_test(&self, point: (f32, f32)) -> Option<SelectionGrip> {
        if !self.valid_point(point) {
            return None;
        }
        if let Some(handles) = self.grip_centers_points() {
            for (grip, center) in handles {
                if (point.0 - center.0).abs() <= GRIP_RADIUS_PT
                    && (point.1 - center.1).abs() <= GRIP_RADIUS_PT
                {
                    return Some(grip);
                }
            }
        }
        if let Some((x, y, width, height)) = self.selected.map(|rect| self.rect_points(rect))
            && point.0 >= x
            && point.0 <= x + width
            && point.1 >= y
            && point.1 <= y + height
        {
            return Some(SelectionGrip::Move);
        }
        Some(SelectionGrip::Create)
    }

    /// Begin a new gesture. Clicking outside an existing region creates a
    /// replacement only if its eventual drag has positive area.
    pub fn begin(&mut self, point: (f32, f32)) -> Option<SelectionGrip> {
        let grip = self.hit_test(point)?;
        self.gesture = Some(Gesture {
            grip,
            anchor: point,
            current: point,
            original: self.selected,
        });
        Some(grip)
    }

    pub fn update(&mut self, point: (f32, f32)) {
        let next = self.clamp_point(point);
        if let (Some(drag), Some(next)) = (&mut self.gesture, next) {
            drag.current = next;
        }
    }

    pub fn end(&mut self, point: (f32, f32)) -> Option<RectPx> {
        self.update(point);
        if let Some(gesture) = self.gesture.take()
            && let Some(candidate) = self.candidate(gesture)
        {
            self.selected = Some(candidate);
        }
        self.selected
    }

    /// Cancels only the live gesture, restoring the last committed rectangle.
    pub fn abort(&mut self) {
        self.gesture = None;
    }

    pub fn reset(&mut self) {
        self.abort();
        self.selected = None;
    }

    pub fn confirmed(&self) -> Option<RectPx> {
        self.gesture.is_none().then_some(self.selected).flatten()
    }

    fn candidate(&self, gesture: Gesture) -> Option<RectPx> {
        let (sx, sy) = self.scales();
        let (cx, cy) = gesture.current;
        let (ax, ay) = gesture.anchor;
        if gesture.grip == SelectionGrip::Create {
            // Convert both endpoints outward to the nearest physical pixel.
            if ax == cx || ay == cy {
                return None;
            }
            let left = (f64::from(ax.min(cx)) * sx).floor() as i32;
            let top = (f64::from(ay.min(cy)) * sy).floor() as i32;
            let right = (f64::from(ax.max(cx)) * sx).ceil() as i32;
            let bottom = (f64::from(ay.max(cy)) * sy).ceil() as i32;
            return Self::build_rect(left, top, right, bottom);
        }
        let original = gesture.original?;
        let left = original.x;
        let top = original.y;
        let right = left + original.width as i32;
        let bottom = top + original.height as i32;
        if gesture.grip == SelectionGrip::Move {
            let dx = (f64::from(cx - ax) * sx).round() as i32;
            let dy = (f64::from(cy - ay) * sy).round() as i32;
            let dx = dx.clamp(-left, self.pixels.0 as i32 - right);
            let dy = dy.clamp(-top, self.pixels.1 as i32 - bottom);
            return Some(RectPx {
                x: left + dx,
                y: top + dy,
                ..original
            });
        }
        let px = (f64::from(cx) * sx).round() as i32;
        let py = (f64::from(cy) * sy).round() as i32;
        let mut new_left = left;
        let mut new_top = top;
        let mut new_right = right;
        let mut new_bottom = bottom;
        if matches!(
            gesture.grip,
            SelectionGrip::NorthWest | SelectionGrip::West | SelectionGrip::SouthWest
        ) {
            new_left = px.clamp(0, right - 1);
        }
        if matches!(
            gesture.grip,
            SelectionGrip::NorthEast | SelectionGrip::East | SelectionGrip::SouthEast
        ) {
            new_right = px.clamp(left + 1, self.pixels.0 as i32);
        }
        if matches!(
            gesture.grip,
            SelectionGrip::NorthWest | SelectionGrip::North | SelectionGrip::NorthEast
        ) {
            new_top = py.clamp(0, bottom - 1);
        }
        if matches!(
            gesture.grip,
            SelectionGrip::SouthWest | SelectionGrip::South | SelectionGrip::SouthEast
        ) {
            new_bottom = py.clamp(top + 1, self.pixels.1 as i32);
        }
        Self::build_rect(new_left, new_top, new_right, new_bottom)
    }

    fn build_rect(left: i32, top: i32, right: i32, bottom: i32) -> Option<RectPx> {
        if left < 0 || top < 0 || right <= left || bottom <= top {
            return None;
        }
        Some(RectPx {
            x: left,
            y: top,
            width: (right - left) as u32,
            height: (bottom - top) as u32,
        })
    }
}
