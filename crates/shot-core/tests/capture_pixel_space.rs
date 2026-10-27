use shot_core::display::OverlayPlacement;
use shot_core::platform::{CapturedFrame, DisplayId, PlatformError};
use shot_core::{RectPx, RgbaFrame};

fn test_frame() -> CapturedFrame {
    let pixels = RgbaFrame::new(4, 3, vec![0x80; 4 * 3 * 4]).unwrap();
    CapturedFrame::for_display(DisplayId(42), pixels)
}

#[test]
fn negative_origin_retina_display_keeps_capture_pixels_local() {
    let placement = OverlayPlacement::new(
        DisplayId(42),
        (-1440.0, -100.0),
        (1440.0, 900.0),
    )
    .unwrap();
    let frame = test_frame();
    assert_eq!(placement.origin_pt(), (-1440.0, -100.0));
    assert_eq!(frame.display_id, placement.display_id());
    assert_eq!(frame.origin_px, (0, 0));
    assert_eq!(
        frame.region_origin_px(RectPx {
            x: 2,
            y: 1,
            width: 2,
            height: 1,
        }),
        Ok((2, 1))
    );
}

#[test]
fn region_origin_rejects_negative_empty_and_out_of_bounds_rectangles() {
    let frame = test_frame();
    let invalid = [
        RectPx { x: -1, y: 0, width: 1, height: 1 },
        RectPx { x: 0, y: -1, width: 1, height: 1 },
        RectPx { x: 0, y: 0, width: 0, height: 1 },
        RectPx { x: 0, y: 0, width: 1, height: 0 },
        RectPx { x: 3, y: 0, width: 2, height: 1 },
        RectPx { x: 0, y: 2, width: 1, height: 2 },
        RectPx { x: i32::MAX, y: 0, width: u32::MAX, height: 1 },
    ];
    for region in invalid {
        assert_eq!(
            frame.region_origin_px(region),
            Err(PlatformError::InvalidInput)
        );
    }
    assert_eq!(frame.origin_px, (0, 0));
}

#[test]
fn crop_origin_addition_checks_overflow_and_preserves_original() {
    let mut frame = test_frame();
    frame.origin_px = (i32::MAX, i32::MAX);
    assert_eq!(
        frame.region_origin_px(RectPx {
            x: 1,
            y: 1,
            width: 1,
            height: 1,
        }),
        Err(PlatformError::InvalidInput)
    );
    assert_eq!(frame.origin_px, (i32::MAX, i32::MAX));
}

#[test]
fn previously_cropped_frame_has_local_origin_relative_to_display() {
    let mut frame = test_frame();
    frame.origin_px = (3, 5);
    assert_eq!(
        frame.region_origin_px(RectPx {
            x: 1,
            y: 2,
            width: 1,
            height: 1,
        }),
        Ok((4, 7))
    );
}
