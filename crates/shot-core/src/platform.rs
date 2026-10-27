use crate::{RectPx, RgbaFrame};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DisplayId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PinId(pub u64);
#[derive(Clone, Debug)]
pub struct CapturedFrame {
    pub display_id: DisplayId,
    /// Physical pixels relative to the captured display's top-left corner.
    /// Never multiply the desktop logical-point origin by a Retina scale.
    pub origin_px: (i32, i32),
    pub image: RgbaFrame,
}

impl CapturedFrame {
    /// The pixel image from a full SCDisplay capture always starts at (0, 0)
    /// in that display's own physical pixel coordinate space.
    pub fn for_display(display_id: DisplayId, image: RgbaFrame) -> Self {
        Self {
            display_id,
            origin_px: (0, 0),
            image,
        }
    }

    /// Validated origin of a cropped region, still in display-local pixels.
    /// Rejection must not mutate or partially crop the original image.
    pub fn region_origin_px(&self, region: RectPx) -> Result<(i32, i32), PlatformError> {
        if region.width == 0 || region.height == 0 {
            return Err(PlatformError::InvalidInput);
        }
        let x = u32::try_from(region.x).map_err(|_| PlatformError::InvalidInput)?;
        let y = u32::try_from(region.y).map_err(|_| PlatformError::InvalidInput)?;
        let right = x
            .checked_add(region.width)
            .ok_or(PlatformError::InvalidInput)?;
        let bottom = y
            .checked_add(region.height)
            .ok_or(PlatformError::InvalidInput)?;
        if right > self.image.width || bottom > self.image.height {
            return Err(PlatformError::InvalidInput);
        }
        let x = self
            .origin_px
            .0
            .checked_add(region.x)
            .ok_or(PlatformError::InvalidInput)?;
        let y = self
            .origin_px
            .1
            .checked_add(region.y)
            .ok_or(PlatformError::InvalidInput)?;
        Ok((x, y))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlatformError {
    PermissionDenied,
    Unavailable,
    InvalidInput,
    Internal(String),
}
pub trait ScreenCapture: Send {
    fn capture_display(&mut self, display: DisplayId) -> Result<CapturedFrame, PlatformError>;
    fn capture_region(
        &mut self,
        display: DisplayId,
        region: RectPx,
    ) -> Result<CapturedFrame, PlatformError>;
}
pub trait WindowController {
    fn create_pin(&mut self, pin: PinId, image: &RgbaFrame) -> Result<(), PlatformError>;
    fn set_passthrough(&mut self, pin: PinId, enabled: bool) -> Result<(), PlatformError>;
    fn close_pin(&mut self, pin: PinId) -> Result<(), PlatformError>;
}
pub trait ClipboardService {
    fn copy_image(&mut self, frame: &RgbaFrame) -> Result<(), PlatformError>;
    fn copy_text(&mut self, text: &str) -> Result<(), PlatformError>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollAxis {
    Vertical,
    Horizontal,
}
pub trait ScrollInput {
    fn scroll_by(&mut self, axis: ScrollAxis, distance_px: i32) -> Result<(), PlatformError>;
}
