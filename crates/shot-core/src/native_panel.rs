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

/// An observable decision for the optional native panel renderer.
///
/// `Suspended` skips rendering while AppKit temporarily reports a zero-sized
/// content view. The surface remains owned by the panel and is reconfigured
/// when a valid size is reported again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelSurfaceUpdate {
    Unchanged,
    Reconfigure(PanelSurfaceSize),
    Suspended,
    Closed,
}

/// Tracks the last accepted physical surface size without depending on AppKit.
/// Changes after `close` must never recreate a drawable over a dead NSView.
#[derive(Clone, Copy, Debug)]
pub struct PanelSurfaceLifecycle {
    configured: Option<PanelSurfaceSize>,
    closed: bool,
}

impl PanelSurfaceLifecycle {
    pub fn new(configured: PanelSurfaceSize) -> Self {
        Self {
            configured: Some(configured),
            closed: false,
        }
    }

    pub fn configured_size(&self) -> Option<PanelSurfaceSize> {
        self.configured
    }

    pub fn observe(
        &mut self,
        width_pt: f64,
        height_pt: f64,
        backing_scale: f64,
    ) -> Result<PanelSurfaceUpdate, PlatformError> {
        if self.closed {
            return Ok(PanelSurfaceUpdate::Closed);
        }
        // AppKit may report a zero content dimension while the panel is
        // minimized; invalid negative/nonfinite geometry is not suspension.
        if !width_pt.is_finite()
            || width_pt < 0.0
            || !height_pt.is_finite()
            || height_pt < 0.0
            || !backing_scale.is_finite()
            || backing_scale <= 0.0
        {
            return Err(PlatformError::InvalidInput);
        }
        if width_pt == 0.0 || height_pt == 0.0 {
            self.configured = None;
            return Ok(PanelSurfaceUpdate::Suspended);
        }
        // A rejected oversized or invalid physical size must not replace the
        // previous valid configuration.
        let target = PanelSurfaceSize::from_points(width_pt, height_pt, backing_scale)?;
        if self.configured == Some(target) {
            Ok(PanelSurfaceUpdate::Unchanged)
        } else {
            self.configured = Some(target);
            Ok(PanelSurfaceUpdate::Reconfigure(target))
        }
    }

    pub fn close(&mut self) -> PanelSurfaceUpdate {
        self.closed = true;
        self.configured = None;
        PanelSurfaceUpdate::Closed
    }
}
