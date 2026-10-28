//! Explicitly invoked AppKit NSPanel + WGPU Metal smoke test.
//!
//! This binary is gated by `native-panel-probe`, and never replaces the
//! production eframe/winit screenshot or pin window backend.
use shot_core::native_panel::PanelSurfaceSize;

#[cfg(target_os = "macos")]
#[path = "ns-panel-probe/panel_egui.rs"]
mod panel_egui;

#[cfg(target_os = "macos")]
#[path = "ns-panel-probe/glass_toolbar.rs"]
mod glass_toolbar;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProbeCommand {
    Check,
    Show,
}

impl ProbeCommand {
    fn parse(args: &[&str]) -> Result<Self, &'static str> {
        match args {
            ["--check"] => Ok(Self::Check),
            ["--show"] => Ok(Self::Show),
            _ => Err("usage: ns-panel-probe --check | --show"),
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
        ProbeCommand::Show => native::show(),
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
    pub fn show() -> Result<(), String> {
        Err(String::from("interactive NSPanel requires macOS 14+"))
    }
}

#[cfg(target_os = "macos")]
mod native {
    use super::PanelSurfaceSize;
    use super::glass_toolbar::ProbeGlassToolbar;
    use super::panel_egui::{ProbeEgui, ProbeViewport};
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSApplication, NSBackingStoreType, NSEventMask, NSEventType, NSPanel, NSWindowStyleMask,
    };
    use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSPoint, NSRect, NSSize, ns_string};
    use raw_window_handle::{AppKitWindowHandle, RawWindowHandle};
    use shot_core::native_panel::{PanelSurfaceLifecycle, PanelSurfaceUpdate};
    use std::ptr::NonNull;

    /// This opt-in probe must run on AppKit's main thread. It pumps native
    /// events itself so that a close event can return control to Rust and
    /// release the Metal surface before AppKit's retained view and panel.
    pub fn show() -> Result<(), String> {
        let mtm =
            MainThreadMarker::new().ok_or("AppKit NSPanel must be created on the main thread")?;
        let application = NSApplication::sharedApplication(mtm);
        let rect = NSRect::new(NSPoint::new(160.0, 160.0), NSSize::new(640.0, 360.0));
        let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
            NSPanel::alloc(mtm),
            rect,
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Resizable,
            NSBackingStoreType::Buffered,
            false,
        );
        // SAFETY: window ownership remains explicit until the WGPU surface
        // has been dropped; the X close button must not release the NSPanel.
        unsafe { panel.setReleasedWhenClosed(false) };
        panel.setFloatingPanel(true);
        panel.setBecomesKeyOnlyIfNeeded(false);
        panel.setTitle(ns_string!("PicPaste · NSPanel + WGPU Metal 实验"));
        panel.setHasShadow(false);
        panel.setMovableByWindowBackground(false);
        panel.setAcceptsMouseMovedEvents(true);
        let content_view = panel
            .contentView()
            .ok_or("AppKit NSPanel has no content NSView")?;
        content_view.setWantsLayer(true);

        panel.center();
        application.finishLaunching();
        panel.makeKeyAndOrderFront(None);
        application.activate();

        let bounds = content_view.bounds();
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
        let mut egui_ui = ProbeEgui::new(&device, config.format);
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
        let glass_toolbar = ProbeGlassToolbar::new(mtm, &content_view);
        println!("Experimental native toolbar: {:?}", glass_toolbar.material);

        let mut lifecycle = PanelSurfaceLifecycle::new(size);
        println!(
            "NSPanel Metal frame presented. Resize or move between Retina displays; close with X."
        );
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
                    let pointer_down = matches!(
                        event.r#type(),
                        NSEventType::LeftMouseDown
                            | NSEventType::RightMouseDown
                            | NSEventType::OtherMouseDown
                    );
                    if !pointer_down || !glass_toolbar.contains_pointer(&event, &content_view) {
                        egui_ui.on_event(&event, &panel, &content_view);
                    }
                    application.sendEvent(&event);
                    if escape {
                        panel.orderOut(None);
                    }
                }
                egui_ui.set_focus(panel.isKeyWindow());
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
                glass_toolbar.relayout(view_bounds);
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
                }
            }
            Ok(())
        })();
        lifecycle.close();
        // A normal close or a rendering error both reach the same cleanup.
        panel.orderOut(None);
        glass_toolbar.detach();
        drop(glass_toolbar);
        drop(egui_ui);
        drop(queue);
        drop(device);
        drop(surface);
        drop(content_view);
        drop(panel);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_accepts_only_explicit_safe_and_interactive_modes() {
        assert_eq!(ProbeCommand::parse(&["--check"]), Ok(ProbeCommand::Check));
        assert_eq!(ProbeCommand::parse(&["--show"]), Ok(ProbeCommand::Show));
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
    fn native_glass_controls_provide_action_queue_contract() {
        let _callback: fn(
            &glass_toolbar::ProbeGlassToolbar,
        ) -> Option<shot_core::glass_toolbar::GlassToolbarAction> =
            glass_toolbar::ProbeGlassToolbar::take_action;
    }
}
