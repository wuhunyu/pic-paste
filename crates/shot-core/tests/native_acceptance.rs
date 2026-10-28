use shot_core::glass_toolbar::GlassMaterial;
use shot_core::native_acceptance::{NativePreflightReport, NativeScreenDiagnostic};

fn left_retina() -> NativeScreenDiagnostic {
    NativeScreenDiagnostic::new((-1440.0, 0.0), (1440.0, 900.0), 2.0).unwrap()
}

#[test]
fn macos_26_glass_and_mixed_backing_scales_are_observations_not_gui_acceptance() {
    let report = NativePreflightReport::new(
        (26, 0, 1),
        false,
        true,
        false,
        vec![
            left_retina(),
            NativeScreenDiagnostic::new((0.0, 0.0), (1920.0, 1080.0), 1.0).unwrap(),
        ],
    )
    .unwrap();
    assert_eq!(report.material(), GlassMaterial::Native);
    assert!(report.has_negative_origin());
    assert!(report.has_mixed_scales());

    let json = report.to_json_line();
    assert!(json.starts_with("{\"schema\":\"picpaste.native-preflight/1\""));
    assert!(json.contains("\"macos\":{\"major\":26,\"minor\":0,\"patch\":1}"));
    assert!(json.contains("\"material\":\"native\""));
    assert!(json.contains("\"screen_recording_preflight\":false"));
    assert!(json.contains("\"screen_count\":2"));
    assert!(json.contains("\"has_negative_origin\":true"));
    assert!(json.contains("\"has_mixed_scales\":true"));
    assert!(json.contains("\"interactive_gui_observed\":false"));
    assert!(json.contains("\"origin_x_pt\":-1440"));
    assert!(json.contains("\"backing_scale\":2"));
    assert_eq!(json.lines().count(), 1);
}

#[test]
fn accessibility_opaque_and_legacy_vibrancy_take_precedence_over_os_label() {
    let reduced = NativePreflightReport::new((26, 1, 0), true, true, true, vec![]).unwrap();
    assert_eq!(reduced.material(), GlassMaterial::Opaque);
    assert!(reduced.to_json_line().contains("\"material\":\"opaque\""));
    let missing_class = NativePreflightReport::new((26, 0, 0), false, false, false, vec![]).unwrap();
    assert_eq!(missing_class.material(), GlassMaterial::Vibrancy);
    let legacy = NativePreflightReport::new((14, 7, 3), false, true, false, vec![]).unwrap();
    assert_eq!(legacy.material(), GlassMaterial::Vibrancy);
    assert!(legacy.to_json_line().contains("\"material\":\"vibrancy\""));
}

#[test]
fn headless_macos_runner_may_report_no_screens_without_claiming_pass() {
    let report = NativePreflightReport::new((26, 0, 0), false, true, false, vec![]).unwrap();
    assert!(!report.has_negative_origin());
    assert!(!report.has_mixed_scales());
    let json = report.to_json_line();
    assert!(json.contains("\"screens\":[]"));
    assert!(json.contains("\"screen_count\":0"));
    assert!(json.contains("\"interactive_gui_observed\":false"));
}

#[test]
fn impossible_logical_geometry_nonfinite_scaling_or_unknown_os_is_rejected() {
    for (origin, size, scale) in [
        ((f64::NAN, 0.0), (800.0, 600.0), 2.0),
        ((0.0, f64::INFINITY), (800.0, 600.0), 2.0),
        ((0.0, 0.0), (0.0, 600.0), 2.0),
        ((0.0, 0.0), (800.0, -1.0), 2.0),
        ((0.0, 0.0), (800.0, 600.0), 0.0),
        ((0.0, 0.0), (800.0, 600.0), f64::NAN),
        ((0.0, 0.0), (800.0, 600.0), f64::INFINITY),
    ] {
        assert!(NativeScreenDiagnostic::new(origin, size, scale).is_err());
    }
    assert!(NativePreflightReport::new((0, 0, 0), false, false, false, vec![]).is_err());
}

#[test]
fn identical_scaling_has_no_mixed_flag_and_tcc_is_only_observed() {
    let screens = vec![
        NativeScreenDiagnostic::new((0.0, 0.0), (1440.0, 900.0), 2.0).unwrap(),
        NativeScreenDiagnostic::new((1440.0, 0.0), (1440.0, 900.0), 2.0).unwrap(),
    ];
    let report = NativePreflightReport::new((15, 5, 0), false, false, true, screens).unwrap();
    assert!(!report.has_negative_origin());
    assert!(!report.has_mixed_scales());
    assert!(report.to_json_line().contains("\"screen_recording_preflight\":true"));
    assert!(report.to_json_line().contains("\"interactive_gui_observed\":false"));
}
