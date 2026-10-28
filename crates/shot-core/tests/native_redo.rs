use shot_core::RectPx;
use shot_core::display::OverlayPlacement;
use shot_core::native_annotation::NativeEditReply;
use shot_core::native_markup::{NativeMark, NativeMarkup, NativeMarkupTool};
use shot_core::native_selection::NativeSelectionReply;
use shot_core::platform::DisplayId;

fn markup() -> NativeMarkup {
    NativeMarkup::new(
        OverlayPlacement::new(DisplayId(12), (-960.0, 0.0), (100.0, 80.0)).unwrap(),
        (200, 160),
    )
    .unwrap()
}

#[test]
fn redo_recovers_committed_marks_and_new_branch_invalidates_future() {
    let mut m = markup();
    assert!(!m.redo());
    assert!(!m.undo());
    m.set_tool(NativeMarkupTool::Rectangle);
    assert!(m.begin((5.0, 5.0)));
    assert!(m.end((15.0, 15.0)));
    assert!(m.begin((20.0, 20.0)));
    assert!(m.end((30.0, 30.0)));
    let history = m.visible().to_vec();
    assert_eq!(history.len(), 2);
    assert!(m.undo());
    assert_eq!(m.visible(), &history[..1]);
    assert!(m.undo());
    assert!(m.visible().is_empty());
    assert!(!m.undo());
    assert!(m.redo());
    assert_eq!(m.visible(), &history[..1]);
    assert!(m.redo());
    assert_eq!(m.visible(), history);
    assert!(!m.redo());

    assert!(m.undo());
    assert!(m.begin((40.0, 25.0)));
    assert!(m.end((50.0, 35.0)));
    assert!(!m.redo(), "a new committed mark forks history");
    assert_eq!(m.visible().len(), 2);
    assert_ne!(m.visible()[1], history[1]);
}

#[test]
fn redo_cancels_in_progress_draft_but_invalid_drag_keeps_history() {
    let mut m = markup();
    m.set_tool(NativeMarkupTool::Arrow);
    assert!(m.begin((5.0, 5.0)));
    assert!(m.end((30.0, 20.0)));
    let original = m.visible().to_vec();
    assert!(m.undo());
    assert!(m.begin((9.0, 9.0)));
    assert!(!m.end((9.0, 9.0)), "zero-length gesture is not a commit");
    assert!(m.redo(), "failed gesture must not discard redo");
    assert_eq!(m.visible(), original);
    assert!(m.undo());
    assert!(m.begin((1.0, 1.0)));
    m.drag((20.0, 20.0));
    assert!(m.draft().is_some());
    assert!(m.redo());
    assert!(m.draft().is_none(), "redo must cancel an uncommitted draft");
    assert_eq!(m.visible(), original);
}

#[test]
fn text_edit_redo_preserves_utf8_and_protocol_only_exports_visible_history() {
    let mut m = markup();
    m.set_tool(NativeMarkupTool::Text);
    assert!(m.place_text((10.0, 12.0), "中文 Input"));
    let saved = m.visible().to_vec();
    assert!(m.undo());
    assert!(m.visible().is_empty());
    assert!(m.redo());
    assert_eq!(m.visible(), saved);

    let reply = NativeEditReply::Annotated {
        selection: NativeSelectionReply::Selected {
            display: DisplayId(12),
            rect: RectPx {
                x: 0,
                y: 0,
                width: 80,
                height: 60,
            },
        },
        pixels: (200, 160),
        marks: m.visible().to_vec(),
    };
    let mut bytes = Vec::new();
    reply.write_to(&mut bytes).unwrap();
    assert_eq!(
        NativeEditReply::read_from(&mut bytes.as_slice()).unwrap(),
        reply
    );
    assert!(matches!(&m.visible()[0], NativeMark::Text {content, ..} if content == "中文 Input"));

    assert!(m.undo());
    assert!(m.place_text((10.0, 12.0), "新分支"));
    assert!(!m.redo());
    assert!(matches!(&m.visible()[0], NativeMark::Text {content, ..} if content == "新分支"));
}
