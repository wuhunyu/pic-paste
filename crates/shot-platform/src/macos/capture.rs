use screencapturekit::prelude::*;
use screencapturekit::screenshot_manager::{CGImageExt, SCScreenshotManager};
use screencapturekit::shareable_content::{SCDisplay, SCShareableContentInfo, SCWindow};
use shot_core::display::OverlayPlacement;
use shot_core::platform::{CapturedFrame, DisplayId, PlatformError, ScreenCapture};
use shot_core::window_candidates::{
    WindowBounds, WindowCandidate, candidates_for_capture, is_self_capture_window,
};
use shot_core::{RectPx, RgbaFrame};

/// macOS 14+ ScreenCaptureKit adapter. Capture calls must run on a worker thread,
/// never on the egui/AppKit event loop.
#[derive(Default)]
pub struct MacCapture;

impl MacCapture {
    pub fn new() -> Self {
        Self
    }

    pub fn available_displays() -> Result<Vec<DisplayId>, PlatformError> {
        let content = SCShareableContent::get().map_err(map_capture_error)?;
        Ok(content
            .displays()
            .iter()
            .map(|display| DisplayId(u64::from(display.display_id())))
            .collect())
    }

    /// Capture the first display before showing any selection overlay.
    pub fn capture_primary() -> Result<CapturedFrame, PlatformError> {
        let content = SCShareableContent::get().map_err(map_capture_error)?;
        let display = content
            .displays()
            .into_iter()
            .next()
            .ok_or(PlatformError::Unavailable)?;
        let windows = content.windows();
        Self::capture_native(&display, &windows)
    }

    /// Capture and native window bounds are resolved from the same SCDisplay.
    /// Logical monitor coordinates are not scaled by Retina pixel density.
    pub fn capture_with_placement(
        display: DisplayId,
    ) -> Result<(CapturedFrame, OverlayPlacement, Vec<WindowCandidate>), PlatformError> {
        let content = SCShareableContent::get().map_err(map_capture_error)?;
        let target = content
            .displays()
            .into_iter()
            .find(|candidate| u64::from(candidate.display_id()) == display.0)
            .ok_or(PlatformError::Unavailable)?;
        let frame = target.frame();
        let placement = OverlayPlacement::new(
            display,
            (frame.origin.x, frame.origin.y),
            (frame.size.width, frame.size.height),
        )?;
        // The same SCShareableContent window snapshot drives both capture
        // exclusions and smart window candidates, avoiding inconsistent frames.
        let all_windows = content.windows();
        let capture = Self::capture_native(&target, &all_windows)?;
        let own_pid =
            i32::try_from(std::process::id()).map_err(|_| PlatformError::InvalidInput)?;
        let windows = all_windows
            .iter()
            .filter(|window| {
                !is_self_capture_window(
                    window.owning_application().map(|app| app.process_id()),
                    own_pid,
                )
            })
            .map(|window| {
                let bounds = window.frame();
                WindowBounds {
                    id: u64::from(window.window_id()),
                    origin_pt: (bounds.origin.x, bounds.origin.y),
                    size_pt: (bounds.size.width, bounds.size.height),
                    layer: window.window_layer(),
                    on_screen: window.is_on_screen(),
                }
            })
            .collect::<Vec<_>>();
        let candidates = candidates_for_capture(
            (frame.origin.x, frame.origin.y),
            (frame.size.width, frame.size.height),
            (capture.image.width, capture.image.height),
            &windows,
        );
        Ok((capture, placement, candidates))
    }

    fn capture_native(
        display: &SCDisplay,
        windows: &[SCWindow],
    ) -> Result<CapturedFrame, PlatformError> {
        let own_pid =
            i32::try_from(std::process::id()).map_err(|_| PlatformError::InvalidInput)?;
        let excluded: Vec<&SCWindow> = windows
            .iter()
            .filter(|window| {
                is_self_capture_window(
                    window.owning_application().map(|app| app.process_id()),
                    own_pid,
                )
            })
            .collect();
        // Exclude all our windows including live preview and pins, not only
        // the (usually hidden) editor root. Other apps stay in the screenshot.
        let filter = SCContentFilter::create()
            .with_display(display)
            .with_excluding_windows(&excluded)
            .build()
            .map_err(map_capture_error)?;

        // The filter reports physical pixel dimensions, including Retina scaling.
        // Do not hard-code 1920x1080 or silently resample a 2x Retina image.
        let (width, height) = SCShareableContentInfo::for_filter(&filter)
            .map(|info| info.pixel_size())
            .unwrap_or((display.width(), display.height()));
        if width == 0 || height == 0 {
            return Err(PlatformError::InvalidInput);
        }
        let config = SCStreamConfiguration::new()
            .with_width(width)
            .with_height(height)
            .with_pixel_format(PixelFormat::BGRA)
            .with_shows_cursor(false);
        let screenshot =
            SCScreenshotManager::capture_image(&filter, &config).map_err(map_capture_error)?;
        let actual_width =
            u32::try_from(screenshot.width()).map_err(|_| PlatformError::InvalidInput)?;
        let actual_height =
            u32::try_from(screenshot.height()).map_err(|_| PlatformError::InvalidInput)?;
        let rgba = screenshot.rgba_data().map_err(map_capture_error)?;
        let image = RgbaFrame::new(actual_width, actual_height, rgba).map_err(|error| {
            PlatformError::Internal(format!("invalid capture pixels: {error:?}"))
        })?;

        // SCDisplay.frame uses desktop coordinates in logical points. For now
        // convert using the filter's scale; mixed-DPI global mapping remains a
        // mandatory on-device integration gate, not covered by headless CI.
        let scale = f64::from(filter.point_pixel_scale());
        let frame = display.frame();
        let x = (frame.origin.x * scale).round();
        let y = (frame.origin.y * scale).round();
        if !x.is_finite()
            || !y.is_finite()
            || x < f64::from(i32::MIN)
            || x > f64::from(i32::MAX)
            || y < f64::from(i32::MIN)
            || y > f64::from(i32::MAX)
        {
            return Err(PlatformError::InvalidInput);
        }
        Ok(CapturedFrame {
            display_id: DisplayId(u64::from(display.display_id())),
            origin_px: (x as i32, y as i32),
            image,
        })
    }
}

impl ScreenCapture for MacCapture {
    fn capture_display(&mut self, display: DisplayId) -> Result<CapturedFrame, PlatformError> {
        let content = SCShareableContent::get().map_err(map_capture_error)?;
        let target = content
            .displays()
            .into_iter()
            .find(|entry| u64::from(entry.display_id()) == display.0)
            .ok_or(PlatformError::Unavailable)?;
        let windows = content.windows();
        Self::capture_native(&target, &windows)
    }

    fn capture_region(
        &mut self,
        display: DisplayId,
        region: RectPx,
    ) -> Result<CapturedFrame, PlatformError> {
        let mut full = self.capture_display(display)?;
        // Region is relative to this display's physical pixel image.
        let image =
            shot_image::crop_rgba(&full.image, region).map_err(|_| PlatformError::InvalidInput)?;
        full.origin_px = (
            full.origin_px
                .0
                .checked_add(region.x)
                .ok_or(PlatformError::InvalidInput)?,
            full.origin_px
                .1
                .checked_add(region.y)
                .ok_or(PlatformError::InvalidInput)?,
        );
        full.image = image;
        Ok(full)
    }
}

fn map_capture_error(error: impl std::fmt::Display) -> PlatformError {
    let message = error.to_string();
    let normalized = message.to_ascii_lowercase();
    if normalized.contains("permission")
        || normalized.contains("denied")
        || normalized.contains("authorized")
    {
        PlatformError::PermissionDenied
    } else {
        PlatformError::Internal(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CI has no Screen Recording TCC grant; this probes actual OS behaviour
    /// only when run intentionally on an interactive Mac.
    #[test]
    #[ignore = "requires interactive macOS Screen Recording permission"]
    fn captures_real_display_frame() {
        let shot = MacCapture::capture_primary().unwrap();
        assert!(shot.image.width > 0);
        assert!(shot.image.height > 0);
        assert_eq!(
            shot.image.rgba.len(),
            shot.image.width as usize * shot.image.height as usize * 4,
        );
    }
}
