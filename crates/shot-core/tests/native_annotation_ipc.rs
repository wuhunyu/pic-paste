use shot_core::RectPx;
use shot_core::native_annotation::NativeEditReply;
use shot_core::native_markup::{NativeMark, MAX_BRUSH_POINTS, MAX_MARKS, MAX_TEXT_BYTES};
use shot_core::native_selection::NativeSelectionReply;
use shot_core::platform::DisplayId;

fn selected() -> NativeSelectionReply {
    NativeSelectionReply::Selected {
        display: DisplayId(19),
        rect: RectPx { x: 2, y: 3, width: 20, height: 12 },
    }
}

fn sample() -> NativeEditReply {
    NativeEditReply::Annotated {
        selection: selected(),
        pixels: (40, 30),
        marks: vec![
            NativeMark::Brush { points: vec![(1, 2), (3, 4), (5, 6)] },
            NativeMark::Arrow { from: (10, 12), to: (3, 5) },
            NativeMark::Rectangle { top_left: (2, 3), bottom_right: (20, 22) },
            NativeMark::Text { at: (8, 9), content: String::from("中文 · World") },
        ],
    }
}

#[test]
fn old_selection_and_cancel_packets_stay_byte_for_byte_compatible() {
    for selection in [selected(), NativeSelectionReply::Cancelled { display: DisplayId(19) }] {
        let mut old = Vec::new();
        selection.write_to(&mut old).unwrap();
        let mut next = Vec::new();
        NativeEditReply::Legacy(selection).write_to(&mut next).unwrap();
        assert_eq!(old, next);
        assert_eq!(next.len(), 33);
        assert_eq!(NativeEditReply::read_from(&mut next.as_slice()).unwrap(),
            NativeEditReply::Legacy(selection));
    }
}

#[test]
fn annotated_packet_roundtrips_all_four_marks_and_utf8() {
    let sample = sample();
    let mut bytes = Vec::new();
    sample.write_to(&mut bytes).unwrap();
    assert_eq!(&bytes[..8], b"PPANN001");
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize, bytes.len() - 12);
    let decoded = NativeEditReply::read_from(&mut bytes.as_slice()).unwrap();
    assert_eq!(decoded, sample);
    let (rect, marks) = decoded.validated_in(DisplayId(19), (40, 30)).unwrap().unwrap();
    assert_eq!(rect, RectPx { x: 2, y: 3, width: 20, height: 12 });
    assert_eq!(marks.len(), 4);
    assert!(decoded.validated_in(DisplayId(18), (40, 30)).is_err());
    assert!(decoded.validated_in(DisplayId(19), (80, 60)).is_err());
}

#[test]
fn malformed_prefix_length_truncation_and_trailing_bytes_are_rejected() {
    let mut bytes = Vec::new();
    sample().write_to(&mut bytes).unwrap();
    for position in [0usize, 1, 8, 11, 12, 29, bytes.len() - 1] {
        assert!(NativeEditReply::read_from(&mut bytes[..position]).is_err(), "{position}");
    }
    let mut bad_magic = bytes.clone();
    bad_magic[0] = b'X';
    assert!(NativeEditReply::read_from(&mut bad_magic.as_slice()).is_err());
    let mut too_large = bytes.clone();
    too_large[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(NativeEditReply::read_from(&mut too_large.as_slice()).is_err());
    let mut bad_len = bytes.clone();
    let size = u32::from_le_bytes(bad_len[8..12].try_into().unwrap());
    bad_len[8..12].copy_from_slice(&(size + 1).to_le_bytes());
    assert!(NativeEditReply::read_from(&mut bad_len.as_slice()).is_err());
    let mut extra = bytes;
    extra.push(0);
    assert!(NativeEditReply::read_from(&mut extra.as_slice()).is_err());
}

#[test]
fn corrupt_tags_counts_points_and_utf8_are_rejected() {
    let mut bytes = Vec::new();
    sample().write_to(&mut bytes).unwrap();
    // Fixed packet payload header = display 8 + pixels 8 + rect 16 + mark count 2.
    let mark_start = 12 + 34;
    let mut tag = bytes.clone();
    tag[mark_start] = 0xff;
    assert!(NativeEditReply::read_from(&mut tag.as_slice()).is_err());
    let mut count = bytes.clone();
    count[12 + 32..12 + 34].copy_from_slice(&u16::MAX.to_le_bytes());
    assert!(NativeEditReply::read_from(&mut count.as_slice()).is_err());
    let mut brush = bytes.clone();
    brush[mark_start + 1..mark_start + 3].copy_from_slice(&u16::MAX.to_le_bytes());
    assert!(NativeEditReply::read_from(&mut brush.as_slice()).is_err());

    let mut wrong_point = bytes.clone();
    wrong_point[mark_start + 3..mark_start + 7].copy_from_slice(&40u32.to_le_bytes());
    assert!(NativeEditReply::read_from(&mut wrong_point.as_slice()).is_err());

    // The final UTF-8 multibyte suffix must remain valid.
    let mut invalid_text = bytes;
    let position = invalid_text.windows(3).position(|w| w == "中".as_bytes()).unwrap();
    invalid_text[position] = 0xff;
    assert!(NativeEditReply::read_from(&mut invalid_text.as_slice()).is_err());
}

#[test]
fn empty_or_out_of_bounds_commands_and_mismatched_selection_fail_closed() {
    for marks in [
        vec![NativeMark::Brush { points: vec![(0, 0)] }],
        vec![NativeMark::Arrow { from: (4, 5), to: (4, 5) }],
        vec![NativeMark::Rectangle { top_left: (2, 3), bottom_right: (2, 4) }],
        vec![NativeMark::Text { at: (0, 0), content: String::new() }],
        vec![NativeMark::Text { at: (40, 0), content: String::from("x") }],
    ] {
        let reply = NativeEditReply::Annotated { selection: selected(), pixels: (40, 30), marks };
        assert!(reply.write_to(&mut Vec::new()).is_err());
    }
    let wrong_rect = NativeEditReply::Annotated {
        selection: NativeSelectionReply::Selected {
            display: DisplayId(19),
            rect: RectPx { x: 21, y: 10, width: 20, height: 15 },
        },
        pixels: (40, 30),
        marks: vec![NativeMark::Text { at: (0, 0), content: String::from("x") }],
    };
    assert!(wrong_rect.write_to(&mut Vec::new()).is_err());
    let cancelled = NativeEditReply::Annotated {
        selection: NativeSelectionReply::Cancelled { display: DisplayId(19) },
        pixels: (40, 30),
        marks: vec![NativeMark::Text { at: (0, 0), content: String::from("x") }],
    };
    assert!(cancelled.write_to(&mut Vec::new()).is_err());
}

#[test]
fn upper_bounds_and_legacy_no_mark_fallback() {
    let mut marks = Vec::new();
    for _ in 0..MAX_MARKS {
        marks.push(NativeMark::Brush { points: vec![(1, 2); MAX_BRUSH_POINTS] });
    }
    let maxed = NativeEditReply::Annotated { selection: selected(), pixels: (40, 30), marks };
    let mut raw = Vec::new();
    maxed.write_to(&mut raw).unwrap();
    assert_eq!(NativeEditReply::read_from(&mut raw.as_slice()).unwrap(), maxed);

    let over = NativeEditReply::Annotated {
        selection: selected(), pixels: (40, 30),
        marks: vec![NativeMark::Text { at: (1, 1), content: "a".repeat(MAX_TEXT_BYTES + 1) }],
    };
    assert!(over.write_to(&mut Vec::new()).is_err());
    let no_marks = NativeEditReply::Annotated { selection: selected(), pixels: (40, 30), marks: vec![] };
    let mut bytes = Vec::new();
    no_marks.write_to(&mut bytes).unwrap();
    assert_eq!(&bytes[..8], b"PPSEL001");
}
