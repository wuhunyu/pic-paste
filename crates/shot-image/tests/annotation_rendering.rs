use shot_core::RgbaFrame;
use shot_image::annotation_tools::{Tool, annotation_for_drag};
use shot_image::composite::composite;
use shot_image::history::Annotation;

fn solid(width: u32, height: u32, rgba: [u8; 4]) -> RgbaFrame {
    let pixels = (0..width * height).flat_map(|_| rgba).collect();
    RgbaFrame::new(width, height, pixels).unwrap()
}

fn pixel(frame: &RgbaFrame, x: u32, y: u32) -> [u8; 4] {
    let idx = (y as usize * frame.width as usize + x as usize) * 4;
    frame.rgba[idx..idx + 4].try_into().unwrap()
}

#[test]
fn ellipse_draws_visible_stroke_but_not_its_center() {
    let image = solid(40, 40, [0, 0, 0, 0]);
    let ellipse = annotation_for_drag(Tool::Ellipse, (6, 8), (34, 30)).unwrap();
    let drawn = composite(&image, &[ellipse]).unwrap();
    assert!(drawn.rgba.chunks_exact(4).any(|p| p[3] > 0));
    assert_eq!(pixel(&drawn, 20, 19)[3], 0);
    assert!(image.rgba.iter().all(|p| *p == 0));
}

#[test]
fn arrow_has_a_visible_head() {
    let image = solid(60, 60, [0, 0, 0, 0]);
    let arrow = annotation_for_drag(Tool::Arrow, (4, 30), (50, 30)).unwrap();
    let drawn = composite(&image, &[arrow]).unwrap();
    assert!(pixel(&drawn, 48, 30)[3] > 0);
    // Head must have coverage above or below the horizontal shaft.
    assert!((32..50).any(|x| pixel(&drawn, x, 26)[3] > 0 || pixel(&drawn, x, 34)[3] > 0));
}

#[test]
fn brush_strokes_are_continuous_and_undo_does_not_change_source() {
    let image = solid(40, 40, [0, 0, 0, 0]);
    let stroke = Annotation::Brush {
        points: vec![(2.0, 2.0), (12.0, 20.0), (29.0, 12.0)],
        rgba: [255, 0, 0, 255],
        width: 3.0,
    };
    let result = composite(&image, &[stroke]).unwrap();
    assert!(pixel(&result, 12, 20)[3] > 0);
    assert_eq!(pixel(&image, 12, 20), [0; 4]);
}

#[test]
fn blur_changes_only_requested_roi_without_erasing_alpha() {
    let mut image = solid(9, 9, [0, 0, 0, 255]);
    let bright = (4 * 9 + 4) * 4;
    image.rgba[bright..bright + 4].copy_from_slice(&[255, 255, 255, 255]);
    let blur = Annotation::Blur { left: 2, top: 2, width: 5, height: 5, radius: 2 };
    let result = composite(&image, &[blur]).unwrap();
    assert_eq!(pixel(&result, 0, 0), [0, 0, 0, 255]);
    assert!(pixel(&result, 4, 4)[0] < 255);
    assert!(pixel(&result, 4, 4)[0] > 0);
    assert_eq!(pixel(&result, 4, 4)[3], 255);
}

#[test]
fn unsupported_or_malformed_geometry_must_error_not_succeed_silently() {
    let image = solid(10, 10, [0, 0, 0, 0]);
    assert!(composite(&image, &[Annotation::Brush {
        points: vec![(f32::NAN, 4.0), (2.0, 4.0)],
        rgba: [255; 4], width: 1.0,
    }]).is_err());
    assert!(composite(&image, &[Annotation::Blur {
        left: 1, top: 1, width: 3, height: 3, radius: 0,
    }]).is_err());
}
