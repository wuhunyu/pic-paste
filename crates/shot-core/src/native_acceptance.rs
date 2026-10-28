//! Read-only Mac NSPanel acceptance environment snapshot.
//!
//! AppKit screen frames use bottom-left, desktop-global **logical points**.
//! Backing scales describe each screen, not a multiplier for desktop origins.
//! A preflight report is deliberately not a claim of observed GUI interaction.

use crate::glass_toolbar::{GlassMaterial, select_glass_material};

#[derive(Clone, Debug, PartialEq)]
pub struct NativeScreenDiagnostic {
    origin_pt: (f64, f64),
    size_pt: (f64, f64),
    backing_scale: f64,
}

impl NativeScreenDiagnostic {
    pub fn new(
        origin_pt: (f64, f64),
        size_pt: (f64, f64),
        backing_scale: f64,
    ) -> Result<Self, String> {
        if ![
            origin_pt.0,
            origin_pt.1,
            size_pt.0,
            size_pt.1,
            backing_scale,
        ]
        .iter()
        .all(|value| value.is_finite())
            || size_pt.0 <= 0.0
            || size_pt.1 <= 0.0
            || backing_scale <= 0.0
        {
            return Err(String::from(
                "invalid AppKit screen points or backing scale",
            ));
        }
        Ok(Self {
            origin_pt,
            size_pt,
            backing_scale,
        })
    }
}

#[derive(Clone, Debug)]
pub struct NativePreflightReport {
    os_version: (u32, u32, u32),
    reduce_transparency: bool,
    material: GlassMaterial,
    tcc_screen_recording_authorized: bool,
    screens: Vec<NativeScreenDiagnostic>,
}

impl NativePreflightReport {
    pub fn new(
        os_version: (u32, u32, u32),
        reduce_transparency: bool,
        glass_class_available: bool,
        tcc_screen_recording_authorized: bool,
        screens: Vec<NativeScreenDiagnostic>,
    ) -> Result<Self, String> {
        if os_version.0 < 14 {
            return Err(String::from("native preflight requires macOS 14 or newer"));
        }
        let material =
            select_glass_material(os_version.0, glass_class_available, reduce_transparency);
        Ok(Self {
            os_version,
            reduce_transparency,
            material,
            tcc_screen_recording_authorized,
            screens,
        })
    }

    pub fn material(&self) -> GlassMaterial {
        self.material
    }

    pub fn has_negative_origin(&self) -> bool {
        self.screens
            .iter()
            .any(|screen| screen.origin_pt.0 < 0.0 || screen.origin_pt.1 < 0.0)
    }

    pub fn has_mixed_scales(&self) -> bool {
        let Some(first) = self.screens.first() else {
            return false;
        };
        self.screens
            .iter()
            .any(|screen| (screen.backing_scale - first.backing_scale).abs() > 0.01)
    }

    /// One JSON record with fixed field names and validated finite numbers.
    /// No machine identifiers, app state, screenshots, usernames or paths.
    pub fn to_json_line(&self) -> String {
        let material = match self.material {
            GlassMaterial::Native => "native",
            GlassMaterial::Vibrancy => "vibrancy",
            GlassMaterial::Opaque => "opaque",
        };
        let screens = self
            .screens
            .iter()
            .map(|screen| {
                format!(
                    r#"{{"origin_x_pt":{},"origin_y_pt":{},"width_pt":{},"height_pt":{},"backing_scale":{}}}"#,
                    screen.origin_pt.0,
                    screen.origin_pt.1,
                    screen.size_pt.0,
                    screen.size_pt.1,
                    screen.backing_scale,
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"schema":"picpaste.native-preflight/1","macos":{{"major":{},"minor":{},"patch":{}}},"material":"{}","reduce_transparency":{},"screen_recording_preflight":{},"screen_count":{},"has_negative_origin":{},"has_mixed_scales":{},"interactive_gui_observed":false,"screens":[{}]}}"#,
            self.os_version.0,
            self.os_version.1,
            self.os_version.2,
            material,
            self.reduce_transparency,
            self.tcc_screen_recording_authorized,
            self.screens.len(),
            self.has_negative_origin(),
            self.has_mixed_scales(),
            screens,
        )
    }
}
