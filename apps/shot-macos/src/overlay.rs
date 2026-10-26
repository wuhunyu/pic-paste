//! A native eframe/winit macOS window using the selected SCDisplay logical frame.
//! Unlike NSWindow fullscreen this does not create a new Spaces fullscreen space.
//! A dedicated AppKit NSPanel and real mixed-DPI device acceptance are future work.
use eframe::egui;
use shot_core::display::OverlayPlacement;

pub fn capture_builder(placement: OverlayPlacement) -> egui::ViewportBuilder {
    let (left, top) = placement.origin_pt();
    let (width, height) = placement.size_pt();
    egui::ViewportBuilder::default()
        .with_title("PicPaste · 截图")
        .with_position(egui::pos2(left, top))
        .with_inner_size(egui::vec2(width, height))
        .with_decorations(false)
        .with_resizable(false)
        .with_fullscreen(false)
        .with_always_on_top()
        .with_has_shadow(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shot_core::platform::DisplayId;

    #[test]
    fn floating_capture_window_uses_target_monitor_without_entering_fullscreen_space() {
        let placement =
            OverlayPlacement::new(DisplayId(7), (-1440.0, 0.0), (1440.0, 900.0)).unwrap();
        let builder = capture_builder(placement);
        assert_eq!(builder.position, Some(egui::pos2(-1440.0, 0.0)));
        assert_eq!(builder.inner_size, Some(egui::vec2(1440.0, 900.0)));
        assert_eq!(builder.fullscreen, Some(false));
        assert_eq!(builder.decorations, Some(false));
        assert_eq!(builder.resizable, Some(false));
        assert_eq!(builder.has_shadow, Some(false));
    }
}
