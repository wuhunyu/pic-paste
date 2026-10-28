//! Source-local non-destructive annotation preview for the opt-in NSPanel.
//!
//! This is deliberately NOT a persisted screenshot edit. Until a separate
//! bounded parent IPC protocol is implemented, native marks must not be
//! included in the existing PPSEL001 selection-only reply.
use crate::display::OverlayPlacement;
use crate::native_snapshot::NativeSnapshot;

pub const MAX_MARKS: usize = 128;
pub const MAX_BRUSH_POINTS: usize = 2048;
pub const MAX_TEXT_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NativeMarkupTool {
    #[default]
    Select,
    Brush,
    Arrow,
    Rectangle,
    Text,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeMark {
    Brush { points: Vec<(u32, u32)> },
    Arrow { from: (u32, u32), to: (u32, u32) },
    Rectangle {
        top_left: (u32, u32),
        bottom_right: (u32, u32),
    },
    Text { at: (u32, u32), content: String },
}

#[derive(Clone, Debug)]
struct MarkGesture {
    tool: NativeMarkupTool,
    start: (u32, u32),
    end: (u32, u32),
    last_local: (f32, f32),
    points: Vec<(u32, u32)>,
}

/// In-memory commands are never applied to screenshot RGBA bytes.
#[derive(Clone, Debug)]
pub struct NativeMarkup {
    logical_points: (f32, f32),
    physical_pixels: (u32, u32),
    tool: NativeMarkupTool,
    marks: Vec<NativeMark>,
    gesture: Option<MarkGesture>,
}

impl NativeMarkup {
    pub fn new(placement: OverlayPlacement, pixels: (u32, u32)) -> Result<Self, String> {
        NativeSnapshot::validate_dimensions(pixels.0, pixels.1)?;
        Ok(Self {
            logical_points: placement.size_pt(),
            physical_pixels: pixels,
            tool: NativeMarkupTool::Select,
            marks: Vec::new(),
            gesture: None,
        })
    }

    pub fn tool(&self) -> NativeMarkupTool {
        self.tool
    }

    pub fn set_tool(&mut self, tool: NativeMarkupTool) {
        self.gesture = None;
        self.tool = tool;
    }

    pub fn visible(&self) -> &[NativeMark] {
        &self.marks
    }

    fn valid_start(&self, pt: (f32, f32)) -> bool {
        pt.0.is_finite()
            && pt.1.is_finite()
            && pt.0 >= 0.0
            && pt.1 >= 0.0
            && pt.0 < self.logical_points.0
            && pt.1 < self.logical_points.1
    }

    fn clamp_local(&self, pt: (f32, f32)) -> Option<(f32, f32)> {
        if !pt.0.is_finite() || !pt.1.is_finite() {
            return None;
        }
        Some((
            pt.0.clamp(0.0, self.logical_points.0),
            pt.1.clamp(0.0, self.logical_points.1),
        ))
    }

    fn physical_point(&self, pt: (f32, f32)) -> Option<(u32, u32)> {
        let (x, y) = self.clamp_local(pt)?;
        let sx = f64::from(self.physical_pixels.0) / f64::from(self.logical_points.0);
        let sy = f64::from(self.physical_pixels.1) / f64::from(self.logical_points.1);
        Some((
            (f64::from(x) * sx)
                .round()
                .clamp(0.0, f64::from(self.physical_pixels.0 - 1)) as u32,
            (f64::from(y) * sy)
                .round()
                .clamp(0.0, f64::from(self.physical_pixels.1 - 1)) as u32,
        ))
    }

    pub fn begin(&mut self, pt: (f32, f32)) -> bool {
        if !self.valid_start(pt)
            || self.marks.len() >= MAX_MARKS
            || matches!(self.tool, NativeMarkupTool::Select | NativeMarkupTool::Text)
        {
            return false;
        }
        let Some(start) = self.physical_point(pt) else {
            return false;
        };
        self.gesture = Some(MarkGesture {
            tool: self.tool,
            start,
            end: start,
            last_local: pt,
            points: if self.tool == NativeMarkupTool::Brush {
                vec![start]
            } else {
                Vec::new()
            },
        });
        true
    }

    pub fn drag(&mut self, pt: (f32, f32)) {
        let Some(clamped) = self.clamp_local(pt) else {
            return;
        };
        let Some(point) = self.physical_point(clamped) else {
            return;
        };
        if let Some(gesture) = self.gesture.as_mut() {
            if gesture.last_local != clamped
                && gesture.tool == NativeMarkupTool::Brush
                && gesture.points.len() < MAX_BRUSH_POINTS
            {
                gesture.points.push(point);
            }
            gesture.last_local = clamped;
            gesture.end = point;
        }
    }

    pub fn draft(&self) -> Option<NativeMark> {
        self.gesture.as_ref().and_then(Self::mark_for_gesture)
    }

    fn mark_for_gesture(gesture: &MarkGesture) -> Option<NativeMark> {
        match gesture.tool {
            NativeMarkupTool::Brush => {
                (gesture.points.len() >= 2).then(|| NativeMark::Brush {
                    points: gesture.points.clone(),
                })
            }
            NativeMarkupTool::Arrow => (gesture.start != gesture.end).then_some(
                NativeMark::Arrow {
                    from: gesture.start,
                    to: gesture.end,
                },
            ),
            NativeMarkupTool::Rectangle => {
                let (a, b) = (gesture.start, gesture.end);
                if a.0 == b.0 || a.1 == b.1 {
                    None
                } else {
                    Some(NativeMark::Rectangle {
                        top_left: (a.0.min(b.0), a.1.min(b.1)),
                        bottom_right: (a.0.max(b.0), a.1.max(b.1)),
                    })
                }
            }
            NativeMarkupTool::Text | NativeMarkupTool::Select => None,
        }
    }

    pub fn end(&mut self, pt: (f32, f32)) -> bool {
        self.drag(pt);
        let Some(gesture) = self.gesture.take() else {
            return false;
        };
        let Some(mark) = Self::mark_for_gesture(&gesture) else {
            return false;
        };
        if self.marks.len() >= MAX_MARKS {
            return false;
        }
        self.marks.push(mark);
        true
    }

    pub fn place_text(&mut self, pt: (f32, f32), text: &str) -> bool {
        if self.tool != NativeMarkupTool::Text
            || !self.valid_start(pt)
            || self.marks.len() >= MAX_MARKS
        {
            return false;
        }
        let content = text.trim();
        if content.is_empty() || content.len() > MAX_TEXT_BYTES {
            return false;
        }
        let Some(at) = self.physical_point(pt) else {
            return false;
        };
        self.marks.push(NativeMark::Text {
            at,
            content: content.to_owned(),
        });
        true
    }

    pub fn undo(&mut self) -> bool {
        self.gesture = None;
        self.marks.pop().is_some()
    }
}
