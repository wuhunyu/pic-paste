//! Offline system-font text rasterization into the screenshot RGBA frame.
//! System fonts are opened lazily only when the first text annotation is used.
use ab_glyph::{Font, FontArc, FontVec, ScaleFont, point};
use shot_core::RgbaFrame;
use std::sync::OnceLock;

static FONTS: OnceLock<Vec<FontArc>> = OnceLock::new();

fn system_fonts() -> &'static [FontArc] {
    FONTS.get_or_init(|| {
        let paths = [
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/System/Library/Fonts/Helvetica.ttc",
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
            "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
            "C:\\Windows\\Fonts\\msyh.ttc",
            "C:\\Windows\\Fonts\\segoeui.ttf",
        ];
        paths
            .iter()
            .filter_map(|path| std::fs::read(path).ok())
            .filter_map(|data| FontVec::try_from_vec_and_index(data, 0).ok())
            .map(FontArc::new)
            .collect::<Vec<_>>()
    })
}

pub(crate) fn draw_text(
    frame: &mut RgbaFrame,
    at: (f32, f32),
    content: &str,
    size: f32,
    rgba: [u8; 4],
) -> Result<(), &'static str> {
    if !at.0.is_finite()
        || !at.1.is_finite()
        || !size.is_finite()
        || !(4.0..=128.0).contains(&size)
        || content.chars().count() > 5000
        || content.trim().is_empty()
    {
        return Err("invalid text annotation");
    }
    let fonts = system_fonts();
    if fonts.is_empty() {
        return Err("no usable offline system fonts for text");
    }
    let (start_x, mut baseline_y) = (at.0, at.1 + size);
    let mut pen_x = start_x;
    let mut painted = false;
    for ch in content.chars() {
        if ch == '\n' {
            pen_x = start_x;
            baseline_y += size * 1.3;
            continue;
        }
        if ch == '\r' {
            continue;
        }
        if ch == '\t' {
            pen_x += size * 2.0;
            continue;
        }
        let chosen = fonts.iter().find(|font| font.glyph_id(ch).0 != 0);
        let Some(font) = chosen else {
            if ch.is_whitespace() {
                pen_x += size * 0.35;
                continue;
            }
            return Err("system font does not contain a required glyph");
        };
        let glyph_id = font.glyph_id(ch);
        let glyph = glyph_id.with_scale_and_position(size, point(pen_x, baseline_y));
        if let Some(outline) = font.outline_glyph(glyph) {
            let bounds = outline.px_bounds();
            outline.draw(|x, y, coverage| {
                let alpha = (f32::from(rgba[3]) * coverage.clamp(0.0, 1.0)).round() as u8;
                if alpha != 0 {
                    crate::composite::blend_at(
                        frame,
                        bounds.min.x as i32 + x as i32,
                        bounds.min.y as i32 + y as i32,
                        [rgba[0], rgba[1], rgba[2], alpha],
                    );
                    painted = true;
                }
            });
        }
        pen_x += font.as_scaled(size).h_advance(glyph_id);
    }
    if !painted {
        return Err("text produced no glyph pixels");
    }
    Ok(())
}
