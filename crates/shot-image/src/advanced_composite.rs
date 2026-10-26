use crate::composite::{blend_at, line};
use crate::history::Annotation;
use shot_core::RgbaFrame;

fn valid_point(point: (f32, f32)) -> bool {
    point.0.is_finite() && point.1.is_finite()
}

fn valid_width(width: f32) -> bool {
    width.is_finite() && width > 0.0 && width <= 128.0
}

fn draw_ellipse(
    frame: &mut RgbaFrame,
    center: (f32, f32),
    radius: (f32, f32),
    rgba: [u8; 4],
    width: f32,
) -> Result<(), &'static str> {
    if !valid_point(center)
        || !valid_point(radius)
        || radius.0 <= 0.0
        || radius.1 <= 0.0
        || !valid_width(width)
    {
        return Err("invalid ellipse");
    }
    let (rx, ry) = radius;
    let x0 = ((center.0 - rx - width).floor() as i32).max(0);
    let y0 = ((center.1 - ry - width).floor() as i32).max(0);
    let x1 = ((center.0 + rx + width).ceil() as i32).min(frame.width as i32 - 1);
    let y1 = ((center.1 + ry + width).ceil() as i32).min(frame.height as i32 - 1);
    let tolerance = width / (2.0 * rx.min(ry));
    for y in y0..=y1 {
        for x in x0..=x1 {
            let px = (x as f32 + 0.5 - center.0) / rx;
            let py = (y as f32 + 0.5 - center.1) / ry;
            let distance = (px * px + py * py).sqrt();
            if (distance - 1.0).abs() <= tolerance {
                blend_at(frame, x, y, rgba);
            }
        }
    }
    Ok(())
}

fn draw_arrow(
    frame: &mut RgbaFrame,
    from: (f32, f32),
    to: (f32, f32),
    rgba: [u8; 4],
    width: f32,
) -> Result<(), &'static str> {
    if !valid_point(from) || !valid_point(to) || !valid_width(width) {
        return Err("invalid arrow");
    }
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let length = dx.hypot(dy);
    if !length.is_finite() || length < 1.0 {
        return Err("arrow too short");
    }
    line(frame, from, to, rgba, width);
    let head_length = (length * 0.28).clamp(7.0, 20.0);
    let angle = dy.atan2(dx);
    let half_angle = std::f32::consts::PI / 6.0;
    for sign in [-1.0_f32, 1.0_f32] {
        let heading = angle + std::f32::consts::PI + sign * half_angle;
        let tip = (
            to.0 + head_length * heading.cos(),
            to.1 + head_length * heading.sin(),
        );
        line(frame, to, tip, rgba, width);
    }
    Ok(())
}

fn draw_brush(
    frame: &mut RgbaFrame,
    points: &[(f32, f32)],
    rgba: [u8; 4],
    width: f32,
) -> Result<(), &'static str> {
    if points.is_empty()
        || points.len() > 500_000
        || !valid_width(width)
        || !points.iter().all(|&p| valid_point(p))
    {
        return Err("invalid brush path");
    }
    if points.len() == 1 {
        line(frame, points[0], points[0], rgba, width);
    } else {
        for pair in points.windows(2) {
            line(frame, pair[0], pair[1], rgba, width);
        }
    }
    Ok(())
}

// Work on a bounded rectangle only. Horizontal and vertical sliding sums
// compute a box blur in O(width * height) time rather than O(pixels * r^2).
// RGB is accumulated premultiplied by alpha to avoid dark transparent halos.
fn blur_roi(
    frame: &mut RgbaFrame,
    left: u32,
    top: u32,
    width: u32,
    height: u32,
    radius: u32,
) -> Result<(), &'static str> {
    if width == 0
        || height == 0
        || radius == 0
        || radius > 32
        || left >= frame.width
        || top >= frame.height
    {
        return Err("invalid blur region");
    }
    let right = left.saturating_add(width).min(frame.width);
    let bottom = top.saturating_add(height).min(frame.height);
    let w = (right - left) as usize;
    let h = (bottom - top) as usize;
    if w.checked_mul(h).is_none_or(|pixels| pixels > 4_000_000) {
        return Err("blur region exceeds 4MP limit");
    }
    let r = radius as usize;
    let sample = |x: usize, y: usize| {
        let index = ((top as usize + y) * frame.width as usize + left as usize + x) * 4;
        let a = u32::from(frame.rgba[index + 3]);
        [
            a,
            a * u32::from(frame.rgba[index]),
            a * u32::from(frame.rgba[index + 1]),
            a * u32::from(frame.rgba[index + 2]),
        ]
    };
    let mut horizontal = vec![[0u32; 4]; w * h];
    for y in 0..h {
        let mut sum = [0u32; 4];
        for x in 0..=r.min(w - 1) {
            let value = sample(x, y);
            for (dst, src) in sum.iter_mut().zip(value) {
                *dst += src;
            }
        }
        for x in 0..w {
            horizontal[y * w + x] = sum;
            if x >= r {
                let value = sample(x - r, y);
                for (dst, src) in sum.iter_mut().zip(value) {
                    *dst -= src;
                }
            }
            if x + r + 1 < w {
                let value = sample(x + r + 1, y);
                for (dst, src) in sum.iter_mut().zip(value) {
                    *dst += src;
                }
            }
        }
    }
    // All source reads end here, so writing frame pixels cannot taint samples.
    let mut vertical = vec![[0u64; 4]; w];
    for y in 0..=r.min(h - 1) {
        for (x, dst) in vertical.iter_mut().enumerate() {
            let src = horizontal[y * w + x];
            for (a, b) in dst.iter_mut().zip(src) {
                *a += u64::from(b);
            }
        }
    }
    for y in 0..h {
        let count_y = (y + r).min(h - 1) - y.saturating_sub(r) + 1;
        for (x, sum) in vertical.iter().enumerate() {
            let count_x = (x + r).min(w - 1) - x.saturating_sub(r) + 1;
            let count = (count_x * count_y) as u64;
            let alpha = ((sum[0] + count / 2) / count) as u8;
            let mut rgba = [0, 0, 0, alpha];
            for (channel, output) in rgba[..3].iter_mut().enumerate() {
                *output = (sum[channel + 1] + sum[0] / 2)
                    .checked_div(sum[0])
                    .unwrap_or(0)
                    .min(255) as u8;
            }
            let index = ((top as usize + y) * frame.width as usize + left as usize + x) * 4;
            frame.rgba[index..index + 4].copy_from_slice(&rgba);
        }
        if y >= r {
            let drop = y - r;
            for (x, dst) in vertical.iter_mut().enumerate() {
                let src = horizontal[drop * w + x];
                for (a, b) in dst.iter_mut().zip(src) {
                    *a -= u64::from(b);
                }
            }
        }
        if y + r + 1 < h {
            let add = y + r + 1;
            for (x, dst) in vertical.iter_mut().enumerate() {
                let src = horizontal[add * w + x];
                for (a, b) in dst.iter_mut().zip(src) {
                    *a += u64::from(b);
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn render_advanced(
    frame: &mut RgbaFrame,
    operation: &Annotation,
) -> Result<bool, &'static str> {
    match operation {
        Annotation::Arrow {
            from,
            to,
            rgba,
            width,
        } => {
            draw_arrow(frame, *from, *to, *rgba, *width)?;
        }
        Annotation::Brush {
            points,
            rgba,
            width,
        } => {
            draw_brush(frame, points, *rgba, *width)?;
        }
        Annotation::Ellipse {
            center,
            radius,
            rgba,
            width,
        } => {
            draw_ellipse(frame, *center, *radius, *rgba, *width)?;
        }
        Annotation::Blur {
            left,
            top,
            width,
            height,
            radius,
        } => {
            blur_roi(frame, *left, *top, *width, *height, *radius)?;
        }
        Annotation::Text {
            at,
            content,
            size,
            rgba,
        } => {
            crate::text_composite::draw_text(frame, *at, content, *size, *rgba)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
