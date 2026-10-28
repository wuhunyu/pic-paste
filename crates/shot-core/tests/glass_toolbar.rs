use shot_core::glass_toolbar::{
    FloatingBarFrame, GlassMaterial, PreviewTheme, select_glass_material,
};

#[test]
fn liquid_glass_is_only_selected_when_macos26_and_runtime_class_are_available() {
    assert_eq!(
        select_glass_material(26, true, false),
        GlassMaterial::Native
    );
    assert_eq!(
        select_glass_material(27, true, false),
        GlassMaterial::Native
    );
    assert_eq!(
        select_glass_material(25, true, false),
        GlassMaterial::Vibrancy
    );
    assert_eq!(
        select_glass_material(14, false, false),
        GlassMaterial::Vibrancy
    );
    assert_eq!(
        select_glass_material(26, false, false),
        GlassMaterial::Vibrancy
    );
}

#[test]
fn reduced_transparency_overrides_glass_and_legacy_vibrancy() {
    assert_eq!(select_glass_material(26, true, true), GlassMaterial::Opaque);
    assert_eq!(
        select_glass_material(25, false, true),
        GlassMaterial::Opaque
    );
}

#[test]
fn glass_capsule_is_centered_in_logical_points_above_canvas() {
    let frame = FloatingBarFrame::for_content_size(640.0, 360.0).unwrap();
    assert_eq!(
        frame,
        FloatingBarFrame {
            x: 202.0,
            y: 20.0,
            width: 236.0,
            height: 44.0,
        }
    );
    assert!(frame.contains(202.0, 20.0));
    assert!(frame.contains(437.0, 63.0));
    assert!(!frame.contains(438.0, 20.0));
    assert!(!frame.contains(202.0, 64.0));
    assert!(!frame.contains(f64::NAN, 32.0));
}

#[test]
fn narrow_zero_and_invalid_windows_hide_glass_instead_of_clipping() {
    for (w, h) in [
        (267.0, 360.0),
        (640.0, 139.0),
        (0.0, 360.0),
        (-1.0, 360.0),
        (640.0, 0.0),
        (f64::NAN, 360.0),
        (640.0, f64::INFINITY),
    ] {
        assert_eq!(FloatingBarFrame::for_content_size(w, h), None);
    }
    let frame = FloatingBarFrame::for_content_size(268.0, 140.0).unwrap();
    assert_eq!(frame.x, 16.0);
    assert_eq!(frame.y, 20.0);
}

#[test]
fn device_scale_never_changes_logical_toolbar_bounds() {
    let frame = FloatingBarFrame::for_content_size(800.0, 600.0).unwrap();
    assert_eq!(frame.width, 236.0);
    assert_eq!(frame.height, 44.0);
    assert_eq!(frame.x, 282.0);
}

#[test]
fn preview_theme_toggle_keeps_label_as_next_action() {
    let theme = PreviewTheme::Dark;
    assert_eq!(theme.toggle_label(), "浅色预览");
    let theme = theme.toggle();
    assert_eq!(theme, PreviewTheme::Light);
    assert_eq!(theme.toggle_label(), "深色预览");
    assert_eq!(theme.toggle(), PreviewTheme::Dark);
}
