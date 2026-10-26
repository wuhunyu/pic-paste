use shot_core::display::OverlayPlacement;
use shot_core::platform::{DisplayId, PlatformError};

#[test]
fn native_overlay_uses_logical_bounds_of_the_captured_monitor() {
    let p = OverlayPlacement::new(DisplayId(72), (-1440.0, -120.0), (1440.0, 900.0)).unwrap();
    assert_eq!(p.display_id(), DisplayId(72));
    assert_eq!(p.origin_pt(), (-1440.0, -120.0));
    assert_eq!(p.size_pt(), (1440.0, 900.0));
    assert!(p.belongs_to(DisplayId(72)));
    assert!(!p.belongs_to(DisplayId(12)));
}

#[test]
fn display_origin_is_not_scaled_using_retina_pixel_ratio() {
    let p = OverlayPlacement::new(DisplayId(9), (-1920.0, 0.0), (1920.0, 1080.0)).unwrap();
    assert_eq!(p.origin_pt(), (-1920.0, 0.0));
    assert_eq!(p.size_pt(), (1920.0, 1080.0));
}

#[test]
fn rejected_geometry_cannot_open_an_offscreen_or_invalid_native_window() {
    assert_eq!(
        OverlayPlacement::new(DisplayId(1), (f64::NAN, 0.0), (100.0, 100.0)),
        Err(PlatformError::InvalidInput),
    );
    assert_eq!(
        OverlayPlacement::new(DisplayId(1), (0.0, 0.0), (0.0, 100.0)),
        Err(PlatformError::InvalidInput),
    );
    assert_eq!(
        OverlayPlacement::new(DisplayId(1), (0.0, 0.0), (f64::INFINITY, 100.0)),
        Err(PlatformError::InvalidInput),
    );
    assert_eq!(
        OverlayPlacement::new(DisplayId(1), (f64::MAX, 0.0), (100.0, 100.0)),
        Err(PlatformError::InvalidInput),
    );
    assert_eq!(
        OverlayPlacement::new(DisplayId(1), (0.0, 0.0), (f64::MAX, 100.0)),
        Err(PlatformError::InvalidInput),
    );
}
