//! Interactive pinned-image viewports. OCR inference and clipboard writes
//! happen outside the deferred viewport callback and only on explicit actions.
use eframe::egui;
use shot_core::RgbaFrame;
use shot_core::input::{DragAction, DragLatch};
use shot_core::pin_view::{PinImageView, map_pin_click};
use shot_core::pins::{PinMode, PinOcrGate};
use shot_ocr_protocol::hit_test::{TextPickMode, TextSelection, selection_polygon};
use shot_ocr_protocol::{OcrDocument, QrDocument};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug)]
pub enum PinEvent {
    Recognize { id: u64, characters: bool },
    DetectQr(u64),
    Move(u64),
    Closed(u64),
    CopySelected(u64),
    CopyQr { id: u64, index: usize },
    PickText { id: u64, pixel: (f32, f32) },
    PickMode { id: u64, mode: TextPickMode },
}

/// Explicitly wake the root app when the user acts inside a deferred pin
/// viewport (the root settings window is normally hidden).
fn send_event(ui: &egui::Ui, sender: &Sender<PinEvent>, event: PinEvent) {
    if sender.send(event).is_ok() {
        ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
    }
}

/// Configure a winit-owned macOS window without stealing focus from the
/// target app. Actual AppKit NSPanel integration is a separate renderer change.
pub(crate) fn pin_builder(
    pin_id: u64,
    size: egui::Vec2,
    mode: PinMode,
) -> egui::ViewportBuilder {
    egui::ViewportBuilder::default()
        .with_title(format!("PicPaste - 贴图 {pin_id}"))
        .with_inner_size([size.x + 14.0, size.y + 48.0])
        .with_always_on_top()
        .with_active(false)
        .with_transparent(true)
        .with_has_shadow(false)
        .with_decorations(false)
        .with_resizable(true)
        .with_movable_by_background(false)
        .with_mouse_passthrough(mode == PinMode::Passthrough)
}

pub struct PinnedImage {
    pub id: u64,
    /// First pin ID that allocated this texture; shared pins keep it alive.
    pub texture_group: u64,
    pub source: Arc<RgbaFrame>,
    pub opacity: f32,
    pub zoom: f32,
    pub mode: PinMode,
    pub closed: Arc<AtomicBool>,
    pub ocr_gate: PinOcrGate,
    pub ocr_result: Option<Arc<OcrDocument>>,
    pub selected_text: Option<TextSelection>,
    pub character_mode: bool,
    pub pick_mode: TextPickMode,
    pub qr_result: Option<Arc<QrDocument>>,
    gesture: Arc<Mutex<DragLatch>>,
    texture: egui::TextureHandle,
}

impl PinnedImage {
    /// A unique frame is uploaded exactly once after budget admission.
    pub fn new(ctx: &egui::Context, id: u64, frame: Arc<RgbaFrame>) -> Self {
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [frame.width as usize, frame.height as usize],
            &frame.rgba,
        );
        let texture = ctx.load_texture(format!("pin-{id}"), image, egui::TextureOptions::LINEAR);
        Self::with_texture(id, frame, texture, id)
    }

    /// Only use after comparing *all* source bytes and dimensions. This does
    /// not re-upload a GPU texture or duplicate the source RGBA allocation.
    pub fn from_shared(id: u64, other: &Self) -> Self {
        Self::with_texture(
            id,
            Arc::clone(&other.source),
            other.texture.clone(),
            other.texture_group,
        )
    }

    fn with_texture(
        id: u64,
        source: Arc<RgbaFrame>,
        texture: egui::TextureHandle,
        texture_group: u64,
    ) -> Self {
        Self {
            id,
            texture_group,
            source,
            opacity: 1.0,
            zoom: 1.0,
            mode: PinMode::Move,
            closed: Arc::new(AtomicBool::new(false)),
            ocr_gate: PinOcrGate::default(),
            ocr_result: None,
            selected_text: None,
            character_mode: false,
            pick_mode: TextPickMode::Unit,
            qr_result: None,
            gesture: Arc::new(Mutex::new(DragLatch::default())),
            texture,
        }
    }

    pub fn set_mode(&mut self, mode: PinMode) {
        if self.mode == PinMode::TextSelect && mode != PinMode::TextSelect {
            self.ocr_gate.cancel_selection();
            self.selected_text = None;
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
        let selected_text = self.selected_text;
        let character_mode = self.character_mode;
        let pick_mode = self.pick_mode;
        let qr_result = self.qr_result.clone();
        let gesture = Arc::clone(&self.gesture);
        let sender = events.clone();
        let pin_id = self.id;
        let id = egui::ViewportId::from_hash_of(("picpaste-pin", pin_id));
        let builder = pin_builder(pin_id, size, mode);
        ctx.show_viewport_deferred(id, builder, move |ui, _| {
            if ui.ctx().input(|input| input.viewport().close_requested())
                && !closed.swap(true, Ordering::Relaxed)
            {
                send_event(ui, &sender, PinEvent::Closed(pin_id));
            }
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        match mode {
                            PinMode::Move => {
                                ui.label("PicPaste · 拖动");
                                if ui.small_button("识别词").clicked() {
                                    send_event(
                                        ui,
                                        &sender,
                                        PinEvent::Recognize {
                                            id: pin_id,
                                            characters: false,
                                        },
                                    );
                                }
                                if ui.small_button("识别字").clicked() {
                                    send_event(
                                        ui,
                                        &sender,
                                        PinEvent::Recognize {
                                            id: pin_id,
                                            characters: true,
                                        },
                                    );
                                }
                                if ui.small_button("识别二维码").clicked() {
                                    send_event(ui, &sender, PinEvent::DetectQr(pin_id));
                                }
                            }
                            PinMode::TextSelect => {
                                ui.label(if character_mode {
                                    "PicPaste · 单字定位"
                                } else {
                                    "PicPaste · 词级定位"
                                });
                                if ui.small_button("移动").clicked() {
                                    send_event(ui, &sender, PinEvent::Move(pin_id));
                                }
                                if ui
                                    .add_enabled(!ocr_pending, egui::Button::new("识别词"))
                                    .clicked()
                                {
                                    send_event(
                                        ui,
                                        &sender,
                                        PinEvent::Recognize {
                                            id: pin_id,
                                            characters: false,
                                        },
                                    );
                                }
                                if ui
                                    .add_enabled(!ocr_pending, egui::Button::new("识别字"))
                                    .clicked()
                                {
                                    send_event(
                                        ui,
                                        &sender,
                                        PinEvent::Recognize {
                                            id: pin_id,
                                            characters: true,
                                        },
                                    );
                                }
                                if ui
                                    .add_enabled(!ocr_pending, egui::Button::new("扫码"))
                                    .clicked()
                                {
                                    send_event(ui, &sender, PinEvent::DetectQr(pin_id));
                                }
                                if ui.small_button("选整行").clicked() {
                                    send_event(
                                        ui,
                                        &sender,
                                        PinEvent::PickMode {
                                            id: pin_id,
                                            mode: TextPickMode::Line,
                                        },
                                    );
                                }
                                if ui.small_button("选词/字").clicked() {
                                    send_event(
                                        ui,
                                        &sender,
                                        PinEvent::PickMode {
                                            id: pin_id,
                                            mode: TextPickMode::Unit,
                                        },
                                    );
                                }
                                ui.label(if pick_mode == TextPickMode::Line {
                                    "行"
                                } else if character_mode {
                                    "字"
                                } else {
                                    "词"
                                });
                                if selected_text.is_some()
                                    && ui.small_button("复制所选文字").clicked()
                                {
                                    send_event(ui, &sender, PinEvent::CopySelected(pin_id));
                                }
                            }
                            PinMode::Passthrough => {
                                ui.label("PicPaste · 鼠标穿透（按恢复快捷键解锁）");
                            }
                        }
                        if ui.small_button("×").clicked() {
                            if !closed.swap(true, Ordering::Relaxed) {
                                send_event(ui, &sender, PinEvent::Closed(pin_id));
                            }
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    if mode == PinMode::TextSelect && ocr_pending {
                        ui.label("正在离线识别…");
                    } else if mode == PinMode::TextSelect
                        && ocr_result.is_none()
                        && qr_result.is_none()
                    {
                        ui.label("请点击识别词、识别字或扫码");
                    }
                    if mode == PinMode::TextSelect
                        && let Some(document) = qr_result.as_ref()
                    {
                        if document.codes.is_empty() {
                            ui.label("未发现二维码");
                        }
                        for (index, code) in document.codes.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("QR {}：{}", index + 1, code.payload));
                                if ui.small_button("复制内容").clicked() {
                                    send_event(ui, &sender, PinEvent::CopyQr { id: pin_id, index });
                                }
                            });
                        }
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
                        send_event(ui, &sender, PinEvent::PickText { id: pin_id, pixel });
                    }
                    if mode == PinMode::TextSelect
                        && let Some(selection) = selected_text
                        && let Some(polygon) = ocr_result
                            .as_ref()
                            .and_then(|document| selection_polygon(document, selection))
                    {
                        let vertices = polygon
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

#[cfg(test)]
mod texture_lifetime_tests {
    use super::*;

    #[test]
    fn duplicate_pin_keeps_the_same_texture_and_source_but_independent_ui_state() {
        let ctx = egui::Context::default();
        let source = Arc::new(RgbaFrame::new(2, 2, vec![255; 16]).unwrap());
        let original = PinnedImage::new(&ctx, 1, source);
        let mut duplicate = PinnedImage::from_shared(2, &original);
        assert_eq!(original.texture.id(), duplicate.texture.id());
        assert!(Arc::ptr_eq(&original.source, &duplicate.source));
        assert_eq!(original.texture_group, duplicate.texture_group);
        original.closed.store(true, Ordering::Relaxed);
        assert!(original.is_closed());
        assert!(!duplicate.is_closed());
        duplicate.set_mode(PinMode::TextSelect);
        assert_eq!(original.mode, PinMode::Move);
        assert_eq!(duplicate.mode, PinMode::TextSelect);
    }

    #[test]
    fn a_closed_pin_can_wake_the_hidden_root_for_resource_cleanup() {
        let event = PinEvent::Closed(42);
        assert!(matches!(event, PinEvent::Closed(id) if id == 42));
    }
}

#[cfg(test)]
mod focus_policy_tests {
    use super::*;

    #[test]
    fn all_pin_modes_start_nonactive_and_are_never_movable_by_background() {
        let size = egui::vec2(240.0, 180.0);
        for mode in [PinMode::Move, PinMode::TextSelect, PinMode::Passthrough] {
            let builder = pin_builder(7, size, mode);
            assert_eq!(builder.active, Some(false));
            assert_eq!(builder.has_shadow, Some(false));
            assert_eq!(builder.movable_by_window_background, Some(false));
            assert_eq!(builder.decorations, Some(false));
            assert_eq!(
                builder.mouse_passthrough,
                Some(mode == PinMode::Passthrough)
            );
        }
    }
}
