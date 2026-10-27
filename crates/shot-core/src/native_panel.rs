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

/// Logical coordinates relative to the top-left corner of an AppKit content
/// view, suitable for egui input. Never uses desktop-global screen pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelPointerPoint {
    pub x: f32,
    pub y: f32,
}

impl PanelPointerPoint {
    /// Map a Cocoa NSView-local point (including its `bounds.origin`) into
    /// egui's top-left logical-point space. Points outside the view remain
    /// valid because a mouse release can occur during an out-of-bounds drag.
    pub fn from_view(
        x: f64,
        y: f64,
        origin_x: f64,
        origin_y: f64,
        width_pt: f64,
        height_pt: f64,
        flipped: bool,
    ) -> Result<Self, PlatformError> {
        if ![x, y, origin_x, origin_y, width_pt, height_pt]
            .iter()
            .all(|v| v.is_finite())
            || width_pt <= 0.0
            || height_pt <= 0.0
        {
            return Err(PlatformError::InvalidInput);
        }

        let local_x = x - origin_x;
        let local_y = if flipped {
            y - origin_y
        } else {
            height_pt - (y - origin_y)
        };
        if ![local_x, local_y]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= f64::from(f32::MAX))
        {
            return Err(PlatformError::InvalidInput);
        }
        Ok(Self {
            x: local_x as f32,
            y: local_y as f32,
        })
    }
}
