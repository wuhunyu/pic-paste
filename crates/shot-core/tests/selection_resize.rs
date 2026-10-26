use shot_core::RectPx;
use shot_core::selection::{ResizeHandle, adjust_selection, hit_selection_handle};

fn selection() -> RectPx {
    RectPx {
        x: 20,
        y: 30,
        width: 40,
        height: 20,
    }
}

#[test]
fn corners_and_edges_resize_in_physical_pixels() {
    assert_eq!(
        adjust_selection(selection(), ResizeHandle::TopLeft, (-10, -5), (100, 100)).unwrap(),
        RectPx {
            x: 10,
            y: 25,
            width: 50,
            height: 25
        }
    );
    assert_eq!(
        adjust_selection(selection(), ResizeHandle::Right, (-100, 0), (100, 100)).unwrap(),
        RectPx {
            x: 20,
            y: 30,
            width: 1,
            height: 20
        }
    );
    assert_eq!(
        adjust_selection(selection(), ResizeHandle::BottomRight, (90, 90), (100, 100)).unwrap(),
        RectPx {
            x: 20,
            y: 30,
            width: 80,
            height: 70
        }
    );
}

#[test]
fn selection_move_is_clamped_and_size_preserved() {
    assert_eq!(
        adjust_selection(selection(), ResizeHandle::Move, (300, 300), (100, 100)).unwrap(),
        RectPx {
            x: 60,
            y: 80,
            width: 40,
            height: 20
        }
    );
    assert_eq!(
        adjust_selection(selection(), ResizeHandle::Move, (-400, -400), (100, 100)).unwrap(),
        RectPx {
            x: 0,
            y: 0,
            width: 40,
            height: 20
        }
    );
}

#[test]
fn handle_hit_testing_prefers_corners_and_does_not_select_outside() {
    assert_eq!(
        hit_selection_handle(selection(), (20, 30), 3),
        Some(ResizeHandle::TopLeft)
    );
    assert_eq!(
        hit_selection_handle(selection(), (40, 30), 3),
        Some(ResizeHandle::Top)
    );
    assert_eq!(
        hit_selection_handle(selection(), (59, 49), 3),
        Some(ResizeHandle::BottomRight)
    );
    assert_eq!(
        hit_selection_handle(selection(), (36, 40), 3),
        Some(ResizeHandle::Move)
    );
    assert_eq!(hit_selection_handle(selection(), (72, 40), 3), None);
}

#[test]
fn invalid_selection_and_canvas_are_rejected_without_wraparound() {
    let invalid = RectPx {
        x: -1,
        y: 0,
        width: 5,
        height: 5,
    };
    assert!(adjust_selection(invalid, ResizeHandle::Move, (1, 1), (100, 100)).is_err());
    assert!(adjust_selection(selection(), ResizeHandle::Move, (1, 1), (0, 100)).is_err());
    assert!(adjust_selection(selection(), ResizeHandle::Move, (1, 1), (30, 100)).is_err());
}
