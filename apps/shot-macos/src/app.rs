use crate::hotkey::{HotkeyAction, HotkeyController};
use crate::pin::PinnedImage;
use arboard::{Clipboard, ImageData};
use eframe::egui;
use shot_core::platform::{CapturedFrame, PlatformError};
use shot_core::selection::from_drag;
use shot_core::{RectPx, RgbaFrame};
use shot_platform::macos::MacCapture;
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

enum WorkerResult {
    Captured(Result<CapturedFrame, PlatformError>),
    Prepared(PrepareAction, Result<RgbaFrame, String>),
    Saved(Result<PathBuf, String>),
}

#[derive(Clone, Copy)]
enum PrepareAction {
    Save,
    Copy,
    Pin,
}

struct SelectionView {
    source: Arc<RgbaFrame>,
    texture: egui::TextureHandle,
    drag_anchor: Option<egui::Pos2>,
    selection: Option<RectPx>,
}

impl SelectionView {
    fn new(ctx: &egui::Context, source: Arc<RgbaFrame>) -> Self {
        let texture = ctx.load_texture(
            "capture-preview",
            egui::ColorImage::from_rgba_unmultiplied(
                [source.width as usize, source.height as usize],
                &source.rgba,
            ),
            egui::TextureOptions::LINEAR,
        );
        Self {
            source,
            texture,
            drag_anchor: None,
            selection: None,
        }
    }

    fn selected_region(&self) -> RectPx {
        self.selection.unwrap_or(RectPx {
            x: 0,
            y: 0,
            width: self.source.width,
            height: self.source.height,
        })
    }

    fn show(&mut self, ui: &mut egui::Ui) {
        ui.label("在预览中拖动选择区域；不选择则处理整个显示器。Esc 取消。");
        let width = self.source.width as f32;
        let height = self.source.height as f32;
        let available = ui.available_size() - egui::vec2(0.0, 12.0);
        let scale = (available.x / width).min(available.y / height).min(1.0).max(0.01);
        let displayed = egui::vec2(width * scale, height * scale);
        let response = ui.add(
            egui::Image::new((self.texture.id(), displayed))
                .sense(egui::Sense::click_and_drag()),
        );

        if response.drag_started() {
            self.drag_anchor = response.interact_pointer_pos();
        }
        if response.dragged() || response.drag_stopped() {
            if let (Some(start), Some(end)) =
                (self.drag_anchor, response.interact_pointer_pos())
            {
                let a = point_to_pixel(&response.rect, &self.source, start);
                let b = point_to_pixel(&response.rect, &self.source, end);
                self.selection = from_drag(a, b).ok();
            }
        }
        if response.drag_stopped() {
            self.drag_anchor = None;
        }
        if let Some(selection) = self.selection {
            let clip = egui::Rect::from_min_size(
                egui::pos2(
                    response.rect.left() + selection.x as f32 * scale,
                    response.rect.top() + selection.y as f32 * scale,
                ),
                egui::vec2(
                    selection.width as f32 * scale,
                    selection.height as f32 * scale,
                ),
            );
            ui.painter().rect_stroke(
                clip,
                0.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 210, 240)),
                egui::StrokeKind::Inside,
            );
            ui.label(format!(
                "选区: {} × {} px",
                selection.width, selection.height
            ));
        }
    }
}

fn point_to_pixel(rect: &egui::Rect, frame: &RgbaFrame, point: egui::Pos2) -> (i32, i32) {
    let x = ((point.x - rect.left()) * frame.width as f32 / rect.width())
        .floor()
        .clamp(0.0, frame.width.saturating_sub(1) as f32);
    let y = ((point.y - rect.top()) * frame.height as f32 / rect.height())
        .floor()
        .clamp(0.0, frame.height.saturating_sub(1) as f32);
    (x as i32, y as i32)
}

fn save_in_pictures(image: &RgbaFrame) -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| String::from("HOME is unavailable"))?;
    let folder = PathBuf::from(home).join("Pictures").join("PicPaste");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let time = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let path = folder.join(format!("PicPaste-{time}.png"));
    shot_image::atomic_save_png(image, &path).map_err(|e| e.to_string())?;
    Ok(path)
}

fn copy_to_clipboard(image: RgbaFrame) -> Result<(), String> {
    Clipboard::new()
        .map_err(|e| e.to_string())?
        .set_image(ImageData {
            width: image.width as usize,
            height: image.height as usize,
            bytes: Cow::Owned(image.rgba),
        })
        .map_err(|e| e.to_string())
}

pub struct PicPasteApp {
    status: String,
    radius: u32,
    capturing: bool,
    processing: bool,
    pending_frame: Option<CapturedFrame>,
    prepared: Vec<(PrepareAction, RgbaFrame)>,
    screen: Option<SelectionView>,
    pins: Vec<PinnedImage>,
    next_pin_id: u64,
    work_tx: Sender<WorkerResult>,
    work_rx: Receiver<WorkerResult>,
    hotkey_rx: Receiver<HotkeyAction>,
    _hotkeys: Option<HotkeyController>,
}

impl PicPasteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (work_tx, work_rx) = mpsc::channel();
        let (hotkey_tx, hotkey_rx) = mpsc::channel();
        let (hotkeys, status) =
            match HotkeyController::install(cc.egui_ctx.clone(), hotkey_tx) {
                Ok(hotkeys) => (Some(hotkeys), String::from("快捷键已注册")),
                Err(error) => (None, format!("热键不可用，仍可使用窗口按钮：{error}")),
            };
        Self {
            status,
            radius: 16,
            capturing: false,
            processing: false,
            pending_frame: None,
            prepared: Vec::new(),
            screen: None,
            pins: Vec::new(),
            next_pin_id: 0,
            work_tx,
            work_rx,
            hotkey_rx,
            _hotkeys: hotkeys,
        }
    }

    fn begin_capture(&mut self, ctx: &egui::Context) {
        if self.capturing || self.processing {
            return;
        }
        self.capturing = true;
        self.status = String::from("正在获取屏幕快照…");
        self.screen = None;
        // Hide our own window first, so it cannot contaminate the screenshot.
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        let sender = self.work_tx.clone();
        let wake = ctx.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(170));
            let result = MacCapture::capture_primary();
            let _ = sender.send(WorkerResult::Captured(result));
            wake.request_repaint();
        });
    }

    fn request_prepared(&mut self, ctx: &egui::Context, action: PrepareAction) {
        if self.processing {
            return;
        }
        let Some(screen) = &self.screen else { return };
        self.processing = true;
        self.status = String::from("正在处理 PNG 图像…");
        let frame = Arc::clone(&screen.source);
        let selection = screen.selected_region();
        let radius = self.radius;
        let sender = self.work_tx.clone();
        let wake = ctx.clone();
        thread::spawn(move || {
            let result = shot_image::crop_rgba(&frame, selection)
                .map_err(|e| format!("crop failed: {e:?}"))
                .map(|mut image| {
                    shot_image::rounded_alpha(&mut image, radius);
                    image
                });
            let _ = sender.send(WorkerResult::Prepared(action, result));
            wake.request_repaint();
        });
    }

    fn paste_clipboard_as_pin(&mut self, ctx: &egui::Context) {
        if self.pins.iter().any(|pin| pin.passthrough) {
            self.recover_pins();
            return;
        }
        let result = (|| {
            let image = Clipboard::new().map_err(|e| e.to_string())?
                .get_image().map_err(|e| e.to_string())?;
            let w = u32::try_from(image.width).map_err(|e| e.to_string())?;
            let h = u32::try_from(image.height).map_err(|e| e.to_string())?;
            RgbaFrame::new(w, h, image.bytes.into_owned())
                .map_err(|e| format!("invalid clipboard frame: {e:?}"))
        })();
        match result {
            Ok(image) => self.create_pin(ctx, image),
            Err(error) => self.status = format!("剪贴板没有可用图片：{error}"),
        }
    }

    fn create_pin(&mut self, ctx: &egui::Context, image: RgbaFrame) {
        let Some(id) = self.next_pin_id.checked_add(1) else {
            self.status = String::from("贴图 ID 已用尽");
            return;
        };
        self.next_pin_id = id;
        self.pins.push(PinnedImage::new(ctx, id, Arc::new(image)));
        self.status = format!("已创建贴图 #{id}");
        ctx.request_repaint();
    }

    fn recover_pins(&mut self) {
        for pin in &mut self.pins {
            pin.passthrough = false;
        }
        self.status = String::from("所有贴图已解除鼠标穿透");
    }

    fn process_events(&mut self, ctx: &egui::Context) {
        let actions: Vec<_> = self.hotkey_rx.try_iter().collect();
        for action in actions {
            match action {
                HotkeyAction::Capture => self.begin_capture(ctx),
                HotkeyAction::PastePin => self.paste_clipboard_as_pin(ctx),
            }
        }
        let events: Vec<_> = self.work_rx.try_iter().collect();
        for event in events {
            match event {
                WorkerResult::Captured(result) => {
                    self.capturing = false;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    match result {
                        Ok(frame) => {
                            self.pending_frame = Some(frame);
                            self.status = String::from("截图已完成，请拖动框选区域");
                        }
                        Err(error) => {
                            self.status = format!(
                                "截图失败: {error:?}。请检查系统设置→隐私与安全→屏幕录制"
                            );
                        }
                    }
                }
                WorkerResult::Prepared(action, result) => {
                    self.processing = false;
                    match result {
                        Ok(frame) => self.prepared.push((action, frame)),
                        Err(error) => self.status = format!("图像处理失败: {error}"),
                    }
                }
                WorkerResult::Saved(result) => {
                    self.status = match result {
                        Ok(path) => format!("PNG 已保存: {}", path.display()),
                        Err(error) => format!("PNG 保存失败: {error}"),
                    };
                }
            }
        }
        self.pins.retain(|pin| !pin.is_closed());
    }

    fn process_ui_images(&mut self, ctx: &egui::Context) {
        if let Some(captured) = self.pending_frame.take() {
            self.screen = Some(SelectionView::new(ctx, Arc::new(captured.image)));
        }
        for (action, image) in std::mem::take(&mut self.prepared) {
            match action {
                PrepareAction::Save => {
                    let sender = self.work_tx.clone();
                    let wake = ctx.clone();
                    thread::spawn(move || {
                        let result = save_in_pictures(&image);
                        let _ = sender.send(WorkerResult::Saved(result));
                        wake.request_repaint();
                    });
                }
                PrepareAction::Copy => {
                    self.status = match copy_to_clipboard(image) {
                        Ok(()) => String::from("截图已复制到剪贴板"),
                        Err(error) => format!("复制失败: {error}"),
                    };
                }
                PrepareAction::Pin => self.create_pin(ctx, image),
            }
        }
    }
}

impl eframe::App for PicPasteApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_events(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.process_ui_images(ui.ctx());
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("PicPaste");
            ui.label("离线截图 · PNG 透明圆角 · 多贴图");
            ui.horizontal(|ui| {
                if ui.button("截图  Ctrl+Option+1").clicked() {
                    self.begin_capture(ui.ctx());
                }
                if ui.button("从剪贴板贴图  Ctrl+Option+2").clicked() {
                    self.paste_clipboard_as_pin(ui.ctx());
                }
                if ui.button("解除所有穿透").clicked() {
                    self.recover_pins();
                }
            });
            ui.horizontal(|ui| {
                ui.label("PNG 圆角（物理像素）");
                ui.add(egui::Slider::new(&mut self.radius, 0..=64));
                if self.capturing || self.processing {
                    ui.spinner();
                }
            });
            ui.label(&self.status);
            ui.separator();

            if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
                self.screen = None;
            }

            let mut selected_action = None;
            if let Some(screen) = &mut self.screen {
                ui.horizontal(|ui| {
                    if ui.button("保存 PNG").clicked() {
                        selected_action = Some(PrepareAction::Save);
                    }
                    if ui.button("复制图片").clicked() {
                        selected_action = Some(PrepareAction::Copy);
                    }
                    if ui.button("贴图").clicked() {
                        selected_action = Some(PrepareAction::Pin);
                    }
                    if ui.button("重置选区").clicked() {
                        screen.selection = None;
                    }
                });
                screen.show(ui);
            } else {
                ui.label("按 Ctrl+Option+1 开始截图；或点击“截图”。");
            }
            if let Some(action) = selected_action {
                self.request_prepared(ui.ctx(), action);
            }
            if !self.pins.is_empty() {
                ui.separator();
                ui.heading("悬浮贴图");
                for pin in &mut self.pins {
                    ui.horizontal(|ui| {
                        ui.label(format!("#{}", pin.id));
                        ui.checkbox(&mut pin.passthrough, "鼠标穿透");
                        ui.add(egui::Slider::new(&mut pin.opacity, 0.2..=1.0).text("透明度"));
                        if ui.small_button("关闭").clicked() {
                            pin.closed.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    });
                }
            }
        });
        for pin in &self.pins {
            if !pin.is_closed() {
                pin.show(ui.ctx());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_maps_logical_preview_to_source_pixels() {
        let frame = RgbaFrame::new(200, 100, vec![0; 200 * 100 * 4]).unwrap();
        let rect = egui::Rect::from_min_size(
            egui::pos2(50.0, 80.0), egui::vec2(100.0, 50.0),
        );
        assert_eq!(point_to_pixel(&rect, &frame, egui::pos2(50.0, 80.0)), (0, 0));
        assert_eq!(point_to_pixel(&rect, &frame, egui::pos2(149.0, 129.0)), (198, 98));
        assert_eq!(point_to_pixel(&rect, &frame, egui::pos2(170.0, 140.0)), (199, 99));
    }
}
