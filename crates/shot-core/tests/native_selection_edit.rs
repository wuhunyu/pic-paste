use shot_core::RectPx;
use shot_core::display::OverlayPlacement;
use shot_core::native_selection_edit::{NativeSelectionEdit, SelectionGrip};
use shot_core::platform::DisplayId;

fn placement() -> OverlayPlacement {
    OverlayPlacement::new(DisplayId(12), (-1000.0, -80.0), (1000.0, 800.0)).unwrap()
}

fn empty() -> NativeSelectionEdit {
    NativeSelectionEdit::new(placement(), (2000, 1600)).unwrap()
}

fn seeded() -> NativeSelectionEdit {
    let mut editor = empty();
    assert_eq!(editor.begin((100.0, 100.0)), Some(SelectionGrip::Create));
    editor.update((300.0, 250.0));
    assert_eq!(
        editor.end((300.0, 250.0)),
        Some(RectPx {
            x: 200,
            y: 200,
            width: 400,
            height: 300
        })
    );
    editor
}

#[test]
fn selection_requires_nonzero_drag_and_explicit_confirmation() {
    let mut editor = empty();
    assert_eq!(editor.confirmed(), None);
    assert_eq!(editor.begin((100.0, 100.0)), Some(SelectionGrip::Create));
    editor.update((300.0, 250.0));
    assert_eq!(editor.confirmed(), None, "uncommitted drag cannot confirm");
    assert!(editor.preview_rect_points().is_some());
    editor.end((300.0, 250.0));
    assert_eq!(
        editor.confirmed(),
        Some(RectPx {
            x: 200,
            y: 200,
            width: 400,
            height: 300
        })
    );
    editor.reset();
    assert_eq!(editor.confirmed(), None);
}

#[test]
fn all_eight_grips_are_display_local_logical_points() {
    let editor = seeded();
    let handles = editor.grip_centers_points().unwrap();
    let expected = [
        (SelectionGrip::NorthWest, (100.0, 100.0)),
        (SelectionGrip::North, (200.0, 100.0)),
        (SelectionGrip::NorthEast, (300.0, 100.0)),
        (SelectionGrip::East, (300.0, 175.0)),
        (SelectionGrip::SouthEast, (300.0, 250.0)),
        (SelectionGrip::South, (200.0, 250.0)),
        (SelectionGrip::SouthWest, (100.0, 250.0)),
        (SelectionGrip::West, (100.0, 175.0)),
    ];
    assert_eq!(handles, expected);
    for (grip, center) in handles {
        assert_eq!(editor.hit_test(center), Some(grip));
    }
    assert_eq!(editor.hit_test((150.0, 150.0)), Some(SelectionGrip::Move));
    assert_eq!(editor.hit_test((400.0, 400.0)), Some(SelectionGrip::Create));
    assert_eq!(editor.hit_test((-1.0, 20.0)), None);
}

#[test]
fn all_cardinal_resizes_anchor_the_opposite_edge() {
    let original = seeded().confirmed().unwrap();
    let cases = [
        (SelectionGrip::West, (100.0, 175.0), (150.0, 175.0),
            RectPx { x: 300, y: 200, width: 300, height: 300 }),
        (SelectionGrip::East, (300.0, 175.0), (350.0, 175.0),
            RectPx { x: 200, y: 200, width: 500, height: 300 }),
        (SelectionGrip::North, (200.0, 100.0), (200.0, 90.0),
            RectPx { x: 200, y: 180, width: 400, height: 320 }),
        (SelectionGrip::South, (200.0, 250.0), (200.0, 300.0),
            RectPx { x: 200, y: 200, width: 400, height: 400 }),
    ];
    for (grip, start, end, expected) in cases {
        let mut editor = seeded();
        assert_eq!(editor.begin(start), Some(grip));
        editor.update(end);
        assert_eq!(editor.current_rect(), Some(expected), "preview {grip:?}");
        assert_eq!(editor.confirmed(), None, "cannot confirm mid-gesture");
        assert_eq!(editor.end(end), Some(expected));
        assert_eq!(editor.confirmed(), Some(expected));
        assert_ne!(editor.confirmed(), Some(original));
    }
}

#[test]
fn corner_grips_resize_two_edges_without_moving_opposite_corner() {
    let cases = [
        (SelectionGrip::NorthWest, (100.0, 100.0), (150.0, 125.0),
            RectPx { x: 300, y: 250, width: 300, height: 250 }),
        (SelectionGrip::NorthEast, (300.0, 100.0), (350.0, 125.0),
            RectPx { x: 200, y: 250, width: 500, height: 250 }),
        (SelectionGrip::SouthWest, (100.0, 250.0), (150.0, 300.0),
            RectPx { x: 300, y: 200, width: 300, height: 400 }),
        (SelectionGrip::SouthEast, (300.0, 250.0), (350.0, 300.0),
            RectPx { x: 200, y: 200, width: 500, height: 400 }),
    ];
    for (grip, start, end, expected) in cases {
        let mut editor = seeded();
        assert_eq!(editor.begin(start), Some(grip));
        assert_eq!(editor.end(end), Some(expected), "{grip:?}");
    }
}

#[test]
fn crossing_resize_cannot_flip_or_create_zero_area_rectangle() {
    let mut editor = seeded();
    assert_eq!(editor.begin((100.0, 100.0)), Some(SelectionGrip::NorthWest));
    editor.update((9999.0, 9999.0));
    assert_eq!(
        editor.end((9999.0, 9999.0)),
        Some(RectPx { x: 599, y: 499, width: 1, height: 1 })
    );
    assert_eq!(editor.confirmed().unwrap().width, 1);
    assert_eq!(editor.confirmed().unwrap().height, 1);
}

#[test]
fn move_clamps_to_display_and_keeps_physical_dimensions_exactly() {
    let mut editor = seeded();
    assert_eq!(editor.begin((150.0, 150.0)), Some(SelectionGrip::Move));
    editor.update((-100.0, -100.0));
    assert_eq!(
        editor.end((-100.0, -100.0)),
        Some(RectPx { x: 0, y: 0, width: 400, height: 300 })
    );

    let mut editor = seeded();
    assert_eq!(editor.begin((150.0, 150.0)), Some(SelectionGrip::Move));
    assert_eq!(
        editor.end((9999.0, 9999.0)),
        Some(RectPx { x: 1600, y: 1300, width: 400, height: 300 })
    );
}

#[test]
fn tiny_drag_preserves_one_source_pixel_and_click_preserves_previous_region() {
    let mut editor = empty();
    assert_eq!(editor.begin((1.25, 1.25)), Some(SelectionGrip::Create));
    assert_eq!(
        editor.end((1.5, 1.5)),
        Some(RectPx { x: 2, y: 2, width: 1, height: 1 })
    );
    let previous = editor.confirmed();
    assert_eq!(editor.begin((900.0, 700.0)), Some(SelectionGrip::Create));
    assert_eq!(editor.end((900.0, 700.0)), previous);
    assert_eq!(editor.confirmed(), previous);
}

#[test]
fn aborted_gesture_and_nonfinite_inputs_never_mutate_committed_region() {
    let mut editor = seeded();
    let original = editor.confirmed();
    assert_eq!(editor.begin((300.0, 175.0)), Some(SelectionGrip::East));
    editor.update((350.0, 175.0));
    editor.abort();
    assert_eq!(editor.confirmed(), original);
    assert_eq!(editor.current_rect(), original);
    assert_eq!(editor.begin((f32::NAN, 100.0)), None);
    assert_eq!(editor.begin((150.0, f32::INFINITY)), None);
    assert_eq!(editor.begin((150.0, 150.0)), Some(SelectionGrip::Move));
    editor.update((f32::INFINITY, 0.0));
    assert_eq!(editor.end((f32::NAN, 0.0)), original);
    assert_eq!(editor.confirmed(), original);
}

#[test]
fn invalid_dimensions_are_rejected_before_creating_editor() {
    assert!(NativeSelectionEdit::new(placement(), (0, 800)).is_err());
    assert!(NativeSelectionEdit::new(placement(), (9000, 800)).is_err());
    assert!(NativeSelectionEdit::new(placement(), (8192, 8192)).is_err());
}
