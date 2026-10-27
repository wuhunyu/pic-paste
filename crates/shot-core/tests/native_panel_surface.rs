use shot_core::native_panel::PanelSurfaceSize;
use shot_core::platform::PlatformError;

#[test]
fn retina_panel_uses_physical_pixels_not_global_window_position() {
    assert_eq!(
        PanelSurfaceSize::from_points(640.0, 360.0, 2.0),
        Ok(PanelSurfaceSize {
            width: 1280,
            height: 720,
        })
    );
    // The NSPanel may be positioned at a negative desktop origin; only its
    // content dimensions and backing scale determine wgpu's surface size.
    assert_eq!(
        PanelSurfaceSize::from_points(640.0, 360.0, 1.0),
        Ok(PanelSurfaceSize {
            width: 640,
            height: 360,
        })
    );
}

#[test]
fn fractional_backing_scale_rounds_dimensions_individually() {
    assert_eq!(
        PanelSurfaceSize::from_points(501.0, 301.0, 1.5),
        Ok(PanelSurfaceSize {
            width: 752,
            height: 452,
        })
    );
}

#[test]
fn invalid_dimensions_scales_and_oversized_surfaces_fail_closed() {
    let invalid = [
        (0.0, 360.0, 2.0),
        (-100.0, 360.0, 2.0),
        (640.0, 0.0, 2.0),
        (640.0, -100.0, 2.0),
        (640.0, 360.0, 0.0),
        (640.0, 360.0, -1.0),
        (f64::NAN, 360.0, 2.0),
        (640.0, f64::INFINITY, 2.0),
        (640.0, 360.0, f64::NAN),
        (640.0, 360.0, f64::INFINITY),
        (4097.0, 300.0, 2.0),
        (f64::MAX, 500.0, 4.0),
        (0.1, 300.0, 1.0),
    ];
    for (width_pt, height_pt, backing_scale) in invalid {
        assert_eq!(
            PanelSurfaceSize::from_points(width_pt, height_pt, backing_scale),
            Err(PlatformError::InvalidInput)
        );
    }
}

#[test]
fn exact_maximum_surface_size_is_allowed() {
    assert_eq!(
        PanelSurfaceSize::from_points(4096.0, 4096.0, 2.0),
        Ok(PanelSurfaceSize {
            width: 8192,
            height: 8192,
        })
    );
}


use shot_core::native_panel::{PanelSurfaceLifecycle, PanelSurfaceUpdate};

#[test]
fn surface_lifecycle_reconfigures_only_when_physical_size_changes() {
    let first = PanelSurfaceSize { width: 1280, height: 720 };
    let mut state = PanelSurfaceLifecycle::new(first);
    assert_eq!(state.configured_size(), Some(first));
    assert_eq!(state.observe(640.0, 360.0, 2.0), Ok(PanelSurfaceUpdate::Unchanged));
    assert_eq!(
        state.observe(700.0, 360.0, 2.0),
        Ok(PanelSurfaceUpdate::Reconfigure(PanelSurfaceSize { width: 1400, height: 720 }))
    );
    assert_eq!(state.observe(700.0, 360.0, 2.0), Ok(PanelSurfaceUpdate::Unchanged));
    assert_eq!(
        state.observe(700.0, 360.0, 1.0),
        Ok(PanelSurfaceUpdate::Reconfigure(PanelSurfaceSize { width: 700, height: 360 }))
    );
}

#[test]
fn zero_sized_content_suspends_and_restoration_reconfigures() {
    let initial = PanelSurfaceSize { width: 1280, height: 720 };
    let mut state = PanelSurfaceLifecycle::new(initial);
    assert_eq!(state.observe(0.0, 360.0, 2.0), Ok(PanelSurfaceUpdate::Suspended));
    assert_eq!(state.configured_size(), None);
    assert_eq!(state.observe(0.0, 360.0, 2.0), Ok(PanelSurfaceUpdate::Suspended));
    assert_eq!(
        state.observe(640.0, 360.0, 2.0),
        Ok(PanelSurfaceUpdate::Reconfigure(initial))
    );
}

#[test]
fn invalid_change_does_not_discard_last_good_surface() {
    let initial = PanelSurfaceSize { width: 1280, height: 720 };
    let mut state = PanelSurfaceLifecycle::new(initial);
    for (w, h, scale) in [
        (-1.0, 360.0, 2.0),
        (f64::NAN, 360.0, 2.0),
        (640.0, 360.0, f64::INFINITY),
        (9000.0, 360.0, 2.0),
        (0.0, f64::NAN, 2.0),
    ] {
        assert_eq!(state.observe(w, h, scale), Err(PlatformError::InvalidInput));
        assert_eq!(state.configured_size(), Some(initial));
    }
    assert_eq!(state.observe(640.0, 360.0, 2.0), Ok(PanelSurfaceUpdate::Unchanged));
}

#[test]
fn closed_panel_never_reconfigures_and_close_is_idempotent() {
    let initial = PanelSurfaceSize { width: 1280, height: 720 };
    let mut state = PanelSurfaceLifecycle::new(initial);
    assert_eq!(state.close(), PanelSurfaceUpdate::Closed);
    assert_eq!(state.configured_size(), None);
    assert_eq!(state.close(), PanelSurfaceUpdate::Closed);
    assert_eq!(state.observe(1000.0, 500.0, 2.0), Ok(PanelSurfaceUpdate::Closed));
    assert_eq!(state.configured_size(), None);
}
