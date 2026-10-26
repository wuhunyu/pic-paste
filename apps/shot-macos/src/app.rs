use crate::hotkey::{HotkeyAction, HotkeyController};
use crate::menu::{MenuAction, MenuBar};
use crate::ocr;
use crate::overlay;
use crate::pin::PinnedImage;
use crate::scroll::{self, ScrollCaptureControl, ScrollEvent};
use arboard::{Clipboard, ImageData};
use eframe::egui;
use shot_core::async_ticket::OcrTicket;
use shot_core::display::{DisplaySelection, OverlayPlacement};
use shot_core::overlay::OverlayState;
use shot_core::platform::{CapturedFrame, DisplayId, PlatformError, ScrollAxis};
use shot_core::selection::{ResizeHandle, adjust_selection, from_drag, hit_selection_handle};
use shot_core::shortcuts::{HotkeyBindings, ShortcutAction};
use shot_core::{RectPx, RgbaFrame};
use shot_image::annotation_tools::{Tool, annotation_for_drag};
use shot_image::history::{Annotation, AnnotationDocument};
use shot_image::pixel_tools::{magnifier_patch, sample_rgba};
use shot_ocr_protocol::OcrDocument;
use shot_ocr_protocol::hit_test::{hit_line_in_capture, selected_text};
use shot_platform::macos::MacCapture;
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

enum WorkerResult {
    Captured {
        selection: DisplaySelection,
        outcome: Result<(CapturedFrame, OverlayPlacement), PlatformError>,
    },
    Prepared(PrepareAction, Result<RgbaFrame, String>),
    Saved(Result<PathBuf, String>),
    ScrollSaved(Result<PathBuf, (String, scroll::ScrollReview)>),
    ScrollEdited(Result<scroll::ScrollReview, (String, scroll::ScrollReview)>),
    Ocr {
        ticket: OcrTicket,
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
    capture_revision: u64,
    edit_revision: u64,
    source: Arc<RgbaFrame>,
    rendered: Arc<RgbaFrame>,
    annotations: AnnotationDocument,
    tool: Tool,
    render_error: Option<String>,
    texture: egui::TextureHandle,
    magnifier_texture: egui::TextureHandle,
    last_hover_color: Option<((i32, i32), [u8; 4])>,
    drag_anchor: Option<egui::Pos2>,
    resize_drag: Option<(ResizeHandle, RectPx, (i32, i32))>,
    brush_points: Vec<(f32, f32)>,
    selection: Option<RectPx>,
    ocr_result: Option<OcrDocument>,
    ocr_origin: (u32, u32),
    ocr_mode: bool,
    ocr_line: Option<usize>,
}

impl SelectionView {
    fn new(
        ctx: &egui::Context,
        display_id: DisplayId,
        capture_revision: u64,
        source: Arc<RgbaFrame>,
    ) -> Self {
        let texture = ctx.load_texture(
            "capture-preview",
            egui::ColorImage::from_rgba_unmultiplied(
                [source.width as usize, source.height as usize],
                &source.rgba,
            ),
            egui::TextureOptions::LINEAR,
        );
        let magnifier_texture = ctx.load_texture(
            "capture-magnifier",
            egui::ColorImage::from_rgba_unmultiplied([9, 9], &[0; 9 * 9 * 4]),
            egui::TextureOptions::NEAREST,
        );
        Self {
            display_id,
            capture_revision,
            edit_revision: 0,
            rendered: Arc::clone(&source),
            annotations: AnnotationDocument::new(Arc::clone(&source)),
            tool: Tool::Select,
            render_error: None,
            source,
            texture,
            magnifier_texture,
            last_hover_color: None,
            drag_anchor: None,
            resize_drag: None,
            brush_points: Vec::new(),
            selection: None,
            ocr_result: None,
            ocr_origin: (0, 0),
            ocr_mode: false,
            ocr_line: None,
        }
    }

    fn invalidate_ocr(&mut self) {
        self.edit_revision = self.edit_revision.saturating_add(1);
        self.ocr_result = None;
        self.ocr_line = None;
        self.ocr_mode = false;
    }

    fn set_selection(&mut self, selection: Option<RectPx>) {
        if self.selection != selection {
            self.selection = selection;
            self.invalidate_ocr();
        }
    }

    fn begin_selection_adjustment(&mut self, point: (i32, i32), tolerance: i32) -> bool {
        let Some(rect) = self.selection else {
            return false;
        };
        let Some(handle) = hit_selection_handle(rect, point, tolerance) else {
            return false;
        };
        self.resize_drag = Some((handle, rect, point));
        true
    }

    fn drag_selection_adjustment(&mut self, point: (i32, i32)) {
        let Some((handle, original, start)) = self.resize_drag else {
            return;
        };
        let delta = (
            point.0.saturating_sub(start.0),
            point.1.saturating_sub(start.1),
        );
        if let Ok(next) = adjust_selection(
            original,
            handle,
            delta,
            (self.source.width, self.source.height),
        ) {
            self.set_selection(Some(next));
        }
    }

    fn end_selection_adjustment(&mut self) {
        self.resize_drag = None;
    }

    fn magnifier_at(&self, point: (i32, i32)) -> Option<(RgbaFrame, [u8; 4])> {
        let color = sample_rgba(&self.source, point)?;
        let patch = magnifier_patch(&self.source, point, 9).ok()?;
        Some((patch, color))
    }

    fn ocr_ticket(&self) -> OcrTicket {
        OcrTicket::new(
            self.capture_revision,
            self.edit_revision,
            self.selected_region(),
        )
    }

    fn accepts_ocr(&self, ticket: OcrTicket) -> bool {
        ticket.matches(
            self.capture_revision,
            self.edit_revision,
            self.selected_region(),
        )
    }

    fn refresh_preview(&mut self) {
        // OCR coordinates belong to a particular immutable screenshot revision.
        self.invalidate_ocr();
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
            ui.label("拖动框选；拖动选区内部可移动，拖动四角和边缘可调整。Esc 取消。");
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
                if self.tool == Tool::Select
                    && let Some(start) = self.drag_anchor
                {
                    let point = point_to_pixel(&response.rect, &self.source, start);
                    let tolerance = (6.0 / scale).ceil().clamp(2.0, 60.0) as i32;
                    self.begin_selection_adjustment(point, tolerance);
                }
                if self.tool == Tool::Brush
                    && let Some(start) = self.drag_anchor
                {
                    let (x, y) = point_to_pixel(&response.rect, &self.source, start);
                    self.brush_points.push((x as f32, y as f32));
                }
            }
            if (response.dragged() || response.drag_stopped())
                && let (Some(start), Some(end)) =
                    (self.drag_anchor, response.interact_pointer_pos())
            {
                let a = point_to_pixel(&response.rect, &self.source, start);
                let b = point_to_pixel(&response.rect, &self.source, end);
                if self.tool == Tool::Select {
                    if self.resize_drag.is_some() {
                        self.drag_selection_adjustment(b);
                    } else {
                        self.set_selection(from_drag(a, b).ok());
                    }
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
                self.end_selection_adjustment();
            }
        }

        if !self.ocr_mode
            && let Some(pointer) = response.hover_pos()
        {
            let point = point_to_pixel(&response.rect, &self.source, pointer);
            if let Some((patch, color)) = self.magnifier_at(point) {
                self.last_hover_color = Some((point, color));
                self.magnifier_texture.set(
                    egui::ColorImage::from_rgba_unmultiplied(
                        [patch.width as usize, patch.height as usize],
                        &patch.rgba,
                    ),
                    egui::TextureOptions::NEAREST,
                );
                let left = (pointer.x + 18.0)
                    .min(response.rect.right() - 134.0)
                    .max(response.rect.left());
                let top = (pointer.y + 18.0)
                    .min(response.rect.bottom() - 150.0)
                    .max(response.rect.top());
                let bubble =
                    egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(134.0, 150.0));
                ui.painter()
                    .rect_filled(bubble, 4.0, egui::Color32::from_black_alpha(235));
                let sample_rect = egui::Rect::from_min_size(
                    egui::pos2(left + 13.0, top + 7.0),
                    egui::vec2(108.0, 108.0),
                );
                ui.painter().image(
                    self.magnifier_texture.id(),
                    sample_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                ui.painter().text(
                    egui::pos2(left + 7.0, top + 123.0),
                    egui::Align2::LEFT_TOP,
                    format!(
                        "{}, {}  #{:02X}{:02X}{:02X}",
                        point.0, point.1, color[0], color[1], color[2]
                    ),
                    egui::FontId::monospace(11.0),
                    egui::Color32::WHITE,
                );
            }
        }
        if self.ocr_mode {
            if response.clicked()
                && let Some(pointer) = response.interact_pointer_pos()
                && let Some(document) = self.ocr_result.as_ref()
            {
                let (x, y) = point_to_pixel(&response.rect, &self.source, pointer);
                self.ocr_line =
                    hit_line_in_capture(document, self.ocr_origin, (x as f32, y as f32));
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
            if self.tool == Tool::Select && !self.ocr_mode {
                let corners_and_edges = [
                    clip.left_top(),
                    egui::pos2(clip.center().x, clip.top()),
                    clip.right_top(),
                    egui::pos2(clip.right(), clip.center().y),
                    clip.right_bottom(),
                    egui::pos2(clip.center().x, clip.bottom()),
                    clip.left_bottom(),
                    egui::pos2(clip.left(), clip.center().y),
                ];
                for center in corners_and_edges {
                    let handle = egui::Rect::from_center_size(center, egui::vec2(7.0, 7.0));
                    ui.painter().rect_filled(handle, 1.0, egui::Color32::WHITE);
                    ui.painter().rect_stroke(
                        handle,
                        1.0,
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(35, 120, 160)),
                        egui::StrokeKind::Inside,
                    );
                }
            }
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
    png_preview_key: Option<(u64, u64, RectPx, u32)>,
    png_preview_texture: Option<egui::TextureHandle>,
    capturing: bool,
    processing: bool,
    overlay: OverlayState,
    capture_revision: u64,
    display_selection: DisplaySelection,
    ocr_waiting: bool,
    ocr_text: String,
    ocr_lines: usize,
    pending_frame: Option<CapturedFrame>,
    overlay_placement: Option<OverlayPlacement>,
    prepared: Vec<(PrepareAction, RgbaFrame)>,
    screen: Option<SelectionView>,
    pins: Vec<PinnedImage>,
    next_pin_id: u64,
    work_tx: Sender<WorkerResult>,
    work_rx: Receiver<WorkerResult>,
    hotkey_rx: Receiver<HotkeyAction>,
    hotkey_tx: Sender<HotkeyAction>,
    shortcuts: HotkeyBindings,
    draft_shortcuts: HotkeyBindings,
    scroll_tx: Sender<ScrollEvent>,
    scroll_rx: Receiver<ScrollEvent>,
    scroll_control: Option<ScrollCaptureControl>,
    scroll_review: Option<scroll::ScrollReview>,
    scroll_preview_pending: Option<RgbaFrame>,
    scroll_preview_texture: Option<egui::TextureHandle>,
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
        let (shortcuts, load_warning) = match HotkeyController::load_settings() {
            Ok(bindings) => (bindings, None),
            Err(error) => (HotkeyBindings::default(), Some(error)),
        };
        let (hotkeys, status) =
            match HotkeyController::install(cc.egui_ctx.clone(), hotkey_tx.clone(), shortcuts) {
                Ok(hotkeys) => (Some(hotkeys), String::from("快捷键已注册")),
                Err(error) => (None, format!("热键不可用，仍可使用窗口按钮：{error}")),
            };
        let status = if let Some(warning) = load_warning {
            format!("{status}；{warning}")
        } else {
            status
        };
        Self {
            status: menu_status.unwrap_or(status),
            radius: 16,
            png_preview_key: None,
            png_preview_texture: None,
            capturing: false,
            processing: false,
            overlay: OverlayState::default(),
            capture_revision: 0,
            display_selection: DisplaySelection::default(),
            ocr_waiting: false,
            ocr_text: String::new(),
            ocr_lines: 0,
            pending_frame: None,
            overlay_placement: None,
            prepared: Vec::new(),
            screen: None,
            pins: Vec::new(),
            next_pin_id: 0,
            work_tx,
            work_rx,
            hotkey_rx,
            hotkey_tx,
            shortcuts,
            draft_shortcuts: shortcuts,
            scroll_tx,
            scroll_rx,
            scroll_control: None,
            scroll_review: None,
            scroll_preview_pending: None,
            scroll_preview_texture: None,
            menu_rx,
            _menu: menu,
            quit_requested: false,
            _hotkeys: hotkeys,
        }
    }

    fn apply_shortcuts(&mut self, ctx: &egui::Context) {
        let next = self.draft_shortcuts;
        let outcome = if let Some(hotkeys) = self._hotkeys.as_mut() {
            hotkeys.rebind(next)
        } else {
            HotkeyController::install(ctx.clone(), self.hotkey_tx.clone(), next).map(|hotkeys| {
                self._hotkeys = Some(hotkeys);
            })
        };
        match outcome {
            Ok(()) => {
                self.shortcuts = next;
                self.status = match HotkeyController::save_settings(next) {
                    Ok(()) => String::from("快捷键已应用并保存"),
                    Err(error) => format!("快捷键已生效，但未能保存：{error}"),
                };
            }
            Err(error) => {
                self.draft_shortcuts = self.shortcuts;
                self.status = format!("快捷键更改失败：{error}");
            }
        }
    }

    fn begin_capture(&mut self, ctx: &egui::Context) {
        self.begin_capture_target(ctx, false);
    }

    fn begin_capture_target(&mut self, ctx: &egui::Context, advance: bool) {
        if self.capturing
            || self.processing
            || self.ocr_waiting
            || self.scroll_control.is_some()
            || self.scroll_review.is_some()
            || !self.overlay.begin_capture()
        {
            return;
        }
        self.capturing = true;
        self.status = String::from("正在获取屏幕快照…");
        self.screen = None;
        self.ocr_text.clear();
        self.ocr_lines = 0;
        // UI must hide before ScreenCaptureKit captures; discovery stays off
        // the main thread. Reconcile unplugged displays on each capture.
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        let sender = self.work_tx.clone();
        let wake = ctx.clone();
        let mut selection = self.display_selection;
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(170));
            let outcome = MacCapture::available_displays().and_then(|displays| {
                let display = if advance {
                    selection.advance(&displays)?
                } else {
                    selection.resolve(&displays)?
                };
                MacCapture::capture_with_placement(display)
            });
            let _ = sender.send(WorkerResult::Captured { selection, outcome });
            wake.request_repaint();
        });
    }

    fn start_manual_scroll(&mut self, ctx: &egui::Context, axis: ScrollAxis) {
        if self.capturing
            || self.processing
            || self.ocr_waiting
            || self.scroll_control.is_some()
            || self.scroll_review.is_some()
        {
            self.status = String::from("已有截图或长图复核任务，请先完成或放弃");
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
            "开始{direction}手动滚动，请滚动目标应用；结束时按 {} 或在菜单栏点击完成",
            self.shortcuts.label(ShortcutAction::FinishScroll)
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
        self.overlay_placement = None;
        ctx.send_viewport_cmd_to(
            egui::ViewportId::ROOT,
            egui::ViewportCommand::Visible(false),
        );
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
        let ticket = screen.ocr_ticket();
        let region = screen.selected_region();
        let sender = self.work_tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let result = shot_image::crop_rgba(&source, region)
                .map_err(|error| format!("OCR 区域无效：{error:?}"))
                .and_then(ocr::recognize);
            let _ = sender.send(WorkerResult::Ocr {
                ticket,
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

    fn save_scroll_review(&mut self, ctx: &egui::Context) {
        if self.processing {
            return;
        }
        let Some(review) = self.scroll_review.take() else {
            return;
        };
        self.processing = true;
        self.status = String::from("正在从磁盘分块导出长图 PNG…");
        let sender = self.work_tx.clone();
        let wake = ctx.clone();
        thread::spawn(move || {
            let result = scroll::export_review(&review).map_err(|error| (error, review));
            let _ = sender.send(WorkerResult::ScrollSaved(result));
            wake.request_repaint();
        });
    }

    fn undo_scroll_review(&mut self, ctx: &egui::Context) {
        if self.processing {
            return;
        }
        let Some(mut review) = self.scroll_review.take() else {
            return;
        };
        self.processing = true;
        self.status = String::from("正在移除最后一段并重新生成预览…");
        let sender = self.work_tx.clone();
        let wake = ctx.clone();
        thread::spawn(move || {
            let result = match review.store.undo_last() {
                Ok(true) => review
                    .store
                    .preview(720, 900)
                    .map_err(|error| format!("重新预览失败：{error}"))
                    .map(|image| {
                        review.preview = image;
                    }),
                Ok(false) => Err(String::from("已经是第一段，不能删除")),
                Err(error) => Err(format!("撤销最后一段失败：{error}")),
            };
            let event = match result {
                Ok(()) => WorkerResult::ScrollEdited(Ok(review)),
                Err(error) => WorkerResult::ScrollEdited(Err((error, review))),
            };
            let _ = sender.send(event);
            wake.request_repaint();
        });
    }

    fn discard_scroll_review(&mut self) {
        // Dropping TileStore removes only its own private tile directory.
        self.scroll_review.take();
        self.scroll_preview_pending = None;
        self.scroll_preview_texture = None;
        self.status = String::from("已放弃长截图，临时分块已清理");
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
                WorkerResult::Captured { selection, outcome } => {
                    self.capturing = false;
                    match outcome {
                        Ok((frame, placement)) if self.overlay.captured(true) => {
                            self.display_selection = selection;
                            self.pending_frame = Some(frame);
                            self.overlay_placement = Some(placement);
                            self.status =
                                String::from("已打开目标屏幕截图窗口；拖动选择，Esc 取消");
                        }
                        Ok(_) => {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                            self.status = String::from("截图已取消，丢弃迟到的捕获结果");
                        }
                        Err(error) => {
                            self.overlay.captured(false);
                            self.overlay_placement = None;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
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
                WorkerResult::Ocr { ticket, outcome } => {
                    self.ocr_waiting = false;
                    if !self
                        .screen
                        .as_ref()
                        .is_some_and(|screen| screen.accepts_ocr(ticket))
                    {
                        self.ocr_text.clear();
                        self.ocr_lines = 0;
                        self.status = String::from("截图或选区已变化，已丢弃过期 OCR 结果");
                        continue;
                    }
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
                                let region = screen.selected_region();
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
                WorkerResult::ScrollSaved(outcome) => {
                    self.processing = false;
                    match outcome {
                        Ok(path) => {
                            self.scroll_preview_texture = None;
                            self.status = format!("长图 PNG 已确认保存：{}", path.display());
                        }
                        Err((error, review)) => {
                            self.scroll_review = Some(review);
                            self.status = format!("长图导出失败，可重新保存：{error}");
                        }
                    }
                }
                WorkerResult::ScrollEdited(outcome) => {
                    self.processing = false;
                    match outcome {
                        Ok(review) => {
                            self.scroll_preview_pending = Some(review.preview.clone());
                            self.status = format!(
                                "已移除最后一段，当前有 {} 段可信截图",
                                review.store.strip_count()
                            );
                            self.scroll_review = Some(review);
                        }
                        Err((error, review)) => {
                            self.scroll_review = Some(review);
                            self.status = error;
                        }
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
                        "长截图正在拼接：{segments} 段，{width}×{height}px。滚动后按 {} 完成",
                        self.shortcuts.label(ShortcutAction::FinishScroll)
                    );
                }
                ScrollEvent::Finished(outcome) => {
                    self.scroll_control.take();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    self.status = match outcome {
                        Ok(Some(review)) => {
                            let (width, height) = review.store.canvas();
                            let segments = review.store.strip_count();
                            let prefix = if review.partial {
                                "匹配不确定或达到限制，已保留可信部分"
                            } else {
                                "采集结束"
                            };
                            self.scroll_preview_pending = Some(review.preview.clone());
                            self.scroll_review = Some(review);
                            format!(
                                "{prefix}：{segments} 段，{width}×{height}px。请复核后保存或放弃"
                            )
                        }
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
            self.capture_revision = self.capture_revision.saturating_add(1);
            self.screen = Some(SelectionView::new(
                ctx,
                captured.display_id,
                self.capture_revision,
                Arc::new(captured.image),
            ));
        }
        if let Some(image) = self.scroll_preview_pending.take() {
            self.scroll_preview_texture = Some(ctx.load_texture(
                "scroll-review-preview",
                egui::ColorImage::from_rgba_unmultiplied(
                    [image.width as usize, image.height as usize],
                    &image.rgba,
                ),
                egui::TextureOptions::LINEAR,
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
                self.overlay_placement = None;
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.process_ui_images(ui.ctx());
        if self.overlay.is_fullscreen() {
            if let Some(placement) = self.overlay_placement {
                let context = ui.ctx().clone();
                let id =
                    egui::ViewportId::from_hash_of(("picpaste-capture", self.capture_revision));
                context.show_viewport_immediate(
                    id,
                    overlay::capture_builder(placement),
                    |overlay_ui, _class| {
                        if overlay_ui.ctx().input(|i| i.viewport().close_requested()) {
                            self.screen = None;
                            self.overlay_placement = None;
                            self.overlay.dismiss();
                            overlay_ui.ctx().send_viewport_cmd_to(
                                egui::ViewportId::ROOT,
                                egui::ViewportCommand::Visible(true),
                            );
                        } else {
                            self.render_content(overlay_ui);
                        }
                    },
                );
            } else {
                // A missing placement must never leave the root window invisible.
                self.overlay.dismiss();
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Visible(true));
                self.status = String::from("截图窗口位置不可用，请重新截图");
            }
        } else {
            self.render_content(ui);
        }
        // Pins are children of the root viewport and must outlive an editor.
        for pin in &self.pins {
            if !pin.is_closed() {
                pin.show(ui.ctx());
            }
        }
    }
}

impl PicPasteApp {
    fn render_content(&mut self, ui: &mut egui::Ui) {
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
                    if ui
                        .button(format!(
                            "截图  {}",
                            self.shortcuts.label(ShortcutAction::Capture)
                        ))
                        .clicked()
                    {
                        self.begin_capture(ui.ctx());
                    }
                    if ui.button("截取下一块屏幕").clicked() {
                        self.begin_capture_target(ui.ctx(), true);
                    }
                    if ui
                        .button(format!(
                            "从剪贴板贴图  {}",
                            self.shortcuts.label(ShortcutAction::PastePin)
                        ))
                        .clicked()
                    {
                        self.paste_clipboard_as_pin(ui.ctx());
                    }
                    if ui
                        .button(format!(
                            "解除穿透 {}",
                            self.shortcuts.label(ShortcutAction::RecoverPins)
                        ))
                        .clicked()
                    {
                        self.recover_pins();
                    }
                });
                if let Some(display) = self.display_selection.current() {
                    ui.label(format!("当前截图显示器 ID: {}", display.0));
                }
                ui.collapsing("全局快捷键设置（Ctrl+Option + 数字键）", |ui| {
                    for (action, name) in [
                        (ShortcutAction::Capture, "截图"),
                        (ShortcutAction::PastePin, "贴图"),
                        (ShortcutAction::RecoverPins, "恢复鼠标交互"),
                        (ShortcutAction::FinishScroll, "完成长截图"),
                    ] {
                        ui.horizontal(|ui| {
                            ui.label(name);
                            let mut chosen = None;
                            egui::ComboBox::from_id_salt(("shortcut-digit", action.index()))
                                .selected_text(self.draft_shortcuts.label(action))
                                .show_ui(ui, |ui| {
                                    for digit in 1..=9 {
                                        if ui
                                            .selectable_label(
                                                self.draft_shortcuts.digit(action) == digit,
                                                format!("{digit}"),
                                            )
                                            .clicked()
                                        {
                                            chosen = Some(digit);
                                        }
                                    }
                                });
                            if let Some(digit) = chosen {
                                match self.draft_shortcuts.with_digit(action, digit) {
                                    Ok(next) => self.draft_shortcuts = next,
                                    Err(error) => {
                                        self.status = format!("快捷键不能重复或超出范围：{error:?}")
                                    }
                                }
                            }
                        });
                    }
                    ui.horizontal(|ui| {
                        if ui.button("应用快捷键").clicked() {
                            self.apply_shortcuts(ui.ctx());
                        }
                        if ui.button("恢复默认快捷键").clicked() {
                            self.draft_shortcuts = HotkeyBindings::default();
                            self.apply_shortcuts(ui.ctx());
                        }
                    });
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
                    if ui
                        .button(format!(
                            "完成长截图 {}",
                            self.shortcuts.label(ShortcutAction::FinishScroll)
                        ))
                        .clicked()
                    {
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
                    self.overlay_placement = None;
                    ui.ctx().send_viewport_cmd_to(
                        egui::ViewportId::ROOT,
                        egui::ViewportCommand::Visible(true),
                    );
                }
            }

            let mut selected_action = None;
            let mut start_scroll = None;
            let mut recognize_text = false;
            let mut color_to_copy = None;
            if let Some(screen) = &mut self.screen {
                ui.horizontal(|ui| {
                    if ui.button("保存 PNG (⌘S)").clicked() {
                        selected_action = Some(PrepareAction::Save);
                    }
                    if ui.button("复制图片").clicked() {
                        selected_action = Some(PrepareAction::Copy);
                    }
                    if ui.button("贴图").clicked() {
                        selected_action = Some(PrepareAction::Pin);
                    }
                    if ui.button("重置选区").clicked() {
                        screen.set_selection(None);
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
                ui.collapsing("PNG 输出预览 · 透明圆角", |ui| {
                    let key = (
                        screen.capture_revision,
                        screen.edit_revision,
                        screen.selected_region(),
                        self.radius,
                    );
                    if self.png_preview_key != Some(key) {
                        self.png_preview_key = Some(key);
                        self.png_preview_texture = shot_image::preview::png_selection_preview(
                            &screen.rendered,
                            key.2,
                            key.3,
                            256,
                        )
                        .ok()
                        .map(|frame| {
                            ui.ctx().load_texture(
                                "png-output-preview",
                                egui::ColorImage::from_rgba_unmultiplied(
                                    [frame.width as usize, frame.height as usize],
                                    &frame.rgba,
                                ),
                                egui::TextureOptions::LINEAR,
                            )
                        });
                    }
                    if let Some(texture) = self.png_preview_texture.as_ref() {
                        ui.add(egui::Image::new((texture.id(), texture.size_vec2())));
                        ui.label("缩略图显示实际 PNG 透明圆角；保存仍使用原始物理分辨率");
                    } else {
                        ui.label("预览不可用，请检查选区是否有效");
                    }
                });
                if let Some((point, rgba)) = screen.last_hover_color {
                    let color = format!("#{:02X}{:02X}{:02X}", rgba[0], rgba[1], rgba[2]);
                    ui.horizontal(|ui| {
                        ui.label(format!("取色 {}（{}, {}）", color, point.0, point.1));
                        if ui.button("复制 HEX").clicked() {
                            color_to_copy = Some(color);
                        }
                    });
                }
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
                ui.label(format!(
                    "按 {} 开始截图；或点击“截图”。",
                    self.shortcuts.label(ShortcutAction::Capture)
                ));
            }
            if let Some(color) = color_to_copy {
                self.status =
                    match Clipboard::new().and_then(|mut clipboard| clipboard.set_text(color)) {
                        Ok(()) => String::from("取色结果已复制"),
                        Err(error) => format!("复制颜色失败：{error}"),
                    };
            }
            if self.screen.is_some()
                && ui
                    .ctx()
                    .input(|input| input.modifiers.command && input.key_pressed(egui::Key::S))
            {
                selected_action = Some(PrepareAction::Save);
            }
            if let Some(action) = selected_action {
                self.request_prepared(ui.ctx(), action);
                if self.overlay.accept() {
                    self.overlay_placement = None;
                    self.screen = None;
                    ui.ctx().send_viewport_cmd_to(
                        egui::ViewportId::ROOT,
                        egui::ViewportCommand::Visible(true),
                    );
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
            let mut review_action = None;
            if let Some(review) = self.scroll_review.as_ref() {
                let (width, height) = review.store.canvas();
                ui.separator();
                ui.heading("长截图复核");
                if review.partial {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        "拼接未全部确认：只能保存已匹配的可信片段",
                    );
                }
                ui.label(format!(
                    "{} 段可信片段 · 原图 {}×{} px · 缩略图不影响导出分辨率",
                    review.store.strip_count(),
                    width,
                    height
                ));
                if let Some(texture) = self.scroll_preview_texture.as_ref() {
                    let full = texture.size_vec2();
                    let available = ui.available_size();
                    let scale = (available.x / full.x)
                        .min((available.y - 42.0).max(1.0) / full.y)
                        .min(1.0);
                    ui.add(egui::Image::new((texture.id(), full * scale)));
                }
                ui.horizontal(|ui| {
                    if ui.button("确认保存 PNG").clicked() {
                        review_action = Some(0);
                    }
                    if ui.button("移除最后一个拼接段").clicked() {
                        review_action = Some(1);
                    }
                    if ui.button("放弃并清理").clicked() {
                        review_action = Some(2);
                    }
                });
            }
            match review_action {
                Some(0) => self.save_scroll_review(ui.ctx()),
                Some(1) => self.undo_scroll_review(ui.ctx()),
                Some(2) => self.discard_scroll_review(),
                _ => {}
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_screen(capture_id: u64) -> SelectionView {
        let ctx = egui::Context::default();
        let image = Arc::new(RgbaFrame::new(80, 60, vec![255; 80 * 60 * 4]).unwrap());
        SelectionView::new(&ctx, DisplayId(1), capture_id, image)
    }

    #[test]
    fn ocr_reply_is_bound_to_original_capture_and_selected_region() {
        let mut screen = test_screen(10);
        screen.selection = Some(RectPx {
            x: 5,
            y: 4,
            width: 20,
            height: 15,
        });
        let ticket = screen.ocr_ticket();
        assert!(screen.accepts_ocr(ticket));
        screen.selection = Some(RectPx {
            x: 6,
            y: 4,
            width: 20,
            height: 15,
        });
        assert!(!screen.accepts_ocr(ticket));
        let new_screen = test_screen(11);
        assert!(!new_screen.accepts_ocr(ticket));
    }

    #[test]
    fn ocr_reply_is_rejected_after_raster_edit_even_if_undone() {
        let mut screen = test_screen(10);
        let ticket = screen.ocr_ticket();
        screen.annotations.apply(Annotation::Line {
            from: (0.0, 0.0),
            to: (10.0, 10.0),
            rgba: [0, 0, 0, 255],
            width: 2.0,
        });
        screen.refresh_preview();
        assert!(!screen.accepts_ocr(ticket));
        screen.undo();
        assert!(!screen.accepts_ocr(ticket));
    }

    #[test]
    fn selection_can_be_resized_from_a_corner_and_moved() {
        let mut screen = test_screen(1);
        screen.set_selection(Some(RectPx {
            x: 20,
            y: 20,
            width: 30,
            height: 20,
        }));
        assert!(screen.begin_selection_adjustment((20, 20), 2));
        screen.drag_selection_adjustment((10, 10));
        assert_eq!(
            screen.selected_region(),
            RectPx {
                x: 10,
                y: 10,
                width: 40,
                height: 30,
            }
        );
        screen.end_selection_adjustment();

        assert!(screen.begin_selection_adjustment((30, 25), 2));
        screen.drag_selection_adjustment((200, 200));
        assert_eq!(
            screen.selected_region(),
            RectPx {
                x: 40,
                y: 30,
                width: 40,
                height: 30,
            }
        );
    }

    #[test]
    fn magnifier_samples_source_pixels_without_altering_screenshot() {
        let screen = test_screen(2);
        let (patch, color) = screen.magnifier_at((0, 0)).unwrap();
        assert_eq!((patch.width, patch.height), (9, 9));
        assert_eq!(color, [255, 255, 255, 255]);
        assert!(screen.magnifier_at((-1, 0)).is_none());
        assert_eq!(screen.source.rgba[0..4], [255, 255, 255, 255]);
    }

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
