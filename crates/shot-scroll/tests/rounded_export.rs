use shot_core::{RgbaFrame, platform::ScrollAxis};
use shot_scroll::tile_store::TileStore;
use std::fs;
#[test]
fn vertical_rounded_export_keeps_rgb_and_masks_alpha() {
    let root = std::env::temp_dir().join(format!("picpaste-rounded-{}",std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let initial=RgbaFrame::new(8,6,vec![200;8*6*4]).unwrap();
    let mut store=TileStore::new(&initial,ScrollAxis::Vertical,&root).unwrap();
    store.append(&initial,2).unwrap();
    let mut bytes=Vec::new();
    store.encode_png_rounded(&mut bytes,3).unwrap();
    let mut decoder=png::Decoder::new(bytes.as_slice());
    decoder.set_transformations(png::Transformations::IDENTITY);
    let mut reader=decoder.read_info().unwrap();
    let mut output=vec![0;reader.output_buffer_size().unwrap()];
    let info=reader.next_frame(&mut output).unwrap();
    let mut expected=RgbaFrame::new(8,8,vec![200;8*8*4]).unwrap();
    shot_image::rounded_alpha(&mut expected,3);
    assert_eq!(&output[..info.buffer_size()],expected.rgba.as_slice());
    let saved=root.join("rounded.png");
    store.save_png_rounded_no_clobber(&saved,3).unwrap();
    assert!(store.save_png_rounded_no_clobber(&saved,3).is_err());
    drop(store);
    fs::remove_dir_all(root).unwrap();
}
