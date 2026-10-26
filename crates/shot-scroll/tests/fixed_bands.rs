use shot_core::{RectPx, RgbaFrame, platform::ScrollAxis};
use shot_scroll::fixed_bands::FixedBands;
use shot_scroll::stitch::match_scroll_frame;
use shot_scroll::tile_store::TileStore;

fn region(x: i32, y: i32, width: u32, height: u32) -> RectPx {
    RectPx {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn vertical_header_and_footer_are_excluded_in_physical_pixels() {
    let bands = FixedBands {
        leading: 12,
        trailing: 9,
    };
    assert_eq!(
        bands
            .content_roi(region(-120, 35, 80, 100), ScrollAxis::Vertical)
            .unwrap(),
        region(-120, 47, 80, 79)
    );
}

#[test]
fn horizontal_fixed_sidebars_are_excluded_without_losing_negative_origin() {
    let bands = FixedBands {
        leading: 14,
        trailing: 10,
    };
    assert_eq!(
        bands
            .content_roi(region(-230, -40, 100, 60), ScrollAxis::Horizontal)
            .unwrap(),
        region(-216, -40, 76, 60)
    );
}

#[test]
fn no_fixed_bands_preserves_entire_capture_region() {
    let roi = region(-400, -100, 200, 160);
    for axis in [ScrollAxis::Vertical, ScrollAxis::Horizontal] {
        assert_eq!(FixedBands::default().content_roi(roi, axis).unwrap(), roi);
    }
}

#[test]
fn invalid_or_too_large_exclusions_fail_before_capture_starts() {
    let roi = region(10, 20, 64, 80);
    for bands in [
        FixedBands {
            leading: 60,
            trailing: 1,
        },
        FixedBands {
            leading: 40,
            trailing: 40,
        },
        FixedBands {
            leading: u32::MAX,
            trailing: 0,
        },
    ] {
        assert!(bands.content_roi(roi, ScrollAxis::Vertical).is_err());
    }
    assert!(
        FixedBands::default()
            .content_roi(region(0, 0, 23, 80), ScrollAxis::Vertical)
            .is_err()
    );
    assert!(
        FixedBands {
            leading: 5,
            trailing: 0
        }
        .content_roi(region(i32::MAX - 2, 0, 64, 80), ScrollAxis::Horizontal)
        .is_err()
    );
}

fn scene(width: u32, height: u32, offset: u32, axis: ScrollAxis, bands: FixedBands) -> RgbaFrame {
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let (at, limit) = match axis {
                ScrollAxis::Vertical => (y, height),
                ScrollAxis::Horizontal => (x, width),
            };
            let value = if at < bands.leading || at >= limit - bands.trailing {
                // Same-position chrome stays fixed even when the page scrolls.
                ((x * 11 + y * 17 + 109) % 251) as u8
            } else {
                let gx = x + if axis == ScrollAxis::Horizontal {
                    offset
                } else {
                    0
                };
                let gy = y + if axis == ScrollAxis::Vertical {
                    offset
                } else {
                    0
                };
                ((gx * gx * 7 + gy * gy * 13 + gx * gy * 11) % 251) as u8
            };
            rgba.extend_from_slice(&[value, value, value, 255]);
        }
    }
    RgbaFrame::new(width, height, rgba).unwrap()
}

fn extracted(full: &RgbaFrame, rect: RectPx) -> RgbaFrame {
    let mut rgba = Vec::new();
    for y in (rect.y as usize)..(rect.y as usize + rect.height as usize) {
        let start = (y * full.width as usize + rect.x as usize) * 4;
        rgba.extend_from_slice(&full.rgba[start..start + rect.width as usize * 4]);
    }
    RgbaFrame::new(rect.width, rect.height, rgba).unwrap()
}

#[test]
fn vertical_fixed_chrome_no_longer_pollutes_match_or_export() {
    let bands = FixedBands {
        leading: 12,
        trailing: 10,
    };
    let axis = ScrollAxis::Vertical;
    let roi = bands.content_roi(region(0, 0, 64, 100), axis).unwrap();
    let before = extracted(&scene(64, 100, 0, axis, bands), roi);
    let after = extracted(&scene(64, 100, 7, axis, bands), roi);
    assert_eq!(match_scroll_frame(&before, &after, axis), Some(7));
    let mut store = TileStore::new(&before, axis, &std::env::temp_dir()).unwrap();
    store.append(&after, 7).unwrap();
    assert_eq!(store.canvas(), (64, 85));
    let mut top = vec![0; 64 * 4];
    store.fill_row(0, &mut top).unwrap();
    assert_eq!(top, before.rgba[..64 * 4]);
    let mut bottom = vec![0; 64 * 4];
    store.fill_row(84, &mut bottom).unwrap();
    assert_eq!(bottom, after.rgba[(77 * 64 * 4)..(78 * 64 * 4)]);
}

#[test]
fn horizontal_fixed_sidebars_allow_trusted_stitching() {
    let bands = FixedBands {
        leading: 10,
        trailing: 8,
    };
    let axis = ScrollAxis::Horizontal;
    let roi = bands.content_roi(region(0, 0, 100, 64), axis).unwrap();
    let before = extracted(&scene(100, 64, 0, axis, bands), roi);
    let after = extracted(&scene(100, 64, 6, axis, bands), roi);
    assert_eq!(match_scroll_frame(&before, &after, axis), Some(6));
    let mut store = TileStore::new(&before, axis, &std::env::temp_dir()).unwrap();
    store.append(&after, 6).unwrap();
    assert_eq!(store.canvas(), (88, 64));
    let mut row = vec![0; 88 * 4];
    store.fill_row(10, &mut row).unwrap();
    assert_eq!(&row[..82 * 4], &before.rgba[10 * 82 * 4..11 * 82 * 4]);
    assert_eq!(
        &row[82 * 4..],
        &after.rgba[10 * 82 * 4 + 76 * 4..11 * 82 * 4]
    );
}

#[test]
fn blank_or_repeated_content_remains_untrusted_even_with_bands() {
    let axis = ScrollAxis::Vertical;
    let bands = FixedBands {
        leading: 8,
        trailing: 8,
    };
    let rect = bands.content_roi(region(0, 0, 64, 80), axis).unwrap();
    let full = RgbaFrame::new(64, 80, vec![128; 64 * 80 * 4]).unwrap();
    let content = extracted(&full, rect);
    assert_eq!(match_scroll_frame(&content, &content, axis), None);
}
