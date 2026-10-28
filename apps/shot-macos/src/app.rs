use crate::hotkey::{HotkeyAction, HotkeyController};
use crate::menu::{MenuAction, MenuBar};
#[cfg(feature = "native-panel-probe")]
use crate::native_probe_launcher::{NativeProbeLauncher, ProbeLaunchResult};
use crate::ocr;
use crate::overlay;
use crate::pin::{PinEvent, PinnedImage};
use crate::scroll::{self, ScrollCaptureControl, ScrollEvent};
use arboard::{Clipboard, ImageData};
use eframe::egui;
use shot_core::async_ticket::OcrTicket;
use shot_core::display::{DisplaySelection, OverlayPlacement};
#[cfg(feature = "native-panel-probe")]
use shot_core::native_annotation::NativeEditReply;
#[cfg(feature = "native-panel-probe")]
use shot_core::native_capture_canary::{CanaryPhase, CanaryReply, CanaryTransition};
#[cfg(all(feature = "native-panel-probe", test))]
use shot_core::native_selection::NativeSelectionReply;
#[cfg(feature = "native-panel-probe")]
use shot_core::native_snapshot::NativeSnapshot;
use shot_core::overlay::OverlayState;
use shot_core::pin_texture_budget::{MAX_PIN_COUNT, MAX_TEXTURE_BYTES, PinTextureBudget};
use shot_core::pins::PinMode;
use shot_core::platform::{CapturedFrame, DisplayId, PlatformError, ScrollAxis};
use shot_core::selection::{ResizeHandle, adjust_selection, from_drag, hit_selection_handle};
use shot_core::shortcuts::{HotkeyBindings, ShortcutAction};
use shot_core::window_candidates::{WindowCandidate, candidate_at};
use shot_core::{RectPx, RgbaFrame};
use shot_image::annotation_tools::{Tool, annotation_for_drag};
use shot_image::history::{Annotation, AnnotationDocument};
use shot_image::pixel_tools::{magnifier_patch, sample_rgba};
use shot_ocr_protocol::hit_test::{
    TextPickMode, TextSelection, hit_text_selection, selected_selection_text, selection_polygon,
};
use shot_ocr_protocol::{MAX_RGBA_BYTES, OcrDocument, QrDocument};
use shot_platform::macos::MacCapture;
use shot_scroll::fixed_bands::FixedBands;
use shot_scroll::review_transfer::{
    default_transfer_rect, map_preview_point, validate_transfer_rect,
};
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

enum WorkerResult {
    Captured {
        selection: DisplaySelection,
        outcome: Result<(CapturedFrame, OverlayPlacement, Vec<WindowCandidate>), PlatformError>,
    },
    Prepared(PrepareAction, Result<RgbaFrame, String>),
    Saved(Result<PathBuf, String>),
    ScrollSaved(Result<PathBuf, (String, scroll::ScrollReview)>),
    ScrollPrepared {
        action: PrepareAction,
        outcome: Result<(RgbaFrame, scroll::ScrollReview), (String, scroll::ScrollReview)>,
    },
    ScrollEdited(Result<scroll::ScrollReview, (String, scroll::ScrollReview)>),
    Ocr {
        ticket: OcrTicket,
        characters: bool,
        outcome: Result<OcrDocument, String>,
    },
    Qr {
        ticket: OcrTicket,
        outcome: Result<QrDocument, String>,
    },
    PinOcr {
        pin_id: u64,
        characters: bool,
        outcome: Result<OcrDocument, String>,
    },
    PinQr {
        pin_id: u64,
        outcome: Result<QrDocument, String>,
    },
}

#[derive(Clone, Copy)]
enum PrepareAction {
    Save,
    Copy,
    Pin,
}

enum ScrollReviewEdit {
    Add(Annotation),
    Undo,
    Redo,
}

struct SelectionView {
    display_id: DisplayId,
    capture_revision: u64,
    edit_revision: u64,
    source: Arc<RgbaFrame>,
    rendered: Arc<RgbaFrame>,
    window_candidates: Vec<WindowCandidate>,
    annotations: AnnotationDocument,
    tool: Tool,
    text_buffer: String,
    text_size: f32,
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
    ocr_pick_mode: TextPickMode,
    ocr_selection: Option<TextSelection>,
    ocr_characters: bool,
    qr_result: Option<QrDocument>,
}

impl SelectionView {
    fn new(
        ctx: &egui::Context,
        display_id: DisplayId,
        capture_revision: u64,
        source: Arc<RgbaFrame>,
        window_candidates: Vec<WindowCandidate>,
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
            window_candidates,
            annotations: AnnotationDocument::new(Arc::clone(&source)),
            tool: Tool::Select,
            text_buffer: String::from("文字"),
            text_size: 26.0,
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
            ocr_pick_mode: TextPickMode::Unit,
            ocr_selection: None,
            ocr_characters: false,
            qr_result: None,
        }
    }

    fn invalidate_ocr(&mut self) {
        self.edit_revision = self.edit_revision.saturating_add(1);
        self.ocr_result = None;
        self.ocr_selection = None;
        self.ocr_mode = false;
        self.qr_result = None;
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

    /// A companion result is tied to the exact screenshot *and edit* revision.
    /// Even valid bytes from a previously started process cannot overwrite
    /// edits performed while that process was running.
    #[cfg(feature = "native-panel-probe")]
    fn accepts_native_revision(&self, expected: (u64, u64)) -> bool {
        (self.capture_revision, self.edit_revision) == expected
    }

    #[cfg(feature = "native-panel-probe")]
    fn accepts_native_session(&self, active: (u64, (u64, u64)), received_id: u64) -> bool {
        active.0 == received_id && self.accepts_native_revision(active.1)
    }

    /// Preserve the exact selected display and edit revision across a native
    /// child process. Never transfer an already-composited or downscaled image.
    #[cfg(feature = "native-panel-probe")]
    fn native_capture_request(
        &self,
        placement: OverlayPlacement,
    ) -> Result<(NativeSnapshot, (u64, u64)), String> {
        if !placement.belongs_to(self.display_id) {
            return Err(String::from("原生框选显示器与截图不一致"));
        }
        NativeSnapshot::validate_dimensions(self.source.width, self.source.height)?;
        let snapshot = NativeSnapshot::new(placement, self.source.as_ref().clone())?;
        Ok((snapshot, (self.capture_revision, self.edit_revision)))
    }

    /// Validate selection, display, image dimensions and every native mark,
    /// then pre-render the entire candidate history before mutating any live
    /// selection, annotation history, texture or PNG preview state.
    #[cfg(feature = "native-panel-probe")]
    fn import_native_reply(&mut self, reply: &NativeEditReply) -> Result<Option<usize>, String> {
        let Some((rect, marks)) =
            reply.validated_in(self.display_id, (self.source.width, self.source.height))?
        else {
            return Ok(None); // Cancelled: preserve selection, markup and PNG.
        };
        if !marks.is_empty() {
            let mut staged = self.annotations.clone();
            shot_image::native_import::import_native_marks(&mut staged, marks)?;
            let image = shot_image::composite::composite(&self.source, staged.visible())
                .map_err(|error| format!("原生标注图片合成失败：{error}"))?;
            let preview = egui::ColorImage::from_rgba_unmultiplied(
                [image.width as usize, image.height as usize],
                &image.rgba,
            );
            self.annotations = staged;
            self.texture.set(preview, egui::TextureOptions::LINEAR);
            self.rendered = Arc::new(image);
            self.render_error = None;
            self.invalidate_ocr();
        }
        self.tool = Tool::Select;
        self.set_selection(Some(rect));
        Ok(Some(marks.len()))
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
            ui.label("文字选择模式：点击 Vision 返回的原生行/词/字框；不推算不存在的字符位置。");
        } else {
            ui.label("单击黄色窗口边框快速选区，或拖动自由框选；可调整边缘和四角。Esc 取消。");
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
        if !self.ocr_mode
            && self.tool == Tool::Text
            && response.clicked()
            && !self.text_buffer.trim().is_empty()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let (x, y) = point_to_pixel(&response.rect, &self.source, pointer);
            self.annotations.apply(Annotation::Text {
                at: (x as f32, y as f32),
                content: self.text_buffer.clone(),
                size: self.text_size,
                rgba: [39, 178, 231, 255],
            });
            self.refresh_preview();
        }

        if !self.ocr_mode
            && self.tool == Tool::Select
            && self.selection.is_none()
            && !response.dragged()
            && let Some(pointer) = response.hover_pos()
        {
            let point = point_to_pixel(&response.rect, &self.source, pointer);
            if let Some(candidate) = candidate_at(&self.window_candidates, point) {
                let scale_x = response.rect.width() / self.source.width as f32;
                let scale_y = response.rect.height() / self.source.height as f32;
                let rect = egui::Rect::from_min_size(
                    egui::pos2(
                        response.rect.left() + candidate.rect.x as f32 * scale_x,
                        response.rect.top() + candidate.rect.y as f32 * scale_y,
                    ),
                    egui::vec2(
                        candidate.rect.width as f32 * scale_x,
                        candidate.rect.height as f32 * scale_y,
                    ),
                );
                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(2.0, egui::Color32::YELLOW),
                    egui::StrokeKind::Inside,
                );
                if response.clicked() {
                    self.set_selection(Some(candidate.rect));
                }
            }
        }
        if !self.ocr_mode && self.tool != Tool::Text {
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
                self.ocr_selection = hit_text_selection(
                    document,
                    self.ocr_origin,
                    (x as f32, y as f32),
                    self.ocr_pick_mode,
                );
            }
            if let Some(document) = self.ocr_result.as_ref() {
                let sx = response.rect.width() / self.source.width as f32;
                let sy = response.rect.height() / self.source.height as f32;
                let screen_pos = |p: [f32; 2]| {
                    egui::pos2(
                        response.rect.left() + (self.ocr_origin.0 as f32 + p[0]) * sx,
                        response.rect.top() + (self.ocr_origin.1 as f32 + p[1]) * sy,
                    )
                };
                let draw_polygon = |polygon: &[[f32; 2]; 4], color: egui::Color32| {
                    for edge in 0..4 {
                        ui.painter().line_segment(
                            [
                                screen_pos(polygon[edge]),
                                screen_pos(polygon[(edge + 1) % 4]),
                            ],
                            egui::Stroke::new(2.0, color),
                        );
                    }
                };
                for line in &document.lines {
                    if self.ocr_pick_mode == TextPickMode::Unit {
                        for word in &line.words {
                            draw_polygon(&word.polygon_px, egui::Color32::from_rgb(72, 192, 245));
                        }
                    } else {
                        draw_polygon(&line.polygon_px, egui::Color32::from_rgb(72, 192, 245));
                    }
                }
                if let Some(polygon) = self
                    .ocr_selection
                    .and_then(|selection| selection_polygon(document, selection))
                {
                    draw_polygon(polygon, egui::Color32::from_rgb(30, 230, 160));
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

/// Live capture thumbnails are always bounded at the TileStore producer.
/// This does not attempt to upload a full long-image canvas to the GPU.
fn update_scroll_live_texture(
    ctx: &egui::Context,
    frame: &RgbaFrame,
    texture: &mut Option<egui::TextureHandle>,
) {
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [frame.width as usize, frame.height as usize],
        &frame.rgba,
    );
    if let Some(existing) = texture.as_mut() {
        existing.set(image, egui::TextureOptions::LINEAR);
    } else {
        *texture = Some(ctx.load_texture(
            "scroll-capture-live-preview",
            image,
            egui::TextureOptions::LINEAR,
        ));
    }
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
    pending_window_candidates: Vec<WindowCandidate>,
    overlay_placement: Option<OverlayPlacement>,
    prepared: Vec<(PrepareAction, RgbaFrame)>,
    screen: Option<SelectionView>,
    pins: Vec<PinnedImage>,
    pin_budget: PinTextureBudget,
    next_pin_id: u64,
    pin_tx: Sender<PinEvent>,
    pin_rx: Receiver<PinEvent>,
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
    scroll_keep_segments: usize,
    scroll_retake_segment: usize,
    scroll_fixed_bands: FixedBands,
    scroll_preview_pending: Option<RgbaFrame>,
    scroll_preview_texture: Option<egui::TextureHandle>,
    scroll_preview_radius: Option<u32>,
    scroll_tool: Tool,
    scroll_text_buffer: String,
    scroll_text_size: f32,
    scroll_drag_anchor: Option<egui::Pos2>,
    scroll_brush_points: Vec<(f32, f32)>,
    scroll_share_region: Option<RectPx>,
    menu_rx: Receiver<MenuAction>,
    _menu: Option<MenuBar>,
    quit_requested: bool,
    _hotkeys: Option<HotkeyController>,
    #[cfg(feature = "native-panel-probe")]
    native_probe: NativeProbeLauncher,
    #[cfg(feature = "native-panel-probe")]
    native_selection_tx: Sender<(u64, Result<NativeEditReply, String>)>,
    #[cfg(feature = "native-panel-probe")]
    native_selection_rx: Receiver<(u64, Result<NativeEditReply, String>)>,
    #[cfg(feature = "native-panel-probe")]
    native_selection_revision: Option<(u64, (u64, u64))>,
    #[cfg(feature = "native-panel-probe")]
    native_selection_next_id: u64,
    #[cfg(feature = "native-panel-probe")]
    native_canary_enabled: bool,
    #[cfg(feature = "native-panel-probe")]
    native_canary_phase: CanaryPhase,
}

impl PicPasteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (work_tx, work_rx) = mpsc::channel();
        #[cfg(feature = "native-panel-probe")]
        let (native_selection_tx, native_selection_rx) = mpsc::channel();
        let (hotkey_tx, hotkey_rx) = mpsc::channel();
        let (scroll_tx, scroll_rx) = mpsc::channel();
        let (pin_tx, pin_rx) = mpsc::channel();
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
            pending_window_candidates: Vec::new(),
            overlay_placement: None,
            prepared: Vec::new(),
            screen: None,
            pins: Vec::new(),
            pin_budget: PinTextureBudget::default(),
            next_pin_id: 0,
            pin_tx,
            pin_rx,
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
            scroll_keep_segments: 1,
            scroll_retake_segment: 2,
            scroll_fixed_bands: FixedBands::default(),
            scroll_preview_pending: None,
            scroll_preview_texture: None,
            scroll_preview_radius: None,
            scroll_tool: Tool::Select,
            scroll_text_buffer: String::from("文字"),
            scroll_text_size: 26.0,
            scroll_drag_anchor: None,
            scroll_brush_points: Vec::new(),
            scroll_share_region: None,
            menu_rx,
            _menu: menu,
            quit_requested: false,
            _hotkeys: hotkeys,
            #[cfg(feature = "native-panel-probe")]
            native_probe: NativeProbeLauncher::default(),
            #[cfg(feature = "native-panel-probe")]
            native_selection_tx,
            #[cfg(feature = "native-panel-probe")]
            native_selection_rx,
            #[cfg(feature = "native-panel-probe")]
            native_selection_revision: None,
            #[cfg(feature = "native-panel-probe")]
            native_selection_next_id: 0,
            #[cfg(feature = "native-panel-probe")]
            native_canary_enabled: false,
            #[cfg(feature = "native-panel-probe")]
            native_canary_phase: CanaryPhase::default(),
        }
    }

    /// Shared by the manual debug button and opt-in automatic capture.
    /// The helper may not be started until we have a valid immutable frame.
    #[cfg(feature = "native-panel-probe")]
    fn launch_native_selection(
        &mut self,
        ctx: &egui::Context,
    ) -> Result<ProbeLaunchResult, String> {
        let screen = self.screen.as_ref().ok_or("截图尚未就绪")?;
        let placement = self.overlay_placement.ok_or("截图显示器位置不可用")?;
        let (snapshot, revision) = screen.native_capture_request(placement)?;
        let request_id = self
            .native_selection_next_id
            .checked_add(1)
            .ok_or("原生框选会话 ID 已耗尽")?;
        match self.native_probe.launch_select(
            snapshot,
            request_id,
            self.native_selection_tx.clone(),
            ctx.clone(),
        )? {
            ProbeLaunchResult::Started => {
                self.native_selection_next_id = request_id;
                self.native_selection_revision = Some((request_id, revision));
                Ok(ProbeLaunchResult::Started)
            }
            ProbeLaunchResult::AlreadyRunning => Ok(ProbeLaunchResult::AlreadyRunning),
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
        {
            return;
        }
        // The opt-in NSPanel demo runs in another process, so capture cannot
        // rely on the current-PID exclusion used for normal PicPaste windows.
        #[cfg(feature = "native-panel-probe")]
        if let Err(error) = self.native_probe.stop_for_capture() {
            self.status = format!("原生实验窗口无法关闭，截图已取消：{error}");
            return;
        }
        #[cfg(feature = "native-panel-probe")]
        self.native_canary_phase.reset();
        if !self.overlay.begin_capture() {
            return;
        }
        #[cfg(feature = "native-panel-probe")]
        {
            // Results from a stopped child must not affect a new capture.
            self.native_selection_revision = None;
        }
        self.capturing = true;
        self.status = String::from("正在获取屏幕快照…");
        self.screen = None;
        self.pending_window_candidates.clear();
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
        let original = screen.selected_region();
        if original.width < 24 || original.height < 24 {
            self.status = String::from("长截图区域太小，宽高均需至少 24 个物理像素");
            return;
        }
        // Exclude fixed page chrome before the FIRST ScreenCaptureKit request.
        // The same cropped region is captured on every frame and every retry,
        // so excluded pixels never enter overlap matching or the exported PNG.
        let selected = match self.scroll_fixed_bands.content_roi(original, axis) {
            Ok(region) => region,
            Err(message) => {
                self.status = format!("长截图固定区域设置无效：{message}；请减少排除边距");
                return;
            }
        };
        let direction = match axis {
            ScrollAxis::Vertical => "纵向",
            ScrollAxis::Horizontal => "横向",
        };
        self.status = format!(
            "开始{direction}手动滚动，已排除固定区域 {} / {} 像素；请滚动目标应用，结束时按 {}",
            self.scroll_fixed_bands.leading,
            self.scroll_fixed_bands.trailing,
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
        self.scroll_preview_texture = None;
        self.scroll_preview_radius = None;
        self.scroll_preview_pending = None;
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
            self.status = String::from("正在整理已确认的长截图分段，稍后复核保存 PNG");
        }
    }

    fn resume_manual_scroll(&mut self) {
        if let Some(control) = &self.scroll_control {
            if control.resume() {
                self.status =
                    String::from("已请求继续采集：请手动滚回有共同内容的位置，再按原方向慢慢滚动");
            } else {
                self.status = String::from("长截图未处于暂停状态，无需重试");
            }
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

    fn request_ocr(&mut self, ctx: &egui::Context, characters: bool) {
        if self.ocr_waiting || self.processing || self.scroll_control.is_some() {
            return;
        }
        let Some(screen) = &self.screen else { return };
        self.ocr_waiting = true;
        self.ocr_text.clear();
        self.ocr_lines = 0;
        self.status = if characters {
            String::from("正在使用离线 Vision Fast 获取逐字原生坐标…")
        } else {
            String::from("正在使用离线 Vision Accurate 获取词级原生坐标…")
        };
        let source = Arc::clone(&screen.rendered);
        let ticket = screen.ocr_ticket();
        let region = screen.selected_region();
        let sender = self.work_tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let result = shot_image::crop_rgba(&source, region)
                .map_err(|error| format!("OCR 区域无效：{error:?}"))
                .and_then(|image| {
                    if characters {
                        ocr::recognize_characters(image)
                    } else {
                        ocr::recognize(image)
                    }
                });
            let _ = sender.send(WorkerResult::Ocr {
                ticket,
                characters,
                outcome: result,
            });
            ctx.request_repaint();
        });
    }

    fn request_qr(&mut self, ctx: &egui::Context) {
        if self.ocr_waiting || self.processing || self.scroll_control.is_some() {
            return;
        }
        let Some(screen) = &self.screen else { return };
        self.ocr_waiting = true;
        self.status = String::from("正在本地 Vision 扫描选区二维码，不联网…");
        let source = Arc::clone(&screen.rendered);
        let ticket = screen.ocr_ticket();
        let region = screen.selected_region();
        let sender = self.work_tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let outcome = shot_image::crop_rgba(&source, region)
                .map_err(|error| format!("二维码区域无效：{error:?}"))
                .and_then(ocr::recognize_qr);
            let _ = sender.send(WorkerResult::Qr { ticket, outcome });
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
            // Reject a giant clipboard image before allocating a ColorImage/GPU texture.
            PinTextureBudget::validate_dimensions(w, h)
                .map_err(|error| format!("贴图资源限制：{error}"))?;
            RgbaFrame::new(w, h, image.bytes.into_owned())
                .map_err(|e| format!("invalid clipboard frame: {e:?}"))
        })();
        match result {
            Ok(image) => self.create_pin(ctx, image),
            Err(error) => self.status = format!("剪贴板贴图失败：{error}"),
        }
    }

    /// Every new viewport must reserve its resident resources *before*
    /// building egui::ColorImage or uploading a GPU texture.
    fn create_pin(&mut self, ctx: &egui::Context, image: RgbaFrame) {
        self.prune_closed_pins();
        let Some(id) = self.next_pin_id.checked_add(1) else {
            self.status = String::from("贴图 ID 已用尽");
            return;
        };

        // Use exact pixel equality, not an unchecked hash. A clone of an
        // identical pin keeps the original texture handle and RGBA Arc alive.
        let shared_index = self.pins.iter().position(|pin| {
            pin.source.width == image.width
                && pin.source.height == image.height
                && pin.source.rgba == image.rgba
        });
        let reuse_group = shared_index.map(|index| self.pins[index].texture_group);
        if let Err(error) = self
            .pin_budget
            .reserve(id, image.width, image.height, reuse_group)
        {
            self.status = format!("贴图未创建：{error}");
            return;
        }
        let reused = shared_index.is_some();
        let pin = if let Some(index) = shared_index {
            PinnedImage::from_shared(id, &self.pins[index])
        } else {
            PinnedImage::new(ctx, id, Arc::new(image))
        };
        self.next_pin_id = id;
        self.pins.push(pin);
        self.status = if reused {
            format!("已复用现有 GPU 纹理创建贴图 #{id}")
        } else {
            format!("已创建贴图 #{id}")
        };
        ctx.request_repaint();
    }

    /// Drop closed pinned windows together with their texture-ledger lease.
    /// egui frees the texture after the last cloned TextureHandle is dropped.
    fn prune_closed_pins(&mut self) {
        let budget = &mut self.pin_budget;
        self.pins.retain(|pin| {
            if pin.is_closed() {
                let released = budget.release(pin.id);
                debug_assert!(released, "closed pin must have a resource lease");
                false
            } else {
                true
            }
        });
    }

    fn recover_pins(&mut self) {
        for pin in &mut self.pins {
            if pin.mode == PinMode::Passthrough {
                pin.set_mode(PinMode::Move);
            }
        }
        self.status = String::from("所有贴图已解除鼠标穿透");
    }

    fn request_pin_ocr(&mut self, ctx: &egui::Context, pin_id: u64, characters: bool) {
        let Some(pin) = self
            .pins
            .iter_mut()
            .find(|pin| pin.id == pin_id && !pin.is_closed())
        else {
            return;
        };
        if pin.source.rgba.len() > MAX_RGBA_BYTES {
            self.status = String::from("贴图超过离线 OCR 80MiB 上限，请缩小原图");
            return;
        }
        if !pin.ocr_gate.begin() {
            return;
        }
        pin.set_mode(PinMode::TextSelect);
        pin.ocr_result = None;
        pin.qr_result = None;
        pin.selected_text = None;
        self.status = format!(
            "贴图 #{pin_id} 正在离线识别{}…",
            if characters { "字符" } else { "词语" }
        );
        let source = Arc::clone(&pin.source);
        let sender = self.work_tx.clone();
        let wake = ctx.clone();
        thread::spawn(move || {
            let outcome = if characters {
                ocr::recognize_characters(source.as_ref().clone())
            } else {
                ocr::recognize(source.as_ref().clone())
            };
            let _ = sender.send(WorkerResult::PinOcr {
                pin_id,
                characters,
                outcome,
            });
            wake.request_repaint();
        });
    }

    fn request_pin_qr(&mut self, ctx: &egui::Context, pin_id: u64) {
        let Some(pin) = self
            .pins
            .iter_mut()
            .find(|pin| pin.id == pin_id && !pin.is_closed())
        else {
            return;
        };
        if pin.source.rgba.len() > MAX_RGBA_BYTES {
            self.status = String::from("贴图超过本地二维码扫描 80MiB 上限，请缩小原图");
            return;
        }
        if !pin.ocr_gate.begin() {
            return;
        }
        pin.set_mode(PinMode::TextSelect);
        pin.ocr_result = None;
        pin.qr_result = None;
        pin.selected_text = None;
        self.status = format!("贴图 #{pin_id} 正在本地扫描二维码…");
        let source = Arc::clone(&pin.source);
        let sender = self.work_tx.clone();
        let wake = ctx.clone();
        thread::spawn(move || {
            let outcome = ocr::recognize_qr(source.as_ref().clone());
            let _ = sender.send(WorkerResult::PinQr { pin_id, outcome });
            wake.request_repaint();
        });
    }

    fn process_pin_event(&mut self, event: PinEvent, ctx: &egui::Context) {
        match event {
            PinEvent::Recognize { id, characters } => self.request_pin_ocr(ctx, id, characters),
            PinEvent::DetectQr(id) => self.request_pin_qr(ctx, id),
            PinEvent::Closed(id) => {
                if let Some(pin) = self.pins.iter().find(|pin| pin.id == id) {
                    pin.closed.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            PinEvent::Move(id) => {
                if let Some(pin) = self
                    .pins
                    .iter_mut()
                    .find(|pin| pin.id == id && !pin.is_closed())
                {
                    pin.set_mode(PinMode::Move);
                }
            }
            PinEvent::PickText { id, pixel } => {
                if let Some(pin) = self
                    .pins
                    .iter_mut()
                    .find(|pin| pin.id == id && !pin.is_closed())
                    && pin.mode == PinMode::TextSelect
                    && let Some(document) = pin.ocr_result.as_ref()
                {
                    pin.selected_text = hit_text_selection(document, (0, 0), pixel, pin.pick_mode);
                    self.status = if pin.selected_text.is_some() {
                        format!("贴图 #{id} 已选中 Vision 原始文字区域，可复制")
                    } else {
                        format!("贴图 #{id} 点击处没有对应的文字框")
                    };
                }
            }
            PinEvent::PickMode { id, mode } => {
                if let Some(pin) = self
                    .pins
                    .iter_mut()
                    .find(|pin| pin.id == id && !pin.is_closed())
                {
                    pin.pick_mode = mode;
                    pin.selected_text = None;
                }
            }
            PinEvent::CopySelected(id) => {
                let selected = self
                    .pins
                    .iter()
                    .find(|pin| pin.id == id && !pin.is_closed())
                    .filter(|pin| pin.mode == PinMode::TextSelect)
                    .and_then(|pin| {
                        selected_selection_text(pin.ocr_result.as_ref()?, pin.selected_text?)
                    });
                if let Some(text) = selected.filter(|text| !text.is_empty()) {
                    self.status = match Clipboard::new()
                        .and_then(|mut clipboard| clipboard.set_text(text.to_string()))
                    {
                        Ok(()) => format!("已复制贴图 #{id} 的精确选中文字"),
                        Err(error) => format!("贴图复制文字失败：{error}"),
                    };
                }
            }
            PinEvent::CopyQr { id, index } => {
                let payload = self
                    .pins
                    .iter()
                    .find(|pin| pin.id == id && !pin.is_closed())
                    .filter(|pin| pin.mode == PinMode::TextSelect)
                    .and_then(|pin| pin.qr_result.as_ref())
                    .and_then(|doc| doc.codes.get(index))
                    .map(|code| code.payload.clone());
                if let Some(payload) = payload {
                    self.status = match Clipboard::new()
                        .and_then(|mut clipboard| clipboard.set_text(payload))
                    {
                        Ok(()) => format!("贴图 #{id} 二维码内容已复制（不会自动打开链接）"),
                        Err(error) => format!("贴图二维码复制失败：{error}"),
                    };
                }
            }
        }
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
        let radius = self.radius;
        let sender = self.work_tx.clone();
        let wake = ctx.clone();
        thread::spawn(move || {
            let result = scroll::export_review(&review, radius).map_err(|error| (error, review));
            let _ = sender.send(WorkerResult::ScrollSaved(result));
            wake.request_repaint();
        });
    }

    fn edit_scroll_review(&mut self, ctx: &egui::Context, command: ScrollReviewEdit) {
        if self.processing {
            return;
        }
        let Some(mut review) = self.scroll_review.take() else {
            return;
        };
        self.processing = true;
        self.status = String::from("正在处理长图标注并重新生成复核预览…");
        let sender = self.work_tx.clone();
        let wake = ctx.clone();
        thread::spawn(move || {
            let result = match command {
                ScrollReviewEdit::Add(annotation) => review
                    .edits
                    .apply(&review.store, annotation)
                    .map_err(|error| format!("添加标注失败：{error}")),
                ScrollReviewEdit::Undo => {
                    if review.edits.undo() {
                        Ok(())
                    } else {
                        Err(String::from("没有可撤销的标注"))
                    }
                }
                ScrollReviewEdit::Redo => {
                    if review.edits.redo() {
                        Ok(())
                    } else {
                        Err(String::from("没有可重做的标注"))
                    }
                }
            };
            let result = result.and_then(|_| {
                review
                    .store
                    .preview_with_edits(&review.edits, 720, 900)
                    .map_err(|error| format!("重新生成标注预览失败：{error}"))
            });
            let event = match result {
                Ok(image) => {
                    review.preview = image;
                    WorkerResult::ScrollEdited(Ok(review))
                }
                Err(error) => WorkerResult::ScrollEdited(Err((error, review))),
            };
            let _ = sender.send(event);
            wake.request_repaint();
        });
    }

    fn prepare_scroll_transfer(&mut self, ctx: &egui::Context, action: PrepareAction) {
        if self.processing {
            return;
        }
        let Some(review) = self.scroll_review.take() else {
            return;
        };
        let rect = self
            .scroll_share_region
            .unwrap_or_else(|| default_transfer_rect(review.store.canvas()));
        if let Err(error) = validate_transfer_rect(rect, review.store.canvas()) {
            self.status = format!("复制/贴图选区无效：{error}");
            self.scroll_review = Some(review);
            return;
        }
        self.processing = true;
        self.status = String::from("正在从长图磁盘分块提取原分辨率选区…");
        let sender = self.work_tx.clone();
        let wake = ctx.clone();
        let radius = self.radius;
        thread::spawn(move || {
            let outcome = match review
                .store
                .render_region_edited(&review.edits, rect, radius)
            {
                Ok(image) => Ok((image, review)),
                Err(error) => Err((format!("提取长图选区失败：{error}"), review)),
            };
            let _ = sender.send(WorkerResult::ScrollPrepared { action, outcome });
            wake.request_repaint();
        });
    }

    fn undo_scroll_review(&mut self, ctx: &egui::Context) {
        if self.processing {
            return;
        }
        if self
            .scroll_review
            .as_ref()
            .is_some_and(|review| review.edits.applied_len() > 0)
        {
            self.status =
                String::from("先撤销所有长图标注，再移除可信片段；片段变化会使标注坐标失效");
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
                Ok(true) => {
                    // Even an entirely undone history may have redo patches
                    // pointing past the new canvas; drop them before reuse.
                    review.edits.clear();
                    review
                        .store
                        .preview(720, 900)
                        .map_err(|error| format!("重新预览失败：{error}"))
                        .map(|image| {
                            review.preview = image;
                        })
                }
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

    /// Restart at the first suspect segment using the last verified viewport.
    /// Later strips are discarded rather than being spliced without proof.
    fn retake_scroll_review(&mut self, ctx: &egui::Context, first_bad_segment: usize) {
        if self.processing || self.capturing || self.ocr_waiting || self.scroll_control.is_some() {
            return;
        }
        let Some(review) = self.scroll_review.take() else {
            return;
        };
        let total = review.store.strip_count();
        if first_bad_segment < 2 || first_bad_segment > total {
            self.status = String::from("重拍起始段应在第 2 段与最后一段之间");
            self.scroll_review = Some(review);
            return;
        }
        if review.edits.applied_len() > 0 {
            self.status = String::from("请先撤销全部标注，再从指定位置重拍后续片段");
            self.scroll_review = Some(review);
            return;
        }
        self.status = format!(
            "准备从第 {first_bad_segment} 段重新采集：保留前 {} 段并丢弃之后所有片段；请手动滚回该可信视口附近",
            first_bad_segment - 1,
        );
        self.scroll_control = Some(scroll::launch_retake(
            review,
            first_bad_segment - 1,
            self.scroll_tx.clone(),
            ctx.clone(),
        ));
        self.scroll_share_region = None;
        self.scroll_preview_pending = None;
        self.scroll_preview_texture = None;
        self.scroll_preview_radius = None;
        self.scroll_drag_anchor = None;
        self.scroll_brush_points.clear();
        ctx.send_viewport_cmd_to(
            egui::ViewportId::ROOT,
            egui::ViewportCommand::Visible(false),
        );
    }

    fn trim_scroll_review(&mut self, ctx: &egui::Context, keep_count: usize) {
        if self.processing {
            return;
        }
        let Some(total) = self
            .scroll_review
            .as_ref()
            .map(|review| review.store.strip_count())
        else {
            return;
        };
        if keep_count == 0 || keep_count >= total {
            self.status = String::from("保留段数无效；只能移除末尾片段，且必须至少保留第一段");
            return;
        }
        if self
            .scroll_review
            .as_ref()
            .is_some_and(|review| review.edits.applied_len() > 0)
        {
            self.status = String::from("先撤销所有长图标注，再裁剪可信片段");
            return;
        }
        let Some(mut review) = self.scroll_review.take() else {
            return;
        };
        review.edits.clear();
        self.processing = true;
        self.status = format!("正在保留前 {keep_count} 段并重建缩略图…");
        let sender = self.work_tx.clone();
        let wake = ctx.clone();
        thread::spawn(move || {
            let result = review
                .store
                .keep_first(keep_count)
                .map_err(|error| format!("裁剪可信后缀失败：{error}"))
                .and_then(|_| {
                    review
                        .store
                        .preview(720, 900)
                        .map_err(|error| format!("重新生成预览失败：{error}"))
                })
                .map(|image| {
                    review.preview = image;
                });
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
        self.scroll_share_region = None;
        self.scroll_drag_anchor = None;
        self.scroll_brush_points.clear();
        self.scroll_keep_segments = 1;
        self.scroll_preview_pending = None;
        self.scroll_preview_texture = None;
        self.status = String::from("已放弃长截图，临时分块已清理");
    }

    fn process_events(&mut self, ctx: &egui::Context) {
        #[cfg(feature = "native-panel-probe")]
        {
            let responses: Vec<_> = self.native_selection_rx.try_iter().collect();
            for (received_id, response) in responses {
                let Some(active) = self.native_selection_revision else {
                    continue; // Stale or killed helper.
                };
                if active.0 != received_id {
                    continue; // Never consume a newer session.
                }
                self.native_selection_revision = None;
                let revision = active.1;
                if !self.overlay.is_fullscreen() {
                    self.native_canary_phase.reset();
                    continue;
                }
                let Some(screen) = self.screen.as_mut() else {
                    self.native_canary_phase.reset();
                    continue;
                };
                if !screen.accepts_native_session(active, received_id) {
                    // Captures/edits changed since launch. Reveal the original
                    // editor without applying a byte of the stale reply.
                    self.native_canary_phase.reset();
                    self.status = String::from("原生框选结果已过期，已恢复稳定截图编辑器");
                    continue;
                }
                let (status, outcome) = match response {
                    Ok(reply) => match screen.import_native_reply(&reply) {
                        Ok(Some(count)) => (
                            format!(
                                "原生选区与 {} 条标注已校验导入；可撤销/重做、复制/贴图或保存带标注 PNG",
                                count
                            ),
                            CanaryReply::Applied,
                        ),
                        Ok(None) => (
                            String::from("已取消 NSPanel 原生框选，原截图和已有标注未修改"),
                            CanaryReply::Cancelled,
                        ),
                        Err(error) => (
                            format!("原生标注结果无效，已恢复稳定编辑器：{error}"),
                            CanaryReply::Rejected,
                        ),
                    },
                    Err(error) => (
                        format!("原生标注进程未返回有效结果，已恢复稳定编辑器：{error}"),
                        CanaryReply::Rejected,
                    ),
                };
                self.status = status;
                if self.native_canary_phase.complete(received_id, outcome)
                    == CanaryTransition::DismissCapture
                {
                    // Escape in an automatically opened NSPanel means cancel
                    // the capture, not silently reopen a second editor.
                    self.screen = None;
                    self.overlay_placement = None;
                    self.overlay.dismiss();
                    ctx.send_viewport_cmd_to(
                        egui::ViewportId::ROOT,
                        egui::ViewportCommand::Visible(true),
                    );
                    continue;
                }
                // Applied edits and all failed/manual results return to the
                // same trusted parent editor for PNG/copy/pin/Undo.
                let id = egui::ViewportId::from_hash_of(("picpaste-capture", revision.0));
                ctx.send_viewport_cmd_to(id, egui::ViewportCommand::Focus);
            }
        }
        let menu_actions: Vec<_> = self.menu_rx.try_iter().collect();
        for action in menu_actions {
            match action {
                MenuAction::Capture => self.begin_capture(ctx),
                MenuAction::Paste => self.paste_clipboard_as_pin(ctx),
                MenuAction::Recover => self.recover_pins(),
                MenuAction::FinishScroll => self.finish_manual_scroll(),
                MenuAction::ResumeScroll => self.resume_manual_scroll(),
                MenuAction::CancelScroll => self.cancel_manual_scroll(),
                MenuAction::Show => {
                    #[cfg(feature = "native-panel-probe")]
                    if self.native_canary_phase.hides_legacy() {
                        self.native_canary_phase.reset();
                        self.native_selection_revision = None;
                        self.status = match self.native_probe.stop_for_capture() {
                            Ok(()) => String::from("已停止原生试运行窗口，恢复稳定截图编辑器"),
                            Err(error) => {
                                format!("原生试运行窗口关闭失败：{error}；已恢复稳定编辑器")
                            }
                        };
                    }
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
                        Ok((frame, placement, windows)) if self.overlay.captured(true) => {
                            self.display_selection = selection;
                            self.pending_frame = Some(frame);
                            self.pending_window_candidates = windows;
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
                WorkerResult::Ocr {
                    ticket,
                    characters,
                    outcome,
                } => {
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
                                screen.ocr_pick_mode = TextPickMode::Unit;
                                screen.ocr_selection = None;
                                screen.ocr_characters = characters;
                                screen.qr_result = None;
                                screen.ocr_result = Some(document);
                            }
                            self.status = format!(
                                "Vision {}识别完成，共 {} 行；点击原图上的{}框可复制",
                                if characters {
                                    "逐字(Fast)"
                                } else {
                                    "词级(Accurate)"
                                },
                                self.ocr_lines,
                                if characters { "字" } else { "词" },
                            );
                        }
                        Err(error) => self.status = format!("文字识别失败：{error}"),
                    }
                }
                WorkerResult::Qr { ticket, outcome } => {
                    self.ocr_waiting = false;
                    if !self
                        .screen
                        .as_ref()
                        .is_some_and(|screen| screen.accepts_ocr(ticket))
                    {
                        self.status = String::from("截图或选区已变化，已丢弃过期二维码结果");
                        continue;
                    }
                    match outcome {
                        Ok(document) => {
                            let count = document.codes.len();
                            if let Some(screen) = self.screen.as_mut() {
                                screen.qr_result = Some(document);
                            }
                            self.status = if count == 0 {
                                String::from("本地 Vision 未发现二维码")
                            } else {
                                format!("本地 Vision 找到 {count} 个二维码；请明确点击复制内容")
                            };
                        }
                        Err(error) => self.status = format!("二维码识别失败：{error}"),
                    }
                }
                WorkerResult::PinOcr {
                    pin_id,
                    characters,
                    outcome,
                } => {
                    if let Some(pin) = self
                        .pins
                        .iter_mut()
                        .find(|pin| pin.id == pin_id && !pin.is_closed())
                        && pin.ocr_gate.pending()
                    {
                        if !pin.ocr_gate.finish(pin.mode) {
                            self.status = format!("贴图 #{pin_id} 已退出选字模式，结果已丢弃");
                            continue;
                        }
                        match outcome {
                            Ok(document) => {
                                let lines = document.lines.len();
                                pin.ocr_result = Some(Arc::new(document));
                                pin.qr_result = None;
                                pin.selected_text = None;
                                pin.pick_mode = TextPickMode::Unit;
                                pin.character_mode = characters;
                                self.status = format!(
                                    "贴图 #{pin_id} {}识别完成，共 {lines} 行；点击识别框复制",
                                    if characters { "逐字" } else { "词级" }
                                );
                            }
                            Err(error) => {
                                pin.ocr_result = None;
                                self.status = format!("贴图 #{pin_id} 离线识别失败：{error}");
                            }
                        }
                    }
                }
                WorkerResult::PinQr { pin_id, outcome } => {
                    if let Some(pin) = self
                        .pins
                        .iter_mut()
                        .find(|pin| pin.id == pin_id && !pin.is_closed())
                        && pin.ocr_gate.pending()
                    {
                        if !pin.ocr_gate.finish(pin.mode) {
                            self.status = format!("贴图 #{pin_id} 已退出识别模式，扫码结果已丢弃");
                            continue;
                        }
                        match outcome {
                            Ok(document) => {
                                let count = document.codes.len();
                                pin.ocr_result = None;
                                pin.qr_result = Some(Arc::new(document));
                                pin.selected_text = None;
                                self.status = format!(
                                    "贴图 #{pin_id} 本地扫码完成，找到 {count} 个二维码；仅显式复制"
                                );
                            }
                            Err(error) => {
                                pin.qr_result = None;
                                self.status = format!("贴图 #{pin_id} 本地扫码失败：{error}");
                            }
                        }
                    }
                }
                WorkerResult::ScrollPrepared { action, outcome } => {
                    self.processing = false;
                    match outcome {
                        Ok((image, review)) => {
                            self.scroll_review = Some(review);
                            self.prepared.push((action, image));
                            self.status =
                                String::from("已提取长图选区，正在发送至剪贴板或悬浮贴图");
                        }
                        Err((error, review)) => {
                            self.scroll_review = Some(review);
                            self.status = error;
                        }
                    }
                }
                WorkerResult::ScrollSaved(outcome) => {
                    self.processing = false;
                    match outcome {
                        Ok(path) => {
                            self.scroll_preview_texture = None;
                            self.scroll_preview_radius = None;
                            self.scroll_share_region = None;
                            self.scroll_keep_segments = 1;
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
                            self.scroll_keep_segments =
                                review.store.strip_count().saturating_sub(1).max(1);
                            if self.scroll_share_region.is_none_or(|rect| {
                                validate_transfer_rect(rect, review.store.canvas()).is_err()
                            }) {
                                self.scroll_share_region =
                                    Some(default_transfer_rect(review.store.canvas()));
                            }
                            self.scroll_preview_pending = Some(review.preview.clone());
                            self.status = format!(
                                "长截图复核已更新：{} 段可信内容、{} 项标注",
                                review.store.strip_count(),
                                review.edits.applied_len()
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
                    if let Some(control) = self.scroll_control.as_ref()
                        && let Some(frame) = control.take_live_preview()
                    {
                        self.scroll_preview_pending = Some(frame);
                    }
                    self.status = format!(
                        "长截图正在拼接：{segments} 段，{width}×{height}px。滚动后按 {} 完成",
                        self.shortcuts.label(ShortcutAction::FinishScroll)
                    );
                }
                ScrollEvent::RecoveryNeeded {
                    segments,
                    width,
                    height,
                } => {
                    self.status = format!(
                        "长截图匹配暂停：已保留 {segments} 段可信内容（{width}×{height}px）。请把页面滚回有重叠的画面，再通过菜单栏选择“长截图慢速重试”，慢慢向原方向滚动"
                    );
                }
                ScrollEvent::RecoveryResumed => {
                    self.status =
                        String::from("长截图继续尝试：请慢慢滚动；只有确认重叠才会继续拼接");
                }
                ScrollEvent::RetakeFailed { review, error } => {
                    self.scroll_control.take();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    let canvas = review.store.canvas();
                    self.scroll_share_region = Some(default_transfer_rect(canvas));
                    self.scroll_retake_segment = review.store.strip_count().max(2);
                    self.scroll_preview_pending = Some(review.preview.clone());
                    self.scroll_review = Some(review);
                    self.status = format!("中间段重拍未成功，已保留可信前缀：{error}");
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
                            self.scroll_keep_segments = segments.saturating_sub(1).max(1);
                            self.scroll_retake_segment = segments.max(2);
                            self.scroll_preview_pending = Some(review.preview.clone());
                            self.scroll_share_region = Some(default_transfer_rect((width, height)));
                            self.scroll_tool = Tool::Select;
                            self.scroll_drag_anchor = None;
                            self.scroll_brush_points.clear();
                            self.scroll_review = Some(review);
                            format!(
                                "{prefix}：{segments} 段，{width}×{height}px。请复核标注、保存、复制选区或贴图"
                            )
                        }
                        Ok(None) => String::from("已取消长截图，所有临时分块已清理"),
                        Err(error) => format!("长截图失败：{error}"),
                    };
                }
            }
        }
        let pin_events: Vec<_> = self.pin_rx.try_iter().collect();
        for event in pin_events {
            self.process_pin_event(event, ctx);
        }
        self.prune_closed_pins();
    }

    fn process_ui_images(&mut self, ctx: &egui::Context) {
        if let Some(captured) = self.pending_frame.take() {
            self.capture_revision = self.capture_revision.saturating_add(1);
            self.screen = Some(SelectionView::new(
                ctx,
                captured.display_id,
                self.capture_revision,
                Arc::new(captured.image),
                std::mem::take(&mut self.pending_window_candidates),
            ));
            #[cfg(feature = "native-panel-probe")]
            if self.native_canary_enabled {
                match self.launch_native_selection(ctx) {
                    Ok(ProbeLaunchResult::Started) => {
                        let session_id = self.native_selection_revision.map(|(id, _)| id);
                        self.native_canary_phase = CanaryPhase::begin(true, session_id);
                        self.status = String::from(
                            "已优先打开 NSPanel 原生框选（试运行）；确认后返回 PNG 编辑器，取消则结束截图",
                        );
                    }
                    Ok(ProbeLaunchResult::AlreadyRunning) => {
                        self.native_canary_phase.reset();
                        self.status = String::from("原生框选窗口忙，已自动回退到稳定截图编辑器");
                    }
                    Err(error) => {
                        self.native_canary_phase.reset();
                        self.status =
                            format!("原生试运行不可用，已自动回退到稳定截图编辑器：{error}");
                    }
                }
            }
        }
        let incoming_preview = self.scroll_preview_pending.take();
        let has_new_review = incoming_preview.is_some();
        if self.scroll_control.is_some()
            && let Some(frame) = incoming_preview.as_ref()
        {
            // A live capture has no ScrollReview yet: upload the bounded
            // latest-frame mailbox directly instead of silently dropping it.
            update_scroll_live_texture(ctx, frame, &mut self.scroll_preview_texture);
        }
        if let Some(review) = self.scroll_review.as_ref()
            && (has_new_review || self.scroll_preview_radius != Some(self.radius))
        {
            // Only clone the already bounded 720x900 thumbnail, never the long canvas.
            let mut preview = review.preview.clone();
            let physical_width = review.store.canvas().0;
            let preview_radius = scroll::preview_radius(self.radius, physical_width, preview.width);
            shot_image::rounded_alpha(&mut preview, preview_radius);
            self.scroll_preview_texture = Some(ctx.load_texture(
                "scroll-review-preview",
                egui::ColorImage::from_rgba_unmultiplied(
                    [preview.width as usize, preview.height as usize],
                    &preview.rgba,
                ),
                egui::TextureOptions::LINEAR,
            ));
            self.scroll_preview_radius = Some(self.radius);
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
                #[cfg(feature = "native-panel-probe")]
                {
                    self.native_canary_phase.reset();
                    self.native_selection_revision = None;
                    let _ = self.native_probe.stop_for_capture();
                }
                self.screen = None;
                self.overlay_placement = None;
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.process_ui_images(ui.ctx());
        if self.scroll_control.is_some() {
            let paused = self
                .scroll_control
                .as_ref()
                .is_some_and(ScrollCaptureControl::is_paused);
            let context = ui.ctx().clone();
            let texture = self.scroll_preview_texture.as_ref();
            context.show_viewport_immediate(
                egui::ViewportId::from_hash_of("picpaste-scroll-live-preview"),
                egui::ViewportBuilder::default()
                    .with_title("PicPaste · 长截图实时预览")
                    .with_inner_size(egui::vec2(265.0, 385.0))
                    .with_position(egui::pos2(40.0, 40.0))
                    .with_decorations(false)
                    .with_resizable(false)
                    .with_always_on_top()
                    .with_active(false)
                    .with_mouse_passthrough(true),
                |preview_ui, _class| {
                    egui::CentralPanel::default().show(preview_ui, |ui| {
                        if paused {
                            ui.heading("长截图匹配暂停");
                            ui.colored_label(
                                egui::Color32::YELLOW,
                                "画面变化太大，未添加不可信内容",
                            );
                            ui.label("请手动滚回上次重叠的位置，再从菜单栏选择“长截图慢速重试”");
                            ui.label("重试后慢慢滚动；也可以从菜单栏结束，保存可信部分");
                        } else {
                            ui.heading("长截图实时预览");
                            ui.label("可鼠标穿透；由你手动滚动，软件只负责自动拼接");
                        }
                        if let Some(texture) = texture {
                            let full = texture.size_vec2();
                            let scale = (240.0 / full.x).min(290.0 / full.y).min(1.0);
                            ui.add(egui::Image::new((texture.id(), full * scale)));
                        } else {
                            ui.spinner();
                            ui.label("正在采集第一段…");
                        }
                    });
                },
            );
        }
        #[cfg(feature = "native-panel-probe")]
        let show_capture_overlay =
            self.overlay.is_fullscreen() && !self.native_canary_phase.hides_legacy();
        #[cfg(not(feature = "native-panel-probe"))]
        let show_capture_overlay = self.overlay.is_fullscreen();
        if show_capture_overlay {
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
        } else if !self.overlay.is_fullscreen() {
            self.render_content(ui);
        }
        // Pins are children of the root viewport and must outlive an editor.
        for pin in &self.pins {
            if !pin.is_closed() {
                pin.show(ui.ctx(), &self.pin_tx);
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
                // Only compiled in explicitly opted-in developer builds.
                // The normal capture path below remains an eframe viewport.
                #[cfg(feature = "native-panel-probe")]
                ui.collapsing("原生 NSPanel 试运行（开发包专用）", |ui| {
                    ui.checkbox(
                        &mut self.native_canary_enabled,
                        "下一次截图优先使用原生 NSPanel 选区与标注",
                    );
                    ui.label("默认关闭、仅本次运行有效。无法启动或回传错误时自动恢复稳定编辑器。");
                    ui.label("原生取消会结束本次截图；菜单栏选择“显示窗口”可随时退出卡住的试运行。");
                    ui.label("须先编译并在应用同目录放置 ns-panel-probe；完整真机验收尚未通过。");
                    if ui.button("启动独立 NSPanel 实验窗口").clicked() {
                        self.status = match self.native_probe.launch() {
                            Ok(ProbeLaunchResult::Started) => String::from(
                                "原生实验进程已启动；请在桌面确认是否正常显示（不代表真机验收通过）",
                            ),
                            Ok(ProbeLaunchResult::AlreadyRunning) => {
                                String::from("原生实验窗口进程正在运行；不会重复创建")
                            }
                            Err(error) => format!(
                                "原生实验窗口未启动：{error}；正式截图仍使用稳定窗口"
                            ),
                        };
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
                let paused = self
                    .scroll_control
                    .as_ref()
                    .is_some_and(ScrollCaptureControl::is_paused);
                ui.horizontal(|ui| {
                    if paused && ui.button("已回滚，继续慢速重试").clicked() {
                        self.resume_manual_scroll();
                    }
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
                #[cfg(feature = "native-panel-probe")]
                {
                    self.native_canary_phase.reset();
                    self.native_selection_revision = None;
                    let _ = self.native_probe.stop_for_capture();
                }
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
            #[cfg(feature = "native-panel-probe")]
            let mut requested_native_preview = false;
            #[cfg(feature = "native-panel-probe")]
            let mut requested_native_selection = false;
            let mut start_scroll = None;
            let mut recognize_text = None;
            let mut recognize_qr = false;
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
                    #[cfg(feature = "native-panel-probe")]
                    if ui.button("NSPanel 冻结画面预览（实验）").clicked() {
                        requested_native_preview = true;
                    }
                    #[cfg(feature = "native-panel-probe")]
                    if ui.button("NSPanel 原生框选（实验）").clicked() {
                        requested_native_selection = true;
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
                    if ui.button("识别词 (Accurate)").clicked() {
                        recognize_text = Some(false);
                    }
                    if ui.button("识别字 (Fast)").clicked() {
                        recognize_text = Some(true);
                    }
                    if ui.button("识别二维码 (离线)").clicked() {
                        recognize_qr = true;
                    }
                });
                // Leading/trailing physical-pixel bands apply to either scroll
                // direction. The chosen offsets stay fixed for this capture.
                ui.horizontal_wrapped(|ui| {
                    let max_band = screen
                        .selected_region()
                        .width
                        .max(screen.selected_region().height)
                        .saturating_sub(24);
                    ui.label("长截图排除固定栏（物理像素）");
                    ui.label("开头（上／左）");
                    ui.add(
                        egui::DragValue::new(&mut self.scroll_fixed_bands.leading)
                            .range(0..=max_band),
                    );
                    ui.label("末尾（下／右）");
                    ui.add(
                        egui::DragValue::new(&mut self.scroll_fixed_bands.trailing)
                            .range(0..=max_band),
                    );
                });
                ui.label("固定栏不进入长图 PNG；纵向忽略顶部/底部，横向忽略左侧/右侧");
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut screen.tool, Tool::Select, "选区");
                    ui.selectable_value(&mut screen.tool, Tool::Text, "文字");
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
                if screen.tool == Tool::Text {
                    ui.horizontal(|ui| {
                        ui.label("文字内容：");
                        ui.add(
                            egui::TextEdit::singleline(&mut screen.text_buffer)
                                .desired_width(230.0),
                        );
                        ui.add(egui::Slider::new(&mut screen.text_size, 12.0..=64.0).text("字号"));
                        ui.label("单击截图定位");
                    });
                }
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
                if let Some(document) = screen.ocr_result.as_ref() {
                    let picked = screen
                        .ocr_selection
                        .and_then(|selection| selected_selection_text(document, selection))
                        .map(str::to_string);
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut screen.ocr_mode, "文字选择模式");
                        let previous_mode = screen.ocr_pick_mode;
                        ui.radio_value(&mut screen.ocr_pick_mode, TextPickMode::Line, "选整行");
                        ui.radio_value(
                            &mut screen.ocr_pick_mode,
                            TextPickMode::Unit,
                            if screen.ocr_characters { "选字" } else { "选词" },
                        );
                        if screen.ocr_pick_mode != previous_mode {
                            screen.ocr_selection = None;
                        }
                        if let Some(text) = picked
                            && !text.is_empty()
                            && ui.button("复制选中的文字").clicked()
                        {
                            self.status = match Clipboard::new()
                                .and_then(|mut clipboard| clipboard.set_text(text))
                            {
                                Ok(()) => String::from("已复制 Vision 原生框对应文字"),
                                Err(error) => format!("复制文字失败：{error}"),
                            };
                        }
                    });
                } else if self.ocr_lines > 0 {
                    self.ocr_lines = 0;
                    self.ocr_text.clear();
                }
                if let Some(document) = screen.qr_result.as_ref() {
                    ui.collapsing(format!("离线二维码 · {} 个", document.codes.len()), |ui| {
                        if document.codes.is_empty() {
                            ui.label("没有识别到二维码；可扩大截图选区或提高原图清晰度");
                        }
                        for (index, code) in document.codes.iter().enumerate() {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(format!("二维码 {}：{}", index + 1, code.payload));
                                if ui.button(format!("复制二维码 {}", index + 1)).clicked() {
                                    self.status = match Clipboard::new()
                                        .and_then(|mut clipboard| clipboard.set_text(code.payload.clone()))
                                    {
                                        Ok(()) => String::from("二维码内容已复制；不会自动打开链接"),
                                        Err(error) => format!("二维码复制失败：{error}"),
                                    };
                                }
                            });
                        }
                    });
                }
            } else {
                ui.label(format!(
                    "按 {} 开始截图；或点击“截图”。",
                    self.shortcuts.label(ShortcutAction::Capture)
                ));
            }
            #[cfg(feature = "native-panel-probe")]
            if requested_native_preview {
                // Validate before cloning the potentially large frame. Both
                // transfer and rendering remain opt-in and read-only.
                let snapshot = (|| -> Result<NativeSnapshot, String> {
                    let screen = self.screen.as_ref().ok_or("截图尚未就绪")?;
                    let placement = self.overlay_placement.ok_or("截图显示器位置不可用")?;
                    if !placement.belongs_to(screen.display_id) {
                        return Err(String::from("原生预览显示器与截图不一致"));
                    }
                    NativeSnapshot::validate_dimensions(
                        screen.source.width,
                        screen.source.height,
                    )?;
                    NativeSnapshot::new(placement, screen.source.as_ref().clone())
                })();
                self.status = match snapshot {
                    Ok(snapshot) => match self.native_probe.launch_snapshot(snapshot) {
                        Ok(ProbeLaunchResult::Started) => String::from(
                            "已启动原生冻结画面预览进程；需真机确认 Metal 显示，原截图未修改",
                        ),
                        Ok(ProbeLaunchResult::AlreadyRunning) => {
                            String::from("原生实验窗口仍在运行，请先关闭后再次预览")
                        }
                        Err(error) => format!("原生冻结画面预览未启动：{error}"),
                    },
                    Err(error) => format!("原生冻结画面预览不可用：{error}"),
                };
            }
            #[cfg(feature = "native-panel-probe")]
            if requested_native_selection {
                self.status = match self.launch_native_selection(ui.ctx()) {
                    Ok(ProbeLaunchResult::Started) => String::from(
                        "已启动 NSPanel 原生框选与标注；确认后回传现有编辑器，Esc 取消（待真机验证）",
                    ),
                    Ok(ProbeLaunchResult::AlreadyRunning) => {
                        String::from("原生实验窗口仍在运行，请先关闭后再次框选")
                    }
                    Err(error) => format!("NSPanel 原生框选未启动：{error}"),
                };
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
                #[cfg(feature = "native-panel-probe")]
                {
                    self.native_canary_phase.reset();
                    self.native_selection_revision = None;
                    let _ = self.native_probe.stop_for_capture();
                }
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
            if let Some(characters) = recognize_text {
                self.request_ocr(ui.ctx(), characters);
            }
            if recognize_qr {
                self.request_qr(ui.ctx());
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
            let mut new_annotation = None;
            if let Some(review) = self.scroll_review.as_ref() {
                let canvas = review.store.canvas();
                let (width, height) = canvas;
                ui.separator();
                ui.heading("长截图复核与标注");
                if review.partial {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        "拼接未全部确认：只能保存已匹配的可信片段",
                    );
                }
                ui.label(format!(
                    "{} 段可信片段 · 原图 {}×{} px · 已应用 {} 个标注",
                    review.store.strip_count(),
                    width,
                    height,
                    review.edits.applied_len(),
                ));
                ui.label("标注使用原始像素坐标并写入完整长图 PNG；长图不会整张加载入内存");
                ui.horizontal_wrapped(|ui| {
                    for (tool, title) in [
                        (Tool::Select, "复制/贴图选区"),
                        (Tool::Line, "线条"),
                        (Tool::Arrow, "箭头"),
                        (Tool::Brush, "画笔"),
                        (Tool::Rectangle, "矩形"),
                        (Tool::Ellipse, "椭圆"),
                        (Tool::Pixelate, "马赛克"),
                        (Tool::Blur, "模糊"),
                        (Tool::Text, "文字"),
                    ] {
                        ui.selectable_value(&mut self.scroll_tool, tool, title);
                    }
                });
                if self.scroll_tool == Tool::Text {
                    ui.horizontal(|ui| {
                        ui.label("文字内容");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.scroll_text_buffer)
                                .desired_width(210.0),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.scroll_text_size, 12.0..=64.0).text("字号"),
                        );
                        ui.label("点击长图定位");
                    });
                }
                ui.horizontal(|ui| {
                    if ui.button("撤销标注").clicked() {
                        review_action = Some(4);
                    }
                    if ui.button("重做标注").clicked() {
                        review_action = Some(5);
                    }
                    ui.label("每项标注最多处理 400 万像素；区域过大会明确拒绝");
                });
                if let Some(texture) = self.scroll_preview_texture.as_ref() {
                    let full = texture.size_vec2();
                    let available = ui.available_size();
                    let scale = (available.x / full.x)
                        .min((available.y - 120.0).max(1.0) / full.y)
                        .min(1.0);
                    let response = ui.add(
                        egui::Image::new((texture.id(), full * scale))
                            .sense(egui::Sense::click_and_drag()),
                    );
                    let to_source = |pointer: egui::Pos2| {
                        map_preview_point(
                            (
                                pointer.x - response.rect.min.x,
                                pointer.y - response.rect.min.y,
                            ),
                            (response.rect.width(), response.rect.height()),
                            canvas,
                        )
                    };
                    if self.scroll_tool == Tool::Text
                        && response.clicked()
                        && !self.scroll_text_buffer.trim().is_empty()
                        && let Some(pointer) = response.interact_pointer_pos()
                        && let Some((x, y)) = to_source(pointer)
                    {
                        new_annotation = Some(Annotation::Text {
                            at: (x as f32, y as f32),
                            content: self.scroll_text_buffer.clone(),
                            size: self.scroll_text_size,
                            rgba: [39, 178, 231, 255],
                        });
                    }
                    if self.scroll_tool != Tool::Text {
                        if response.drag_started() {
                            self.scroll_drag_anchor = response.interact_pointer_pos();
                            self.scroll_brush_points.clear();
                        }
                        if (response.dragged() || response.drag_stopped())
                            && let Some(start) = self.scroll_drag_anchor
                            && let Some(end) = response.interact_pointer_pos()
                            && let (Some(a), Some(b)) = (to_source(start), to_source(end))
                        {
                            if self.scroll_tool == Tool::Select {
                                if response.drag_stopped() {
                                    self.scroll_share_region = from_drag(a, b).ok();
                                }
                            } else if self.scroll_tool == Tool::Brush {
                                if self.scroll_brush_points.is_empty() {
                                    self.scroll_brush_points.push((a.0 as f32, a.1 as f32));
                                }
                                let current = (b.0 as f32, b.1 as f32);
                                if self.scroll_brush_points.last() != Some(&current)
                                    && self.scroll_brush_points.len() < 20_000
                                {
                                    self.scroll_brush_points.push(current);
                                }
                                if response.drag_stopped() && !self.scroll_brush_points.is_empty() {
                                    new_annotation = Some(Annotation::Brush {
                                        points: std::mem::take(&mut self.scroll_brush_points),
                                        rgba: [39, 178, 231, 255],
                                        width: 3.0,
                                    });
                                }
                            } else if response.drag_stopped() {
                                new_annotation = annotation_for_drag(self.scroll_tool, a, b);
                            }
                        }
                        if response.drag_stopped() {
                            self.scroll_drag_anchor = None;
                            self.scroll_brush_points.clear();
                        }
                    }
                    if let Some(region) = self.scroll_share_region {
                        let sx = response.rect.width() / width as f32;
                        let sy = response.rect.height() / height as f32;
                        let selected = egui::Rect::from_min_size(
                            egui::pos2(
                                response.rect.left() + region.x as f32 * sx,
                                response.rect.top() + region.y as f32 * sy,
                            ),
                            egui::vec2(region.width as f32 * sx, region.height as f32 * sy),
                        );
                        let valid = validate_transfer_rect(region, canvas).is_ok();
                        ui.painter().rect_stroke(
                            selected,
                            0.0,
                            egui::Stroke::new(
                                2.0,
                                if valid {
                                    egui::Color32::from_rgb(72, 210, 175)
                                } else {
                                    egui::Color32::RED
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
                ui.label("复制/贴图仅针对绿色选区，按真实像素提取；不降低分辨率");
                if let Some(region) = self.scroll_share_region.as_mut() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("X");
                        ui.add(
                            egui::DragValue::new(&mut region.x)
                                .range(0..=width.saturating_sub(1) as i32),
                        );
                        ui.label("Y");
                        ui.add(
                            egui::DragValue::new(&mut region.y)
                                .range(0..=height.saturating_sub(1) as i32),
                        );
                        ui.label("宽");
                        ui.add(egui::DragValue::new(&mut region.width).range(1..=width));
                        ui.label("高");
                        ui.add(egui::DragValue::new(&mut region.height).range(1..=height));
                    });
                }
                let valid_selection = self
                    .scroll_share_region
                    .is_some_and(|region| validate_transfer_rect(region, canvas).is_ok());
                if !valid_selection {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        "选区需在长图范围内，最多 400 万像素，单边不超过 4096 像素",
                    );
                }
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(valid_selection, egui::Button::new("复制选区图片"))
                        .clicked()
                    {
                        review_action = Some(6);
                    }
                    if ui
                        .add_enabled(valid_selection, egui::Button::new("悬浮贴图选区"))
                        .clicked()
                    {
                        review_action = Some(7);
                    }
                    if ui.button("选区重置到安全大小").clicked() {
                        self.scroll_share_region = Some(default_transfer_rect(canvas));
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("确认保存完整 PNG").clicked() {
                        review_action = Some(0);
                    }
                    if ui.button("移除最后一个拼接段").clicked() {
                        review_action = Some(1);
                    }
                    if ui.button("放弃并清理").clicked() {
                        review_action = Some(2);
                    }
                });
                if review.store.strip_count() > 1 {
                    ui.horizontal(|ui| {
                        ui.label("回退到前");
                        ui.add(
                            egui::DragValue::new(&mut self.scroll_keep_segments)
                                .range(1..=review.store.strip_count()),
                        );
                        ui.label("段");
                        if ui
                            .add_enabled(
                                self.scroll_keep_segments < review.store.strip_count(),
                                egui::Button::new("裁剪后续所有片段"),
                            )
                            .clicked()
                        {
                            review_action = Some(3);
                        }
                    });
                    ui.label("先撤销标注，再裁剪可信片段；仅裁剪尾部，不强行拼接中间不可信段");
                    ui.horizontal_wrapped(|ui| {
                        ui.label("发现中间接缝错误？从第");
                        self.scroll_retake_segment = self
                            .scroll_retake_segment
                            .clamp(2, review.store.strip_count());
                        ui.add(
                            egui::DragValue::new(&mut self.scroll_retake_segment)
                                .range(2..=review.store.strip_count()),
                        );
                        ui.label("段开始重拍后续");
                        if let Some((start, end)) =
                            review.store.segment_span(self.scroll_retake_segment)
                        {
                            let axis_label = match review.target.axis {
                                ScrollAxis::Vertical => "Y",
                                ScrollAxis::Horizontal => "X",
                            };
                            ui.label(format!("{axis_label} {start}..{end} px"));
                        }
                        if ui
                            .add_enabled(
                                review.edits.applied_len() == 0,
                                egui::Button::new("保留可信前缀并重新手动采集"),
                            )
                            .clicked()
                        {
                            review_action = Some(8);
                        }
                    });
                    ui.label("纠错会丢弃所选段及全部后续段；先手动滚回上一个可信视口，再从原方向慢慢滚动，后段不直接拼接");
                }
            }
            if let Some(annotation) = new_annotation {
                self.edit_scroll_review(ui.ctx(), ScrollReviewEdit::Add(annotation));
            } else {
                match review_action {
                    Some(0) => self.save_scroll_review(ui.ctx()),
                    Some(1) => self.undo_scroll_review(ui.ctx()),
                    Some(2) => self.discard_scroll_review(),
                    Some(3) => self.trim_scroll_review(ui.ctx(), self.scroll_keep_segments),
                    Some(4) => self.edit_scroll_review(ui.ctx(), ScrollReviewEdit::Undo),
                    Some(5) => self.edit_scroll_review(ui.ctx(), ScrollReviewEdit::Redo),
                    Some(6) => self.prepare_scroll_transfer(ui.ctx(), PrepareAction::Copy),
                    Some(7) => self.prepare_scroll_transfer(ui.ctx(), PrepareAction::Pin),
                    Some(8) => self.retake_scroll_review(ui.ctx(), self.scroll_retake_segment),
                    _ => {}
                }
            }
            if !self.pins.is_empty() {
                ui.separator();
                ui.heading("悬浮贴图");
                ui.label(format!(
                    "GPU RGBA 纹理估算：{:.1} / {:.0} MiB · {}/{} 个贴图（相同图像复用）",
                    self.pin_budget.resident_bytes() as f64 / 1_048_576.0,
                    MAX_TEXTURE_BYTES as f64 / 1_048_576.0,
                    self.pin_budget.active_pins(),
                    MAX_PIN_COUNT,
                ));
                for pin in &mut self.pins {
                    ui.horizontal(|ui| {
                        ui.label(format!("#{}", pin.id));
                        let current = match pin.mode {
                            PinMode::Move => "移动",
                            PinMode::TextSelect => "文字选择",
                            PinMode::Passthrough => "穿透",
                        };
                        let mut next_mode = pin.mode;
                        egui::ComboBox::from_id_salt(("pin-mode", pin.id))
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut next_mode, PinMode::Move, "移动");
                                ui.selectable_value(
                                    &mut next_mode,
                                    PinMode::TextSelect,
                                    "文字选择",
                                );
                                ui.selectable_value(
                                    &mut next_mode,
                                    PinMode::Passthrough,
                                    "鼠标穿透",
                                );
                            });
                        if next_mode != pin.mode {
                            pin.set_mode(next_mode);
                        }
                        if ui.small_button("词 OCR").clicked() {
                            let _ = self.pin_tx.send(PinEvent::Recognize {
                                id: pin.id,
                                characters: false,
                            });
                        }
                        if ui.small_button("字 OCR").clicked() {
                            let _ = self.pin_tx.send(PinEvent::Recognize {
                                id: pin.id,
                                characters: true,
                            });
                        }
                        if ui.small_button("QR").clicked() {
                            let _ = self.pin_tx.send(PinEvent::DetectQr(pin.id));
                        }
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
        SelectionView::new(&ctx, DisplayId(1), capture_id, image, Vec::new())
    }

    #[cfg(feature = "native-panel-probe")]
    #[test]
    fn automatic_native_capture_validates_frozen_source_and_capture_revision() {
        let screen = test_screen(77);
        let placement = OverlayPlacement::new(DisplayId(1), (-100.0, 20.0), (80.0, 60.0)).unwrap();
        let original = screen.source.rgba.clone();
        let (snapshot, revision) = screen.native_capture_request(placement).unwrap();
        assert_eq!(revision, (77, 0));
        assert_eq!(snapshot.image().rgba, original);
        assert_eq!(screen.source.rgba, original);

        let wrong_screen =
            OverlayPlacement::new(DisplayId(2), (-100.0, 20.0), (80.0, 60.0)).unwrap();
        assert!(screen.native_capture_request(wrong_screen).is_err());
    }

    #[cfg(feature = "native-panel-probe")]
    #[test]
    fn native_marks_import_into_screenshot_png_preview_and_undo_redo() {
        use shot_core::native_markup::NativeMark;
        let mut screen = test_screen(77);
        let original = screen.source.rgba.clone();
        let packet = NativeEditReply::Annotated {
            selection: NativeSelectionReply::Selected {
                display: DisplayId(1),
                rect: RectPx {
                    x: 2,
                    y: 3,
                    width: 50,
                    height: 40,
                },
            },
            pixels: (80, 60),
            marks: vec![
                NativeMark::Rectangle {
                    top_left: (5, 5),
                    bottom_right: (25, 20),
                },
                NativeMark::Arrow {
                    from: (3, 4),
                    to: (30, 15),
                },
            ],
        };
        assert_eq!(screen.import_native_reply(&packet).unwrap(), Some(2));
        assert_eq!(screen.annotations.visible().len(), 2);
        assert_ne!(screen.rendered.rgba, original);
        assert_eq!(screen.selection.unwrap().width, 50);
        assert_eq!(screen.tool, Tool::Select);
        screen.undo();
        assert_eq!(screen.annotations.visible().len(), 1);
        screen.undo();
        assert_eq!(screen.annotations.visible().len(), 0);
        assert_eq!(screen.rendered.rgba, original);
        screen.redo();
        assert_ne!(screen.rendered.rgba, original);
    }

    #[cfg(feature = "native-panel-probe")]
    #[test]
    fn native_reply_rejects_invalid_second_mark_without_partial_import_or_selection() {
        use shot_core::native_markup::NativeMark;
        let mut screen = test_screen(77);
        let original = screen.source.rgba.clone();
        let packet = NativeEditReply::Annotated {
            selection: NativeSelectionReply::Selected {
                display: DisplayId(1),
                rect: RectPx {
                    x: 2,
                    y: 3,
                    width: 50,
                    height: 40,
                },
            },
            pixels: (80, 60),
            marks: vec![
                NativeMark::Arrow {
                    from: (3, 4),
                    to: (30, 15),
                },
                NativeMark::Text {
                    at: (80, 10),
                    content: String::from("bad"),
                },
            ],
        };
        assert!(screen.import_native_reply(&packet).is_err());
        assert!(screen.annotations.visible().is_empty());
        assert_eq!(screen.rendered.rgba, original);
        assert!(screen.selection.is_none());

        let expected = (screen.capture_revision, screen.edit_revision);
        assert!(screen.accepts_native_revision(expected));
        assert!(screen.accepts_native_session((101, expected), 101));
        assert!(!screen.accepts_native_session((101, expected), 100));
        screen.invalidate_ocr();
        assert!(!screen.accepts_native_session((101, expected), 101));
        assert!(!screen.accepts_native_revision(expected));
        assert!(!screen.accepts_native_revision((76, screen.edit_revision)));
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
    #[test]
    fn bounded_live_thumbnail_upload_keeps_original_pixel_dimensions() {
        let ctx = egui::Context::default();
        let frame = RgbaFrame::new(24, 12, vec![255; 24 * 12 * 4]).unwrap();
        let mut texture = None;
        update_scroll_live_texture(&ctx, &frame, &mut texture);
        let original_id = texture.as_ref().unwrap().id();
        assert_eq!(texture.as_ref().unwrap().size(), [24, 12]);
        let changed = RgbaFrame::new(20, 15, vec![42; 20 * 15 * 4]).unwrap();
        update_scroll_live_texture(&ctx, &changed, &mut texture);
        assert_eq!(texture.as_ref().unwrap().id(), original_id);
        assert_eq!(texture.as_ref().unwrap().size(), [20, 15]);
    }
}
