use shot_core::{RgbaFrame, platform::ScrollAxis};
use shot_scroll::stitch::match_scroll_frame;

fn screenshot(width: u32, height: u32, offset_x: u32, offset_y: u32) -> RgbaFrame {
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let gx = x + offset_x;
            let gy = y + offset_y;
            let value = ((gx * gx * 7 + gy * gy * 13 + gx * gy * 11) % 251) as u8;
            rgba.extend_from_slice(&[value, value, value, 255]);
        }
    }
    RgbaFrame::new(width, height, rgba).unwrap()
}

#[test]
fn matches_vertical_physical_pixel_shift_without_misplacing_rows() {
    let before = screenshot(32, 48, 0, 0);
    let after = screenshot(32, 48, 0, 7);
    assert_eq!(
        match_scroll_frame(&before, &after, ScrollAxis::Vertical),
        Some(7)
    );
}

#[test]
fn matches_horizontal_physical_pixel_shift_without_misplacing_columns() {
    let before = screenshot(48, 24, 0, 0);
    let after = screenshot(48, 24, 5, 0);
    assert_eq!(
        match_scroll_frame(&before, &after, ScrollAxis::Horizontal),
        Some(5)
    );
}

#[test]
fn unchanged_or_low_information_frames_are_never_appended() {
    let frame = screenshot(32, 48, 0, 0);
    assert_eq!(
        match_scroll_frame(&frame, &frame, ScrollAxis::Vertical),
        None
    );
    let flat = RgbaFrame::new(32, 48, vec![140; 32 * 48 * 4]).unwrap();
    assert_eq!(match_scroll_frame(&flat, &flat, ScrollAxis::Vertical), None);
}

#[test]
fn different_dimensions_or_invalid_rgba_are_rejected() {
    let before = screenshot(32, 48, 0, 0);
    let bad = RgbaFrame {
        width: 32,
        height: 48,
        rgba: Vec::new(),
    };
    assert_eq!(
        match_scroll_frame(&before, &bad, ScrollAxis::Vertical),
        None
    );
    assert_eq!(
        match_scroll_frame(&before, &screenshot(32, 45, 0, 4), ScrollAxis::Vertical),
        None
    );
}

fn animate_cross_axis_band(frame: &mut RgbaFrame, axis: ScrollAxis, percent: u32) {
    let cross_size = match axis {
        ScrollAxis::Vertical => frame.width,
        ScrollAxis::Horizontal => frame.height,
    };
    let limit = cross_size * percent / 100;
    for y in 0..frame.height {
        for x in 0..frame.width {
            let cross = match axis {
                ScrollAxis::Vertical => x,
                ScrollAxis::Horizontal => y,
            };
            if cross < limit {
                let i = (y as usize * frame.width as usize + x as usize) * 4;
                let v = ((x * 37 + y * 59 + 113) % 251) as u8;
                frame.rgba[i] = v;
                frame.rgba[i + 1] = v.wrapping_mul(3);
                frame.rgba[i + 2] = v.wrapping_add(47);
            }
        }
    }
}

#[test]
fn accepts_scrolling_with_small_animated_side_band_in_both_directions() {
    let mut vertical = screenshot(80, 96, 0, 11);
    animate_cross_axis_band(&mut vertical, ScrollAxis::Vertical, 20);
    assert_eq!(
        match_scroll_frame(&screenshot(80, 96, 0, 0), &vertical, ScrollAxis::Vertical),
        Some(11)
    );

    let mut horizontal = screenshot(96, 80, 9, 0);
    animate_cross_axis_band(&mut horizontal, ScrollAxis::Horizontal, 20);
    assert_eq!(
        match_scroll_frame(
            &screenshot(96, 80, 0, 0),
            &horizontal,
            ScrollAxis::Horizontal
        ),
        Some(9)
    );
}

#[test]
fn accepts_scrolling_with_small_animated_full_width_banner() {
    let mut next = screenshot(80, 96, 0, 9);
    for y in 20..36 {
        for x in 0..next.width {
            let i = (y as usize * next.width as usize + x as usize) * 4;
            next.rgba[i] = ((x * 29 + y * 37 + 91) % 251) as u8;
            next.rgba[i + 1] = 13;
            next.rgba[i + 2] = 204;
        }
    }
    assert_eq!(
        match_scroll_frame(&screenshot(80, 96, 0, 0), &next, ScrollAxis::Vertical),
        Some(9)
    );
}

#[test]
fn rejects_large_dynamic_regions_animation_only_and_repeated_wallpaper() {
    let baseline = screenshot(80, 96, 0, 0);
    let mut changed = screenshot(80, 96, 0, 9);
    animate_cross_axis_band(&mut changed, ScrollAxis::Vertical, 50);
    assert_eq!(
        match_scroll_frame(&baseline, &changed, ScrollAxis::Vertical),
        None
    );

    let mut no_motion = baseline.clone();
    animate_cross_axis_band(&mut no_motion, ScrollAxis::Vertical, 20);
    assert_eq!(
        match_scroll_frame(&baseline, &no_motion, ScrollAxis::Vertical),
        None
    );

    let wallpaper = |offset: u32| {
        let mut pixels = Vec::new();
        for y in offset..offset + 96 {
            for x in 0..80 {
                let v = ((y % 8 * 31 + x * 13) % 251) as u8;
                pixels.extend_from_slice(&[v, v, v, 255]);
            }
        }
        RgbaFrame::new(80, 96, pixels).unwrap()
    };
    assert_eq!(
        match_scroll_frame(&wallpaper(0), &wallpaper(8), ScrollAxis::Vertical),
        None
    );
}
