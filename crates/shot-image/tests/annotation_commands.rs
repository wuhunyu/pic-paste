use shot_image::annotation_tools::{Tool, annotation_for_drag};
use shot_image::history::Annotation;

#[test]
fn reverse_drag_produces_bounded_mosaic_region() {
    let annotation = annotation_for_drag(Tool::Pixelate, (90, 80), (10, 20)).unwrap();
    assert!(matches!(
        annotation,
        Annotation::Pixelate {
            left: 10,
            top: 20,
            width: 81,
            height: 61,
            block_size: 12
        }
    ));
}

#[test]
fn selection_mode_never_mutates_image() {
    assert!(annotation_for_drag(Tool::Select, (1, 1), (40, 20)).is_none());
}

#[test]
fn rectangle_and_line_have_a_visible_default_style() {
    let rectangle = annotation_for_drag(Tool::Rectangle, (0, 0), (8, 8)).unwrap();
    assert!(matches!(rectangle, Annotation::Rectangle { width, .. } if width > 0.0));
    let line = annotation_for_drag(Tool::Line, (1, 2), (8, 8)).unwrap();
    assert!(matches!(line, Annotation::Line { width, .. } if width > 0.0));
}
