use shot_core::{RgbaFrame, platform::ScrollAxis};
use shot_image::history::Annotation;
use shot_scroll::edits::EditStack;
use shot_scroll::tile_store::TileStore;
use std::{fs, io::Cursor};

fn source(height: u32, offset: u32) -> RgbaFrame {
    let mut rgba = Vec::new();
    for y in offset..offset + height {
        for x in 0..8 {
            let v = ((x * 23 + y * 17) % 251) as u8;
            rgba.extend_from_slice(&[v, 255 - v, x as u8 * 14, 255]);
        }
    }
    RgbaFrame::new(8, height, rgba).unwrap()
}

fn decode(bytes: Vec<u8>) -> RgbaFrame {
    let mut reader = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
    let mut rgba = vec![0; reader.output_buffer_size().unwrap()];
    let frame = reader.next_frame(&mut rgba).unwrap();
    RgbaFrame::new(
        frame.width,
        frame.height,
        rgba[..frame.buffer_size()].to_vec(),
    )
    .unwrap()
}

#[test]
fn overlapping_vector_pixelate_and_blur_match_full_composite_byte_for_byte() {
    let root = std::env::temp_dir().join(format!("picpaste-edits-{}-overlap", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let mut store = TileStore::new(&source(8, 0), ScrollAxis::Vertical, &root).unwrap();
    store.append(&source(8, 4), 4).unwrap();
    let ops = [
        Annotation::Line {
            from: (0.0, 0.0),
            to: (7.0, 11.0),
            rgba: [200, 0, 0, 255],
            width: 2.0,
        },
        Annotation::Pixelate {
            left: 2,
            top: 3,
            width: 4,
            height: 5,
            block_size: 2,
        },
        Annotation::Blur {
            left: 1,
            top: 4,
            width: 5,
            height: 5,
            radius: 2,
        },
    ];
    let mut edits = EditStack::default();
    for op in ops.iter().cloned() {
        edits.apply(&store, op).unwrap();
    }
    let mut png = Vec::new();
    store.encode_png_with_edits(&mut png, 0, &edits).unwrap();
    let expected = shot_image::composite::composite(&source(12, 0), &ops).unwrap();
    assert_eq!(decode(png).rgba, expected.rgba);
    assert_eq!(
        store.preview_with_edits(&edits, 8, 12).unwrap().rgba,
        expected.rgba
    );

    assert!(edits.undo());
    assert!(edits.redo());
    assert_eq!(edits.applied_len(), 3);
    assert!(edits.undo());
    edits
        .apply(
            &store,
            Annotation::Rectangle {
                top_left: (2.0, 2.0),
                bottom_right: (5.0, 6.0),
                rgba: [0, 255, 0, 255],
                width: 2.0,
            },
        )
        .unwrap();
    assert!(!edits.redo());
    assert_eq!(edits.applied_len(), 3);
    let png_path = root.join("edited.png");
    store
        .save_png_edited_no_clobber(&png_path, 3, &edits)
        .unwrap();
    assert!(
        store
            .save_png_edited_no_clobber(&png_path, 3, &edits)
            .is_err()
    );
    assert_eq!(fs::read(&png_path).unwrap()[..8], *b"\x89PNG\r\n\x1a\n");
    drop(edits);
    drop(store);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn horizontal_edits_and_native_selection_match_the_full_canvas() {
    let root =
        std::env::temp_dir().join(format!("picpaste-edits-{}-horizontal", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let frame = |offset: u32, width: u32| {
        let mut rgba = Vec::new();
        for y in 0..8 {
            for x in offset..offset + width {
                let v = ((x * 19 + y * 31) % 251) as u8;
                rgba.extend_from_slice(&[v, 255 - v, 61, 255]);
            }
        }
        RgbaFrame::new(width, 8, rgba).unwrap()
    };
    let mut store = TileStore::new(&frame(0, 8), ScrollAxis::Horizontal, &root).unwrap();
    store.append(&frame(4, 8), 4).unwrap();
    let mark = Annotation::Arrow {
        from: (2.0, 2.0),
        to: (10.0, 5.0),
        rgba: [200, 40, 100, 255],
        width: 2.0,
    };
    let mut edits = EditStack::default();
    edits.apply(&store, mark.clone()).unwrap();
    let mut png = Vec::new();
    store.encode_png_with_edits(&mut png, 0, &edits).unwrap();
    let expected = shot_image::composite::composite(&frame(0, 12), &[mark]).unwrap();
    assert_eq!(decode(png).rgba, expected.rgba);

    let rect = shot_core::RectPx {
        x: 4,
        y: 1,
        width: 5,
        height: 4,
    };
    let actual = store.render_region_edited(&edits, rect, 2).unwrap();
    let mut cropped = shot_image::crop_rgba(&expected, rect).unwrap();
    shot_image::rounded_alpha(&mut cropped, 2);
    assert_eq!(actual.rgba, cropped.rgba);
    assert!(
        store
            .render_region_edited(
                &edits,
                shot_core::RectPx {
                    x: 0,
                    y: 0,
                    width: 13,
                    height: 8
                },
                0,
            )
            .is_err()
    );

    let before = edits.applied_len();
    assert!(
        edits
            .apply(
                &store,
                Annotation::Pixelate {
                    left: 1,
                    top: 1,
                    width: 5,
                    height: 5,
                    block_size: 0,
                }
            )
            .is_err()
    );
    assert_eq!(edits.applied_len(), before);

    drop(edits);
    drop(store);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    fs::remove_dir_all(root).unwrap();
}
