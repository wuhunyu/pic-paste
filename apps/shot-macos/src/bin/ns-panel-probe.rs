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
        NSApplication, NSBackingStoreType, NSPanel, NSWindowStyleMask,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize, ns_string};
    use raw_window_handle::{AppKitWindowHandle, RawWindowHandle};
    use std::ptr::NonNull;

    /// Must be called from the process main thread on an interactive Mac.
    /// We retain the native NSPanel + NSView for the entire WGPU surface
    /// lifetime and drop the surface before releasing either owner.
    pub fn show() -> Result<(), String> {
        let mtm = MainThreadMarker::new()
            .ok_or("AppKit NSPanel must be created on the main thread")?;
        let application = NSApplication::sharedApplication(mtm);
        let rect = NSRect::new(
            NSPoint::new(160.0, 160.0),
            NSSize::new(640.0, 360.0),
        );
        // SAFETY: MainThreadMarker proves this executes on the AppKit main
        // thread. This is a newly allocated genuine NSPanel, not a winit
        // NSWindow whose Objective-C class is being changed in place.
        let panel = unsafe {
            NSPanel::initWithContentRect_styleMask_backing_defer(
                NSPanel::alloc(mtm),
                rect,
                NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        // SAFETY: The panel is strongly retained by this function; AppKit
        // must not autorelease it on close while Metal still has a surface.
        unsafe { panel.setReleasedWhenClosed(false) };
        panel.setFloatingPanel(true);
        panel.setBecomesKeyOnlyIfNeeded(false);
        panel.setTitle(ns_string!("PicPaste · NSPanel + WGPU Metal 实验"));
        panel.setHasShadow(false);
        panel.setMovableByWindowBackground(false);
        let content_view = panel
            .contentView()
            .ok_or("AppKit NSPanel has no content NSView")?;

        let scale = panel.backingScaleFactor();
        let size = PanelSurfaceSize::from_points(640.0, 360.0, scale)
            .map_err(|error| format!("invalid Metal size: {error:?}"))?;

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..Default::default()
        });
        let handle = AppKitWindowHandle::new(
            NonNull::from(content_view.as_ref()).cast(),
        );
        // SAFETY: The retained NSView and NSPanel remain alive on the AppKit
        // main thread until *after* this WGPU Surface is dropped. The NSView
        // itself is the raw-window-handle AppKit target, not an NSWindow.
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: None,
                raw_window_handle: RawWindowHandle::AppKit(handle),
            })
        }
        .map_err(|error| format!("create Metal surface: {error}"))?;

        let adapter = pollster::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            },
        ))
        .map_err(|error| format!("request Metal adapter: {error}"))?;
        if adapter.get_info().backend != wgpu::Backend::Metal {
            return Err(String::from("NSPanel probe unexpectedly chose non-Metal backend"));
        }
        let (device, queue) = pollster::block_on(
            adapter.request_device(&wgpu::DeviceDescriptor::default()),
        )
        .map_err(|error| format!("request WGPU device: {error}"))?;
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or("Metal adapter does not support NSPanel surface")?;
        surface.configure(&device, &config);

        panel.center();
        application.finishLaunching();
        panel.makeKeyAndOrderFront(None);
        application.activate();

        let frame = surface
            .get_current_texture()
            .map_err(|error| format!("acquire NSPanel Metal frame: {error}"))?;
        let texture_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("PicPaste NSPanel probe clear"),
            },
        );
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
        frame.present();
        println!("NSPanel + WGPU Metal frame submitted. Close with Ctrl+C in the terminal.");
        // Manual interactive lifecycle only. We do not claim close/IME/focus
        // correctness until the full NSPanel event bridge is implemented.
        application.run();

        // Surface MUST go before its raw NSView/NSPanel owners.
        drop(surface);
        drop(content_view);
        drop(panel);
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
        assert!(ProbeCommand::parse(&[]).is_err());
        assert!(ProbeCommand::parse(&["--show", "--check"]).is_err());
        assert!(ProbeCommand::parse(&["--unknown"]).is_err());
    }
}
