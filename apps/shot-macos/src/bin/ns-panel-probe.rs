//! Explicitly invoked AppKit NSPanel + WGPU Metal smoke test.
//!
//! This binary is gated by `native-panel-probe`, and never replaces the
//! production eframe/winit screenshot or pin window backend.
use shot_core::native_panel::PanelSurfaceSize;

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
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSApplication, NSBackingStoreType, NSEventMask, NSPanel, NSWindowStyleMask,
    };
    use objc2_foundation::{
        NSDate, NSDefaultRunLoopMode, NSPoint, NSRect, NSSize, ns_string,
    };
    use raw_window_handle::{AppKitWindowHandle, RawWindowHandle};
    use shot_core::native_panel::{PanelSurfaceLifecycle, PanelSurfaceUpdate};
    use std::ptr::NonNull;

    // All WGPU GPU texture locals die before returning; a Surface must be
    // dropped before its NSView/NSPanel owners can be released.
    fn render_frame(
        surface: &wgpu::Surface<'_>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), String> {
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            status => return Err(format!("acquire NSPanel Metal frame: {status:?}")),
        };
        let texture_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("PicPaste NSPanel probe clear"),
        });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("NSPanel Metal render pass"),
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
            });
        }
        queue.submit([encoder.finish()]);
        queue.present(frame);
        Ok(())
    }

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
            NSWindowStyleMask::Titled
                | NSWindowStyleMask::Closable
                | NSWindowStyleMask::Resizable,
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
        let content_view = panel
            .contentView()
            .ok_or("AppKit NSPanel has no content NSView")?;
        content_view.setWantsLayer(true);

        panel.center();
        application.finishLaunching();
        panel.makeKeyAndOrderFront(None);
        application.activate();

        let bounds = content_view.bounds();
        let size = PanelSurfaceSize::from_points(
            bounds.size.width,
            bounds.size.height,
            panel.backingScaleFactor(),
        )
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
        render_frame(&surface, &device, &queue)?;

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
                    application.sendEvent(&event);
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
                match lifecycle
                    .observe(
                        view_bounds.size.width,
                        view_bounds.size.height,
                        panel.backingScaleFactor(),
                    )
                    .map_err(|error| format!("invalid resized Metal surface: {error:?}"))?
                {
                    PanelSurfaceUpdate::Reconfigure(next) => {
                        config.width = next.width;
                        config.height = next.height;
                        surface.configure(&device, &config);
                        render_frame(&surface, &device, &queue)?;
                    }
                    PanelSurfaceUpdate::Unchanged | PanelSurfaceUpdate::Suspended => {}
                    PanelSurfaceUpdate::Closed => break,
                }
            }
            Ok(())
        })();
        lifecycle.close();
        // A normal close or a rendering error both reach the same cleanup.
        panel.orderOut(None);
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
}
