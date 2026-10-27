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
