//! Explicitly invoked AppKit NSPanel + WGPU Metal smoke test.
//!
//! This binary is gated by `native-panel-probe`, and never replaces the
//! production eframe/winit screenshot or pin window backend.
use shot_core::native_panel::PanelSurfaceSize;
use shot_core::native_snapshot::NativeSnapshot;

#[cfg(target_os = "macos")]
#[path = "ns-panel-probe/panel_egui.rs"]
mod panel_egui;

#[cfg(target_os = "macos")]
#[path = "ns-panel-probe/glass_toolbar.rs"]
mod glass_toolbar;

#[cfg(target_os = "macos")]
#[path = "ns-panel-probe/ime.rs"]
mod ime;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProbeCommand {
    Check,
    Show,
    Snapshot,
    Select,
}

impl ProbeCommand {
    fn parse(args: &[&str]) -> Result<Self, &'static str> {
        match args {
            ["--check"] => Ok(Self::Check),
            ["--show"] => Ok(Self::Show),
            ["--snapshot"] => Ok(Self::Snapshot),
            ["--select"] => Ok(Self::Select),
            _ => Err("usage: ns-panel-probe --check | --show | --snapshot | --select"),
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match ProbeCommand::parse(&args)? {
        ProbeCommand::Check => {
            // Pure startup contract: do not initialize NSApplication, Metal,
            // the window server, TCC or the GPU in automated CI.
            let size = PanelSurfaceSize::from_points(640.0, 360.0, 2.0)
                .map_err(|error| format!("{error:?}"))?;
            println!(
                "PicPaste NSPanel/WGPU probe OK (no GUI): {}x{} physical pixels",
                size.width, size.height
            );
            Ok(())
        }
        ProbeCommand::Show => native::show(None, false),
        ProbeCommand::Snapshot => {
            // Bound and verify the complete one-shot stdin frame before
            // initializing AppKit or allocating a Metal surface.
            let stdin = std::io::stdin();
            let snapshot = NativeSnapshot::read_from(&mut stdin.lock())?;
            native::show(Some(snapshot), false)
        }
        ProbeCommand::Select => {
            // Reserve stdout for one PPANN001 or PPSEL001 reply; diagnostics use stderr.
            // Decode the complete bounded screenshot before initializing AppKit.
            let stdin = std::io::stdin();
            let snapshot = NativeSnapshot::read_from(&mut stdin.lock())?;
            native::show(Some(snapshot), true)
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("PicPaste native panel probe: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "macos"))]
mod native {
    use super::NativeSnapshot;

    pub fn show(_snapshot: Option<NativeSnapshot>, _selection_mode: bool) -> Result<(), String> {
        Err(String::from("interactive NSPanel requires macOS 14+"))
    }
}

#[cfg(target_os = "macos")]
mod native {
    use super::glass_toolbar::ProbeGlassToolbar;
    use super::ime::ProbeImeBridge;
    use super::panel_egui::{ProbeEgui, ProbeViewport};
    use super::{NativeSnapshot, PanelSurfaceSize};
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSApplication, NSBackingStoreType, NSEventMask, NSEventModifierFlags, NSEventType, NSPanel,
        NSScreen, NSWindowCollectionBehavior, NSWindowStyleMask,
    };
    use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSPoint, NSRect, NSSize, ns_string};
    use raw_window_handle::{AppKitWindowHandle, RawWindowHandle};
    use shot_core::glass_toolbar::{GlassToolbarAction, PreviewTheme};
    use shot_core::ime::{ImeKeyRoute, ImeKeyRouting};
    use shot_core::native_markup::NativeMarkup;
    use shot_core::native_panel::{PanelSurfaceLifecycle, PanelSurfaceUpdate};
    use shot_core::native_annotation::NativeEditReply;
    use shot_core::native_selection::{AppKitSelectionFrame, NativeSelectionReply};
    use shot_core::native_selection_edit::NativeSelectionEdit;
    use std::ptr::NonNull;

    /// This opt-in probe must run on AppKit's main thread. It pumps native
    /// events itself so that a close event can return control to Rust and
    /// release the Metal surface before AppKit's retained view and panel.
    pub fn show(snapshot: Option<NativeSnapshot>, selection_mode: bool) -> Result<(), String> {
        let frozen_preview = snapshot.is_some();
        let source_pixels = snapshot.as_ref().map(|snapshot| {
            let source = snapshot.image();
            (source.width, source.height)
        });
        let selection_placement = if selection_mode {
            Some(
                snapshot
                    .as_ref()
                    .ok_or("native selection requires screenshot")?
                    .placement(),
            )
        } else {
            None
        };
        let selected_display = selection_placement.map(|placement| placement.display_id());
        let mtm =
            MainThreadMarker::new().ok_or("AppKit NSPanel must be created on the main thread")?;
        let application = NSApplication::sharedApplication(mtm);
        let default_rect = NSRect::new(NSPoint::new(160.0, 160.0), NSSize::new(640.0, 360.0));
        let selection_rect = if let Some(placement) = selection_placement {
            let screens = NSScreen::screens(mtm);
            let primary = screens.firstObject().ok_or("no active AppKit screen")?;
            let primary_bounds = primary.frame();
            let primary_top_y = primary_bounds.origin.y + primary_bounds.size.height;
            let target = AppKitSelectionFrame::from_quartz(placement, primary_top_y)?;
            let known_display = screens.iter().any(|screen| {
                let bounds = screen.frame();
                (bounds.origin.x - target.x).abs() <= 1.0
                    && (bounds.origin.y - target.y).abs() <= 1.0
                    && (bounds.size.width - target.width).abs() <= 1.0
                    && (bounds.size.height - target.height).abs() <= 1.0
            });
            if !known_display {
                return Err(String::from(
                    "capture display no longer matches an AppKit screen",
                ));
            }
            Some(NSRect::new(
                NSPoint::new(target.x, target.y),
                NSSize::new(target.width, target.height),
            ))
        } else {
            None
        };
        let rect = selection_rect.unwrap_or(default_rect);
        let style = if selection_mode {
            NSWindowStyleMask::Borderless
        } else {
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Resizable
        };
        let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
            NSPanel::alloc(mtm),
            rect,
            style,
            NSBackingStoreType::Buffered,
            false,
        );
        // SAFETY: window ownership remains explicit until the WGPU surface
        // has been dropped; the X close button must not release the NSPanel.
        unsafe { panel.setReleasedWhenClosed(false) };
        panel.setFloatingPanel(true);
        panel.setBecomesKeyOnlyIfNeeded(false);
        if selection_mode {
            panel.setCollectionBehavior(
                NSWindowCollectionBehavior::CanJoinAllSpaces
                    | NSWindowCollectionBehavior::FullScreenAuxiliary,
            );
        }
        if selection_mode {
            panel.setTitle(ns_string!("PicPaste · 原生截图框选（实验）"));
        } else if frozen_preview {
            panel.setTitle(ns_string!("PicPaste · 冻结截图原生预览（实验）"));
        } else {
            panel.setTitle(ns_string!("PicPaste · NSPanel + WGPU Metal 实验"));
        }
        panel.setHasShadow(false);
        panel.setMovableByWindowBackground(false);
        panel.setAcceptsMouseMovedEvents(true);
        let content_view = panel
            .contentView()
            .ok_or("AppKit NSPanel has no content NSView")?;
        content_view.setWantsLayer(true);

        if !selection_mode {
            panel.center();
        }
        application.finishLaunching();
        panel.makeKeyAndOrderFront(None);
        application.activate();
        if selection_mode {
            panel.orderFrontRegardless();
        }

        let bounds = content_view.bounds();
        if let Some(placement) = selection_placement {
            let (width, height) = placement.size_pt();
            if (bounds.size.width - f64::from(width)).abs() > 1.0
                || (bounds.size.height - f64::from(height)).abs() > 1.0
            {
                return Err(String::from(
                    "AppKit native selection content geometry changed",
                ));
            }
        }
        let scale = panel.backingScaleFactor();
        let size = PanelSurfaceSize::from_points(bounds.size.width, bounds.size.height, scale)
            .map_err(|error| format!("invalid initial Metal size: {error:?}"))?;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });
        let handle = AppKitWindowHandle::new(NonNull::from(&*content_view).cast());
        // SAFETY: the retained NSView and its owner panel stay alive on the
        // main thread until the WGPU surface has been dropped.
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: None,
                raw_window_handle: RawWindowHandle::AppKit(handle),
            })
        }
        .map_err(|error| format!("create Metal surface: {error}"))?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .map_err(|error| format!("request Metal adapter: {error}"))?;
        if adapter.get_info().backend != wgpu::Backend::Metal {
            return Err(String::from(
                "NSPanel probe unexpectedly chose non-Metal backend",
            ));
        }
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(|error| format!("request WGPU device: {error}"))?;
        let mut config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or("Metal adapter does not support NSPanel surface")?;
        surface.configure(&device, &config);
        let native_selection = if let Some(placement) = selection_placement {
            let frame = snapshot
                .as_ref()
                .ok_or("selection frame unavailable")?
                .image();
            Some(NativeSelectionEdit::new(
                placement,
                (frame.width, frame.height),
            )?)
        } else {
            None
        };
        let native_markup = if let Some(placement) = selection_placement {
            let frame = snapshot
                .as_ref()
                .ok_or("annotation source unavailable")?
                .image();
            Some(NativeMarkup::new(placement, (frame.width, frame.height))?)
        } else {
            None
        };
        let mut egui_ui = ProbeEgui::new(
            &device,
            config.format,
            snapshot.map(NativeSnapshot::into_image),
            native_selection,
            native_markup,
        );
        egui_ui.set_focus(panel.isKeyWindow());
        egui_ui.paint(
            &surface,
            &device,
            &queue,
            ProbeViewport {
                size,
                width_points: bounds.size.width,
                height_points: bounds.size.height,
                backing_scale: scale,
            },
        )?;
        // Native AppKit overlay floats over Metal. The existing NSView is
        // still the WGPU Surface owner; screenshot pixels remain untouched.
        let glass_toolbar = Some(if selection_mode {
            ProbeGlassToolbar::new_selection(mtm, &content_view)
        } else {
            ProbeGlassToolbar::new(mtm, &content_view)
        });
        if let Some(toolbar) = glass_toolbar.as_ref() {
            if selection_mode {
                eprintln!("Native system annotation material: {:?}", toolbar.material);
            } else {
                println!("Experimental native toolbar: {:?}", toolbar.material);
            }
        }
        let mut ime_bridge = ProbeImeBridge::new(mtm, &content_view);
        let mut key_routing = ImeKeyRouting::default();
        ime_bridge.set_cursor(egui_ui.ime_cursor_rect());
        ime_bridge.set_active(panel.isKeyWindow() && egui_ui.wants_text_input());
        let mut preview_theme = PreviewTheme::Dark;

        let mut lifecycle = PanelSurfaceLifecycle::new(size);
        if selection_mode {
            eprintln!("Native display selection panel running; drag a region or press Escape.");
        } else {
            println!(
                "NSPanel Metal frame presented. Resize or move between Retina displays; close with X."
            );
        }
        // Time-bounded native event wait. Do not busy-spin or enter AppKit
        // run() forever: the window close button must exit the probe.
        let result = (|| -> Result<(), String> {
            while panel.isVisible() {
                let deadline = NSDate::dateWithTimeIntervalSinceNow(0.05);
                // SAFETY: Apple's immutable default-runloop-mode global has
                // process-wide static lifetime.
                let mode = unsafe { NSDefaultRunLoopMode };
                if let Some(event) = application.nextEventMatchingMask_untilDate_inMode_dequeue(
                    NSEventMask::Any,
                    Some(&deadline),
                    mode,
                    true,
                ) {
                    let escape = event.windowNumber() == panel.windowNumber()
                        && event.r#type() == NSEventType::KeyDown
                        && event.keyCode() == 53;
                    let enter = selection_mode
                        && !egui_ui.wants_text_input()
                        && event.windowNumber() == panel.windowNumber()
                        && event.r#type() == NSEventType::KeyDown
                        && matches!(event.keyCode(), 36 | 76);
                    let pointer_down = matches!(
                        event.r#type(),
                        NSEventType::LeftMouseDown
                            | NSEventType::RightMouseDown
                            | NSEventType::OtherMouseDown
                    );
                    let native_text_active =
                        ime_bridge.is_active() && event.windowNumber() == panel.windowNumber();
                    // Cocoa produces the text/preedit callbacks. Do not also
                    // inject NSEvent.characters() for the same key press.
                    // Preserve normal Command/Control shortcuts in egui.
                    let native_eligible = native_text_active
                        && !event.modifierFlags().intersects(
                            NSEventModifierFlags::Command | NSEventModifierFlags::Control,
                        );
                    let route = match event.r#type() {
                        NSEventType::KeyDown => {
                            let handled = native_eligible && ime_bridge.handle_key(&event);
                            key_routing.key_down(event.keyCode(), native_eligible, handled)
                        }
                        NSEventType::KeyUp => key_routing.key_up(event.keyCode()),
                        _ => ImeKeyRoute::Egui,
                    };
                    if route == ImeKeyRoute::Egui
                        && (!pointer_down
                            || glass_toolbar.as_ref().is_none_or(|toolbar| {
                                !toolbar.contains_pointer(&event, &content_view)
                            }))
                    {
                        // This key was not consumed by the native context; allow
                        // the original ASCII fallback for this event.
                        egui_ui.on_event(&event, &panel, &content_view, false);
                    }
                    application.sendEvent(&event);
                    for change in ime_bridge.drain() {
                        egui_ui.push_ime_change(change);
                    }
                    if let Some(action) = glass_toolbar.as_ref().and_then(|bar| bar.take_action()) {
                        match action {
                            GlassToolbarAction::ToggleTheme => {
                                preview_theme = preview_theme.toggle();
                                if let Some(bar) = glass_toolbar.as_ref() {
                                    bar.set_preview_theme(preview_theme);
                                }
                                egui_ui.set_preview_theme(preview_theme);
                            }
                            GlassToolbarAction::CloseProbe => panel.orderOut(None),
                            GlassToolbarAction::ChooseTool(tool) => {
                                egui_ui.choose_markup_tool(tool);
                                if let Some(bar) = glass_toolbar.as_ref() {
                                    bar.set_markup_tool(tool);
                                }
                            }
                            GlassToolbarAction::UndoMarkup => {
                                egui_ui.undo_markup();
                            }
                            GlassToolbarAction::CancelSelection => {
                                egui_ui.cancel_selection();
                                panel.orderOut(None);
                            }
                        }
                    }
                    if enter && egui_ui.confirm_selection() {
                        panel.orderOut(None);
                    }
                    if escape {
                        if selection_mode {
                            egui_ui.cancel_selection();
                        }
                        panel.orderOut(None);
                    }
                }
                egui_ui.set_focus(panel.isKeyWindow());
                let should_activate_ime = panel.isKeyWindow() && egui_ui.wants_text_input();
                if ime_bridge.is_active() && !should_activate_ime {
                    key_routing.reset();
                }
                ime_bridge.set_active(should_activate_ime);
                for change in ime_bridge.drain() {
                    egui_ui.push_ime_change(change);
                }
                if !panel.isVisible() {
                    break;
                }
                // Minimize invalidates visible content. Force one redraw on
                // restoration even when dimensions have not changed.
                if panel.isMiniaturized() {
                    let _ = lifecycle.observe(0.0, 0.0, 1.0);
                    continue;
                }
                let view_bounds = content_view.bounds();
                if let Some(bar) = glass_toolbar.as_ref() {
                    bar.relayout(view_bounds);
                }
                let change = lifecycle
                    .observe(
                        view_bounds.size.width,
                        view_bounds.size.height,
                        panel.backingScaleFactor(),
                    )
                    .map_err(|error| format!("invalid resized Metal surface: {error:?}"))?;
                let mut repaint = egui_ui.needs_repaint();
                match change {
                    PanelSurfaceUpdate::Reconfigure(next) => {
                        config.width = next.width;
                        config.height = next.height;
                        surface.configure(&device, &config);
                        repaint = true;
                    }
                    PanelSurfaceUpdate::Unchanged => {}
                    PanelSurfaceUpdate::Suspended => continue,
                    PanelSurfaceUpdate::Closed => break,
                }
                if repaint {
                    let current_size = lifecycle
                        .configured_size()
                        .ok_or("native panel has no configured surface")?;
                    egui_ui.paint(
                        &surface,
                        &device,
                        &queue,
                        ProbeViewport {
                            size: current_size,
                            width_points: view_bounds.size.width,
                            height_points: view_bounds.size.height,
                            backing_scale: panel.backingScaleFactor(),
                        },
                    )?;
                    ime_bridge.set_cursor(egui_ui.ime_cursor_rect());
                    ime_bridge.set_active(panel.isKeyWindow() && egui_ui.wants_text_input());
                    if selection_mode && egui_ui.selection_finished() {
                        panel.orderOut(None);
                    }
                }
            }
            Ok(())
        })();
        lifecycle.close();
        // A normal close or a rendering error both reach the same cleanup.
        panel.orderOut(None);
        if let Some(bar) = glass_toolbar.as_ref() {
            bar.detach();
        }
        drop(glass_toolbar);
        // Release the Cocoa input context before its retained NSView and
        // before the WGPU Surface is destroyed.
        ime_bridge.set_active(false);
        drop(ime_bridge);
        let selection_reply = selected_display.map(|display| {
            let selection = if let Some(rect) = egui_ui.selection_result() {
                NativeSelectionReply::Selected { display, rect }
            } else {
                NativeSelectionReply::Cancelled { display }
            };
            if matches!(selection, NativeSelectionReply::Selected { .. })
                && !egui_ui.committed_native_marks().is_empty()
            {
                NativeEditReply::Annotated {
                    selection,
                    pixels: source_pixels.expect("selection always has source pixels"),
                    marks: egui_ui.committed_native_marks().to_vec(),
                }
            } else {
                NativeEditReply::Legacy(selection)
            }
        });
        drop(egui_ui);
        drop(queue);
        drop(device);
        drop(surface);
        drop(content_view);
        drop(panel);
        result?;
        if let Some(reply) = selection_reply {
            // stdout is exclusively PPANN001 or legacy PPSEL001; diagnostic
            // output must never contaminate this one-shot child reply.
            reply.write_to(&mut std::io::stdout().lock())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_accepts_only_explicit_safe_and_interactive_modes() {
        assert_eq!(ProbeCommand::parse(&["--check"]), Ok(ProbeCommand::Check));
        assert_eq!(ProbeCommand::parse(&["--show"]), Ok(ProbeCommand::Show));
        assert_eq!(
            ProbeCommand::parse(&["--snapshot"]),
            Ok(ProbeCommand::Snapshot)
        );
        assert_eq!(ProbeCommand::parse(&["--select"]), Ok(ProbeCommand::Select));
        assert!(ProbeCommand::parse(&["--select", "--snapshot"]).is_err());
        assert!(ProbeCommand::parse(&[]).is_err());
        assert!(ProbeCommand::parse(&["--show", "--check"]).is_err());
        assert!(ProbeCommand::parse(&["--unknown"]).is_err());
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn egui_controls_emit_shapes_without_a_gpu_or_window() {
        let ctx = egui::Context::default();
        let mut controls = panel_egui::ProbeControls::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(640.0, 360.0),
            )),
            ..Default::default()
        };
        let output = ctx.run_ui(input, |ui| controls.ui(ui));
        assert!(
            !output.shapes.is_empty(),
            "egui controls should generate drawable shapes without Metal"
        );
        // This test deliberately has no GPU renderer; explicitly discard
        // the generated font texture updates rather than leaking them.
        output.drop_without_applying_deltas();
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn native_snapshot_texture_produces_egui_shapes_without_gpu_or_window() {
        let frame = shot_core::RgbaFrame::new(2, 2, vec![255; 16]).unwrap();
        let ctx = egui::Context::default();
        let texture = panel_egui::snapshot_texture(&ctx, &frame);
        assert_eq!(texture.size(), [2, 2]);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(640.0, 360.0),
            )),
            ..Default::default()
        };
        let output = ctx.run_ui(input, |ui| {
            panel_egui::show_snapshot(ui, &texture);
        });
        assert!(!output.shapes.is_empty());
        output.drop_without_applying_deltas();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_selection_overlay_paints_frozen_pixels_with_drag_surface() {
        use shot_core::display::OverlayPlacement;
        use shot_core::platform::DisplayId;
        let placement =
            OverlayPlacement::new(DisplayId(17), (-100.0, -20.0), (100.0, 80.0)).unwrap();
        let mut selection =
            shot_core::native_selection_edit::NativeSelectionEdit::new(placement, (200, 160))
                .unwrap();
        let ctx = egui::Context::default();
        let frame = shot_core::RgbaFrame::new(2, 2, vec![255; 16]).unwrap();
        let texture = panel_egui::snapshot_texture(&ctx, &frame);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(100.0, 80.0),
            )),
            ..Default::default()
        };
        let mut selected = None;
        let mut markup =
            shot_core::native_markup::NativeMarkup::new(placement, (200, 160)).unwrap();
        let mut text = String::new();
        let output = ctx.run_ui(input, |ui| {
            panel_egui::show_native_selection(
                ui,
                &texture,
                &mut selection,
                &mut markup,
                &mut text,
                &mut selected,
            );
        });
        assert!(!output.shapes.is_empty());
        assert!(selected.is_none());
        output.drop_without_applying_deltas();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn preview_theme_switches_headless_egui_visuals() {
        use shot_core::glass_toolbar::PreviewTheme;
        let ctx = egui::Context::default();
        panel_egui::apply_preview_theme(&ctx, PreviewTheme::Light);
        assert_eq!(ctx.theme(), egui::Theme::Light);
        panel_egui::apply_preview_theme(&ctx, PreviewTheme::Dark);
        assert_eq!(ctx.theme(), egui::Theme::Dark);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_selection_action_requires_committed_region_and_explicit_confirm() {
        use panel_egui::{SelectionAction, SelectionDecision, resolve_selection_action};
        use shot_core::RectPx;
        use shot_core::display::OverlayPlacement;
        use shot_core::native_selection_edit::NativeSelectionEdit;
        use shot_core::platform::DisplayId;

        let placement =
            OverlayPlacement::new(DisplayId(6), (-1440.0, -60.0), (100.0, 80.0)).unwrap();
        let mut selection = NativeSelectionEdit::new(placement, (200, 160)).unwrap();
        assert_eq!(
            resolve_selection_action(&selection, SelectionAction::Confirm),
            None
        );
        assert_eq!(
            resolve_selection_action(&selection, SelectionAction::Cancel),
            Some(SelectionDecision::Cancelled)
        );
        selection.begin((10.0, 20.0));
        selection.update((35.0, 50.0));
        assert_eq!(
            resolve_selection_action(&selection, SelectionAction::Confirm),
            None,
            "must not confirm a live/uncommitted drag"
        );
        selection.end((35.0, 50.0));
        assert_eq!(
            resolve_selection_action(&selection, SelectionAction::Confirm),
            Some(SelectionDecision::Confirmed(RectPx {
                x: 20,
                y: 40,
                width: 50,
                height: 60,
            }))
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_ime_changes_map_to_single_egui_events() {
        use shot_core::ime::ImeChange;
        let preedit = panel_egui::egui_event_for_ime_change(ImeChange::Preedit {
            text: "你".into(),
            active_range_chars: Some(1..1),
        });
        assert!(matches!(
            preedit,
            Some(egui::Event::Ime(egui::ImeEvent::Preedit { .. }))
        ));
        let commit = panel_egui::egui_event_for_ime_change(ImeChange::Commit {
            text: "你".into(),
            was_marked: true,
        });
        assert!(matches!(
            commit,
            Some(egui::Event::Ime(egui::ImeEvent::Commit(ref text))) if text == "你"
        ));
        let ascii = panel_egui::egui_event_for_ime_change(ImeChange::Commit {
            text: "abc".into(),
            was_marked: false,
        });
        assert!(matches!(ascii, Some(egui::Event::Text(ref text)) if text == "abc"));
        assert!(
            panel_egui::egui_event_for_ime_change(ImeChange::Commit {
                text: String::new(),
                was_marked: false,
            })
            .is_none()
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_text_input_exports_context_and_composition_event_queue() {
        let _key_handler: fn(&ime::ProbeImeBridge, &objc2_app_kit::NSEvent) -> bool =
            ime::ProbeImeBridge::handle_key;
        let _drain: fn(&ime::ProbeImeBridge) -> Vec<shot_core::ime::ImeChange> =
            ime::ProbeImeBridge::drain;
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_glass_selection_buttons_and_markup_overlay_have_headless_contracts() {
        use shot_core::display::OverlayPlacement;
        use shot_core::native_markup::{NativeMarkup, NativeMarkupTool};
        use shot_core::platform::DisplayId;

        let _new: fn(
            objc2::MainThreadMarker,
            &objc2_app_kit::NSView,
        ) -> glass_toolbar::ProbeGlassToolbar = glass_toolbar::ProbeGlassToolbar::new_selection;
        let _choose: fn(&glass_toolbar::ProbeGlassToolbar, NativeMarkupTool) =
            glass_toolbar::ProbeGlassToolbar::set_markup_tool;

        let placement = OverlayPlacement::new(DisplayId(4), (-200.0, 0.0), (100.0, 80.0)).unwrap();
        let mut markup = NativeMarkup::new(placement, (200, 160)).unwrap();
        markup.set_tool(NativeMarkupTool::Arrow);
        assert!(markup.begin((10.0, 20.0)));
        assert!(markup.end((30.0, 40.0)));
        let ctx = egui::Context::default();
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            panel_egui::paint_native_marks(ui, egui::Pos2::ZERO, &markup);
        });
        assert!(!output.shapes.is_empty());
        output.drop_without_applying_deltas();
        assert!(markup.undo());
        assert_eq!(markup.visible().len(), 0);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_glass_controls_provide_action_queue_contract() {
        let _callback: fn(
            &glass_toolbar::ProbeGlassToolbar,
        ) -> Option<shot_core::glass_toolbar::GlassToolbarAction> =
            glass_toolbar::ProbeGlassToolbar::take_action;
    }
}
