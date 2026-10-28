//! Cached, bounded pixel previews for the opt-in native NSPanel only.
//!
//! The source texture is immutable; exact committed effect patches are
//! separate egui textures rendered only when history changes.
use egui::{Color32, ColorImage, Context, Painter, Pos2, Rect, TextureHandle, TextureOptions};
use shot_core::RgbaFrame;
use shot_core::native_markup::{NativeMark, NativeMarkup};
use shot_image::native_effect_preview::{
    can_preview_native_effects, render_native_effect_patch,
};

struct EffectTexturePatch {
    left: u32,
    top: u32,
    size: (u32, u32),
    texture: TextureHandle,
}

/// A conservative cache: any mixed or overlapping history uses honest
/// region outlines instead of misleading pixels. Drafts never enter here.
#[derive(Default)]
pub(crate) struct NativeEffectTextureCache {
    signature: Option<Vec<NativeMark>>,
    patches: Vec<EffectTexturePatch>,
    exact: bool,
}

impl NativeEffectTextureCache {
    pub(crate) fn is_exact(&self) -> bool {
        self.exact
    }

    #[cfg(test)]
    pub(crate) fn patch_count(&self) -> usize {
        self.patches.len()
    }

    /// Returns true when the committed history changes. All allocations are
    /// bounded by the shot-image 1MP aggregate preview budget.
    pub(crate) fn sync(
        &mut self,
        ctx: &Context,
        source: Option<&RgbaFrame>,
        marks: &[NativeMark],
    ) -> bool {
        if self.signature.as_deref() == Some(marks) {
            return false;
        }
        self.signature = Some(marks.to_vec());
        self.patches.clear();
        self.exact = false;

        let Some(source) = source else {
            return true;
        };
        if !can_preview_native_effects(marks, (source.width, source.height)) {
            return true;
        }
        let mut textures = Vec::with_capacity(marks.len());
        for (index, mark) in marks.iter().enumerate() {
            let Ok(patch) = render_native_effect_patch(source, mark) else {
                return true;
            };
            let size = (patch.image.width, patch.image.height);
            let image = ColorImage::from_rgba_unmultiplied(
                [size.0 as usize, size.1 as usize],
                &patch.image.rgba,
            );
            // NEAREST avoids any extra interpolation within the exact ROI.
            let texture = ctx.load_texture(
                format!("native-effect-patch-{index}"),
                image,
                TextureOptions::NEAREST,
            );
            textures.push(EffectTexturePatch {
                left: patch.left,
                top: patch.top,
                size,
                texture,
            });
        }
        self.patches = textures;
        self.exact = true;
        true
    }

    pub(crate) fn paint(&self, painter: &Painter, origin: Pos2, markup: &NativeMarkup) {
        if !self.exact {
            return;
        }
        for patch in &self.patches {
            let (x0, y0) = markup.to_logical_point((patch.left, patch.top));
            let right = patch.left + patch.size.0;
            let bottom = patch.top + patch.size.1;
            let (x1, y1) = markup.to_logical_point((right, bottom));
            let target = Rect::from_min_max(
                origin + egui::vec2(x0, y0),
                origin + egui::vec2(x1, y1),
            );
            painter.image(
                patch.texture.id(),
                target,
                Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }
}
