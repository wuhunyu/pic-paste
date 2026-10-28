//! Pure layout and macOS material policy for an *experimental* floating
//! AppKit toolbar. No screenshot pixels, WGPU objects, or native handles live here.

use crate::native_markup::NativeMarkupTool;

/// What kind of native material to create for a small floating control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlassMaterial {
    /// macOS 26+ AppKit `NSGlassEffectView`.
    Native,
    /// macOS 14–25 (or missing class): `NSVisualEffectView`.
    Vibrancy,
    /// Accessibility "Reduce Transparency": an opaque system-colored view.
    Opaque,
}

/// The system accessibility preference always takes priority over cosmetics.
///
/// A class check is required in addition to the OS version so an unavailable
/// runtime class cannot be instantiated by accident.
pub fn select_glass_material(
    mac_major: u32,
    native_class_available: bool,
    reduce_transparency: bool,
) -> GlassMaterial {
    if reduce_transparency {
        GlassMaterial::Opaque
    } else if mac_major >= 26 && native_class_available {
        GlassMaterial::Native
    } else {
        GlassMaterial::Vibrancy
    }
}

/// A capsule in the content view's **logical point** coordinate system.
/// Not physical screen pixels, global desktop positions, or cropped PNG space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloatingBarFrame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl FloatingBarFrame {
    /// Position one 236x44 pt capsule 20 pt above the bottom edge.
    ///
    /// Return `None` rather than clipping into screenshot controls when the
    /// host is too small or AppKit reports transient invalid geometry.
    pub fn for_content_size(width_pt: f64, height_pt: f64) -> Option<Self> {
        if !width_pt.is_finite() || !height_pt.is_finite() || width_pt < 268.0 || height_pt < 140.0
        {
            return None;
        }
        Some(Self {
            x: (width_pt - 236.0) / 2.0,
            y: 20.0,
            width: 236.0,
            height: 44.0,
        })
    }

    /// The seven-button selection/annotation capsule, also in logical points.
    /// Width is independent of Retina density; tiny hosts hide the capsule.
    pub fn for_annotation_size(width_pt: f64, height_pt: f64) -> Option<Self> {
        if !width_pt.is_finite() || !height_pt.is_finite() || width_pt < 580.0 || height_pt < 140.0
        {
            return None;
        }
        Some(Self {
            x: (width_pt - 560.0) / 2.0,
            y: 20.0,
            width: 560.0,
            height: 48.0,
        })
    }

    /// Half-open rectangle for protecting native overlay hit testing.
    pub fn contains(self, x: f64, y: f64) -> bool {
        x.is_finite()
            && y.is_finite()
            && x >= self.x
            && y >= self.y
            && x < self.x + self.width
            && y < self.y + self.height
    }
}

/// Preview background choice for the opt-in NSPanel demo. It is deliberately
/// independent from screenshot/crop pixels and from the system's appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PreviewTheme {
    #[default]
    Dark,
    Light,
}

impl PreviewTheme {
    /// Toggle only the WGPU/egui demonstration canvas, never captured pixels.
    pub fn toggle(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }

    /// The visible button title describes what clicking it will do next.
    pub fn toggle_label(self) -> &'static str {
        match self {
            Self::Dark => "浅色预览",
            Self::Light => "深色预览",
        }
    }
}

/// Commands issued by the AppKit-native toolbar. They are queued on the
/// main thread and applied by the probe event loop after Cocoa dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlassToolbarAction {
    ToggleTheme,
    CloseProbe,
    ChooseTool(NativeMarkupTool),
    UndoMarkup,
    RedoMarkup,
    CancelSelection,
}

impl GlassToolbarAction {
    pub fn tool(self) -> Option<NativeMarkupTool> {
        match self {
            Self::ChooseTool(tool) => Some(tool),
            Self::ToggleTheme
            | Self::CloseProbe
            | Self::UndoMarkup
            | Self::RedoMarkup
            | Self::CancelSelection => None,
        }
    }
}
