#![forbid(unsafe_code)]
mod advanced_composite;
pub mod annotation_tools;
mod export;
pub use export::{atomic_save_png, crop_rgba};
pub mod composite;
pub mod history;
pub mod pixel_tools;
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
/// Anti-alias transparent corners in the output copy, leaving source pixels intact.
pub fn rounded_alpha(frame: &mut RgbaFrame, requested_radius: u32) {
    let width = frame.width;
    let height = frame.height;
    let r = requested_radius.min(width.min(height) / 2);
    if r == 0 {
        return;
    }
    let rf = r as f32;
    let wf = width as f32;
    let hf = height as f32;
    for y in 0..height {
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
            let idx = ((y as usize * width as usize) + x as usize) * 4 + 3;
            frame.rgba[idx] = ((u32::from(frame.rgba[idx]) * coverage + 8) / 16) as u8;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
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
