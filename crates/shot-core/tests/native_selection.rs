use shot_core::RectPx;
use shot_core::display::OverlayPlacement;
use shot_core::native_selection::{AppKitSelectionFrame, NativeSelection, NativeSelectionReply};
use shot_core::platform::DisplayId;

fn placement() -> OverlayPlacement {
    OverlayPlacement::new(DisplayId(47), (-1440.0, -100.0), (1440.0, 900.0)).unwrap()
}

#[test]
fn primary_quartz_top_origin_maps_to_appkit_bottom_origin_without_retina_scaling() {
    let frame = AppKitSelectionFrame::from_quartz(placement(), 900.0).unwrap();
    assert_eq!((frame.x, frame.y), (-1440.0, 100.0));
    assert_eq!((frame.width, frame.height), (1440.0, 900.0));
    let upper = OverlayPlacement::new(DisplayId(48), (0.0, -900.0), (1440.0, 900.0)).unwrap();
    let frame = AppKitSelectionFrame::from_quartz(upper, 900.0).unwrap();
    assert_eq!((frame.x, frame.y), (0.0, 900.0));
    assert!(AppKitSelectionFrame::from_quartz(placement(), f64::NAN).is_err());
}

#[test]
fn selection_handles_reverse_drag_in_local_physical_pixels_not_desktop_origin() {
    let mut state = NativeSelection::new(placement(), (2880, 1800)).unwrap();
    assert!(state.press((100.0, 80.0)));
    state.drag((10.0, 20.0));
    assert_eq!(state.preview_rect_points(), Some((10.0, 20.0, 90.0, 60.0)));
    assert_eq!(
        state.release((10.0, 20.0)),
        Some(RectPx {
            x: 20,
            y: 40,
            width: 180,
            height: 120
        })
    );
}

#[test]
fn selection_clamps_out_of_bounds_drag_and_rejects_click_without_area() {
    let mut state = NativeSelection::new(placement(), (2880, 1800)).unwrap();
    assert!(!state.press((-10.0, 4.0)));
    assert_eq!(state.release((12.0, 12.0)), None);
    assert!(state.press((10.0, 20.0)));
    state.drag((9999.0, 9999.0));
    assert_eq!(
        state.release((9999.0, 9999.0)),
        Some(RectPx {
            x: 20,
            y: 40,
            width: 2860,
            height: 1760,
        })
    );
    assert!(state.press((20.0, 20.0)));
    assert_eq!(state.release((20.0, 20.0)), None);
}

#[test]
fn selection_preserves_single_physical_pixel_when_drag_is_subpoint() {
    let mut state = NativeSelection::new(placement(), (2880, 1800)).unwrap();
    assert!(state.press((1.25, 1.25)));
    assert_eq!(
        state.release((1.5, 1.5)),
        Some(RectPx {
            x: 2,
            y: 2,
            width: 1,
            height: 1
        })
    );
}

#[test]
fn selection_reply_roundtrips_and_checks_active_display_and_bounds() {
    let reply = NativeSelectionReply::Selected {
        display: DisplayId(47),
        rect: RectPx {
            x: 20,
            y: 40,
            width: 180,
            height: 120,
        },
    };
    let mut data = Vec::new();
    reply.write_to(&mut data).unwrap();
    assert_eq!(data.len(), 33);
    assert_eq!(&data[..8], b"PPSEL001");
    assert_eq!(
        NativeSelectionReply::read_from(&mut data.as_slice()).unwrap(),
        reply
    );
    assert_eq!(
        reply.validated_in(DisplayId(47), (2880, 1800)).unwrap(),
        Some(RectPx {
            x: 20,
            y: 40,
            width: 180,
            height: 120
        })
    );
    assert!(reply.validated_in(DisplayId(1), (2880, 1800)).is_err());
    assert!(reply.validated_in(DisplayId(47), (100, 100)).is_err());
}

#[test]
fn cancel_reply_preserves_display_id_and_never_creates_selection() {
    let reply = NativeSelectionReply::Cancelled {
        display: DisplayId(47),
    };
    let mut bytes = Vec::new();
    reply.write_to(&mut bytes).unwrap();
    assert_eq!(
        NativeSelectionReply::read_from(&mut bytes.as_slice()).unwrap(),
        reply
    );
    assert_eq!(
        reply.validated_in(DisplayId(47), (2880, 1800)).unwrap(),
        None
    );
    assert!(reply.validated_in(DisplayId(1), (2880, 1800)).is_err());
}

#[test]
fn wire_rejects_empty_invalid_truncated_unknown_status_and_trailing_bytes() {
    let reply = NativeSelectionReply::Selected {
        display: DisplayId(47),
        rect: RectPx {
            x: 1,
            y: 2,
            width: 3,
            height: 4,
        },
    };
    let mut data = Vec::new();
    reply.write_to(&mut data).unwrap();
    assert!(NativeSelectionReply::read_from(&mut &[][..]).is_err());
    let mut truncated = data.clone();
    truncated.pop();
    assert!(NativeSelectionReply::read_from(&mut truncated.as_slice()).is_err());
    let mut bad_magic = data.clone();
    bad_magic[0] = b'X';
    assert!(NativeSelectionReply::read_from(&mut bad_magic.as_slice()).is_err());
    let mut bad_status = data.clone();
    bad_status[8] = 3;
    assert!(NativeSelectionReply::read_from(&mut bad_status.as_slice()).is_err());
    let mut trailing = data.clone();
    trailing.push(0);
    assert!(NativeSelectionReply::read_from(&mut trailing.as_slice()).is_err());
    let mut empty = data;
    empty[25..29].copy_from_slice(&0u32.to_le_bytes());
    assert!(NativeSelectionReply::read_from(&mut empty.as_slice()).is_err());
}

#[test]
fn rejects_invalid_window_and_pixel_sizes_without_panic() {
    let geometry = OverlayPlacement::new(DisplayId(1), (0.0, 0.0), (1440.0, 900.0)).unwrap();
    assert!(NativeSelection::new(geometry, (0, 1)).is_err());
    assert!(NativeSelection::new(geometry, (9000, 1)).is_err());
}
