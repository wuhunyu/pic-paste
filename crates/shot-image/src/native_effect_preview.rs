//! Exact, bounded, screenshot-local previews for native redaction tools.
//!
//! The opt-in NSPanel only keeps CPU source bytes for modest snapshots. A
//! preview patch is made from the *same* PNG compositor and is only shown
//! when all committed marks are disjoint redactions; otherwise the UI must
//! show an honest outlined region rather than an incorrect composition.
use shot_core::RgbaFrame;
use shot_core::native_annotation::validate_marks;
use shot_core::native_markup::NativeMark;

use crate::composite::composite;
use crate::history::Annotation;
use crate::native_import::{NATIVE_BLUR_RADIUS, NATIVE_PIXELATE_BLOCK_SIZE};

pub const MAX_NATIVE_PREVIEW_SOURCE_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_NATIVE_PREVIEW_PIXELS: u64 = 1_000_000;

#[derive(Clone, Copy, Debug)]
struct EffectRoi {
    left: u32,
    top: u32,
    width: u32,
    height: u32,
}

fn effect_roi(mark: &NativeMark) -> Option<EffectRoi> {
    match mark {
        NativeMark::Pixelate {
            left,
            top,
            width,
            height,
        }
        | NativeMark::Blur {
            left,
            top,
            width,
            height,
        } => Some(EffectRoi {
            left: *left,
            top: *top,
            width: *width,
            height: *height,
        }),
        _ => None,
    }
}

fn intersects(a: EffectRoi, b: EffectRoi) -> bool {
    let ar = u64::from(a.left) + u64::from(a.width);
    let br = u64::from(b.left) + u64::from(b.width);
    let ab = u64::from(a.top) + u64::from(a.height);
    let bb = u64::from(b.top) + u64::from(b.height);
    u64::from(a.left) < br
        && u64::from(b.left) < ar
        && u64::from(a.top) < bb
        && u64::from(b.top) < ab
}

pub fn preview_source_supported(width: u32, height: u32) -> bool {
    width > 0
        && height > 0
        && u64::from(width) * u64::from(height) * 4 <= MAX_NATIVE_PREVIEW_SOURCE_BYTES
}

/// Conservative guarantee: every native patch agrees with the parent editor
/// only when effects do not overlap and there are no vector/text operations.
pub fn can_preview_native_effects(marks: &[NativeMark], pixels: (u32, u32)) -> bool {
    if marks.is_empty() || validate_marks(marks, pixels).is_err() {
        return false;
    }
    let mut total_pixels = 0u64;
    let mut rois = Vec::with_capacity(marks.len());
    for mark in marks {
        let Some(roi) = effect_roi(mark) else {
            return false;
        };
        total_pixels += u64::from(roi.width) * u64::from(roi.height);
        if total_pixels > MAX_NATIVE_PREVIEW_PIXELS
            || rois.iter().any(|&prior| intersects(roi, prior))
        {
            return false;
        }
        rois.push(roi);
    }
    true
}

#[derive(Debug)]
pub struct NativeEffectPatch {
    pub left: u32,
    pub top: u32,
    pub image: RgbaFrame,
}

/// Allocate and render one small ROI only (not a copy of the full display).
///
/// Both existing redaction effects use ROI-local edges: shifting the origin
/// to (0,0) leaves their block alignment and alpha-premultiplied blur intact.
pub fn render_native_effect_patch(
    source: &RgbaFrame,
    mark: &NativeMark,
) -> Result<NativeEffectPatch, String> {
    let roi = effect_roi(mark).ok_or("not a native redaction mark")?;
    validate_marks(std::slice::from_ref(mark), (source.width, source.height))?;
    let count = u64::from(roi.width) * u64::from(roi.height);
    if count > MAX_NATIVE_PREVIEW_PIXELS {
        return Err(String::from("native pixel preview exceeds 1MP budget"));
    }
    let row_bytes = usize::try_from(roi.width)
        .map_err(|error| error.to_string())?
        .checked_mul(4)
        .ok_or("invalid native preview row stride")?;
    let len = usize::try_from(count * 4).map_err(|error| error.to_string())?;
    let mut bytes = Vec::with_capacity(len);
    for y in roi.top..roi.top + roi.height {
        let begin = (y as usize * source.width as usize + roi.left as usize) * 4;
        bytes.extend_from_slice(&source.rgba[begin..begin + row_bytes]);
    }
    let cropped = RgbaFrame::new(roi.width, roi.height, bytes)
        .map_err(|error| format!("invalid cropped preview: {error:?}"))?;
    let local = match mark {
        NativeMark::Pixelate { .. } => Annotation::Pixelate {
            left: 0,
            top: 0,
            width: roi.width,
            height: roi.height,
            block_size: NATIVE_PIXELATE_BLOCK_SIZE,
        },
        NativeMark::Blur { .. } => Annotation::Blur {
            left: 0,
            top: 0,
            width: roi.width,
            height: roi.height,
            radius: NATIVE_BLUR_RADIUS,
        },
        _ => return Err(String::from("not a native redaction mark")),
    };
    let rendered = composite(&cropped, &[local]).map_err(str::to_owned)?;
    Ok(NativeEffectPatch {
        left: roi.left,
        top: roi.top,
        image: rendered,
    })
}
