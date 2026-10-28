//! Bounded one-shot native annotation reply over the authenticated sibling's
//! anonymous stdout pipe. PPSEL001 stays byte-for-byte compatible for older
//! child binaries, cancellations and selections with no native marks.
use std::io::{Read, Write};

use crate::RectPx;
use crate::native_markup::{
    MAX_BRUSH_POINTS, MAX_MARKS, MAX_NATIVE_EFFECT_PIXELS, MAX_TEXT_BYTES, NativeMark,
};
use crate::native_selection::NativeSelectionReply;
use crate::native_snapshot::NativeSnapshot;
use crate::platform::DisplayId;

const MAGIC: &[u8; 8] = b"PPANN001";
/// 128 brushes × (2,048 physical pixel points × 8 bytes) plus framing.
pub const MAX_ANNOTATION_BODY_BYTES: usize = 2_100_000;
const MIN_BODY_BYTES: usize = 34;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeEditReply {
    Legacy(NativeSelectionReply),
    Annotated {
        selection: NativeSelectionReply,
        pixels: (u32, u32),
        marks: Vec<NativeMark>,
    },
}

/// Validate *all* marks before they are written, imported or composited.
/// These are capture-local physical pixels; never apply desktop offsets.
pub fn validate_marks(marks: &[NativeMark], pixels: (u32, u32)) -> Result<(), String> {
    NativeSnapshot::validate_dimensions(pixels.0, pixels.1)?;
    if marks.len() > MAX_MARKS {
        return Err(String::from("native annotation count exceeds limit"));
    }
    let inside = |point: &(u32, u32)| point.0 < pixels.0 && point.1 < pixels.1;
    for mark in marks {
        match mark {
            NativeMark::Brush { points } => {
                if points.len() < 2
                    || points.len() > MAX_BRUSH_POINTS
                    || !points.iter().all(&inside)
                {
                    return Err(String::from("invalid native brush points"));
                }
            }
            NativeMark::Line { from, to } | NativeMark::Arrow { from, to } => {
                if !inside(from) || !inside(to) || from == to {
                    return Err(String::from("invalid native arrow"));
                }
            }
            NativeMark::Rectangle {
                top_left,
                bottom_right,
            }
            | NativeMark::Ellipse {
                top_left,
                bottom_right,
            } => {
                if !inside(top_left)
                    || !inside(bottom_right)
                    || top_left.0 >= bottom_right.0
                    || top_left.1 >= bottom_right.1
                {
                    return Err(String::from("invalid native rectangle"));
                }
            }
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
            } => {
                let valid = *width > 0
                    && *height > 0
                    && left.checked_add(*width).is_some_and(|right| right <= pixels.0)
                    && top.checked_add(*height).is_some_and(|bottom| bottom <= pixels.1)
                    && u64::from(*width) * u64::from(*height) <= MAX_NATIVE_EFFECT_PIXELS;
                if !valid {
                    return Err(String::from("invalid native redaction region"));
                }
            }
            NativeMark::Text { at, content } => {
                if !inside(at) || content.trim().is_empty() || content.len() > MAX_TEXT_BYTES {
                    return Err(String::from("invalid native text"));
                }
            }
        }
    }
    Ok(())
}

impl NativeEditReply {
    pub fn validated_in(
        &self,
        expected_display: DisplayId,
        pixels: (u32, u32),
    ) -> Result<Option<(RectPx, &[NativeMark])>, String> {
        let (selection, source_pixels, marks): (&NativeSelectionReply, _, &[NativeMark]) =
            match self {
                Self::Legacy(selection) => (selection, pixels, &[]),
                Self::Annotated {
                    selection,
                    pixels,
                    marks,
                } => (selection, *pixels, marks.as_slice()),
            };
        if source_pixels != pixels {
            return Err(String::from("native annotation source dimensions mismatch"));
        }
        let result = selection.validated_in(expected_display, pixels)?;
        validate_marks(marks, pixels)?;
        match result {
            Some(rect) => Ok(Some((rect, marks))),
            None if marks.is_empty() => Ok(None),
            None => Err(String::from("cancelled native reply included annotations")),
        }
    }

    pub fn write_to<W: Write>(&self, sink: &mut W) -> Result<(), String> {
        match self {
            Self::Legacy(selection) => selection.write_to(sink),
            Self::Annotated {
                selection,
                pixels,
                marks,
            } => {
                if marks.is_empty() {
                    return selection.write_to(sink);
                }
                let Some((rect, _)) = self.validated_in(
                    match selection {
                        NativeSelectionReply::Selected { display, .. }
                        | NativeSelectionReply::Cancelled { display } => *display,
                    },
                    *pixels,
                )?
                else {
                    return Err(String::from("native cancellation cannot transport marks"));
                };
                let display = match selection {
                    NativeSelectionReply::Selected { display, .. } => *display,
                    NativeSelectionReply::Cancelled { .. } => {
                        return Err(String::from("native cancellation cannot transport marks"));
                    }
                };
                let mut body = Vec::new();
                body.extend_from_slice(&display.0.to_le_bytes());
                body.extend_from_slice(&pixels.0.to_le_bytes());
                body.extend_from_slice(&pixels.1.to_le_bytes());
                body.extend_from_slice(&rect.x.to_le_bytes());
                body.extend_from_slice(&rect.y.to_le_bytes());
                body.extend_from_slice(&rect.width.to_le_bytes());
                body.extend_from_slice(&rect.height.to_le_bytes());
                body.extend_from_slice(&(marks.len() as u16).to_le_bytes());
                for mark in marks {
                    match mark {
                        NativeMark::Brush { points } => {
                            body.push(1);
                            body.extend_from_slice(&(points.len() as u16).to_le_bytes());
                            for &(x, y) in points {
                                write_point(&mut body, (x, y));
                            }
                        }
                        NativeMark::Arrow { from, to } | NativeMark::Line { from, to } => {
                            body.push(if matches!(mark, NativeMark::Line { .. }) {
                                6
                            } else {
                                2
                            });
                            write_point(&mut body, *from);
                            write_point(&mut body, *to);
                        }
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
                        } => {
                            body.push(if matches!(mark, NativeMark::Pixelate { .. }) {
                                7
                            } else {
                                8
                            });
                            for value in [left, top, width, height] {
                                body.extend_from_slice(&value.to_le_bytes());
                            }
                        }
                        NativeMark::Rectangle {
                            top_left,
                            bottom_right,
                        } => {
                            body.push(3);
                            write_point(&mut body, *top_left);
                            write_point(&mut body, *bottom_right);
                        }
                        NativeMark::Ellipse {
                            top_left,
                            bottom_right,
                        } => {
                            body.push(5);
                            write_point(&mut body, *top_left);
                            write_point(&mut body, *bottom_right);
                        }
                        NativeMark::Text { at, content } => {
                            body.push(4);
                            write_point(&mut body, *at);
                            body.extend_from_slice(&(content.len() as u16).to_le_bytes());
                            body.extend_from_slice(content.as_bytes());
                        }
                    }
                    if body.len() > MAX_ANNOTATION_BODY_BYTES {
                        return Err(String::from("native annotation reply too large"));
                    }
                }
                let length = u32::try_from(body.len())
                    .map_err(|_| String::from("native annotation reply length overflow"))?;
                sink.write_all(MAGIC).map_err(|error| error.to_string())?;
                sink.write_all(&length.to_le_bytes())
                    .map_err(|error| error.to_string())?;
                sink.write_all(&body).map_err(|error| error.to_string())
            }
        }
    }

    pub fn read_from<R: Read>(source: &mut R) -> Result<Self, String> {
        let mut magic = [0u8; 8];
        source
            .read_exact(&mut magic)
            .map_err(|error| error.to_string())?;
        if &magic == b"PPSEL001" {
            let mut prefix = std::io::Cursor::new(magic).chain(source);
            return NativeSelectionReply::read_from(&mut prefix).map(Self::Legacy);
        }
        if &magic != MAGIC {
            return Err(String::from(
                "invalid native annotation reply magic/version",
            ));
        }
        let mut length = [0u8; 4];
        source
            .read_exact(&mut length)
            .map_err(|error| error.to_string())?;
        let length = u32::from_le_bytes(length) as usize;
        if !(MIN_BODY_BYTES..=MAX_ANNOTATION_BODY_BYTES).contains(&length) {
            return Err(String::from(
                "native annotation reply length outside bounds",
            ));
        }
        let mut body = vec![0u8; length];
        source
            .read_exact(&mut body)
            .map_err(|error| error.to_string())?;
        let mut extra = [0u8; 1];
        if source.read(&mut extra).map_err(|error| error.to_string())? != 0 {
            return Err(String::from("trailing bytes after native annotation reply"));
        }
        let mut cursor = 0usize;
        let display = DisplayId(read_u64(&body, &mut cursor)?);
        let pixels = (read_u32(&body, &mut cursor)?, read_u32(&body, &mut cursor)?);
        NativeSnapshot::validate_dimensions(pixels.0, pixels.1)?;
        let rect = RectPx {
            x: read_i32(&body, &mut cursor)?,
            y: read_i32(&body, &mut cursor)?,
            width: read_u32(&body, &mut cursor)?,
            height: read_u32(&body, &mut cursor)?,
        };
        let count = read_u16(&body, &mut cursor)? as usize;
        if count == 0 || count > MAX_MARKS {
            return Err(String::from("invalid native annotation count"));
        }
        let selection = NativeSelectionReply::Selected { display, rect };
        selection.validated_in(display, pixels)?;
        let mut marks = Vec::with_capacity(count);
        for _ in 0..count {
            let tag = take::<1>(&body, &mut cursor)?[0];
            let mark = match tag {
                1 => {
                    let count = read_u16(&body, &mut cursor)? as usize;
                    if !(2..=MAX_BRUSH_POINTS).contains(&count) {
                        return Err(String::from("invalid native brush count"));
                    }
                    let mut points = Vec::with_capacity(count);
                    for _ in 0..count {
                        points.push(read_point(&body, &mut cursor)?);
                    }
                    NativeMark::Brush { points }
                }
                2 => NativeMark::Arrow {
                    from: read_point(&body, &mut cursor)?,
                    to: read_point(&body, &mut cursor)?,
                },
                6 => NativeMark::Line {
                    from: read_point(&body, &mut cursor)?,
                    to: read_point(&body, &mut cursor)?,
                },
                7 => NativeMark::Pixelate {
                    left: read_u32(&body, &mut cursor)?,
                    top: read_u32(&body, &mut cursor)?,
                    width: read_u32(&body, &mut cursor)?,
                    height: read_u32(&body, &mut cursor)?,
                },
                8 => NativeMark::Blur {
                    left: read_u32(&body, &mut cursor)?,
                    top: read_u32(&body, &mut cursor)?,
                    width: read_u32(&body, &mut cursor)?,
                    height: read_u32(&body, &mut cursor)?,
                },
                3 => NativeMark::Rectangle {
                    top_left: read_point(&body, &mut cursor)?,
                    bottom_right: read_point(&body, &mut cursor)?,
                },
                5 => NativeMark::Ellipse {
                    top_left: read_point(&body, &mut cursor)?,
                    bottom_right: read_point(&body, &mut cursor)?,
                },
                4 => {
                    let at = read_point(&body, &mut cursor)?;
                    let size = read_u16(&body, &mut cursor)? as usize;
                    if size == 0 || size > MAX_TEXT_BYTES {
                        return Err(String::from("invalid native text length"));
                    }
                    let bytes = take_slice(&body, &mut cursor, size)?;
                    let content = std::str::from_utf8(bytes)
                        .map_err(|_| String::from("invalid native annotation UTF-8"))?
                        .to_owned();
                    NativeMark::Text { at, content }
                }
                _ => return Err(String::from("unknown native annotation kind")),
            };
            marks.push(mark);
        }
        if cursor != body.len() {
            return Err(String::from("invalid trailing native annotation payload"));
        }
        validate_marks(&marks, pixels)?;
        Ok(Self::Annotated {
            selection,
            pixels,
            marks,
        })
    }
}

fn write_point(out: &mut Vec<u8>, (x, y): (u32, u32)) {
    out.extend_from_slice(&x.to_le_bytes());
    out.extend_from_slice(&y.to_le_bytes());
}

fn take_slice<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8], String> {
    let end = cursor
        .checked_add(len)
        .ok_or("native annotation offset overflow")?;
    let result = bytes
        .get(*cursor..end)
        .ok_or("truncated native annotation reply")?;
    *cursor = end;
    Ok(result)
}

fn take<const N: usize>(bytes: &[u8], cursor: &mut usize) -> Result<[u8; N], String> {
    take_slice(bytes, cursor, N)?
        .try_into()
        .map_err(|_| String::from("invalid native annotation field"))
}

fn read_u16(data: &[u8], cursor: &mut usize) -> Result<u16, String> {
    Ok(u16::from_le_bytes(take(data, cursor)?))
}
fn read_u32(data: &[u8], cursor: &mut usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(take(data, cursor)?))
}
fn read_i32(data: &[u8], cursor: &mut usize) -> Result<i32, String> {
    Ok(i32::from_le_bytes(take(data, cursor)?))
}
fn read_u64(data: &[u8], cursor: &mut usize) -> Result<u64, String> {
    Ok(u64::from_le_bytes(take(data, cursor)?))
}
fn read_point(data: &[u8], cursor: &mut usize) -> Result<(u32, u32), String> {
    Ok((read_u32(data, cursor)?, read_u32(data, cursor)?))
}
