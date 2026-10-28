use shot_core::RgbaFrame;
use shot_core::display::OverlayPlacement;
use shot_core::native_snapshot::NativeSnapshot;
use shot_core::platform::DisplayId;

fn sample() -> NativeSnapshot {
    let placement =
        OverlayPlacement::new(DisplayId(42), (-1440.0, -100.0), (1440.0, 900.0)).unwrap();
    let image = RgbaFrame::new(
        2,
        2,
        vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 0, 0, 0, 0],
    )
    .unwrap();
    NativeSnapshot::new(placement, image).unwrap()
}

#[test]
fn frozen_snapshot_keeps_physical_pixels_and_separate_logical_placement() {
    let original = sample();
    let mut wire = Vec::new();
    original.write_to(&mut wire).unwrap();
    let decoded = NativeSnapshot::read_from(&mut wire.as_slice()).unwrap();
    assert_eq!(decoded.placement().display_id(), DisplayId(42));
    assert_eq!(decoded.placement().origin_pt(), (-1440.0, -100.0));
    assert_eq!(decoded.placement().size_pt(), (1440.0, 900.0));
    assert_eq!((decoded.image().width, decoded.image().height), (2, 2));
    assert_eq!(decoded.image().rgba, original.image().rgba);
    assert_eq!(&wire[..8], b"PPSNAP01");
}

#[test]
fn malformed_magic_and_truncated_rgba_fail_closed() {
    let mut wire = Vec::new();
    sample().write_to(&mut wire).unwrap();
    wire[0] = b'X';
    assert!(NativeSnapshot::read_from(&mut wire.as_slice()).is_err());
    wire[0] = b'P';
    wire.pop();
    assert!(NativeSnapshot::read_from(&mut wire.as_slice()).is_err());
}

#[test]
fn declared_byte_count_and_nonfinite_logical_coordinates_are_rejected() {
    let mut wire = Vec::new();
    sample().write_to(&mut wire).unwrap();
    wire[40..44].copy_from_slice(&0u32.to_le_bytes());
    assert!(NativeSnapshot::read_from(&mut wire.as_slice()).is_err());
    let mut malformed = Vec::new();
    sample().write_to(&mut malformed).unwrap();
    malformed[24..28].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(NativeSnapshot::read_from(&mut malformed.as_slice()).is_err());
}

#[test]
fn limits_are_checked_before_large_payload_allocation() {
    assert!(NativeSnapshot::validate_dimensions(0, 1).is_err());
    assert!(NativeSnapshot::validate_dimensions(8193, 1).is_err());
    assert!(NativeSnapshot::validate_dimensions(8192, 8192).is_err());
    assert!(NativeSnapshot::validate_dimensions(5120, 2880).is_ok());
    assert_eq!(NativeSnapshot::validate_dimensions(2, 2).unwrap(), 16);
    let mut wire = Vec::new();
    sample().write_to(&mut wire).unwrap();
    wire[16..20].copy_from_slice(&8193u32.to_le_bytes());
    assert!(NativeSnapshot::read_from(&mut wire.as_slice()).is_err());
}
