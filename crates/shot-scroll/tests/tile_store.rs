use shot_core::{RgbaFrame, platform::ScrollAxis};
use shot_scroll::matcher::{GrayFrame, match_horizontal};
use shot_scroll::tile_store::TileStore;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

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
    let path =
        std::env::temp_dir().join(format!("picpaste-tile-test-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
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
