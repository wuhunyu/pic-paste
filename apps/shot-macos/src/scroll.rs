//! macOS manual long screenshot worker. Neither file I/O nor ScreenCaptureKit
//! runs on the egui/AppKit event thread.
use eframe::egui;
use shot_core::platform::{DisplayId, ScreenCapture, ScrollAxis};
use shot_core::{RectPx, RgbaFrame};
use shot_platform::macos::MacCapture;
use shot_scroll::edits::EditStack;
use shot_scroll::live_preview::LatestPreview;
use shot_scroll::recovery::{FrameObservation, RecoveryDecision, RecoveryTracker};
use shot_scroll::stitch::match_scroll_frame;
use shot_scroll::tile_store::TileStore;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const RUNNING: u8 = 0;
const FINISH: u8 = 1;
const CANCEL: u8 = 2;
const PAUSED: u8 = 3;
const RETRY: u8 = 4;

pub struct ScrollCaptureControl {
    command: Arc<AtomicU8>,
    live_preview: LatestPreview,
}

impl ScrollCaptureControl {
    pub fn take_live_preview(&self) -> Option<RgbaFrame> {
        self.live_preview.take()
    }

    /// Finish works both while capturing and while waiting for manual retry.
    /// A previous cancellation must never be overwritten.
    pub fn finish(&self) {
        let mut state = self.command.load(Ordering::SeqCst);
        while matches!(state, RUNNING | PAUSED | RETRY) {
            match self
                .command
                .compare_exchange(state, FINISH, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => return,
                Err(actual) => state = actual,
            }
        }
    }

    pub fn is_paused(&self) -> bool {
        self.command.load(Ordering::SeqCst) == PAUSED
    }

    /// Only an explicit user action can resume sampling. No page scrolling is
    /// ever injected; the user must manually backtrack and scroll slowly.
    pub fn resume(&self) -> bool {
        self.command
            .compare_exchange(PAUSED, RETRY, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    pub fn cancel(&self) {
        self.command.store(CANCEL, Ordering::SeqCst);
    }
}

impl Drop for ScrollCaptureControl {
    fn drop(&mut self) {
        if matches!(
            self.command.load(Ordering::SeqCst),
            RUNNING | PAUSED | RETRY
        ) {
            self.cancel();
        }
    }
}

/// A trusted, still-editable draft. Temporary strips survive until
/// the user explicitly saves or discards this review.
pub struct ScrollReview {
    pub store: TileStore,
    pub edits: EditStack,
    pub preview: RgbaFrame,
    pub partial: bool,
}

pub enum ScrollEvent {
    Progress {
        segments: usize,
        width: u32,
        height: u32,
    },
    RecoveryNeeded {
        segments: usize,
        width: u32,
        height: u32,
    },
    RecoveryResumed,
    Finished(Result<Option<ScrollReview>, String>),
}

fn output_path(partial: bool) -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| String::from("HOME environment variable is missing"))?;
    let root = PathBuf::from(home).join("Pictures").join("PicPaste");
    std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let label = if partial { "partial-" } else { "" };
    Ok(root.join(format!("PicPaste-scroll-{label}{now}.png")))
}

/// Publish an explicitly approved long screenshot. Failed exports retain
/// the source TileStore in the caller, so the user can retry.
pub fn export_review(review: &ScrollReview, radius: u32) -> Result<PathBuf, String> {
    let path = output_path(review.partial)?;
    review
        .store
        .save_png_edited_no_clobber(&path, radius, &review.edits)
        .map_err(|error| format!("scroll PNG export failed: {error}"))?;
    Ok(path)
}

fn capture_loop(
    display: DisplayId,
    roi: RectPx,
    axis: ScrollAxis,
    sender: &Sender<ScrollEvent>,
    command: &AtomicU8,
    live_preview: &LatestPreview,
    wake: &egui::Context,
) -> Result<Option<ScrollReview>, String> {
    // The main viewport is hidden before this delay. A new frame is taken
    // rather than trusting the earlier selection screenshot as the first tile.
    thread::sleep(Duration::from_millis(270));
    if command.load(Ordering::SeqCst) == CANCEL {
        return Ok(None);
    }

    let mut capturer = MacCapture::new();
    let first = capturer
        .capture_region(display, roi)
        .map_err(|error| format!("initial capture failed: {error:?}"))?;
    let mut store = TileStore::new(&first.image, axis, &std::env::temp_dir())
        .map_err(|error| format!("cannot start scroll tile store: {error}"))?;
    let mut previous = first.image;
    // Publish the first trusted region so the user can see a preview before
    // the first wheel event. Its size stays bounded regardless of canvas size.
    if let Ok(image) = store.preview(420, 600) {
        live_preview.publish(image);
        let (width, height) = store.canvas();
        let _ = sender.send(ScrollEvent::Progress {
            segments: store.strip_count(),
            width,
            height,
        });
        wake.request_repaint();
    }
    let mut recovery = RecoveryTracker::default();
    let mut partial = false;
    loop {
        match command.load(Ordering::SeqCst) {
            FINISH | CANCEL => break,
            PAUSED => {
                // Keep trusted frames on disk without taking any screenshots.
                // The user can scroll backward while the preview stays passive.
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            RETRY => {
                if command
                    .compare_exchange(RETRY, RUNNING, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    let _ = recovery.resume();
                    let _ = sender.send(ScrollEvent::RecoveryResumed);
                    wake.request_repaint();
                }
                continue;
            }
            RUNNING => {}
            _ => return Err(String::from("invalid manual scroll command")),
        }
        thread::sleep(Duration::from_millis(260));
        if command.load(Ordering::SeqCst) != RUNNING {
            continue;
        }
        let next = capturer
            .capture_region(display, roi)
            .map_err(|error| format!("continuous capture failed: {error:?}"))?;
        if command.load(Ordering::SeqCst) != RUNNING {
            continue;
        }
        if next.image.rgba == previous.rgba {
            // No motion: neither create duplicate strips nor increase failures.
            let _ = recovery.observe(FrameObservation::Unchanged);
            continue;
        }
        if let Some(displacement) = match_scroll_frame(&previous, &next.image, axis) {
            match store.append(&next.image, displacement) {
                Ok(()) => {
                    previous = next.image;
                    let _ = recovery.observe(FrameObservation::Matched);
                    if let Ok(image) = store.preview(420, 600) {
                        live_preview.publish(image);
                    }
                    let (width, height) = store.canvas();
                    let _ = sender.send(ScrollEvent::Progress {
                        segments: store.strip_count(),
                        width,
                        height,
                    });
                    wake.request_repaint();
                }
                Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => {
                    // Size limit or invalid append: preserve a partial trusted result.
                    partial = true;
                    break;
                }
                Err(error) => return Err(format!("scroll tile write failed: {error}")),
            }
        } else if recovery.observe(FrameObservation::Untrusted) == RecoveryDecision::Pause
            && command
                .compare_exchange(RUNNING, PAUSED, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        {
            // Keep the last trusted frame and all previous strips. The user
            // must backtrack and explicitly request slow manual retry.
            let (width, height) = store.canvas();
            let _ = sender.send(ScrollEvent::RecoveryNeeded {
                segments: store.strip_count(),
                width,
                height,
            });
            wake.request_repaint();
        }
    }
    if command.load(Ordering::SeqCst) == CANCEL {
        // Dropping TileStore removes its private raw strips. No PNG is emitted.
        return Ok(None);
    }
    let preview = store
        .preview(720, 900)
        .map_err(|error| format!("cannot build scroll review thumbnail: {error}"))?;
    Ok(Some(ScrollReview {
        store,
        edits: EditStack::default(),
        preview,
        partial: partial || recovery.needs_partial_review(),
    }))
}

pub fn launch(
    display: DisplayId,
    roi: RectPx,
    axis: ScrollAxis,
    sender: Sender<ScrollEvent>,
    ctx: egui::Context,
) -> ScrollCaptureControl {
    let command = Arc::new(AtomicU8::new(RUNNING));
    let worker_command = Arc::clone(&command);
    let live_preview = LatestPreview::default();
    let worker_preview = live_preview.clone();
    thread::spawn(move || {
        let outcome = capture_loop(
            display,
            roi,
            axis,
            &sender,
            &worker_command,
            &worker_preview,
            &ctx,
        );
        let _ = sender.send(ScrollEvent::Finished(outcome));
        ctx.request_repaint();
    });
    ScrollCaptureControl {
        command,
        live_preview,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_control(state: u8) -> ScrollCaptureControl {
        ScrollCaptureControl {
            command: Arc::new(AtomicU8::new(state)),
            live_preview: LatestPreview::default(),
        }
    }

    #[test]
    fn retry_is_only_available_while_paused() {
        let control = test_control(PAUSED);
        assert!(control.is_paused());
        assert!(control.resume());
        assert_eq!(control.command.load(Ordering::SeqCst), RETRY);
        assert!(!control.resume());
        assert!(!control.is_paused());
    }

    #[test]
    fn paused_capture_can_finish_without_discarding_trusted_segments() {
        let control = test_control(PAUSED);
        control.finish();
        assert_eq!(control.command.load(Ordering::SeqCst), FINISH);
    }

    #[test]
    fn cancel_cannot_be_overwritten_by_finish_or_retry() {
        let control = test_control(PAUSED);
        control.cancel();
        control.finish();
        assert!(!control.resume());
        assert_eq!(control.command.load(Ordering::SeqCst), CANCEL);
    }
}

/// Scale physical corner radius to a bounded preview using its width.
pub fn preview_radius(radius: u32, canvas_width: u32, preview_width: u32) -> u32 {
    if canvas_width == 0 || preview_width == 0 {
        return 0;
    }
    (u64::from(radius) * u64::from(preview_width) / u64::from(canvas_width)) as u32
}

#[cfg(test)]
mod rounded_preview_tests {
    use super::preview_radius;
    #[test]
    fn thumbnail_radius_tracks_physical_pixels_without_upscaling() {
        assert_eq!(preview_radius(0, 100, 100), 0);
        assert_eq!(preview_radius(16, 200, 100), 8);
        assert_eq!(preview_radius(64, 60_000, 60), 0);
        assert_eq!(preview_radius(16, 100, 100), 16);
    }
}
