use shot_core::{CoreError, RectPx, RgbaFrame};
use shot_image::preview::png_selection_preview;

#[test]
fn preview_is_bounded_and_preserves_source_pixels() {
    let frame = RgbaFrame::new(120, 60, vec![255; 120 * 60 * 4]).unwrap();
    let preview = png_selection_preview(
        &frame,
        RectPx {
            x: 0,
            y: 0,
            width: 120,
            height: 60,
        },
        0,
        30,
    )
    .unwrap();
    assert_eq!((preview.width, preview.height), (30, 15));
    assert!(preview.rgba.iter().all(|value| *value == 255));
    assert_eq!(frame.rgba[3], 255);
}

#[test]
fn preview_uses_only_selected_region_and_retains_source_alpha() {
    let mut pixels = vec![0u8; 8 * 4 * 4];
    for y in 0..4 {
        for x in 0..8 {
            let p = (y * 8 + x) * 4;
            pixels[p..p + 4].copy_from_slice(if x < 4 {
                &[255, 0, 0, 255]
            } else {
                &[0, 255, 0, 150]
            });
        }
    }
    let frame = RgbaFrame::new(8, 4, pixels).unwrap();
    let preview = png_selection_preview(
        &frame,
        RectPx {
            x: 4,
            y: 0,
            width: 4,
            height: 4,
        },
        0,
        128,
    )
    .unwrap();
    assert_eq!((preview.width, preview.height), (4, 4));
    assert_eq!(&preview.rgba[..4], &[0, 255, 0, 150]);
    assert_eq!(&frame.rgba[..4], &[255, 0, 0, 255]);
}

#[test]
fn rounded_transparent_corners_are_visible_in_preview() {
    let frame = RgbaFrame::new(16, 16, vec![255; 16 * 16 * 4]).unwrap();
    let preview = png_selection_preview(
        &frame,
        RectPx {
            x: 0,
            y: 0,
            width: 16,
            height: 16,
        },
        8,
        16,
    )
    .unwrap();
    assert!(preview.rgba[3] < 255);
    assert_eq!(preview.rgba[(8 * 16 + 8) * 4 + 3], 255);
    assert_eq!(frame.rgba[3], 255);
}

#[test]
fn rejects_invalid_selections_and_unbounded_thumbnail_requests() {
    let frame = RgbaFrame::new(8, 8, vec![255; 8 * 8 * 4]).unwrap();
    let invalid = RectPx {
        x: -1,
        y: 0,
        width: 4,
        height: 4,
    };
    assert_eq!(
        png_selection_preview(&frame, invalid, 0, 32).unwrap_err(),
        CoreError::OutOfBounds
    );
    assert!(
        png_selection_preview(
            &frame,
            RectPx {
                x: 0,
                y: 0,
                width: 8,
                height: 8
            },
            0,
            0
        )
        .is_err()
    );
}
