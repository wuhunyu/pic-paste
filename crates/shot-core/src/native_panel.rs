//! Size validation for an experimental AppKit NSPanel WGPU drawable.
//!
//! Content dimensions are logical **points**. Backing scale determines the
//! physical pixel surface; global desktop origin never participates.
use crate::platform::PlatformError;

pub const MAX_PANEL_SURFACE_EDGE: u32 = 8192;

/// WGPU swapchain dimensions in physical pixels for an AppKit content view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelSurfaceSize {
    pub width: u32,
    pub height: u32,
}

impl PanelSurfaceSize {
    pub fn from_points(
        width_pt: f64,
        height_pt: f64,
        backing_scale: f64,
    ) -> Result<Self, PlatformError> {
        let numbers = [width_pt, height_pt, backing_scale];
        if !numbers.iter().all(|v| v.is_finite() && *v > 0.0) {
            return Err(PlatformError::InvalidInput);
        }

        fn physical_edge(points: f64, scale: f64) -> Result<u32, PlatformError> {
            let physical = (points * scale).round();
            if !physical.is_finite()
                || physical < 1.0
                || physical > f64::from(MAX_PANEL_SURFACE_EDGE)
            {
                return Err(PlatformError::InvalidInput);
            }
            Ok(physical as u32)
        }

        Ok(Self {
            width: physical_edge(width_pt, backing_scale)?,
            height: physical_edge(height_pt, backing_scale)?,
        })
    }
}
