//! Interactive pinned-image viewports. OCR inference and clipboard writes
//! happen outside the deferred viewport callback and only on explicit actions.
use eframe::egui;
use shot_core::RgbaFrame;
use shot_core::input::{DragAction, DragLatch};
use shot_core::pin_view::{PinImageView, map_pin_click};
use shot_core::pins::{PinMode, PinOcrGate};
use shot_ocr_protocol::OcrDocument;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug)]
pub enum PinEvent {
    Recognize(u64),
    Move(u64),
    CopySelected(u64),
    PickLine { id: u64, pixel: (f32, f32) },
}

/// Explicitly wake the root app when the user acts inside a deferred pin
/// viewport (the root settings window is normally hidden).
fn send_event(ui: &egui::Ui, sender: &Sender<PinEvent>, event: PinEvent) {
    if sender.send(event).is_ok() {
        ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
    }
}

pub struct PinnedImage {
    pub id: u64,
    pub source: Arc<RgbaFrame>,
    pub opacity: f32,
    pub zoom: f32,
    pub mode: PinMode,
    pub closed: Arc<AtomicBool>,
    pub ocr_gate: PinOcrGate,
    pub ocr_result: Option<Arc<OcrDocument>>,
    pub selected_line: Option<usize>,
    gesture: Arc<Mutex<DragLatch>>,
    texture: egui::TextureHandle,
}

impl PinnedImage {
    pub fn new(ctx: &egui::Context, id: u64, frame: Arc<RgbaFrame>) -> Self {
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [frame.width as usize, frame.height as usize],
            &frame.rgba,
        );
        let texture = ctx.load_texture(format!("pin-{id}"), image, egui::TextureOptions::LINEAR);
        Self {
            id,
            source: frame,
            opacity: 1.0,
            zoom: 1.0,
            mode: PinMode::Move,
            closed: Arc::new(AtomicBool::new(false)),
            ocr_gate: PinOcrGate::default(),
            ocr_result: None,
            selected_line: None,
            gesture: Arc::new(Mutex::new(DragLatch::default())),
            texture,
        }
    }

    pub fn set_mode(&mut self, mode: PinMode) {
        if self.mode == PinMode::TextSelect && mode != PinMode::TextSelect {
            self.ocr_gate.cancel_selection();
            self.selected_line = None;
        }
        self.mode = mode;
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    pub fn show(&self, ctx: &egui::Context, events: &Sender<PinEvent>) {
        let image_width = self.source.width;
        let image_height = self.source.height;
        let width = image_width as f32;
        let height = image_height as f32;
        let scale = (600.0 / width).min(420.0 / height).min(1.0);
        let size = egui::vec2(width * scale, height * scale) * self.zoom.clamp(0.25, 3.0);
        let texture = self.texture.clone();
        let opacity = self.opacity;
        let mode = self.mode;
        let closed = Arc::clone(&self.closed);
        let ocr_pending = self.ocr_gate.pending();
        let ocr_result = self.ocr_result.clone();
        let selected_line = self.selected_line;
        let gesture = Arc::clone(&self.gesture);
        let sender = events.clone();
        let pin_id = self.id;
        let id = egui::ViewportId::from_hash_of(("picpaste-pin", pin_id));
        let builder = egui::ViewportBuilder::default()
            .with_title(format!("PicPaste - 贴图 {}", pin_id))
            .with_inner_size([size.x + 14.0, size.y + 48.0])
            .with_always_on_top()
            .with_transparent(true)
            .with_decorations(false)
            .with_resizable(true)
            .with_mouse_passthrough(mode == PinMode::Passthrough);
        ctx.show_viewport_deferred(id, builder, move |ui, _| {
            if ui.ctx().input(|input| input.viewport().close_requested()) {
                closed.store(true, Ordering::Relaxed);
            }
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        match mode {
                            PinMode::Move => {
                                ui.label("PicPaste · 拖动");
                                if ui.small_button("识别文字 (Full)").clicked() {
                                    send_event(ui, &sender, PinEvent::Recognize(pin_id));
                                }
                            }
                            PinMode::TextSelect => {
                                ui.label("PicPaste · 点击选字");
                                if ui.small_button("移动").clicked() {
                                    send_event(ui, &sender, PinEvent::Move(pin_id));
                                }
                                if ui
                                    .add_enabled(!ocr_pending, egui::Button::new("重新识别"))
                                    .clicked()
                                {
                                    send_event(ui, &sender, PinEvent::Recognize(pin_id));
                                }
                                if selected_line.is_some()
                                    && ui.small_button("复制选中行").clicked()
                                {
                                    send_event(ui, &sender, PinEvent::CopySelected(pin_id));
                                }
                            }
                            PinMode::Passthrough => {
                                ui.label("PicPaste · 鼠标穿透（按恢复快捷键解锁）");
                            }
                        }
                        if ui.small_button("×").clicked() {
                            closed.store(true, Ordering::Relaxed);
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    if mode == PinMode::TextSelect && ocr_pending {
                        ui.label("正在离线识别…");
                    } else if mode == PinMode::TextSelect && ocr_result.is_none() {
                        ui.label("请点击重新识别获取文字行");
                    }
                    let available = ui.available_size();
                    let shrink = (available.x / size.x).min(available.y / size.y).min(1.0);
                    let view_size = size * shrink;
                    let tint =
                        egui::Color32::from_white_alpha((opacity.clamp(0.0, 1.0) * 255.0) as u8);
                    let response = ui.add(
                        egui::Image::new((texture.id(), view_size))
                            .tint(tint)
                            .sense(egui::Sense::click_and_drag()),
                    );
                    if response.drag_started()
                        && let Ok(mut latch) = gesture.lock()
                    {
                        let shift = ui.input(|input| input.modifiers.shift);
                        if latch.down(mode, shift) == DragAction::MovePin {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                    }
                    if response.drag_stopped()
                        && let Ok(mut latch) = gesture.lock()
                    {
                        latch.up();
                    }
                    if mode == PinMode::TextSelect
                        && response.clicked()
                        && let Some(pointer) = response.interact_pointer_pos()
                        && let Some(pixel) = map_pin_click(
                            PinImageView {
                                image_width,
                                image_height,
                                left: response.rect.left(),
                                top: response.rect.top(),
                                width: response.rect.width(),
                                height: response.rect.height(),
                            },
                            (pointer.x, pointer.y),
                        )
                    {
                        send_event(ui, &sender, PinEvent::PickLine { id: pin_id, pixel });
                    }
                    if mode == PinMode::TextSelect
                        && let Some(index) = selected_line
                        && let Some(line) = ocr_result
                            .as_ref()
                            .and_then(|document| document.lines.get(index))
                    {
                        let vertices = line
                            .polygon_px
                            .iter()
                            .map(|[x, y]| {
                                egui::pos2(
                                    response.rect.left()
                                        + *x / image_width as f32 * response.rect.width(),
                                    response.rect.top()
                                        + *y / image_height as f32 * response.rect.height(),
                                )
                            })
                            .collect();
                        ui.painter().add(egui::Shape::closed_line(
                            vertices,
                            egui::Stroke::new(2.0, egui::Color32::YELLOW),
                        ));
                    }
                });
        });
    }
}
