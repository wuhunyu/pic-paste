use eframe::egui;
use shot_core::RgbaFrame;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct PinnedImage {
    pub id: u64,
    pub source: Arc<RgbaFrame>,
    pub opacity: f32,
    pub passthrough: bool,
    pub closed: Arc<AtomicBool>,
    texture: egui::TextureHandle,
}

impl PinnedImage {
    pub fn new(ctx: &egui::Context, id: u64, frame: Arc<RgbaFrame>) -> Self {
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [frame.width as usize, frame.height as usize],
            &frame.rgba,
        );
        let texture = ctx.load_texture(
            format!("pin-{id}"),
            image,
            egui::TextureOptions::LINEAR,
        );
        Self {
            id,
            source: frame,
            opacity: 1.0,
            passthrough: false,
            closed: Arc::new(AtomicBool::new(false)),
            texture,
        }
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    pub fn show(&self, ctx: &egui::Context) {
        let width = self.source.width as f32;
        let height = self.source.height as f32;
        let scale = (600.0 / width).min(420.0 / height).min(1.0);
        let size = egui::vec2(width * scale, height * scale);
        let texture = self.texture.clone();
        let opacity = self.opacity;
        let closed = Arc::clone(&self.closed);
        let id = egui::ViewportId::from_hash_of(("picpaste-pin", self.id));
        let builder = egui::ViewportBuilder::default()
            .with_title(format!("PicPaste - 贴图 {}", self.id))
            .with_inner_size([size.x + 14.0, size.y + 40.0])
            .with_always_on_top()
            .with_transparent(true)
            .with_decorations(false)
            .with_resizable(true)
            .with_mouse_passthrough(self.passthrough);
        ctx.show_viewport_deferred(id, builder, move |ui, _| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                closed.store(true, Ordering::Relaxed);
            }
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("PicPaste · 拖动图片移动");
                        if ui.small_button("×").clicked() {
                            closed.store(true, Ordering::Relaxed);
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    let available = ui.available_size();
                    let shrink = (available.x / size.x).min(available.y / size.y).min(1.0);
                    let view_size = size * shrink;
                    let tint = egui::Color32::from_white_alpha(
                        (opacity.clamp(0.0, 1.0) * 255.0) as u8,
                    );
                    let reply = ui.add(
                        egui::Image::new((texture.id(), view_size))
                            .tint(tint)
                            .sense(egui::Sense::drag()),
                    );
                    if reply.drag_started() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                });
        });
    }
}
