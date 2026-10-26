use shot_core::pin_view::{PinImageView, map_pin_click};

#[test]
fn pin_click_maps_scaled_source_pixels_without_resampling() {
    let view = PinImageView {
        image_width: 400,
        image_height: 200,
        left: 20.0,
        top: 40.0,
        width: 200.0,
        height: 100.0,
    };
    assert_eq!(map_pin_click(view, (70.0, 65.0)), Some((100.0, 50.0)));
    assert_eq!(map_pin_click(view, (20.0, 40.0)), Some((0.0, 0.0)));
    assert!(map_pin_click(view, (219.9, 139.9)).is_some());
    assert_eq!(map_pin_click(view, (220.0, 140.0)), None);
}

#[test]
fn pin_click_rejects_toolbar_margins_and_nonfinite_coordinates() {
    let view = PinImageView {
        image_width: 400,
        image_height: 200,
        left: 20.0,
        top: 40.0,
        width: 200.0,
        height: 100.0,
    };
    for position in [
        (18.0, 60.0),
        (70.0, 38.0),
        (230.0, 65.0),
        (70.0, 140.0),
        (f32::NAN, 65.0),
        (70.0, f32::INFINITY),
    ] {
        assert_eq!(map_pin_click(view, position), None);
    }
}

#[test]
fn bad_pin_view_dimensions_do_not_generate_invalid_ocr_coordinates() {
    let mut view = PinImageView {
        image_width: 400,
        image_height: 200,
        left: 20.0,
        top: 40.0,
        width: 200.0,
        height: 100.0,
    };
    for bad in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        view.width = bad;
        assert_eq!(map_pin_click(view, (70.0, 65.0)), None);
    }
    view.width = 200.0;
    view.image_width = 0;
    assert_eq!(map_pin_click(view, (70.0, 65.0)), None);
}
