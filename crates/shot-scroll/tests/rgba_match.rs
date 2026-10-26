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
    assert_eq!(
        match_scroll_frame(&flat, &flat, ScrollAxis::Vertical),
        None
    );
}

#[test]
fn different_dimensions_or_invalid_rgba_are_rejected() {
    let before = screenshot(32, 48, 0, 0);
    let bad = RgbaFrame {
        width: 32,
        height: 48,
        rgba: Vec::new(),
    };
    assert_eq!(match_scroll_frame(&before, &bad, ScrollAxis::Vertical), None);
    assert_eq!(
        match_scroll_frame(&before, &screenshot(32, 45, 0, 4), ScrollAxis::Vertical),
        None
    );
}
