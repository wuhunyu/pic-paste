//! Experimental egui/wgpu renderer and *basic* AppKit input bridge.
//!
//! Never used by production screenshot, scrolling or pin windows. In
//! particular, handling direct-key text here is NOT an NSTextInputClient
//! implementation: marked-text IME composition requires a separate bridge.
use super::effect_preview::NativeEffectTextureCache;
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType, NSPanel, NSView};
use shot_core::glass_toolbar::PreviewTheme;
use shot_core::ime::ImeChange;
use shot_core::native_markup::{NativeMark, NativeMarkup, NativeMarkupTool};
use shot_core::native_panel::{PanelPointerPoint, PanelSurfaceSize};
use shot_core::native_selection_edit::NativeSelectionEdit;
use shot_image::native_effect_preview::preview_source_supported;

/// Convert native input-context callbacks to exactly one egui event.
pub(crate) fn egui_event_for_ime_change(change: ImeChange) -> Option<egui::Event> {
    match change {
        ImeChange::Preedit {
            text,
            active_range_chars,
        } => Some(egui::Event::Ime(egui::ImeEvent::Preedit {
            text,
            active_range_chars,
        })),
        ImeChange::Commit { text, was_marked } if !text.is_empty() => {
            if was_marked {
                Some(egui::Event::Ime(egui::ImeEvent::Commit(text)))
            } else {
                Some(egui::Event::Text(text))
            }
        }
        ImeChange::Commit { .. } => None,
    }
}

/// Theme changes only the egui demo canvas. This does not change system
/// Liquid Glass or any screenshot/PNG source pixels.
pub(crate) fn apply_preview_theme(ctx: &egui::Context, theme: PreviewTheme) {
    ctx.set_theme(match theme {
        PreviewTheme::Dark => egui::Theme::Dark,
        PreviewTheme::Light => egui::Theme::Light,
    });
}

/// The native renderer receives the source RGBA bytes verbatim and only
/// scales the *presentation* to fit the experimental panel. Source geometry
/// and PNG/Alpha output remain in the normal screenshot editor.
pub(crate) fn snapshot_texture(
    ctx: &egui::Context,
    frame: &shot_core::RgbaFrame,
) -> egui::TextureHandle {
    ctx.load_texture(
        "native-frozen-snapshot",
        egui::ColorImage::from_rgba_unmultiplied(
            [frame.width as usize, frame.height as usize],
            &frame.rgba,
        ),
        egui::TextureOptions::LINEAR,
    )
}

/// Paint a read-only image with no selection/annotation side effects.
pub(crate) fn show_snapshot(root: &mut egui::Ui, texture: &egui::TextureHandle) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(root, |ui| {
            let bounds = ui.max_rect();
            let source = texture.size_vec2();
            if source.x <= 0.0 || source.y <= 0.0 {
                return;
            }
            let scale = (bounds.width() / source.x).min(bounds.height() / source.y);
            if !scale.is_finite() || scale <= 0.0 {
                return;
            }
            let target = egui::Rect::from_center_size(bounds.center(), source * scale);
            ui.painter().image(
                texture.id(),
                target,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        });
}

/// A native selection is not returned when the first drag is released.
/// The user may adjust it indefinitely, then explicitly confirm or cancel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectionAction {
    Confirm,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectionDecision {
    Confirmed(shot_core::RectPx),
    Cancelled,
}

pub(crate) fn resolve_selection_action(
    selection: &NativeSelectionEdit,
    action: SelectionAction,
) -> Option<SelectionDecision> {
    match action {
        SelectionAction::Confirm => selection.confirmed().map(SelectionDecision::Confirmed),
        SelectionAction::Cancel => Some(SelectionDecision::Cancelled),
    }
}

/// Marks are drawn in a separate WGPU/egui layer, never composited into
/// the immutable screenshot texture. Their coordinates are physical source
/// pixels projected into the panel's local logical-point canvas.
pub(crate) fn paint_native_marks(ui: &egui::Ui, origin: egui::Pos2, markup: &NativeMarkup) {
    paint_native_marks_with_mode(ui, origin, markup, false);
}

fn paint_native_marks_with_mode(
    ui: &egui::Ui,
    origin: egui::Pos2,
    markup: &NativeMarkup,
    exact_effects: bool,
) {
    let painter = ui.painter();
    let stroke = egui::Stroke::new(3.0, egui::Color32::from_rgb(39, 178, 231));
    let at = |pixel: (u32, u32)| {
        let (x, y) = markup.to_logical_point(pixel);
        origin + egui::vec2(x, y)
    };
    let draw = |mark: &NativeMark| match mark {
        NativeMark::Brush { points } => {
            if points.len() >= 2 {
                let positions: Vec<_> = points.iter().copied().map(at).collect();
                painter.add(egui::Shape::line(positions, stroke));
            }
        }
        NativeMark::Line { from, to } => {
            painter.line_segment([at(*from), at(*to)], stroke);
        }
        NativeMark::Pixelate {
            left,
            top,
            width,
            height,
        }
        | NativeMark::Blur {
            left,
            top,
            width,
            height,
        } => {
            // ROI-only indicator: the source texture stays immutable.
            // The parent editor previews the actual effect after confirmation.
            let rect = egui::Rect::from_min_max(
                at((*left, *top)),
                at((left.saturating_add(*width), top.saturating_add(*height))),
            );
            painter.rect_filled(
                rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(39, 178, 231, 40),
            );
            painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
            let label = if matches!(mark, NativeMark::Pixelate { .. }) {
                "马赛克区域"
            } else {
                "模糊区域"
            };
            painter.text(
                rect.min + egui::vec2(4.0, 4.0),
                egui::Align2::LEFT_TOP,
                label,
                egui::FontId::proportional(14.0),
                stroke.color,
            );
        }
        NativeMark::Arrow { from, to } => {
            let a = at(*from);
            let b = at(*to);
            painter.arrow(a, b - a, stroke);
        }
        NativeMark::Rectangle {
            top_left,
            bottom_right,
        } => {
            painter.rect_stroke(
                egui::Rect::from_two_pos(at(*top_left), at(*bottom_right)),
                0.0,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        NativeMark::Ellipse {
            top_left,
            bottom_right,
        } => {
            let rect = egui::Rect::from_two_pos(at(*top_left), at(*bottom_right));
            painter.add(egui::Shape::ellipse_stroke(
                rect.center(),
                rect.size() / 2.0,
                stroke,
            ));
        }
        NativeMark::Text { at: point, content } => {
            painter.text(
                at(*point),
                egui::Align2::LEFT_TOP,
                content,
                egui::FontId::proportional(26.0),
                stroke.color,
            );
        }
    };
    for mark in markup.visible() {
        if exact_effects && matches!(mark, NativeMark::Pixelate { .. } | NativeMark::Blur { .. }) {
            continue;
        }
        draw(mark);
    }
    if let Some(draft) = markup.draft() {
        draw(&draft);
    }
}

/// Native selectable screenshot with eight adjustable grips plus local-only
/// annotation overlay. The annotation buttons do not alter PPSEL001 or PNG.
pub(crate) fn show_native_selection(
    root: &mut egui::Ui,
    texture: &egui::TextureHandle,
    selection: &mut NativeSelectionEdit,
    markup: &mut NativeMarkup,
    effect_preview: &NativeEffectTextureCache,
    text_buffer: &mut String,
    action: &mut Option<SelectionAction>,
) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(root, |ui| {
            let bounds = ui.max_rect();
            let response = ui.allocate_rect(bounds, egui::Sense::click_and_drag());
            ui.painter().image(
                texture.id(),
                bounds,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            let pointer = response
                .interact_pointer_pos()
                .or_else(|| ui.ctx().input(|input| input.pointer.latest_pos()));
            if let Some(pointer) = pointer {
                let point = (pointer.x - bounds.min.x, pointer.y - bounds.min.y);
                if markup.tool() == NativeMarkupTool::Select {
                    if response.drag_started() {
                        selection.begin(point);
                    }
                    if response.dragged() {
                        selection.update(point);
                    }
                    if response.drag_stopped() {
                        selection.end(point);
                    }
                } else if markup.tool() == NativeMarkupTool::Text {
                    if response.clicked() {
                        markup.place_text(point, text_buffer);
                    }
                } else {
                    if response.drag_started() {
                        markup.begin(point);
                    }
                    if response.dragged() {
                        markup.drag(point);
                    }
                    if response.drag_stopped() {
                        markup.end(point);
                    }
                }
            }
            // Cached RGBA patches sit above the frozen screenshot, below
            // draft outlines and selection grips. No source pixels mutate.
            effect_preview.paint(ui.painter(), bounds.min, markup);
            paint_native_marks_with_mode(ui, bounds.min, markup, effect_preview.is_exact());
            if let Some((x, y, width, height)) = selection.preview_rect_points()
                && width > 0.0
                && height > 0.0
            {
                let rect = egui::Rect::from_min_size(
                    bounds.min + egui::vec2(x, y),
                    egui::vec2(width, height),
                );
                ui.painter()
                    .rect_filled(rect, 0.0, egui::Color32::from_white_alpha(20));
                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(2.0, egui::Color32::YELLOW),
                    egui::StrokeKind::Inside,
                );
                if let Some(handles) = selection.grip_centers_points() {
                    for (_grip, (hx, hy)) in handles {
                        let grip_rect = egui::Rect::from_center_size(
                            bounds.min + egui::vec2(hx, hy),
                            egui::vec2(9.0, 9.0),
                        );
                        ui.painter()
                            .rect_filled(grip_rect, 2.0, egui::Color32::WHITE);
                        ui.painter().rect_stroke(
                            grip_rect,
                            2.0,
                            egui::Stroke::new(1.0, egui::Color32::BLACK),
                            egui::StrokeKind::Outside,
                        );
                    }
                }
            }
        });

    if markup.tool() == NativeMarkupTool::Text {
        egui::Area::new(egui::Id::unique("native-markup-text"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-18.0, 18.0))
            .show(root.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("文字内容");
                        ui.add(egui::TextEdit::singleline(text_buffer).desired_width(200.0));
                        ui.small("在画面中单击放置");
                    });
                });
            });
    }

    egui::Area::new(egui::Id::unique("native-selection-actions"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-18.0, -18.0))
        .show(root.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let has_effect = markup.visible().iter().any(|mark| {
                        matches!(mark, NativeMark::Pixelate { .. } | NativeMark::Blur { .. })
                    });
                    if effect_preview.is_exact() {
                        ui.small("已提交非重叠效果使用像素预览；拖动时仅标框");
                    } else if has_effect {
                        ui.small("复杂或大面积效果仅标框；确认后查看最终 PNG 效果");
                    } else {
                        ui.small("绘制后确认，将标注导入 PNG 编辑器");
                    }
                    if let Some(rect) = selection.current_rect() {
                        ui.label(format!("{} × {} px", rect.width, rect.height));
                    }
                    if ui
                        .add_enabled(
                            selection.confirmed().is_some(),
                            egui::Button::new("确认 (Enter)"),
                        )
                        .clicked()
                    {
                        *action = Some(SelectionAction::Confirm);
                    }
                    if ui.button("取消 (Esc)").clicked() {
                        *action = Some(SelectionAction::Cancel);
                    }
                });
            });
        });
}

pub(crate) struct ProbeViewport {
    pub size: PanelSurfaceSize,
    pub width_points: f64,
    pub height_points: f64,
    pub backing_scale: f64,
}

#[derive(Default)]
pub(crate) struct ProbeControls {
    clicks: u32,
    text: String,
}

impl ProbeControls {
    pub(crate) fn ui(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default().show(root, |ui| {
            ui.heading("PicPaste Native Panel");
            ui.label("egui + WGPU Metal / AppKit test");
            ui.separator();
            if ui
                .button(format!("Click counter: {}", self.clicks))
                .clicked()
            {
                self.clicks = self.clicks.saturating_add(1);
            }
            ui.label("Click the field and type ASCII text:");
            ui.text_edit_singleline(&mut self.text);
            ui.label("Escape hides the window; X closes it.");
            ui.separator();
            ui.small("Experimental only. CJK marked-text IME is not implemented.");
        });
    }
}

pub(crate) struct ProbeEgui {
    ctx: egui::Context,
    renderer: egui_wgpu::Renderer,
    controls: ProbeControls,
    snapshot: Option<egui::TextureHandle>,
    // Retain source CPU bytes only in opt-in selection mode and <=32 MiB.
    preview_source: Option<shot_core::RgbaFrame>,
    effect_previews: NativeEffectTextureCache,
    selection: Option<NativeSelectionEdit>,
    markup: Option<NativeMarkup>,
    text_buffer: String,
    selection_result: Option<shot_core::RectPx>,
    selection_finished: bool,
    events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
    focused: bool,
    repaint: bool,
    ime_cursor: Option<egui::Rect>,
}

impl ProbeEgui {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        image: Option<shot_core::RgbaFrame>,
        selection: Option<NativeSelectionEdit>,
        markup: Option<NativeMarkup>,
    ) -> Self {
        let ctx = egui::Context::default();
        apply_preview_theme(&ctx, PreviewTheme::Dark);
        let snapshot = image.as_ref().map(|frame| snapshot_texture(&ctx, frame));
        let preview_source = if selection.is_some() {
            image.filter(|frame| preview_source_supported(frame.width, frame.height))
        } else {
            None
        };
        Self {
            ctx,
            renderer: egui_wgpu::Renderer::new(
                device,
                format,
                egui_wgpu::RendererOptions::default(),
            ),
            controls: ProbeControls::default(),
            snapshot,
            preview_source,
            effect_previews: NativeEffectTextureCache::default(),
            selection,
            markup,
            text_buffer: String::from("文字"),
            selection_result: None,
            selection_finished: false,
            events: Vec::new(),
            modifiers: egui::Modifiers::default(),
            focused: false,
            repaint: true,
            ime_cursor: None,
        }
    }

    pub(crate) fn choose_markup_tool(&mut self, tool: NativeMarkupTool) {
        if let Some(markup) = self.markup.as_mut() {
            markup.set_tool(tool);
            self.repaint = true;
        }
    }

    pub(crate) fn undo_markup(&mut self) -> bool {
        let changed = self.markup.as_mut().is_some_and(NativeMarkup::undo);
        if changed {
            self.repaint = true;
        }
        changed
    }

    pub(crate) fn redo_markup(&mut self) -> bool {
        let changed = self.markup.as_mut().is_some_and(NativeMarkup::redo);
        if changed {
            self.repaint = true;
        }
        changed
    }

    /// Only committed marks can cross the IPC boundary; in-flight drafts are
    /// excluded. The caller copies them after explicit selection confirmation.
    pub(crate) fn committed_native_marks(&self) -> &[shot_core::native_markup::NativeMark] {
        self.markup.as_ref().map_or(&[], NativeMarkup::visible)
    }

    pub(crate) fn selection_result(&self) -> Option<shot_core::RectPx> {
        self.selection_result
    }

    pub(crate) fn selection_finished(&self) -> bool {
        self.selection_finished
    }

    pub(crate) fn confirm_selection(&mut self) -> bool {
        self.apply_selection_action(SelectionAction::Confirm)
    }

    pub(crate) fn cancel_selection(&mut self) {
        self.apply_selection_action(SelectionAction::Cancel);
    }

    fn apply_selection_action(&mut self, action: SelectionAction) -> bool {
        let Some(selection) = self.selection.as_ref() else {
            return false;
        };
        let Some(decision) = resolve_selection_action(selection, action) else {
            return false;
        };
        if self.selection_finished {
            return false;
        }
        self.selection_result = match decision {
            SelectionDecision::Confirmed(rect) => Some(rect),
            SelectionDecision::Cancelled => None,
        };
        self.selection_finished = true;
        self.repaint = true;
        true
    }

    pub(crate) fn set_preview_theme(&mut self, theme: PreviewTheme) {
        apply_preview_theme(&self.ctx, theme);
        self.repaint = true;
    }

    /// IME is only activated while egui has a focused TextEdit.
    pub(crate) fn wants_text_input(&self) -> bool {
        self.ime_cursor.is_some()
    }

    pub(crate) fn ime_cursor_rect(&self) -> Option<egui::Rect> {
        self.ime_cursor
    }

    pub(crate) fn push_ime_change(&mut self, change: ImeChange) {
        if let Some(event) = egui_event_for_ime_change(change) {
            self.events.push(event);
            self.repaint = true;
        }
    }

    pub(crate) fn needs_repaint(&self) -> bool {
        self.repaint
    }

    pub(crate) fn set_focus(&mut self, focused: bool) {
        if self.focused != focused {
            self.focused = focused;
            self.events.push(egui::Event::WindowFocused(focused));
            self.repaint = true;
        }
    }

    fn point_for(event: &NSEvent, view: &NSView) -> Option<egui::Pos2> {
        // convertPoint:fromView:nil maps from window-base into this NSView.
        let local = view.convertPoint_fromView(event.locationInWindow(), None);
        let bounds = view.bounds();
        let point = PanelPointerPoint::from_view(
            local.x,
            local.y,
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.width,
            bounds.size.height,
            view.isFlipped(),
        )
        .ok()?;
        Some(egui::pos2(point.x, point.y))
    }

    fn event_modifiers(event: &NSEvent) -> egui::Modifiers {
        let flags = event.modifierFlags();
        let command = flags.contains(NSEventModifierFlags::Command);
        egui::Modifiers {
            alt: flags.contains(NSEventModifierFlags::Option),
            ctrl: flags.contains(NSEventModifierFlags::Control),
            shift: flags.contains(NSEventModifierFlags::Shift),
            mac_cmd: command,
            command,
        }
    }

    fn key_for(key_code: u16) -> Option<egui::Key> {
        Some(match key_code {
            36 | 76 => egui::Key::Enter,
            48 => egui::Key::Tab,
            49 => egui::Key::Space,
            51 => egui::Key::Backspace,
            53 => egui::Key::Escape,
            117 => egui::Key::Delete,
            123 => egui::Key::ArrowLeft,
            124 => egui::Key::ArrowRight,
            125 => egui::Key::ArrowDown,
            126 => egui::Key::ArrowUp,
            // Common macOS virtual keys (US keyboard). Other layouts are
            // not asserted correct until actual keyboard/IME acceptance.
            0 => egui::Key::A,
            11 => egui::Key::B,
            8 => egui::Key::C,
            2 => egui::Key::D,
            14 => egui::Key::E,
            3 => egui::Key::F,
            5 => egui::Key::G,
            4 => egui::Key::H,
            34 => egui::Key::I,
            38 => egui::Key::J,
            40 => egui::Key::K,
            37 => egui::Key::L,
            46 => egui::Key::M,
            45 => egui::Key::N,
            31 => egui::Key::O,
            35 => egui::Key::P,
            12 => egui::Key::Q,
            15 => egui::Key::R,
            1 => egui::Key::S,
            17 => egui::Key::T,
            32 => egui::Key::U,
            9 => egui::Key::V,
            13 => egui::Key::W,
            7 => egui::Key::X,
            16 => egui::Key::Y,
            6 => egui::Key::Z,
            _ => return None,
        })
    }

    pub(crate) fn on_event(
        &mut self,
        event: &NSEvent,
        panel: &NSPanel,
        view: &NSView,
        native_text_active: bool,
    ) {
        // AppKit continues to own events for title bar, close and resizing.
        // A probe must not siphon input destined for another application view.
        if event.windowNumber() != panel.windowNumber() {
            return;
        }
        self.modifiers = Self::event_modifiers(event);
        let event_type = event.r#type();
        match event_type {
            NSEventType::MouseMoved
            | NSEventType::LeftMouseDragged
            | NSEventType::RightMouseDragged
            | NSEventType::OtherMouseDragged => {
                if let Some(pos) = Self::point_for(event, view) {
                    self.events.push(egui::Event::PointerMoved(pos));
                }
            }
            NSEventType::LeftMouseDown
            | NSEventType::LeftMouseUp
            | NSEventType::RightMouseDown
            | NSEventType::RightMouseUp
            | NSEventType::OtherMouseDown
            | NSEventType::OtherMouseUp => {
                if let Some(pos) = Self::point_for(event, view) {
                    let primary = matches!(
                        event_type,
                        NSEventType::LeftMouseDown | NSEventType::LeftMouseUp
                    );
                    let secondary = matches!(
                        event_type,
                        NSEventType::RightMouseDown | NSEventType::RightMouseUp
                    );
                    let pressed = matches!(
                        event_type,
                        NSEventType::LeftMouseDown
                            | NSEventType::RightMouseDown
                            | NSEventType::OtherMouseDown
                    );
                    let button = if primary {
                        egui::PointerButton::Primary
                    } else if secondary {
                        egui::PointerButton::Secondary
                    } else {
                        egui::PointerButton::Middle
                    };
                    self.events.push(egui::Event::PointerButton {
                        pos,
                        button,
                        pressed,
                        modifiers: self.modifiers,
                    });
                }
            }
            NSEventType::MouseExited => self.events.push(egui::Event::PointerGone),
            NSEventType::ScrollWheel => {
                let unit = if event.hasPreciseScrollingDeltas() {
                    egui::MouseWheelUnit::Point
                } else {
                    egui::MouseWheelUnit::Line
                };
                self.events.push(egui::Event::MouseWheel {
                    unit,
                    delta: egui::vec2(
                        event.scrollingDeltaX() as f32,
                        event.scrollingDeltaY() as f32,
                    ),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Unknown,
                    modifiers: self.modifiers,
                });
            }
            NSEventType::FlagsChanged => {
                self.events
                    .push(egui::Event::ModifiersChanged(self.modifiers));
            }
            NSEventType::KeyDown | NSEventType::KeyUp => {
                let pressed = event_type == NSEventType::KeyDown;
                if let Some(key) = Self::key_for(event.keyCode()) {
                    self.events.push(egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed,
                        repeat: pressed && event.isARepeat(),
                        modifiers: self.modifiers,
                    });
                }
                // Direct-key ASCII proof only. Native text input / IME
                // marked-text candidates require NSTextInputClient.
                if pressed
                    && !native_text_active
                    && !self.modifiers.command
                    && !self.modifiers.ctrl
                    && !self.modifiers.alt
                    && let Some(chars) = event.characters()
                {
                    let text = chars.to_string();
                    if !text.is_empty() && text.is_ascii() && text.chars().all(|c| !c.is_control())
                    {
                        self.events.push(egui::Event::Text(text));
                    }
                }
            }
            _ => return,
        }
        self.repaint = true;
    }

    pub(crate) fn paint(
        &mut self,
        surface: &wgpu::Surface<'_>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        viewport: ProbeViewport,
    ) -> Result<(), String> {
        let scale = viewport.backing_scale as f32;
        if !scale.is_finite() || scale <= 0.0 {
            return Err("invalid native pixels-per-point".to_owned());
        }
        let mut raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(viewport.width_points as f32, viewport.height_points as f32),
            )),
            focused: self.focused,
            events: std::mem::take(&mut self.events),
            ..Default::default()
        };
        if let Some(viewport) = raw.viewports.get_mut(&egui::ViewportId::ROOT) {
            viewport.native_pixels_per_point = Some(scale);
            viewport.focused = Some(self.focused);
        }
        // A prior AppKit toolbar undo/redo may have changed the history.
        // Discard stale GPU patches *before* this frame is painted.
        if let Some(markup) = self.markup.as_ref() {
            self.effect_previews.sync(
                &self.ctx,
                self.preview_source.as_ref(),
                markup.visible(),
            );
        }
        let controls = &mut self.controls;
        let snapshot = self.snapshot.as_ref();
        let effect_previews = &self.effect_previews;
        let selection = &mut self.selection;
        let markup = &mut self.markup;
        let text_buffer = &mut self.text_buffer;
        let mut selection_action = None;
        let output = self.ctx.run_ui(raw, |ui| {
            if let Some(texture) = snapshot {
                if let (Some(selection), Some(markup)) = (selection.as_mut(), markup.as_mut()) {
                    show_native_selection(
                        ui,
                        texture,
                        selection,
                        markup,
                        effect_previews,
                        text_buffer,
                        &mut selection_action,
                    );
                } else {
                    show_snapshot(ui, texture);
                }
            } else {
                controls.ui(ui);
            }
        });
        if let Some(action) = selection_action {
            self.apply_selection_action(action);
        }
        // Gestures commit during run_ui. Prepare the changed preview only
        // once and paint it on the next frame, never on every drag event.
        let preview_changed = self.markup.as_ref().is_some_and(|markup| {
            self.effect_previews.sync(
                &self.ctx,
                self.preview_source.as_ref(),
                markup.visible(),
            )
        });
        // egui reports the actual text caret in logical points. The native
        // NSTextInputClient converts it to AppKit screen coordinates.
        self.ime_cursor = output
            .platform_output
            .ime
            .as_ref()
            .map(|ime| ime.cursor_rect);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [viewport.size.width, viewport.size.height],
            pixels_per_point: output.pixels_per_point,
        };
        let paint_jobs = self.ctx.tessellate(output.shapes, screen.pixels_per_point);
        for (id, images) in &output.textures_delta.set {
            for image in images {
                self.renderer.update_texture(device, queue, *id, image);
            }
        }
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            status => return Err(format!("acquire NSPanel egui frame: {status:?}")),
        };
        let texture_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("PicPaste NSPanel egui"),
        });
        let user_commands =
            self.renderer
                .update_buffers(device, queue, &mut encoder, &paint_jobs, &screen);
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("PicPaste egui Metal pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &texture_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.08,
                                g: 0.17,
                                b: 0.34,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            self.renderer.render(&mut pass, &paint_jobs, &screen);
        }
        queue.submit(
            user_commands
                .into_iter()
                .chain(std::iter::once(encoder.finish())),
        );
        queue.present(frame);
        for id in &output.textures_delta.free {
            self.renderer.free_texture(id);
        }
        self.repaint = preview_changed;
        Ok(())
    }
}
