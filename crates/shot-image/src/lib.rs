#![forbid(unsafe_code)]
mod advanced_composite;
pub mod annotation_tools;
mod export;
mod text_composite;
pub use export::{atomic_save_png, crop_rgba};
pub mod composite;
pub mod history;
pub mod native_effect_preview;
pub mod native_import;
pub mod pixel_tools;
pub mod preview;
pub mod stream_png;
use shot_core::RgbaFrame;
use std::io::Write;

/// PNG-only lossless RGBA export.
pub fn encode_png<W: Write>(frame: &RgbaFrame, sink: W) -> Result<(), png::EncodingError> {
    let mut encoder = png::Encoder::new(sink, frame.width, frame.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&frame.rgba)?;
    writer.finish()?;
    Ok(())
}
/// Apply exactly the same anti-aliased rounded-corner mask to one RGBA row.
/// This allows multi-gigapixel long captures to be encoded without full RGBA allocation.
pub fn rounded_alpha_row(row: &mut [u8], width: u32, height: u32, y: u32, requested_radius: u32) {
    let r = requested_radius.min(width.min(height) / 2);
    if r == 0 || y >= height || row.len() != width as usize * 4 {
        return;
    }
    let rf = r as f32;
    let wf = width as f32;
    let hf = height as f32;
    for x in 0..width {
        let cx = x as f32 + 0.5;
        let cy = y as f32 + 0.5;
        if (cx >= rf && cx <= wf - rf) || (cy >= rf && cy <= hf - rf) {
            continue;
        }
        let mut coverage = 0u32;
        for sy in 0..4 {
            for sx in 0..4 {
                let px = x as f32 + (sx as f32 + 0.5) / 4.0;
                let py = y as f32 + (sy as f32 + 0.5) / 4.0;
                let qx = if px < rf {
                    rf - px
                } else if px > wf - rf {
                    px - (wf - rf)
                } else {
                    0.0
                };
                let qy = if py < rf {
                    rf - py
                } else if py > hf - rf {
                    py - (hf - rf)
                } else {
                    0.0
                };
                if qx * qx + qy * qy <= rf * rf {
                    coverage += 1;
                }
            }
        }
        let idx = x as usize * 4 + 3;
        row[idx] = ((u32::from(row[idx]) * coverage + 8) / 16) as u8;
    }
}

/// Anti-alias transparent corners in the output copy, leaving source pixels intact.
pub fn rounded_alpha(frame: &mut RgbaFrame, requested_radius: u32) {
    let row_bytes = frame.width as usize * 4;
    for (y, row) in frame.rgba.chunks_exact_mut(row_bytes).enumerate() {
        rounded_alpha_row(row, frame.width, frame.height, y as u32, requested_radius);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rounded_rows_match_full_image_exactly() {
        let mut expected = RgbaFrame::new(8, 6, vec![200; 8 * 6 * 4]).unwrap();
        rounded_alpha(&mut expected, 3);
        let mut rows = vec![200; 8 * 6 * 4];
        for (y, row) in rows.as_chunks_mut::<32>().0.iter_mut().enumerate() {
            rounded_alpha_row(row, 8, 6, y as u32, 3);
        }
        assert_eq!(rows, expected.rgba);
        assert!(rows[3] < 200);
        assert_eq!(rows[0], 200);
    }

    #[test]
    fn encodes_png_signature() {
        let src = RgbaFrame::new(2, 2, vec![255; 16]).unwrap();
        let mut dst = Vec::new();
        encode_png(&src, &mut dst).unwrap();
        assert_eq!(&dst[..8], b"\x89PNG\r\n\x1a\n");
    }
    #[test]
    fn rounds_corners_but_preserves_center() {
        let mut src = RgbaFrame::new(8, 8, vec![255; 8 * 8 * 4]).unwrap();
        rounded_alpha(&mut src, 4);
        assert!(src.rgba[3] < 255);
        assert_eq!(src.rgba[(4 * 8 + 4) * 4 + 3], 255);
    }
}
