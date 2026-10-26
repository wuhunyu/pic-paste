use shot_core::{RectPx, RgbaFrame};
use shot_image::{atomic_save_png, crop_rgba};
use std::time::{SystemTime, UNIX_EPOCH};

fn tiny() -> RgbaFrame {
    let pixels = (0..24).map(|x| x as u8).collect();
    RgbaFrame::new(3, 2, pixels).unwrap()
}

#[test]
fn crop_is_inclusive_at_left_top_and_exact_at_right_bottom() {
    let crop = crop_rgba(
        &tiny(),
        RectPx { x: 1, y: 0, width: 2, height: 2 },
    ).unwrap();
    assert_eq!((crop.width, crop.height), (2, 2));
    assert_eq!(crop.rgba, vec![
        4,5,6,7,8,9,10,11,
        16,17,18,19,20,21,22,23,
    ]);
}

#[test]
fn crop_rejects_outside_and_empty_regions() {
    let frame = tiny();
    for rect in [
        RectPx { x: -1, y: 0, width: 1, height: 1 },
        RectPx { x: 0, y: 0, width: 0, height: 1 },
        RectPx { x: 2, y: 1, width: 2, height: 1 },
        RectPx { x: i32::MAX, y: 0, width: u32::MAX, height: 1 },
    ] {
        assert!(crop_rgba(&frame, rect).is_err());
    }
}

#[test]
fn png_atomic_save_does_not_replace_previous_file_on_failure() {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("picpaste-export-{}-{nonce}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let output = dir.join("screenshot.png");
    std::fs::write(&output, b"original").unwrap();

    // Invalid RGBA payload fails *after* temporary file creation.
    let invalid = RgbaFrame { width: 2, height: 2, rgba: Vec::new() };
    assert!(atomic_save_png(&invalid, &output).is_err());
    assert_eq!(std::fs::read(&output).unwrap(), b"original");
    let entries = std::fs::read_dir(&dir).unwrap().count();
    assert_eq!(entries, 1, "failed save left temporary artifacts");

    assert!(atomic_save_png(&tiny(), &output).is_err(), "must not overwrite existing files");
    assert_eq!(std::fs::read(&output).unwrap(), b"original");
    let fresh = dir.join("fresh.png");
    atomic_save_png(&tiny(), &fresh).unwrap();
    let bytes = std::fs::read(&fresh).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    std::fs::remove_dir_all(dir).unwrap();
}
