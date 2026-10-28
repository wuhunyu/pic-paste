use shot_core::RectPx;
use shot_core::display::OverlayPlacement;
use shot_core::native_annotation::NativeEditReply;
use shot_core::native_markup::{NativeMark, NativeMarkup, NativeMarkupTool};
use shot_core::native_selection::NativeSelectionReply;
use shot_core::platform::DisplayId;

#[test]
fn ellipse_gesture_projects_retina_pixels_and_supports_reverse_drag_and_undo() {
    let placement =
        OverlayPlacement::new(DisplayId(7), (-1440.0, -100.0), (100.0, 80.0)).unwrap();
    let mut markup = NativeMarkup::new(placement, (200, 160)).unwrap();
    markup.set_tool(NativeMarkupTool::Ellipse);
    assert!(markup.begin((30.0, 20.0)));
    markup.drag((10.0, 5.0));
    let expected = NativeMark::Ellipse {
        top_left: (20, 10),
        bottom_right: (60, 40),
    };
    assert_eq!(markup.draft(), Some(expected.clone()));
    assert!(markup.visible().is_empty());
    assert!(markup.end((10.0, 5.0)));
    assert_eq!(markup.visible(), &[expected]);
    assert!(markup.undo());
    assert!(markup.visible().is_empty());
}

#[test]
fn ellipse_rejects_zero_radii_and_clamps_drag_to_source_pixels() {
    let placement = OverlayPlacement::new(DisplayId(7), (0.0, 0.0), (100.0, 80.0)).unwrap();
    let mut markup = NativeMarkup::new(placement, (200, 160)).unwrap();
    markup.set_tool(NativeMarkupTool::Ellipse);
    assert!(markup.begin((10.0, 10.0)));
    assert!(!markup.end((10.0, 40.0)));
    assert!(markup.visible().is_empty());
    assert!(markup.begin((10.0, 10.0)));
    assert!(markup.end((10000.0, 10000.0)));
    assert_eq!(
        markup.visible(),
        &[NativeMark::Ellipse {
            top_left: (20, 20),
            bottom_right: (199, 159),
        }]
    );
}

#[test]
fn native_ellipse_ipc_roundtrips_with_new_tag_without_changing_legacy_selection() {
    let selected = NativeSelectionReply::Selected {
        display: DisplayId(7),
        rect: RectPx {
            x: 0,
            y: 0,
            width: 100,
            height: 80,
        },
    };
    let reply = NativeEditReply::Annotated {
        selection: selected,
        pixels: (200, 160),
        marks: vec![NativeMark::Ellipse {
            top_left: (20, 10),
            bottom_right: (60, 40),
        }],
    };
    let mut bytes = Vec::new();
    reply.write_to(&mut bytes).unwrap();
    assert_eq!(&bytes[..8], b"PPANN001");
    assert_eq!(bytes[46], 5);
    assert_eq!(
        NativeEditReply::read_from(&mut bytes.as_slice()).unwrap(),
        reply
    );

    let mut invalid = bytes;
    invalid[55..59].copy_from_slice(&20u32.to_le_bytes());
    assert!(NativeEditReply::read_from(&mut invalid.as_slice()).is_err());
}

#[test]
fn invalid_ellipse_cannot_be_written_or_imported() {
    for mark in [
        NativeMark::Ellipse {
            top_left: (30, 20),
            bottom_right: (30, 40),
        },
        NativeMark::Ellipse {
            top_left: (30, 20),
            bottom_right: (40, 20),
        },
        NativeMark::Ellipse {
            top_left: (30, 20),
            bottom_right: (200, 40),
        },
    ] {
        let reply = NativeEditReply::Annotated {
            selection: NativeSelectionReply::Selected {
                display: DisplayId(7),
                rect: RectPx {
                    x: 0,
                    y: 0,
                    width: 100,
                    height: 80,
                },
            },
            pixels: (200, 160),
            marks: vec![mark],
        };
        assert!(reply.write_to(&mut Vec::new()).is_err());
    }
}
