//! Deterministic display selection and display-local pixel geometry.
//!
//! Every screen has its own logical bounds and physical pixel density.
//! Never multiply an absolute desktop origin by a particular screen's scale.
use crate::platform::{DisplayId, PlatformError};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DisplaySelection {
    current: Option<DisplayId>,
}

impl DisplaySelection {
    pub fn current(&self) -> Option<DisplayId> {
        self.current
    }

    pub fn resolve(&mut self, available: &[DisplayId]) -> Result<DisplayId, PlatformError> {
        let Some(&first) = available.first() else {
            return Err(PlatformError::Unavailable);
        };
        let selected = self
            .current
            .filter(|id| available.contains(id))
            .unwrap_or(first);
        self.current = Some(selected);
        Ok(selected)
    }

    pub fn advance(&mut self, available: &[DisplayId]) -> Result<DisplayId, PlatformError> {
        if available.is_empty() {
            return Err(PlatformError::Unavailable);
        }
        let index = self
            .current
            .and_then(|id| available.iter().position(|candidate| *candidate == id))
            .map(|previous| (previous + 1) % available.len())
            .unwrap_or_else(|| if available.len() > 1 { 1 } else { 0 });
        let selected = available[index];
        self.current = Some(selected);
        Ok(selected)
    }
}

/// A top-left logical coordinate rectangle for one display and the dimensions
/// of the corresponding native-pixel capture. Global points are not pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayMapping {
    origin_pt: (f64, f64),
    size_pt: (f64, f64),
    size_px: (u32, u32),
}

impl DisplayMapping {
    pub fn new(
        origin_pt: (f64, f64),
        size_pt: (f64, f64),
        size_px: (u32, u32),
    ) -> Result<Self, PlatformError> {
        if ![origin_pt.0, origin_pt.1, size_pt.0, size_pt.1]
            .iter()
            .all(|v| v.is_finite())
            || size_pt.0 <= 0.0
            || size_pt.1 <= 0.0
            || size_px.0 == 0
            || size_px.1 == 0
        {
            return Err(PlatformError::InvalidInput);
        }
        Ok(Self {
            origin_pt,
            size_pt,
            size_px,
        })
    }

    /// Returns a display-local source pixel, or None if the point is outside.
    pub fn pixel_at(&self, global_pt: (f64, f64)) -> Option<(u32, u32)> {
        if !global_pt.0.is_finite() || !global_pt.1.is_finite() {
            return None;
        }
        let local_x = global_pt.0 - self.origin_pt.0;
        let local_y = global_pt.1 - self.origin_pt.1;
        if !local_x.is_finite()
            || !local_y.is_finite()
            || local_x < 0.0
            || local_y < 0.0
            || local_x >= self.size_pt.0
            || local_y >= self.size_pt.1
        {
            return None;
        }
        let x = ((local_x / self.size_pt.0) * f64::from(self.size_px.0)).floor();
        let y = ((local_y / self.size_pt.1) * f64::from(self.size_px.1)).floor();
        Some((
            (x as u32).min(self.size_px.0 - 1),
            (y as u32).min(self.size_px.1 - 1),
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlayPlacement {
    display_id: DisplayId,
    origin_pt: (f32, f32),
    size_pt: (f32, f32),
}

impl OverlayPlacement {
    pub fn new(
        display_id: DisplayId,
        origin_pt: (f64, f64),
        size_pt: (f64, f64),
    ) -> Result<Self, PlatformError> {
        let numbers = [origin_pt.0, origin_pt.1, size_pt.0, size_pt.1];
        if !numbers.iter().all(|n| n.is_finite())
            || size_pt.0 <= 0.0
            || size_pt.1 <= 0.0
            || !numbers.iter().all(|n| (*n as f32).is_finite())
        {
            return Err(PlatformError::InvalidInput);
        }
        let origin = (origin_pt.0 as f32, origin_pt.1 as f32);
        let size = (size_pt.0 as f32, size_pt.1 as f32);
        if size.0 <= 0.0 || size.1 <= 0.0 {
            return Err(PlatformError::InvalidInput);
        }
        Ok(Self {
            display_id,
            origin_pt: origin,
            size_pt: size,
        })
    }

    pub fn display_id(self) -> DisplayId {
        self.display_id
    }

    pub fn origin_pt(self) -> (f32, f32) {
        self.origin_pt
    }

    pub fn size_pt(self) -> (f32, f32) {
        self.size_pt
    }

    pub fn belongs_to(self, display: DisplayId) -> bool {
        self.display_id == display
    }
}
