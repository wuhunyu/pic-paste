use crate::history::Annotation;
use shot_core::RgbaFrame;

pub(crate) fn blend_at(frame: &mut RgbaFrame, x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 || x >= frame.width as i32 || y >= frame.height as i32 {
        return;
    }
    let idx = (y as usize * frame.width as usize + x as usize) * 4;
    let src_a = u32::from(color[3]);
    let dst_a = u32::from(frame.rgba[idx + 3]);
    let out_alpha_255 = src_a * 255 + dst_a * (255 - src_a);
    if out_alpha_255 == 0 {
        frame.rgba[idx..idx + 4].fill(0);
        return;
    }
    for (channel, source) in color.iter().enumerate().take(3) {
        let numerator = u32::from(*source) * src_a * 255
            + u32::from(frame.rgba[idx + channel]) * dst_a * (255 - src_a);
        frame.rgba[idx + channel] = ((numerator + out_alpha_255 / 2) / out_alpha_255) as u8;
    }
    frame.rgba[idx + 3] = ((out_alpha_255 + 127) / 255) as u8;
}

fn circle(frame: &mut RgbaFrame, cx: f32, cy: f32, radius: f32, color: [u8; 4]) {
    let r = radius.max(0.5);
    let x0 = ((cx - r).floor() as i32).max(0);
    let y0 = ((cy - r).floor() as i32).max(0);
    let x1 = ((cx + r).ceil() as i32).min(frame.width as i32 - 1);
    let y1 = ((cy + r).ceil() as i32).min(frame.height as i32 - 1);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            if dx * dx + dy * dy <= r * r {
                blend_at(frame, x, y, color);
            }
        }
    }
}

pub(crate) fn line(
    frame: &mut RgbaFrame,
    from: (f32, f32),
    to: (f32, f32),
    rgba: [u8; 4],
    width: f32,
) {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let steps = dx.abs().max(dy.abs()).ceil() as usize;
    if steps > 1_000_000 {
        return;
    }
    for step in 0..=steps {
        let fraction = if steps == 0 {
            0.0
        } else {
            step as f32 / steps as f32
        };
        circle(
            frame,
            from.0 + dx * fraction,
            from.1 + dy * fraction,
            width / 2.0,
            rgba,
        );
    }
}

/// Render supported vector commands onto a copy of the original frame.
/// Text/font shaping, blur and ellipse are intentionally excluded until an
/// explicit renderer is integrated; unsupported operations return an error.
pub fn composite(base: &RgbaFrame, operations: &[Annotation]) -> Result<RgbaFrame, &'static str> {
    let mut result = RgbaFrame::new(base.width, base.height, base.rgba.clone())
        .map_err(|_| "invalid base frame")?;
    for op in operations {
        match op {
            Annotation::Line {
                from,
                to,
                rgba,
                width,
            } => {
                if !width.is_finite()
                    || *width <= 0.0
                    || ![from.0, from.1, to.0, to.1].iter().all(|x| x.is_finite())
                {
                    return Err("invalid line");
                }
                line(&mut result, *from, *to, *rgba, *width);
            }
            Annotation::Rectangle {
                top_left,
                bottom_right,
                rgba,
                width,
            } => {
                if !width.is_finite()
                    || *width <= 0.0
                    || ![top_left.0, top_left.1, bottom_right.0, bottom_right.1]
                        .iter()
                        .all(|x| x.is_finite())
                {
                    return Err("invalid rectangle");
                }
                let a = (
                    top_left.0.min(bottom_right.0),
                    top_left.1.min(bottom_right.1),
                );
                let b = (
                    top_left.0.max(bottom_right.0),
                    top_left.1.max(bottom_right.1),
                );
                line(&mut result, (a.0, a.1), (b.0, a.1), *rgba, *width);
                line(&mut result, (b.0, a.1), b, *rgba, *width);
                line(&mut result, b, (a.0, b.1), *rgba, *width);
                line(&mut result, (a.0, b.1), a, *rgba, *width);
            }
            Annotation::Pixelate {
                left,
                top,
                width,
                height,
                block_size,
            } => {
                if *block_size == 0 {
                    return Err("pixelate block size is zero");
                }
                let right = left.saturating_add(*width).min(result.width);
                let bottom = top.saturating_add(*height).min(result.height);
                for by in (*top..bottom).step_by(*block_size as usize) {
                    for bx in (*left..right).step_by(*block_size as usize) {
                        let ex = bx.saturating_add(*block_size).min(right);
                        let ey = by.saturating_add(*block_size).min(bottom);
                        let mut sums = [0u64; 4];
                        let mut count = 0u64;
                        for y in by..ey {
                            for x in bx..ex {
                                let idx = (y as usize * result.width as usize + x as usize) * 4;
                                for (channel, sum) in sums.iter_mut().enumerate() {
                                    *sum += u64::from(result.rgba[idx + channel]);
                                }
                                count += 1;
                            }
                        }
                        if count > 0 {
                            let average = sums.map(|v| (v / count) as u8);
                            for y in by..ey {
                                for x in bx..ex {
                                    let idx = (y as usize * result.width as usize + x as usize) * 4;
                                    result.rgba[idx..idx + 4].copy_from_slice(&average);
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                if !crate::advanced_composite::render_advanced(&mut result, op)? {
                    return Err("annotation tool is not yet implemented");
                }
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drawing_is_non_destructive() {
        let source = RgbaFrame::new(16, 16, vec![0; 16 * 16 * 4]).unwrap();
        let painted = composite(
            &source,
            &[Annotation::Rectangle {
                top_left: (2.0, 2.0),
                bottom_right: (12.0, 12.0),
                rgba: [255, 0, 0, 255],
                width: 2.0,
            }],
        )
        .unwrap();
        assert!(painted.rgba.iter().any(|x| *x != 0));
        assert!(source.rgba.iter().all(|x| *x == 0));
    }
    #[test]
    fn rejects_empty_text_instead_of_silently_dropping_it() {
        let source = RgbaFrame::new(2, 2, vec![0; 16]).unwrap();
        assert!(
            composite(
                &source,
                &[Annotation::Text {
                    at: (0.0, 0.0),
                    content: String::new(),
                    size: 12.0,
                    rgba: [255; 4],
                }]
            )
            .is_err()
        );
    }
    #[test]
    fn pixelate_rejects_zero_block_size() {
        let source = RgbaFrame::new(2, 2, vec![100; 16]).unwrap();
        assert!(
            composite(
                &source,
                &[Annotation::Pixelate {
                    left: 0,
                    top: 0,
                    width: 2,
                    height: 2,
                    block_size: 0,
                }]
            )
            .is_err()
        );
    }
}
