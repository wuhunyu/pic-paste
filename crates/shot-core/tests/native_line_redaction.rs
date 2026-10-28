use shot_core::RectPx;
use shot_core::display::OverlayPlacement;
use shot_core::native_annotation::NativeEditReply;
use shot_core::native_markup::{NativeMark, NativeMarkup, NativeMarkupTool};
use shot_core::native_selection::NativeSelectionReply;
use shot_core::platform::DisplayId;

fn markup() -> NativeMarkup {
    NativeMarkup::new(
        OverlayPlacement::new(DisplayId(9), (-1440.0, 0.0), (2000.0, 1000.0)).unwrap(),
        (4000, 2000),
    )
    .unwrap()
}

#[test]
fn line_uses_source_local_retina_pixels_and_reverse_drag() {
    let mut m = markup();
    m.set_tool(NativeMarkupTool::Line);
    assert!(m.begin((30.0, 20.0)));
    assert_eq!(m.draft(), None);
    m.drag((10.0, 5.0));
    assert_eq!(
        m.draft(),
        Some(NativeMark::Line {
            from: (60, 40),
            to: (20, 10),
        })
    );
    assert!(m.end((10.0, 5.0)));
    assert_eq!(
        m.visible(),
        &[NativeMark::Line {
            from: (60, 40),
            to: (20, 10),
        }]
    );
    assert!(m.undo());
    assert!(m.redo());
    assert_eq!(m.visible().len(), 1);
    assert!(m.begin((5.0, 5.0)));
    assert!(!m.end((5.0, 5.0)));
    assert_eq!(m.visible().len(), 1);
}

#[test]
fn pixelate_and_blur_are_bounded_half_open_source_rois() {
    let mut m = markup();
    m.set_tool(NativeMarkupTool::Pixelate);
    assert!(m.begin((30.0, 20.0)));
    m.drag((10.0, 5.0));
    assert_eq!(
        m.draft(),
        Some(NativeMark::Pixelate {
            left: 20,
            top: 10,
            width: 41,
            height: 31,
        })
    );
    assert!(m.end((10.0, 5.0)));
    m.set_tool(NativeMarkupTool::Blur);
    assert!(m.begin((5.0, 8.0)));
    assert!(m.end((12.0, 18.0)));
    assert_eq!(
        m.visible()[1],
        NativeMark::Blur {
            left: 10,
            top: 16,
            width: 15,
            height: 21,
        }
    );
}

#[test]
fn four_megapixel_limit_does_not_destroy_redo_history_on_invalid_gesture() {
    let mut m = markup();
    m.set_tool(NativeMarkupTool::Pixelate);
    assert!(m.begin((0.0, 0.0)));
    assert!(m.end((999.5, 999.5)), "2000 x 2000 is allowed");
    assert!(m.undo());
    assert!(m.visible().is_empty());
    m.set_tool(NativeMarkupTool::Blur);
    assert!(m.begin((0.0, 0.0)));
    assert!(!m.end((1000.0, 999.5)), "2001 x 2000 exceeds 4MP");
    assert!(m.visible().is_empty());
    assert!(m.redo(), "rejected gesture must keep redo");
    assert!(matches!(m.visible()[0], NativeMark::Pixelate { .. }));
}

fn selection() -> NativeSelectionReply {
    NativeSelectionReply::Selected {
        display: DisplayId(9),
        rect: RectPx {
            x: 0,
            y: 0,
            width: 120,
            height: 100,
        },
    }
}

#[test]
fn new_tags_roundtrip_without_changing_ppann001_version() {
    for (mark, tag) in [
        (
            NativeMark::Line {
                from: (2, 3),
                to: (42, 24),
            },
            6u8,
        ),
        (
            NativeMark::Pixelate {
                left: 5,
                top: 9,
                width: 60,
                height: 50,
            },
            7u8,
        ),
        (
            NativeMark::Blur {
                left: 7,
                top: 11,
                width: 60,
                height: 50,
            },
            8u8,
        ),
    ] {
        let reply = NativeEditReply::Annotated {
            selection: selection(),
            pixels: (4000, 2000),
            marks: vec![mark],
        };
        let mut bytes = Vec::new();
        reply.write_to(&mut bytes).unwrap();
        assert_eq!(&bytes[..8], b"PPANN001");
        assert_eq!(bytes[46], tag);
        assert_eq!(
            NativeEditReply::read_from(&mut bytes.as_slice()).unwrap(),
            reply
        );
        assert!(reply.validated_in(DisplayId(10), (4000, 2000)).is_err());
    }
}

#[test]
fn invalid_effect_geometry_and_corrupted_wire_fail_closed() {
    for mark in [
        NativeMark::Pixelate {
            left: 2,
            top: 3,
            width: 0,
            height: 10,
        },
        NativeMark::Blur {
            left: 3999,
            top: 3,
            width: 2,
            height: 10,
        },
        NativeMark::Pixelate {
            left: 0,
            top: 0,
            width: 4000,
            height: 2000,
        },
        NativeMark::Blur {
            left: u32::MAX,
            top: 0,
            width: 30,
            height: 30,
        },
        NativeMark::Line {
            from: (4, 5),
            to: (4, 5),
        },
    ] {
        let reply = NativeEditReply::Annotated {
            selection: selection(),
            pixels: (4000, 2000),
            marks: vec![mark],
        };
        assert!(reply.write_to(&mut Vec::new()).is_err());
    }
    let mut wire = Vec::new();
    NativeEditReply::Annotated {
        selection: selection(),
        pixels: (4000, 2000),
        marks: vec![NativeMark::Blur {
            left: 12,
            top: 14,
            width: 25,
            height: 35,
        }],
    }
    .write_to(&mut wire)
    .unwrap();
    wire[55..59].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(NativeEditReply::read_from(&mut wire.as_slice()).is_err());
}
