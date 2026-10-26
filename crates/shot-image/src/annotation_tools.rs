use crate::history::Annotation;

/// Tools that are implemented in the lightweight built-in raster compositor.
/// This list does not imply OCR or text/blur tools are already usable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tool {
    #[default]
    Select,
    Line,
    Arrow,
    Brush,
    Rectangle,
    Ellipse,
    Pixelate,
    Blur,
}

pub fn annotation_for_drag(
    tool: Tool,
    start: (i32, i32),
    end: (i32, i32),
) -> Option<Annotation> {
    if tool == Tool::Select {
        return None;
    }
    if [start.0, start.1, end.0, end.1]
        .iter()
        .any(|component| *component < 0)
    {
        return None;
    }
    let color = [39, 178, 231, 255];
    let from = (start.0 as f32, start.1 as f32);
    let to = (end.0 as f32, end.1 as f32);
    match tool {
        Tool::Select => None,
        Tool::Line => Some(Annotation::Line {
            from,
            to,
            rgba: color,
            width: 3.0,
        }),
        Tool::Arrow => Some(Annotation::Arrow {
            from,
            to,
            rgba: color,
            width: 3.0,
        }),
        Tool::Brush => Some(Annotation::Brush {
            points: vec![from, to],
            rgba: color,
            width: 3.0,
        }),
        Tool::Ellipse => {
            let center = ((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0);
            let radius = ((from.0 - to.0).abs() / 2.0, (from.1 - to.1).abs() / 2.0);
            if radius.0 < 1.0 || radius.1 < 1.0 {
                return None;
            }
            Some(Annotation::Ellipse {
                center,
                radius,
                rgba: color,
                width: 3.0,
            })
        }
        Tool::Rectangle => Some(Annotation::Rectangle {
            top_left: from,
            bottom_right: to,
            rgba: color,
            width: 3.0,
        }),
        Tool::Pixelate | Tool::Blur => {
            let dx = (i64::from(start.0) - i64::from(end.0)).unsigned_abs() + 1;
            let dy = (i64::from(start.1) - i64::from(end.1)).unsigned_abs() + 1;
            let left = start.0.min(end.0) as u32;
            let top = start.1.min(end.1) as u32;
            let width = u32::try_from(dx).ok()?;
            let height = u32::try_from(dy).ok()?;
            if tool == Tool::Blur {
                Some(Annotation::Blur { left, top, width, height, radius: 6 })
            } else {
                Some(Annotation::Pixelate { left, top, width, height, block_size: 12 })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_coordinates_do_not_produce_unbounded_masks() {
        assert!(annotation_for_drag(Tool::Pixelate, (-1, 1), (10, 20)).is_none());
        assert!(annotation_for_drag(Tool::Line, (1, -1), (10, 20)).is_none());
    }
}
