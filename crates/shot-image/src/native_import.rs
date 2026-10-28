//! Validated native physical-pixel marks into existing screenshot history.
//! All input is checked before the first command is applied, so a malformed
//! trailing mark cannot partially alter the current editor.
use shot_core::native_annotation::validate_marks;
use shot_core::native_markup::NativeMark;

use crate::history::{Annotation, AnnotationDocument};

const CYAN: [u8; 4] = [39, 178, 231, 255];
const STROKE: f32 = 3.0;
const TEXT_SIZE: f32 = 26.0;

/// Add one validated native batch to the existing undo/redo history.
/// Returned commands are full-frame source-local physical coordinates,
/// never a window origin, logical AppKit point, or cropped PNG offset.
pub fn import_native_marks(
    document: &mut AnnotationDocument,
    marks: &[NativeMark],
) -> Result<usize, String> {
    validate_marks(marks, (document.source.width, document.source.height))?;
    let annotations: Vec<Annotation> = marks
        .iter()
        .map(|mark| match mark {
            NativeMark::Brush { points } => Annotation::Brush {
                points: points.iter().map(|&(x, y)| (x as f32, y as f32)).collect(),
                rgba: CYAN,
                width: STROKE,
            },
            NativeMark::Arrow { from, to } => Annotation::Arrow {
                from: (from.0 as f32, from.1 as f32),
                to: (to.0 as f32, to.1 as f32),
                rgba: CYAN,
                width: STROKE,
            },
            NativeMark::Rectangle {
                top_left,
                bottom_right,
            } => Annotation::Rectangle {
                top_left: (top_left.0 as f32, top_left.1 as f32),
                bottom_right: (bottom_right.0 as f32, bottom_right.1 as f32),
                rgba: CYAN,
                width: STROKE,
            },
            NativeMark::Ellipse {
                top_left,
                bottom_right,
            } => Annotation::Ellipse {
                center: (
                    (top_left.0 as f32 + bottom_right.0 as f32) / 2.0,
                    (top_left.1 as f32 + bottom_right.1 as f32) / 2.0,
                ),
                radius: (
                    (bottom_right.0 - top_left.0) as f32 / 2.0,
                    (bottom_right.1 - top_left.1) as f32 / 2.0,
                ),
                rgba: CYAN,
                width: STROKE,
            },
            NativeMark::Text { at, content } => Annotation::Text {
                at: (at.0 as f32, at.1 as f32),
                content: content.clone(),
                size: TEXT_SIZE,
                rgba: CYAN,
            },
        })
        .collect();

    let count = annotations.len();
    for command in annotations {
        document.apply(command);
    }
    Ok(count)
}
