//! Experimental egui/wgpu renderer and *basic* AppKit input bridge.
//!
//! Never used by production screenshot, scrolling or pin windows. In
//! particular, handling direct-key text here is NOT an NSTextInputClient
//! implementation: marked-text IME composition requires a separate bridge.
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType, NSPanel, NSView};
use shot_core::native_panel::{PanelPointerPoint, PanelSurfaceSize};

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
    events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
    focused: bool,
    repaint: bool,
}

impl ProbeEgui {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        Self {
            ctx,
            renderer: egui_wgpu::Renderer::new(
                device,
                format,
                egui_wgpu::RendererOptions::default(),
            ),
            controls: ProbeControls::default(),
            events: Vec::new(),
            modifiers: egui::Modifiers::default(),
            focused: false,
            repaint: true,
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

    pub(crate) fn on_event(&mut self, event: &NSEvent, panel: &NSPanel, view: &NSView) {
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
        let controls = &mut self.controls;
        let output = self.ctx.run_ui(raw, |ui| controls.ui(ui));
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
        self.repaint = false;
        Ok(())
    }
}
