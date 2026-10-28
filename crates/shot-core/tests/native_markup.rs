use shot_core::display::OverlayPlacement;
use shot_core::native_markup::{NativeMark, NativeMarkup, NativeMarkupTool};
use shot_core::platform::DisplayId;

fn markup() -> NativeMarkup {
    let placement =
        OverlayPlacement::new(DisplayId(7), (-1440.0, -100.0), (100.0, 80.0)).unwrap();
    NativeMarkup::new(placement, (200, 160)).unwrap()
}

#[test]
fn brush_is_a_non_destructive_physical_pixel_gesture() {
    let mut marks = markup();
    assert_eq!(marks.tool(), NativeMarkupTool::Select);
    assert!(!marks.begin((1.0, 2.0)));
    marks.set_tool(NativeMarkupTool::Brush);
    assert!(marks.begin((1.0, 2.0)));
    marks.drag((2.0, 3.0));
    marks.drag((3.0, 4.0));
    assert!(marks.visible().is_empty(), "draft must not commit before release");
    assert!(matches!(marks.draft(), Some(NativeMark::Brush { .. })));
    assert!(marks.end((3.0, 4.0)));
    assert_eq!(
        marks.visible(),
        &[NativeMark::Brush {
            points: vec![(2, 4), (4, 6), (6, 8)],
        }]
    );
    assert!(marks.undo());
    assert!(marks.visible().is_empty());
    assert!(!marks.undo());
}

#[test]
fn arrows_and_rectangles_are_bounded_and_reverse_drag_safe() {
    let mut marks = markup();
    marks.set_tool(NativeMarkupTool::Arrow);
    assert!(marks.begin((20.0, 25.0)));
    assert!(marks.end((10.0, 5.0)));
    assert_eq!(
        marks.visible(),
        &[NativeMark::Arrow {
            from: (40, 50),
            to: (20, 10),
        }]
    );
    marks.set_tool(NativeMarkupTool::Rectangle);
    assert!(marks.begin((30.0, 30.0)));
    assert!(marks.end((5.0, 10.0)));
    assert_eq!(
        marks.visible()[1],
        NativeMark::Rectangle {
            top_left: (10, 20),
            bottom_right: (60, 60),
        }
    );
    assert_eq!(marks.visible().len(), 2);
}

#[test]
fn negative_origin_is_not_multiplied_by_retina_scale() {
    let mut marks = markup();
    marks.set_tool(NativeMarkupTool::Brush);
    assert!(marks.begin((99.5, 79.5)));
    assert!(marks.end((100.0, 80.0)));
    let NativeMark::Brush { points } = &marks.visible()[0] else {
        panic!("expected brush");
    };
    assert_eq!(points[0], (199, 159));
    assert_eq!(points[1], (199, 159));
}

#[test]
fn text_requires_nonempty_bounded_utf8_and_valid_point() {
    let mut marks = markup();
    marks.set_tool(NativeMarkupTool::Text);
    assert!(!marks.place_text((2.0, 3.0), "  "));
    assert!(!marks.place_text((2.0, 3.0), &"中".repeat(90)));
    assert!(!marks.place_text((-1.0, 2.0), "你好"));
    assert!(!marks.place_text((f32::NAN, 3.0), "你好"));
    assert!(marks.place_text((2.0, 3.0), "你好 · 例子"));
    assert_eq!(
        marks.visible(),
        &[NativeMark::Text {
            at: (4, 6),
            content: String::from("你好 · 例子"),
        }]
    );
    assert!(marks.undo());
}

#[test]
fn invalid_drags_do_not_create_marks_or_leak_a_stale_draft() {
    let mut marks = markup();
    marks.set_tool(NativeMarkupTool::Rectangle);
    assert!(!marks.begin((-1.0, 1.0)));
    assert!(!marks.begin((f32::INFINITY, 1.0)));
    assert!(marks.begin((20.0, 20.0)));
    assert!(!marks.end((20.0, 20.0)));
    assert!(marks.visible().is_empty());
    assert!(marks.begin((5.0, 5.0)));
    marks.drag((10.0, 10.0));
    marks.set_tool(NativeMarkupTool::Select);
    assert!(marks.draft().is_none());
    assert!(!marks.end((10.0, 10.0)));
    assert!(marks.visible().is_empty());
}

#[test]
fn brush_point_and_command_limits_fail_closed() {
    let mut marks = markup();
    marks.set_tool(NativeMarkupTool::Brush);
    assert!(marks.begin((1.0, 1.0)));
    for i in 0..4000 {
        let x = (i % 100) as f32;
        let y = (i / 100) as f32;
        marks.drag((x, y));
    }
    assert!(marks.end((99.0, 70.0)));
    let NativeMark::Brush { points } = &marks.visible()[0] else {
        panic!("expected bounded brush");
    };
    assert!(points.len() <= 2048);
    marks.set_tool(NativeMarkupTool::Text);
    for _ in 1..128 {
        assert!(marks.place_text((1.0, 1.0), "x"));
    }
    assert_eq!(marks.visible().len(), 128);
    assert!(!marks.place_text((1.0, 1.0), "overflow"));
    assert!(marks.undo());
    assert!(marks.place_text((1.0, 1.0), "again"));
}

#[test]
fn invalid_display_dimensions_are_rejected() {
    let placement = OverlayPlacement::new(DisplayId(1), (0.0, 0.0), (100.0, 80.0)).unwrap();
    assert!(NativeMarkup::new(placement, (0, 160)).is_err());
    assert!(NativeMarkup::new(placement, (9000, 160)).is_err());
}
