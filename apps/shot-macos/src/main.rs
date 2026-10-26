#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod hotkey;
#[cfg(target_os = "macos")]
mod menu;
#[cfg(target_os = "macos")]
mod pin;
#[cfg(target_os = "macos")]
mod scroll;

#[cfg(target_os = "macos")]
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("PicPaste")
            .with_inner_size([760.0, 660.0])
            .with_min_inner_size([480.0, 380.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "PicPaste",
        options,
        Box::new(|creation_context| Ok(Box::new(app::PicPasteApp::new(creation_context)))),
    )
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("PicPaste native UI currently requires macOS 14 or later");
}
