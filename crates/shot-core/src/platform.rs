use crate::{RectPx, RgbaFrame};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DisplayId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PinId(pub u64);
#[derive(Clone, Debug)]
pub struct CapturedFrame { pub display_id: DisplayId, pub origin_px: (i32, i32), pub image: RgbaFrame }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlatformError { PermissionDenied, Unavailable, InvalidInput, Internal(String) }
pub trait ScreenCapture: Send {
    fn capture_display(&mut self, display: DisplayId) -> Result<CapturedFrame, PlatformError>;
    fn capture_region(&mut self, display: DisplayId, region: RectPx) -> Result<CapturedFrame, PlatformError>;
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
pub enum ScrollAxis { Vertical, Horizontal }
pub trait ScrollInput {
    fn scroll_by(&mut self, axis: ScrollAxis, distance_px: i32) -> Result<(), PlatformError>;
}
