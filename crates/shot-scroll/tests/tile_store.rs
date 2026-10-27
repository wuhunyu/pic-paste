use shot_core::{RgbaFrame, platform::ScrollAxis};
use shot_scroll::matcher::{GrayFrame, match_horizontal};
use shot_scroll::tile_store::TileStore;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_TEST_ROOT: AtomicU64 = AtomicU64::new(0);

fn frame(width: u32, height: u32, offset_x: u32, offset_y: u32) -> RgbaFrame {
    let mut pixels = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let v = (((x + offset_x) * 19 + (y + offset_y) * 31) % 251) as u8;
            pixels.extend_from_slice(&[v, 255 - v, 128, 255]);
        }
    }
    RgbaFrame::new(width, height, pixels).unwrap()
}

fn root() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let serial = NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "picpaste-tile-test-{}-{nonce}-{serial}",
        std::process::id()
    ));
    fs::create_dir(&path).unwrap();
    path
}

#[test]
fn vertical_append_reads_exact_rows_without_duplicate_overlap() {
    let dir = root();
    {
        let mut store = TileStore::new(&frame(4, 4, 0, 0), ScrollAxis::Vertical, &dir).unwrap();
        store.append(&frame(4, 4, 0, 2), 2).unwrap();
        assert_eq!(store.canvas(), (4, 6));
        for y in 0..6 {
            let mut row = vec![0; 16];
            store.fill_row(y, &mut row).unwrap();
            assert_eq!(row, frame(4, 1, 0, y).rgba);
        }
        let mut bytes = Vec::new();
        store.encode_png(&mut bytes).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert!(fs::read_dir(store.working_dir()).unwrap().count() >= 2);
    }
    assert_eq!(
        fs::read_dir(&dir).unwrap().count(),
        0,
        "store must clean private tiles on drop"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn horizontal_append_joins_new_columns_with_correct_row_stride() {
    let dir = root();
    {
        let mut store = TileStore::new(&frame(4, 3, 0, 0), ScrollAxis::Horizontal, &dir).unwrap();
        store.append(&frame(4, 3, 2, 0), 2).unwrap();
        assert_eq!(store.canvas(), (6, 3));
        for y in 0..3 {
            let mut row = vec![0; 6 * 4];
            store.fill_row(y, &mut row).unwrap();
            assert_eq!(row, frame(6, 1, 0, y).rgba);
        }
    }
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn mismatch_and_no_scroll_do_not_mutate_trusted_anchor() {
    let dir = root();
    let mut store = TileStore::new(&frame(4, 4, 0, 0), ScrollAxis::Vertical, &dir).unwrap();
    assert!(store.append(&frame(4, 4, 0, 0), 0).is_err());
    assert!(store.append(&frame(4, 4, 0, 0), 4).is_err());
    assert!(store.append(&frame(3, 4, 0, 0), 2).is_err());
    assert_eq!(store.canvas(), (4, 4));
    assert_eq!(store.strip_count(), 1);
    drop(store);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn removing_a_trusted_strip_is_reversible_only_by_recapture() {
    let dir = root();
    let mut store = TileStore::new(&frame(4, 4, 0, 0), ScrollAxis::Vertical, &dir).unwrap();
    store.append(&frame(4, 4, 0, 2), 2).unwrap();
    assert!(store.undo_last().unwrap());
    assert_eq!(store.canvas(), (4, 4));
    assert!(!store.undo_last().unwrap());
    drop(store);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn horizontal_match_discovers_rightward_shift_and_rejects_flat_frames() {
    let sample = |offset: usize| {
        let data = (0..12)
            .flat_map(|y| {
                (0..30).map(move |x| {
                    let v = x + offset;
                    ((v * v * 17 + 31 * y + 9 * v * y) % 251) as u8
                })
            })
            .collect();
        GrayFrame::new(30, 12, data).unwrap()
    };
    assert_eq!(
        match_horizontal(&sample(0), &sample(4), 0.40, 1.0, 1.0)
            .unwrap()
            .displacement_px,
        4
    );
    let flat = GrayFrame::new(30, 12, vec![128; 360]).unwrap();
    assert!(match_horizontal(&flat, &flat, 0.40, 1.0, 1.0).is_none());
}

#[test]
fn exported_long_png_is_atomic_and_never_clobbers_an_existing_file() {
    let dir = root();
    let store = TileStore::new(&frame(4, 4, 0, 0), ScrollAxis::Vertical, &dir).unwrap();
    let existing = dir.join("existing.png");
    fs::write(&existing, b"untouched").unwrap();
    assert!(store.save_png_no_clobber(&existing).is_err());
    assert_eq!(fs::read(&existing).unwrap(), b"untouched");

    let png = dir.join("new-scroll.png");
    store.save_png_no_clobber(&png).unwrap();
    let bytes = fs::read(&png).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert!(store.save_png_no_clobber(&dir.join("bad.jpg")).is_err());
    assert!(
        fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .all(|file| !file.file_name().to_string_lossy().ends_with(".part"))
    );
    drop(store);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn trimming_multiple_vertical_tail_segments_preserves_exact_prefix_pixels() {
    let dir = root();
    let mut store = TileStore::new(&frame(4, 4, 0, 0), ScrollAxis::Vertical, &dir).unwrap();
    store.append(&frame(4, 4, 0, 2), 2).unwrap();
    store.append(&frame(4, 4, 0, 4), 2).unwrap();
    store.append(&frame(4, 4, 0, 6), 2).unwrap();
    assert_eq!(store.canvas(), (4, 10));
    assert_eq!(store.keep_first(2).unwrap(), 2);
    assert_eq!(store.strip_count(), 2);
    assert_eq!(store.canvas(), (4, 6));
    assert_eq!(fs::read_dir(store.working_dir()).unwrap().count(), 2);
    for y in 0..6 {
        let mut row = vec![0; 4 * 4];
        store.fill_row(y, &mut row).unwrap();
        assert_eq!(row, frame(4, 1, 0, y).rgba);
    }
    let png = dir.join("trimmed.png");
    store.save_png_no_clobber(&png).unwrap();
    assert_eq!(&fs::read(&png).unwrap()[..8], b"\x89PNG\r\n\x1a\n");
    drop(store);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn horizontal_prefix_trim_resets_dimensions_and_allows_new_append() {
    let dir = root();
    let mut store = TileStore::new(&frame(4, 3, 0, 0), ScrollAxis::Horizontal, &dir).unwrap();
    store.append(&frame(4, 3, 2, 0), 2).unwrap();
    store.append(&frame(4, 3, 4, 0), 2).unwrap();
    assert_eq!(store.keep_first(1).unwrap(), 2);
    assert_eq!(store.canvas(), (4, 3));
    store.append(&frame(4, 3, 2, 0), 2).unwrap();
    assert_eq!(store.canvas(), (6, 3));
    for y in 0..3 {
        let mut row = vec![0; 6 * 4];
        store.fill_row(y, &mut row).unwrap();
        assert_eq!(row, frame(6, 1, 0, y).rgba);
    }
    drop(store);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn invalid_prefix_trim_does_not_modify_files_or_dimensions() {
    let dir = root();
    let mut store = TileStore::new(&frame(4, 4, 0, 0), ScrollAxis::Vertical, &dir).unwrap();
    store.append(&frame(4, 4, 0, 2), 2).unwrap();
    for count in [0, 3, usize::MAX] {
        assert!(store.keep_first(count).is_err());
        assert_eq!(store.strip_count(), 2);
        assert_eq!(store.canvas(), (4, 6));
        assert_eq!(fs::read_dir(store.working_dir()).unwrap().count(), 2);
    }
    assert_eq!(store.keep_first(2).unwrap(), 0);
    assert_eq!(store.keep_first(1).unwrap(), 1);
    assert_eq!(store.canvas(), (4, 4));
    drop(store);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn trimmed_middle_retake_restores_last_native_viewport_and_can_reappend() {
    let dir = root();
    let mut store = TileStore::new(&frame(12, 32, 0, 0), ScrollAxis::Vertical, &dir).unwrap();
    store.append(&frame(12, 32, 0, 7), 7).unwrap();
    store.append(&frame(12, 32, 0, 15), 8).unwrap();
    store.append(&frame(12, 32, 0, 22), 7).unwrap();
    assert_eq!(store.segment_span(2), Some((32, 39)));
    assert_eq!(store.segment_span(3), Some((39, 47)));
    assert_eq!(store.segment_span(5), None);
    assert_eq!(store.keep_first(2).unwrap(), 2);
    assert_eq!(
        store.last_viewport().unwrap().rgba,
        frame(12, 32, 0, 7).rgba
    );
    assert_eq!(store.canvas(), (12, 39));
    store.append(&frame(12, 32, 0, 13), 6).unwrap();
    assert_eq!(store.canvas(), (12, 45));
    for y in 0..45 {
        let mut row = vec![0; 12 * 4];
        store.fill_row(y, &mut row).unwrap();
        assert_eq!(row, frame(12, 1, 0, y).rgba);
    }
    drop(store);
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn horizontal_middle_retake_anchor_is_exact_and_does_not_keep_untrusted_suffix() {
    let dir = root();
    let mut store = TileStore::new(&frame(32, 12, 0, 0), ScrollAxis::Horizontal, &dir).unwrap();
    store.append(&frame(32, 12, 8, 0), 8).unwrap();
    store.append(&frame(32, 12, 16, 0), 8).unwrap();
    assert_eq!(store.segment_span(2), Some((32, 40)));
    assert_eq!(store.keep_first(2).unwrap(), 1);
    assert_eq!(
        store.last_viewport().unwrap().rgba,
        frame(32, 12, 8, 0).rgba
    );
    store.append(&frame(32, 12, 13, 0), 5).unwrap();
    assert_eq!(store.canvas(), (45, 12));
    for y in 0..12 {
        let mut row = vec![0; 45 * 4];
        store.fill_row(y, &mut row).unwrap();
        assert_eq!(row, frame(45, 1, 0, y).rgba);
    }
    drop(store);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn native_retake_anchor_remains_available_after_hundreds_of_trusted_segments() {
    let dir = root();
    let mut store = TileStore::new(&frame(24, 32, 0, 0), ScrollAxis::Vertical, &dir).unwrap();
    for index in 1..=260 {
        store.append(&frame(24, 32, 0, index * 2), 2).unwrap();
    }
    assert_eq!(store.strip_count(), 261);
    assert_eq!(store.last_viewport().unwrap().rgba, frame(24, 32, 0, 520).rgba);
    drop(store);
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    fs::remove_dir_all(dir).unwrap();
}
