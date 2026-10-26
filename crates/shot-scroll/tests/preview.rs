use shot_core::{RgbaFrame, platform::ScrollAxis};
use shot_scroll::tile_store::TileStore;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn frame(width: u32, height: u32, ox: u32, oy: u32) -> RgbaFrame {
    let mut data = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let r = (x + ox) as u8;
            let g = (y + oy) as u8;
            data.extend_from_slice(&[r, g, 91, 255]);
        }
    }
    RgbaFrame::new(width, height, data).unwrap()
}

static TEST_DIR_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn temp_dir() -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let serial = TEST_DIR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "picpaste-preview-test-{}-{stamp}-{serial}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    root
}

#[test]
fn vertical_review_preview_preserves_pixel_rows_and_is_not_full_canvas_only() {
    let root = temp_dir();
    let mut store = TileStore::new(&frame(4, 4, 0, 0), ScrollAxis::Vertical, &root).unwrap();
    store.append(&frame(4, 4, 0, 2), 2).unwrap();
    let full = store.preview(4, 6).unwrap();
    assert_eq!((full.width, full.height), (4, 6));
    assert_eq!(full.rgba, frame(4, 6, 0, 0).rgba);
    let bounded = store.preview(2, 3).unwrap();
    assert_eq!((bounded.width, bounded.height), (2, 3));
    assert_eq!(&bounded.rgba[0..4], &[0, 0, 91, 255]);
    assert_eq!(
        &bounded.rgba[(2 * 2 * 4)..(2 * 2 * 4 + 4)],
        &[0, 4, 91, 255]
    );
    drop(store);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn horizontal_preview_has_correct_joined_row_stride_and_bounded_size() {
    let root = temp_dir();
    let mut store = TileStore::new(&frame(4, 3, 0, 0), ScrollAxis::Horizontal, &root).unwrap();
    store.append(&frame(4, 3, 2, 0), 2).unwrap();
    let full = store.preview(6, 3).unwrap();
    assert_eq!((full.width, full.height), (6, 3));
    assert_eq!(full.rgba, frame(6, 3, 0, 0).rgba);
    let bounded = store.preview(2, 2).unwrap();
    assert_eq!((bounded.width, bounded.height), (2, 1));
    assert_eq!(&bounded.rgba[4..8], &[3, 0, 91, 255]);
    assert!(store.preview(0, 8).is_err());
    drop(store);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn review_undo_updates_preview_without_touching_the_original_source() {
    let root = temp_dir();
    let mut store = TileStore::new(&frame(4, 4, 0, 0), ScrollAxis::Vertical, &root).unwrap();
    store.append(&frame(4, 4, 0, 2), 2).unwrap();
    assert!(store.undo_last().unwrap());
    assert_eq!(store.preview(8, 8).unwrap().rgba, frame(4, 4, 0, 0).rgba);
    assert_eq!(store.strip_count(), 1);
    drop(store);
    fs::remove_dir_all(root).unwrap();
}
