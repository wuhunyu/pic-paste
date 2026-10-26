use crate::hotkey::{HotkeyAction, HotkeyController};
use crate::menu::{MenuAction, MenuBar};
use crate::ocr;
use crate::pin::PinnedImage;
use crate::scroll::{self, ScrollCaptureControl, ScrollEvent};
use arboard::{Clipboard, ImageData};
use eframe::egui;
use shot_core::overlay::OverlayState;
use shot_core::platform::{CapturedFrame, DisplayId, PlatformError, ScrollAxis};
use shot_core::selection::from_drag;
use shot_core::{RectPx, RgbaFrame};
use shot_image::annotation_tools::{Tool, annotation_for_drag};
use shot_image::history::{Annotation, AnnotationDocument};
use shot_platform::macos::MacCapture;
use shot_ocr_protocol::hit_test::{hit_line_in_capture, selected_text};
use shot_ocr_protocol::OcrDocument;
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

enum WorkerResult {
    Captured(Result<CapturedFrame, PlatformError>),
    Prepared(PrepareAction, Result<RgbaFrame, String>),
    Saved(Result<PathBuf, String>),
    Ocr {
        region: RectPx,
        outcome: Result<OcrDocument, String>,
    },
}

#[derive(Clone, Copy)]
enum PrepareAction {
    Save,
    Copy,
    Pin,
}

struct SelectionView {
    display_id: DisplayId,
    source: Arc<RgbaFrame>,
    rendered: Arc<RgbaFrame>,
    annotations: AnnotationDocument,
    tool: Tool,
    render_error: Option<String>,
    texture: egui::TextureHandle,
    drag_anchor: Option<egui::Pos2>,
    brush_points: Vec<(f32, f32)>,
    selection: Option<RectPx>,
    ocr_result: Option<OcrDocument>,
    ocr_origin: (u32, u32),
    ocr_mode: bool,
    ocr_line: Option<usize>,
}

impl SelectionView {
    fn new(ctx: &egui::Context, display_id: DisplayId, source: Arc<RgbaFrame>) -> Self {
        let texture = ctx.load_texture(
            "capture-preview",
            egui::ColorImage::from_rgba_unmultiplied(
                [source.width as usize, source.height as usize],
                &source.rgba,
            ),
            egui::TextureOptions::LINEAR,
        );
        Self {
            display_id,
            rendered: Arc::clone(&source),
            annotations: AnnotationDocument::new(Arc::clone(&source)),
            tool: Tool::Select,
            render_error: None,
            source,
            texture,
            drag_anchor: None,
            brush_points: Vec::new(),
            selection: None,
            ocr_result: None,
            ocr_origin: (0, 0),
            ocr_mode: false,
            ocr_line: None,
        }
    }

    fn refresh_preview(&mut self) {
        // OCR positions belong to the specific composed image. Any raster edit
        // invalidates them; a new explicit OCR action is required.
        self.ocr_result = None;
        self.ocr_line = None;
        self.ocr_mode = false;
        match shot_image::composite::composite(&self.source, self.annotations.visible()) {
            Ok(image) => {
                self.texture.set(
                    egui::ColorImage::from_rgba_unmultiplied(
                        [image.width as usize, image.height as usize],
                        &image.rgba,
                    ),
                    egui::TextureOptions::LINEAR,
                );
                self.rendered = Arc::new(image);
                self.render_error = None;
            }
            Err(error) => self.render_error = Some(error.to_owned()),
        }
    }

    fn undo(&mut self) {
        if self.annotations.undo() {
            self.refresh_preview();
        }
    }

    fn redo(&mut self) {
        if self.annotations.redo() {
            self.refresh_preview();
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
        if self.ocr_mode {
            ui.label("文字选择模式：点击高亮的识别行以选中；仅当明确启用时才响应文字点击。");
        } else {
            ui.label("在预览中拖动选择区域；不选择则处理整个显示器。Esc 取消。");
        }
        let width = self.source.width as f32;
        let height = self.source.height as f32;
        let available = ui.available_size() - egui::vec2(0.0, 12.0);
        let scale = (available.x / width)
            .min(available.y / height)
            .clamp(0.01, 1.0);
        let displayed = egui::vec2(width * scale, height * scale);
        let response = ui.add(
            egui::Image::new((self.texture.id(), displayed)).sense(egui::Sense::click_and_drag()),
        );

        if !self.ocr_mode {
            if response.drag_started() {
                self.drag_anchor = response.interact_pointer_pos();
                self.brush_points.clear();
                if self.tool == Tool::Brush
                    && let Some(start) = self.drag_anchor
                {
                    let (x, y) = point_to_pixel(&response.rect, &self.source, start);
                    self.brush_points.push((x as f32, y as f32));
                }
            }
            if (response.dragged() || response.drag_stopped())
                && let (Some(start), Some(end)) = (self.drag_anchor, response.interact_pointer_pos())
            {
                let a = point_to_pixel(&response.rect, &self.source, start);
                let b = point_to_pixel(&response.rect, &self.source, end);
                if self.tool == Tool::Select {
                    self.selection = from_drag(a, b).ok();
                } else if self.tool == Tool::Brush {
                    let last = (b.0 as f32, b.1 as f32);
                    if self.brush_points.last() != Some(&last) && self.brush_points.len() < 20_000 {
                        self.brush_points.push(last);
                    }
                    if response.drag_stopped() && !self.brush_points.is_empty() {
                        self.annotations.apply(Annotation::Brush {
                            points: std::mem::take(&mut self.brush_points),
                            rgba: [39, 178, 231, 255],
                            width: 3.0,
                        });
                        self.refresh_preview();
                    }
                } else if response.drag_stopped()
                    && let Some(annotation) = annotation_for_drag(self.tool, a, b)
                {
                    self.annotations.apply(annotation);
                    self.refresh_preview();
                }
            }
            if response.drag_stopped() {
                self.drag_anchor = None;
            }
        }

        if self.ocr_mode {
            if response.clicked()
                && let Some(pointer) = response.interact_pointer_pos()
                && let Some(document) = self.ocr_result.as_ref()
            {
                let (x, y) = point_to_pixel(&response.rect, &self.source, pointer);
                self.ocr_line = hit_line_in_capture(
                    document,
                    self.ocr_origin,
                    (x as f32, y as f32),
                );
            }
            if let Some(document) = self.ocr_result.as_ref() {
                let sx = response.rect.width() / self.source.width as f32;
                let sy = response.rect.height() / self.source.height as f32;
                for (index, line) in document.lines.iter().enumerate() {
                    let color = if self.ocr_line == Some(index) {
                        egui::Color32::from_rgb(30, 230, 160)
                    } else {
                        egui::Color32::from_rgb(72, 192, 245)
                    };
                    for edge in 0..4 {
                        let a = line.polygon_px[edge];
                        let b = line.polygon_px[(edge + 1) % 4];
                        let screen_pos = |p: [f32; 2]| {
                            egui::pos2(
                                response.rect.left() + (self.ocr_origin.0 as f32 + p[0]) * sx,
                                response.rect.top() + (self.ocr_origin.1 as f32 + p[1]) * sy,
                            )
                        };
                        ui.painter().line_segment(
                            [screen_pos(a), screen_pos(b)],
                            egui::Stroke::new(2.0, color),
                        );
                    }
                }
            }
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
    let home = std::env::var_os("HOME").ok_or_else(|| String::from("HOME is unavailable"))?;
    let folder = PathBuf::from(home).join("Pictures").join("PicPaste");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
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
    overlay: OverlayState,
    ocr_waiting: bool,
    ocr_text: String,
    ocr_lines: usize,
    pending_frame: Option<CapturedFrame>,
    prepared: Vec<(PrepareAction, RgbaFrame)>,
    screen: Option<SelectionView>,
    pins: Vec<PinnedImage>,
    next_pin_id: u64,
    work_tx: Sender<WorkerResult>,
    work_rx: Receiver<WorkerResult>,
    hotkey_rx: Receiver<HotkeyAction>,
    scroll_tx: Sender<ScrollEvent>,
    scroll_rx: Receiver<ScrollEvent>,
    scroll_control: Option<ScrollCaptureControl>,
    menu_rx: Receiver<MenuAction>,
    _menu: Option<MenuBar>,
    quit_requested: bool,
    _hotkeys: Option<HotkeyController>,
}

impl PicPasteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (work_tx, work_rx) = mpsc::channel();
        let (hotkey_tx, hotkey_rx) = mpsc::channel();
        let (scroll_tx, scroll_rx) = mpsc::channel();
        let (menu_tx, menu_rx) = mpsc::channel();
        let (menu, menu_status) = match MenuBar::install(cc.egui_ctx.clone(), menu_tx) {
            Ok(menu) => (Some(menu), None),
            Err(error) => (None, Some(format!("菜单栏不可用：{error}"))),
        };
        let (hotkeys, status) = match HotkeyController::install(cc.egui_ctx.clone(), hotkey_tx) {
            Ok(hotkeys) => (Some(hotkeys), String::from("快捷键已注册")),
            Err(error) => (None, format!("热键不可用，仍可使用窗口按钮：{error}")),
        };
        Self {
            status: menu_status.unwrap_or(status),
            radius: 16,
            capturing: false,
            processing: false,
            overlay: OverlayState::default(),
            ocr_waiting: false,
            ocr_text: String::new(),
            ocr_lines: 0,
            pending_frame: None,
            prepared: Vec::new(),
            screen: None,
            pins: Vec::new(),
            next_pin_id: 0,
            work_tx,
            work_rx,
            hotkey_rx,
            scroll_tx,
            scroll_rx,
            scroll_control: None,
            menu_rx,
            _menu: menu,
            quit_requested: false,
            _hotkeys: hotkeys,
        }
    }

    fn begin_capture(&mut self, ctx: &egui::Context) {
        if self.capturing
            || self.processing
            || self.ocr_waiting
            || self.scroll_control.is_some()
            || !self.overlay.begin_capture()
        {
            return;
        }
        self.capturing = true;
        self.status = String::from("正在获取屏幕快照…");
        self.screen = None;
        self.ocr_text.clear();
        self.ocr_lines = 0;
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

    fn start_manual_scroll(&mut self, ctx: &egui::Context, axis: ScrollAxis) {
        if self.capturing || self.processing || self.scroll_control.is_some() {
            self.status = String::from("已有截图任务正在执行，请先完成或取消");
            return;
        }
        let Some(screen) = &self.screen else {
            self.status = String::from("请先选择一张截图及滚动区域");
            return;
        };
        if !screen.annotations.visible().is_empty() {
            self.status = String::from("长截图只能处理原始区域。请先撤销标注或重新截图");
            return;
        }
        let selected = screen.selected_region();
        if selected.width < 24 || selected.height < 24 {
            self.status = String::from("长截图区域太小，宽高均需至少 24 个物理像素");
            return;
        }
        let direction = match axis {
            ScrollAxis::Vertical => "纵向",
            ScrollAxis::Horizontal => "横向",
        };
        self.status = format!(
            "开始{direction}手动滚动，请滚动目标应用；结束时按 Ctrl+Option+4 或在菜单栏点击完成"
        );
        // The selection is a physical-pixel, display-local rectangle.
        let control = scroll::launch(
            screen.display_id,
            selected,
            axis,
            self.scroll_tx.clone(),
            ctx.clone(),
        );
        self.scroll_control = Some(control);
        self.overlay.dismiss();
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }

    fn finish_manual_scroll(&mut self) {
        if let Some(control) = &self.scroll_control {
            control.finish();
            self.status = String::from("结束滚动，正在写入 PNG 文件");
        }
    }

    fn cancel_manual_scroll(&mut self) {
        if let Some(control) = &self.scroll_control {
            control.cancel();
            self.status = String::from("取消滚动，正在删除临时图片");
        }
    }

    fn request_prepared(&mut self, ctx: &egui::Context, action: PrepareAction) {
        if self.processing {
            return;
        }
        let Some(screen) = &self.screen else { return };
        self.processing = true;
        self.status = String::from("正在处理 PNG 图像…");
        let frame = Arc::clone(&screen.rendered);
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

    fn request_ocr(&mut self, ctx: &egui::Context) {
        if self.ocr_waiting || self.processing || self.scroll_control.is_some() {
            return;
        }
        let Some(screen) = &self.screen else { return };
        self.ocr_waiting = true;
        self.ocr_text.clear();
        self.ocr_lines = 0;
        self.status = String::from("正在使用离线 Vision Accurate 识别文字…");
        let source = Arc::clone(&screen.rendered);
        let region = screen.selected_region();
        let sender = self.work_tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let result = shot_image::crop_rgba(&source, region)
                .map_err(|error| format!("OCR 区域无效：{error:?}"))
                .and_then(ocr::recognize);
            let _ = sender.send(WorkerResult::Ocr {
                region,
                outcome: result,
            });
            ctx.request_repaint();
        });
    }

    fn paste_clipboard_as_pin(&mut self, ctx: &egui::Context) {
        let result = (|| {
            let image = Clipboard::new()
                .map_err(|e| e.to_string())?
                .get_image()
                .map_err(|e| e.to_string())?;
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
        let menu_actions: Vec<_> = self.menu_rx.try_iter().collect();
        for action in menu_actions {
            match action {
                MenuAction::Capture => self.begin_capture(ctx),
                MenuAction::Paste => self.paste_clipboard_as_pin(ctx),
                MenuAction::Recover => self.recover_pins(),
                MenuAction::FinishScroll => self.finish_manual_scroll(),
                MenuAction::CancelScroll => self.cancel_manual_scroll(),
                MenuAction::Show => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                MenuAction::Quit => {
                    self.quit_requested = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        let actions: Vec<_> = self.hotkey_rx.try_iter().collect();
        for action in actions {
            match action {
                HotkeyAction::Capture => self.begin_capture(ctx),
                HotkeyAction::PastePin => self.paste_clipboard_as_pin(ctx),
                HotkeyAction::RecoverPins => self.recover_pins(),
                HotkeyAction::FinishScroll => self.finish_manual_scroll(),
            }
        }
        let events: Vec<_> = self.work_rx.try_iter().collect();
        for event in events {
            match event {
                WorkerResult::Captured(result) => {
                    self.capturing = false;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    match result {
                        Ok(frame) if self.overlay.captured(true) => {
                            self.pending_frame = Some(frame);
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                            self.status = String::from("全屏截图选区已打开；拖动选择，Esc 取消");
                        }
                        Ok(_) => {
                            self.status = String::from("截图已取消，丢弃迟到的捕获结果");
                        }
                        Err(error) => {
                            self.overlay.captured(false);
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                            self.status =
                                format!("截图失败: {error:?}。请检查系统设置→隐私与安全→屏幕录制");
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
                WorkerResult::Ocr { region, outcome } => {
                    self.ocr_waiting = false;
                    match outcome {
                        Ok(document) => {
                            self.ocr_lines = document.lines.len();
                            self.ocr_text = document
                                .lines
                                .iter()
                                .map(|line| line.text.as_str())
                                .collect::<Vec<_>>()
                                .join("\n");
                            if let Some(screen) = self.screen.as_mut() {
                                screen.ocr_origin = (region.x as u32, region.y as u32);
                                screen.ocr_mode = true;
                                screen.ocr_line = None;
                                screen.ocr_result = Some(document);
                            }
                            self.status = format!(
                                "Vision OCR 完成，共 {} 行；可点击原图上的文字框选行",
                                self.ocr_lines
                            );
                        }
                        Err(error) => self.status = format!("文字识别失败：{error}"),
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
        let scroll_events: Vec<_> = self.scroll_rx.try_iter().collect();
        for event in scroll_events {
            match event {
                ScrollEvent::Progress {
                    segments,
                    width,
                    height,
                } => {
                    self.status = format!(
                        "长截图正在拼接：{segments} 段，{width}×{height}px。滚动后按 Ctrl+Option+4 完成"
                    );
                }
                ScrollEvent::Finished(outcome) => {
                    self.scroll_control.take();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    self.status = match outcome {
                        Ok(Some(completion)) if completion.partial => format!(
                            "滚动位置匹配失败或达到限制，已安全保存前 {} 段为部分 PNG：{}",
                            completion.segments,
                            completion.path.display()
                        ),
                        Ok(Some(completion)) => format!(
                            "长截图完成：{} 段，PNG 位于 {}",
                            completion.segments,
                            completion.path.display()
                        ),
                        Ok(None) => String::from("已取消长截图，所有临时分块已清理"),
                        Err(error) => format!("长截图失败：{error}"),
                    };
                }
            }
        }
        self.pins.retain(|pin| !pin.is_closed());
    }

    fn process_ui_images(&mut self, ctx: &egui::Context) {
        if let Some(captured) = self.pending_frame.take() {
            self.screen = Some(SelectionView::new(
                ctx,
                captured.display_id,
                Arc::new(captured.image),
            ));
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
        // macOS convention: closing the settings window hides it while the
        // status bar icon and global hotkeys stay active.
        if self._menu.is_some()
            && !self.quit_requested
            && ctx.input(|input| input.viewport().close_requested())
        {
            if self.overlay.dismiss() {
                self.screen = None;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.process_ui_images(ui.ctx());
        egui::CentralPanel::default().show(ui, |ui| {
            if self.overlay.is_fullscreen() {
                ui.heading("PicPaste · 全屏截图");
                ui.label("拖动框选或标注 · 复制/保存后返回窗口 · Esc 放弃截图");
            } else {
                ui.heading("PicPaste");
                ui.label("离线截图 · PNG 透明圆角 · 多贴图");
            }
            if !self.overlay.is_fullscreen() {
                ui.horizontal(|ui| {
                if ui.button("截图  Ctrl+Option+1").clicked() {
                    self.begin_capture(ui.ctx());
                }
                if ui.button("从剪贴板贴图  Ctrl+Option+2").clicked() {
                    self.paste_clipboard_as_pin(ui.ctx());
                }
                if ui.button("解除穿透 Ctrl+Option+3").clicked() {
                    self.recover_pins();
                }
                });
            }
            ui.horizontal(|ui| {
                ui.label("PNG 圆角（物理像素）");
                ui.add(egui::Slider::new(&mut self.radius, 0..=64));
                if self.capturing || self.processing {
                    ui.spinner();
                }
            });
            ui.label(&self.status);
            if self.scroll_control.is_some() {
                ui.horizontal(|ui| {
                    if ui.button("完成长截图 Ctrl+Option+4").clicked() {
                        self.finish_manual_scroll();
                    }
                    if ui.button("取消并清理").clicked() {
                        self.cancel_manual_scroll();
                    }
                });
            }
            ui.separator();

            if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
                self.screen = None;
                if self.overlay.dismiss() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                }
            }

            let mut selected_action = None;
            let mut start_scroll = None;
            let mut recognize_text = false;
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
                    if ui.button("手动长截图 ↓").clicked() {
                        start_scroll = Some(ScrollAxis::Vertical);
                    }
                    if ui.button("手动长截图 →").clicked() {
                        start_scroll = Some(ScrollAxis::Horizontal);
                    }
                    if ui.button("识别文字 (Full)").clicked() {
                        recognize_text = true;
                    }
                });
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut screen.tool, Tool::Select, "选区");
                    ui.selectable_value(&mut screen.tool, Tool::Line, "线条");
                    ui.selectable_value(&mut screen.tool, Tool::Arrow, "箭头");
                    ui.selectable_value(&mut screen.tool, Tool::Brush, "画笔");
                    ui.selectable_value(&mut screen.tool, Tool::Rectangle, "矩形");
                    ui.selectable_value(&mut screen.tool, Tool::Ellipse, "椭圆");
                    ui.selectable_value(&mut screen.tool, Tool::Pixelate, "马赛克");
                    ui.selectable_value(&mut screen.tool, Tool::Blur, "模糊");
                    if ui.small_button("撤销").clicked() {
                        screen.undo();
                    }
                    if ui.small_button("重做").clicked() {
                        screen.redo();
                    }
                });
                if ui
                    .ctx()
                    .input(|input| input.modifiers.command && input.key_pressed(egui::Key::Z))
                {
                    if ui.ctx().input(|input| input.modifiers.shift) {
                        screen.redo();
                    } else {
                        screen.undo();
                    }
                }
                if let Some(error) = &screen.render_error {
                    ui.colored_label(egui::Color32::RED, format!("预览渲染失败：{error}"));
                }
                screen.show(ui);
                if screen.ocr_result.is_some() {
                    let picked = screen.ocr_line.and_then(|index| {
                        screen
                            .ocr_result
                            .as_ref()
                            .map(|document| selected_text(document, &[index]))
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut screen.ocr_mode, "文字选择模式");
                        if let Some(text) = picked
                            && !text.is_empty()
                            && ui.button("复制选中的文字行").clicked()
                        {
                            self.status = match Clipboard::new()
                                .and_then(|mut clipboard| clipboard.set_text(text))
                            {
                                Ok(()) => String::from("已复制选中的 OCR 文字行"),
                                Err(error) => format!("复制选中行失败：{error}"),
                            };
                        }
                    });
                } else if self.ocr_lines > 0 {
                    self.ocr_lines = 0;
                    self.ocr_text.clear();
                }
            } else {
                ui.label("按 Ctrl+Option+1 开始截图；或点击“截图”。");
            }
            if let Some(action) = selected_action {
                self.request_prepared(ui.ctx(), action);
                if self.overlay.accept() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                }
            }
            if let Some(axis) = start_scroll {
                self.start_manual_scroll(ui.ctx(), axis);
            }
            if recognize_text {
                self.request_ocr(ui.ctx());
            }
            if self.ocr_waiting {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Vision OCR 正在独立进程中运行");
                });
            }
            if self.ocr_lines > 0 {
                ui.separator();
                ui.heading(format!("识别文字 · {} 行", self.ocr_lines));
                if ui.button("复制全部文字").clicked() {
                    self.status = match Clipboard::new()
                        .and_then(|mut clipboard| clipboard.set_text(self.ocr_text.clone()))
                    {
                        Ok(()) => String::from("识别文字已复制到剪贴板"),
                        Err(error) => format!("文本复制失败：{error}"),
                    };
                }
                ui.add(egui::TextEdit::multiline(&mut self.ocr_text).desired_rows(7));
            }
            if !self.pins.is_empty() {
                ui.separator();
                ui.heading("悬浮贴图");
                for pin in &mut self.pins {
                    ui.horizontal(|ui| {
                        ui.label(format!("#{}", pin.id));
                        ui.checkbox(&mut pin.passthrough, "鼠标穿透");
                        ui.add(egui::Slider::new(&mut pin.opacity, 0.2..=1.0).text("透明度"));
                        ui.add(egui::Slider::new(&mut pin.zoom, 0.25..=3.0).text("缩放"));
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
        let rect = egui::Rect::from_min_size(egui::pos2(50.0, 80.0), egui::vec2(100.0, 50.0));
        assert_eq!(
            point_to_pixel(&rect, &frame, egui::pos2(50.0, 80.0)),
            (0, 0)
        );
        assert_eq!(
            point_to_pixel(&rect, &frame, egui::pos2(149.0, 129.0)),
            (198, 98)
        );
        assert_eq!(
            point_to_pixel(&rect, &frame, egui::pos2(170.0, 140.0)),
            (199, 99)
        );
    }
}
