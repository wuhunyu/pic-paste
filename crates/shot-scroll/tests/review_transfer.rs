use shot_core::RectPx;
use shot_scroll::review_transfer::{default_transfer_rect, map_preview_point, validate_transfer_rect};

#[test]
fn full_small_canvas_is_a_native_pixel_transfer() {
    let rect = default_transfer_rect((1200, 800));
    assert_eq!(
        rect,
        RectPx {
            x: 0,
            y: 0,
            width: 1200,
            height: 800
        }
    );
    assert!(validate_transfer_rect(rect, (1200, 800)).is_ok());
}

#[test]
fn huge_canvas_defaults_to_bounded_unscaled_crop() {
    let rect = default_transfer_rect((1920, 60_000));
    assert_eq!(rect.width, 1920);
    assert!(rect.height < 60_000);
    assert!(u64::from(rect.width) * u64::from(rect.height) <= 4_000_000);
    assert!(validate_transfer_rect(rect, (1920, 60_000)).is_ok());
    assert!(
        validate_transfer_rect(
            RectPx { x: 0, y: 0, width: 1920, height: 60_000 },
            (1920, 60_000)
        )
        .is_err()
    );
}

#[test]
fn invalid_or_excessive_crop_is_explicitly_rejected() {
    let canvas = (10_000, 20_000);
    for rect in [
        RectPx { x: -1, y: 0, width: 12, height: 12 },
        RectPx { x: 0, y: 0, width: 0, height: 1 },
        RectPx { x: 0, y: 0, width: 4097, height: 1 },
        RectPx { x: 0, y: 0, width: 2000, height: 2001 },
        RectPx { x: 9999, y: 0, width: 10, height: 1 },
    ] {
        assert!(validate_transfer_rect(rect, canvas).is_err(), "{rect:?}");
    }
}

#[test]
fn scaled_preview_maps_to_full_resolution_and_clamps_edges() {
    assert_eq!(
        map_preview_point((50.0, 100.0), (100.0, 200.0), (2000, 4000)),
        Some((1000, 2000))
    );
    assert_eq!(
        map_preview_point((-10.0, -10.0), (100.0, 200.0), (2000, 4000)),
        Some((0, 0))
    );
    assert_eq!(
        map_preview_point((110.0, 300.0), (100.0, 200.0), (2000, 4000)),
        Some((1999, 3999))
    );
    assert_eq!(
        map_preview_point((f32::NAN, 2.0), (100.0, 200.0), (2000, 4000)),
        None
    );
    assert_eq!(
        map_preview_point((3.0, 2.0), (0.0, 200.0), (2000, 4000)),
        None
    );
}
