//! Developer-only native NSPanel selection geometry and fixed-width response.
//! ScreenCaptureKit desktop origins are global **logical points**; selection
//! rectangles are always target-display-local **physical source pixels**.
use std::io::{Read, Write};

use crate::RectPx;
use crate::display::OverlayPlacement;
use crate::native_snapshot::NativeSnapshot;
use crate::platform::DisplayId;

const REPLY_MAGIC: &[u8; 8] = b"PPSEL001";
const REPLY_BYTES: usize = 33;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppKitSelectionFrame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl AppKitSelectionFrame {
    /// Convert a CoreGraphics top-left desktop origin to AppKit bottom-left.
    /// `primary_top_y` is the global maxY of the screen with the menu bar.
    /// No per-screen Retina multiplier is applied to a global origin.
    pub fn from_quartz(
        placement: OverlayPlacement,
        primary_top_y: f64,
    ) -> Result<Self, String> {
        let (x, y) = placement.origin_pt();
        let (width, height) = placement.size_pt();
        let frame = Self {
            x: f64::from(x),
            y: primary_top_y - f64::from(y) - f64::from(height),
            width: f64::from(width),
            height: f64::from(height),
        };
        if ![frame.x, frame.y, frame.width, frame.height]
            .iter()
            .all(|value| value.is_finite())
            || frame.width <= 0.0
            || frame.height <= 0.0
        {
            return Err(String::from("invalid AppKit selection frame geometry"));
        }
        Ok(frame)
    }
}

/// One drag gesture inside a native panel, in top-left view-local logical points.
#[derive(Clone, Debug)]
pub struct NativeSelection {
    size_pt: (f32, f32),
    size_px: (u32, u32),
    anchor: Option<(f32, f32)>,
    current: Option<(f32, f32)>,
}

impl NativeSelection {
    pub fn new(placement: OverlayPlacement, pixels: (u32, u32)) -> Result<Self, String> {
        NativeSnapshot::validate_dimensions(pixels.0, pixels.1)?;
        Ok(Self {
            size_pt: placement.size_pt(),
            size_px: pixels,
            anchor: None,
            current: None,
        })
    }

    fn is_inside(&self, point: (f32, f32)) -> bool {
        point.0.is_finite()
            && point.1.is_finite()
            && point.0 >= 0.0
            && point.1 >= 0.0
            && point.0 <= self.size_pt.0
            && point.1 <= self.size_pt.1
    }

    fn clamp(&self, point: (f32, f32)) -> Option<(f32, f32)> {
        if !point.0.is_finite() || !point.1.is_finite() {
            return None;
        }
        Some((
            point.0.clamp(0.0, self.size_pt.0),
            point.1.clamp(0.0, self.size_pt.1),
        ))
    }

    pub fn press(&mut self, point_pt: (f32, f32)) -> bool {
        if !self.is_inside(point_pt) {
            return false;
        }
        self.anchor = Some(point_pt);
        self.current = Some(point_pt);
        true
    }

    pub fn drag(&mut self, point_pt: (f32, f32)) {
        if self.anchor.is_some()
            && let Some(point) = self.clamp(point_pt)
        {
            self.current = Some(point);
        }
    }

    /// Preview coordinates are view-local points, never global desktop pixels.
    pub fn preview_rect_points(&self) -> Option<(f32, f32, f32, f32)> {
        let (ax, ay) = self.anchor?;
        let (cx, cy) = self.current?;
        Some((
            ax.min(cx),
            ay.min(cy),
            (ax - cx).abs(),
            (ay - cy).abs(),
        ))
    }

    pub fn release(&mut self, point_pt: (f32, f32)) -> Option<RectPx> {
        self.drag(point_pt);
        let rect = self.preview_rect_points();
        self.anchor = None;
        self.current = None;
        let (left, top, width, height) = rect?;
        if width <= 0.0 || height <= 0.0 {
            return None;
        }
        let right = left + width;
        let bottom = top + height;
        let scale_x = f64::from(self.size_px.0) / f64::from(self.size_pt.0);
        let scale_y = f64::from(self.size_px.1) / f64::from(self.size_pt.1);
        // Outward rounding preserves even subpoint one-pixel rectangles.
        let x0 = (f64::from(left) * scale_x).floor() as i32;
        let y0 = (f64::from(top) * scale_y).floor() as i32;
        let x1 = (f64::from(right) * scale_x).ceil() as i32;
        let y1 = (f64::from(bottom) * scale_y).ceil() as i32;
        let x0 = x0.clamp(0, self.size_px.0 as i32);
        let y0 = y0.clamp(0, self.size_px.1 as i32);
        let x1 = x1.clamp(0, self.size_px.0 as i32);
        let y1 = y1.clamp(0, self.size_px.1 as i32);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        Some(RectPx {
            x: x0,
            y: y0,
            width: (x1 - x0) as u32,
            height: (y1 - y0) as u32,
        })
    }
}

/// Exactly one 33-byte binary reply on the child's stdout. Diagnostic text
/// must use stderr: stdout is reserved for this authenticated child channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeSelectionReply {
    Selected { display: DisplayId, rect: RectPx },
    Cancelled { display: DisplayId },
}

impl NativeSelectionReply {
    pub fn validated_in(
        &self,
        expected_display: DisplayId,
        pixels: (u32, u32),
    ) -> Result<Option<RectPx>, String> {
        NativeSnapshot::validate_dimensions(pixels.0, pixels.1)?;
        let display = match self {
            Self::Selected { display, .. } | Self::Cancelled { display } => *display,
        };
        if display != expected_display {
            return Err(String::from("native selection reply display does not match capture"));
        }
        let Self::Selected { rect, .. } = self else {
            return Ok(None);
        };
        if rect.x < 0 || rect.y < 0 || rect.width == 0 || rect.height == 0 {
            return Err(String::from("native selection rectangle is empty or negative"));
        }
        let right = (rect.x as u32)
            .checked_add(rect.width)
            .ok_or("native selection rectangle x overflow")?;
        let bottom = (rect.y as u32)
            .checked_add(rect.height)
            .ok_or("native selection rectangle y overflow")?;
        if right > pixels.0 || bottom > pixels.1 {
            return Err(String::from("native selection rectangle exceeds capture bounds"));
        }
        Ok(Some(*rect))
    }

    pub fn write_to<W: Write>(&self, sink: &mut W) -> Result<(), String> {
        let mut buffer = [0u8; REPLY_BYTES];
        buffer[..8].copy_from_slice(REPLY_MAGIC);
        let (display, rectangle) = match *self {
            Self::Selected { display, rect } => {
                if rect.x < 0 || rect.y < 0 || rect.width == 0 || rect.height == 0 {
                    return Err(String::from("invalid native selection before writing reply"));
                }
                buffer[8] = 1;
                (display, Some(rect))
            }
            Self::Cancelled { display } => (display, None),
        };
        buffer[9..17].copy_from_slice(&display.0.to_le_bytes());
        if let Some(rect) = rectangle {
            buffer[17..21].copy_from_slice(&rect.x.to_le_bytes());
            buffer[21..25].copy_from_slice(&rect.y.to_le_bytes());
            buffer[25..29].copy_from_slice(&rect.width.to_le_bytes());
            buffer[29..33].copy_from_slice(&rect.height.to_le_bytes());
        }
        sink.write_all(&buffer).map_err(|error| error.to_string())
    }

    pub fn read_from<R: Read>(source: &mut R) -> Result<Self, String> {
        let mut data = [0u8; REPLY_BYTES];
        source.read_exact(&mut data).map_err(|error| error.to_string())?;
        if &data[..8] != REPLY_MAGIC {
            return Err(String::from("invalid native selection reply magic/version"));
        }
        let display = DisplayId(u64::from_le_bytes(
            data[9..17].try_into().expect("fixed display-id field"),
        ));
        let reply = match data[8] {
            0 => {
                if data[17..33].iter().any(|value| *value != 0) {
                    return Err(String::from("cancelled native selection carried unexpected rect"));
                }
                Self::Cancelled { display }
            }
            1 => {
                let rect = RectPx {
                    x: i32::from_le_bytes(data[17..21].try_into().expect("fixed x field")),
                    y: i32::from_le_bytes(data[21..25].try_into().expect("fixed y field")),
                    width: u32::from_le_bytes(data[25..29].try_into().expect("fixed width field")),
                    height: u32::from_le_bytes(data[29..33].try_into().expect("fixed height field")),
                };
                if rect.x < 0 || rect.y < 0 || rect.width == 0 || rect.height == 0 {
                    return Err(String::from("invalid native selection rectangle"));
                }
                Self::Selected { display, rect }
            }
            _ => return Err(String::from("unknown native selection reply status")),
        };
        let mut trailing = [0u8; 1];
        if source.read(&mut trailing).map_err(|error| error.to_string())? != 0 {
            return Err(String::from("trailing bytes after native selection reply"));
        }
        Ok(reply)
    }
}
