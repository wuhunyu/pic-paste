//! Explicit OCR line/word picking in physical image pixels, without new OCR
//! inference on pointer hover. Apple Vision supplies quadrilaterals.
use crate::OcrDocument;

fn inside_quad(quad: &[[f32; 2]; 4], point: (f32, f32)) -> bool {
    if !point.0.is_finite() || !point.1.is_finite() {
        return false;
    }
    let mut winding = 0i8;
    let mut double_area = 0.0_f32;
    for index in 0..4 {
        let a = quad[index];
        let b = quad[(index + 1) % 4];
        let cross = (b[0] - a[0]) * (point.1 - a[1]) - (b[1] - a[1]) * (point.0 - a[0]);
        if !cross.is_finite() {
            return false;
        }
        double_area += a[0] * b[1] - b[0] * a[1];
        if cross > 0.001 {
            if winding == -1 {
                return false;
            }
            winding = 1;
        } else if cross < -0.001 {
            if winding == 1 {
                return false;
            }
            winding = -1;
        }
    }
    double_area.abs() > 0.001
}

fn local_point(
    document: &OcrDocument,
    roi_origin: (u32, u32),
    source_pixel: (f32, f32),
) -> Option<(f32, f32)> {
    let x = source_pixel.0 - roi_origin.0 as f32;
    let y = source_pixel.1 - roi_origin.1 as f32;
    if !x.is_finite()
        || !y.is_finite()
        || x < 0.0
        || y < 0.0
        || x > document.pixel_width as f32
        || y > document.pixel_height as f32
    {
        return None;
    }
    Some((x, y))
}

/// Pick only on an explicit click, after OCR has already completed. The
/// origin is the image-local top-left of the region sent to the helper.
pub fn hit_line_in_capture(
    document: &OcrDocument,
    roi_origin: (u32, u32),
    source_pixel: (f32, f32),
) -> Option<usize> {
    let local = local_point(document, roi_origin, source_pixel)?;
    document
        .lines
        .iter()
        .position(|line| inside_quad(&line.polygon_px, local))
}

/// Word-level selection is enabled only when a provider supplied word boxes.
/// Never invent single-character positions from line bounding boxes.
pub fn hit_word_in_capture(
    document: &OcrDocument,
    line_index: usize,
    roi_origin: (u32, u32),
    source_pixel: (f32, f32),
) -> Option<usize> {
    let local = local_point(document, roi_origin, source_pixel)?;
    document
        .lines
        .get(line_index)?
        .words
        .iter()
        .position(|word| inside_quad(&word.polygon_px, local))
}

/// Copy selected lines once each, preserving the OCR provider's reading order.
pub fn selected_text(document: &OcrDocument, chosen: &[usize]) -> String {
    document
        .lines
        .iter()
        .enumerate()
        .filter(|(index, _)| chosen.contains(index))
        .map(|(_, line)| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
