use shot_core::RgbaFrame;
use shot_image::pixel_tools::{magnifier_patch, sample_rgba};

fn gradient() -> RgbaFrame {
    let mut rgba = Vec::new();
    for y in 0..3_u8 {
        for x in 0..3_u8 {
            rgba.extend_from_slice(&[x * 20, y * 30, x + y, 255]);
        }
    }
    RgbaFrame::new(3, 3, rgba).unwrap()
}

#[test]
fn exact_color_sampling_uses_native_image_pixels() {
    let frame = gradient();
    assert_eq!(sample_rgba(&frame, (2, 1)), Some([40, 30, 3, 255]));
    assert_eq!(sample_rgba(&frame, (-1, 0)), None);
    assert_eq!(sample_rgba(&frame, (3, 0)), None);
}

#[test]
fn magnifier_preserves_center_and_clamps_edges() {
    let frame = gradient();
    let center = magnifier_patch(&frame, (1, 1), 3).unwrap();
    assert_eq!(center.rgba, frame.rgba);
    let edge = magnifier_patch(&frame, (0, 0), 3).unwrap();
    assert_eq!(&edge.rgba[0..4], &[0, 0, 0, 255]);
    assert_eq!(
        &edge.rgba[(2 * 3 + 2) * 4..(2 * 3 + 2) * 4 + 4],
        &[20, 30, 2, 255]
    );
}

#[test]
fn magnifier_rejects_invalid_size_and_invalid_pixel_buffers() {
    let frame = gradient();
    for size in [0, 2, 24] {
        assert!(magnifier_patch(&frame, (0, 0), size).is_err());
    }
    let invalid = RgbaFrame {
        width: 3,
        height: 3,
        rgba: vec![],
    };
    assert!(magnifier_patch(&invalid, (0, 0), 3).is_err());
}
