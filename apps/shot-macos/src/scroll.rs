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

/// Physical display-local target selected before the first capture.
/// Keep it immutable across a middle retake so exclusions are not applied twice.
#[derive(Clone, Copy, Debug)]
pub struct ScrollTarget {
    pub display: DisplayId,
    pub roi: RectPx,
    pub axis: ScrollAxis,
}

/// A trusted, still-editable draft. Temporary strips survive until
/// the user explicitly saves or discards this review.
pub struct ScrollReview {
    pub target: ScrollTarget,
    pub store: TileStore,
    pub edits: EditStack,
    pub preview: RgbaFrame,
    pub partial: bool,
}

impl ScrollReview {
    /// Safely repair a suspect middle segment by removing that segment and
    /// its entire suffix. Never splice later strips without fresh evidence.
    pub fn retake_prefix(&mut self, keep_count: usize) -> Result<RgbaFrame, String> {
        let total = self.store.strip_count();
        if keep_count == 0 || keep_count >= total {
            return Err(String::from("重拍位置无效：必须保留至少第一段并移除一个后续片段"));
        }
        if self.edits.applied_len() != 0 {
            return Err(String::from("先撤销所有标注，再进行几何拼接纠错"));
        }
        // A failed truncation still returns the partially trusted source review.
        self.partial = true;
        self.edits.clear();
        self.store
            .keep_first(keep_count)
            .map_err(|error| format!("无法裁剪不可信后缀：{error}"))?;
        let anchor = self
            .store
            .last_viewport()
            .map_err(|error| format!("无法从已确认的片段恢复原始视口：{error}"))?;
        self.preview = self
            .store
            .preview(720, 900)
            .map_err(|error| format!("无法重建可信前缀预览：{error}"))?;
        Ok(anchor)
    }
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
    /// Unlike a fresh capture, failed retakes return the trusted prefix to UI.
    RetakeFailed { review: ScrollReview, error: String },
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

/// Shared manual worker for a fresh screenshot and for a trusted middle
/// retake. On retake failures, never drop the user's disk-backed prefix.
fn capture_from_anchor(
    mut review: ScrollReview,
    mut previous: RgbaFrame,
    sender: &Sender<ScrollEvent>,
    command: &AtomicU8,
    live_preview: &LatestPreview,
    wake: &egui::Context,
    resumed_from_prefix: bool,
) -> Result<Option<ScrollReview>, (String, ScrollReview)> {
    let mut capturer = MacCapture::new();
    let mut recovery = RecoveryTracker::default();
    let mut partial = review.partial;
    loop {
        match command.load(Ordering::SeqCst) {
            FINISH | CANCEL => break,
            PAUSED => {
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
            _ => return Err((String::from("invalid manual scroll command"), review)),
        }
        thread::sleep(Duration::from_millis(260));
        if command.load(Ordering::SeqCst) != RUNNING {
            continue;
        }
        let next = match capturer.capture_region(review.target.display, review.target.roi) {
            Ok(frame) => frame,
            Err(error) => {
                return Err((format!("连续长图采集失败：{error:?}"), review));
            }
        };
        if command.load(Ordering::SeqCst) != RUNNING {
            continue;
        }
        if next.image.rgba == previous.rgba {
            let _ = recovery.observe(FrameObservation::Unchanged);
            continue;
        }
        if let Some(displacement) =
            match_scroll_frame(&previous, &next.image, review.target.axis)
        {
            match review.store.append(&next.image, displacement) {
                Ok(()) => {
                    previous = next.image;
                    let _ = recovery.observe(FrameObservation::Matched);
                    // Once a trusted positive displacement is re-established,
                    // the old discarded suffix no longer marks this as partial.
                    if resumed_from_prefix {
                        partial = false;
                    }
                    if let Ok(image) = review.store.preview(420, 600) {
                        live_preview.publish(image);
                    }
                    let (width, height) = review.store.canvas();
                    let _ = sender.send(ScrollEvent::Progress {
                        segments: review.store.strip_count(),
                        width,
                        height,
                    });
                    wake.request_repaint();
                }
                Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => {
                    partial = true;
                    break;
                }
                Err(error) => {
                    return Err((format!("写入长图可信片段失败：{error}"), review));
                }
            }
        } else if recovery.observe(FrameObservation::Untrusted) == RecoveryDecision::Pause
            && command
                .compare_exchange(RUNNING, PAUSED, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        {
            let (width, height) = review.store.canvas();
            let _ = sender.send(ScrollEvent::RecoveryNeeded {
                segments: review.store.strip_count(),
                width,
                height,
            });
            wake.request_repaint();
        }
    }

    if command.load(Ordering::SeqCst) == CANCEL && !resumed_from_prefix {
        // Fresh cancellation discards the new draft as before.
        return Ok(None);
    }
    if command.load(Ordering::SeqCst) == CANCEL {
        // A cancelled retake must retain the already-verified prefix.
        partial = true;
    }
    review.partial = partial || recovery.needs_partial_review();
    review.preview = match review.store.preview(720, 900) {
        Ok(image) => image,
        Err(error) => {
            return Err((format!("无法构建长图复核预览：{error}"), review));
        }
    };
    Ok(Some(review))
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
    thread::sleep(Duration::from_millis(270));
    if command.load(Ordering::SeqCst) == CANCEL {
        return Ok(None);
    }
    let mut capturer = MacCapture::new();
    let first = capturer
        .capture_region(display, roi)
        .map_err(|error| format!("initial capture failed: {error:?}"))?;
    let store = TileStore::new(&first.image, axis, &std::env::temp_dir())
        .map_err(|error| format!("cannot start scroll tile store: {error}"))?;
    let preview = store
        .preview(720, 900)
        .map_err(|error| format!("cannot build initial scroll thumbnail: {error}"))?;
    let review = ScrollReview {
        target: ScrollTarget { display, roi, axis },
        store,
        edits: EditStack::default(),
        preview,
        partial: false,
    };
    if let Ok(image) = review.store.preview(420, 600) {
        live_preview.publish(image);
        let (width, height) = review.store.canvas();
        let _ = sender.send(ScrollEvent::Progress {
            segments: review.store.strip_count(),
            width,
            height,
        });
        wake.request_repaint();
    }
    match capture_from_anchor(
        review,
        first.image,
        sender,
        command,
        live_preview,
        wake,
        false,
    ) {
        Ok(result) => Ok(result),
        Err((error, _draft)) => Err(error),
    }
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

/// Continue after correcting a suspect middle segment. The suffix is
/// discarded before sampling, but the prior trusted prefix is never lost
/// to an asynchronous capture error or cancellation.
pub fn launch_retake(
    mut review: ScrollReview,
    keep_count: usize,
    sender: Sender<ScrollEvent>,
    ctx: egui::Context,
) -> ScrollCaptureControl {
    let command = Arc::new(AtomicU8::new(RUNNING));
    let worker_command = Arc::clone(&command);
    let live_preview = LatestPreview::default();
    let worker_preview = live_preview.clone();
    thread::spawn(move || {
        let outcome = if worker_command.load(Ordering::SeqCst) == CANCEL {
            ScrollEvent::Finished(Ok(Some(review)))
        } else {
            match review.retake_prefix(keep_count) {
                Ok(anchor) => {
                    if let Ok(image) = review.store.preview(420, 600) {
                        worker_preview.publish(image);
                        let (width, height) = review.store.canvas();
                        let _ = sender.send(ScrollEvent::Progress {
                            segments: review.store.strip_count(),
                            width,
                            height,
                        });
                        ctx.request_repaint();
                    }
                    // Let the root settings viewport disappear before any
                    // subsequent capture. Never inject a scrolling gesture.
                    thread::sleep(Duration::from_millis(270));
                    match capture_from_anchor(
                        review,
                        anchor,
                        &sender,
                        &worker_command,
                        &worker_preview,
                        &ctx,
                        true,
                    ) {
                        Ok(result) => ScrollEvent::Finished(Ok(result)),
                        Err((error, review)) => ScrollEvent::RetakeFailed { review, error },
                    }
                }
                Err(error) => ScrollEvent::RetakeFailed { review, error },
            }
        };
        let _ = sender.send(outcome);
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
    #[test]
    fn retake_prefix_rejects_active_edits_and_reuses_trusted_pixel_anchor() {
        use shot_image::history::Annotation;
        let temp =
            std::env::temp_dir().join(format!("picpaste-retake-test-{}", std::process::id()));
        std::fs::create_dir_all(&temp).unwrap();
        let source = |offset: u32| {
            let mut bytes = Vec::new();
            for y in offset..offset + 32 {
                for x in 0..24 {
                    let v = ((x * 17 + y * 31) % 251) as u8;
                    bytes.extend_from_slice(&[v, v, v, 255]);
                }
            }
            RgbaFrame::new(24, 32, bytes).unwrap()
        };
        let mut store = TileStore::new(&source(0), ScrollAxis::Vertical, &temp).unwrap();
        store.append(&source(7), 7).unwrap();
        store.append(&source(14), 7).unwrap();
        let mut review = ScrollReview {
            store,
            edits: EditStack::default(),
            preview: source(0),
            partial: false,
            target: ScrollTarget {
                display: DisplayId(42),
                roi: RectPx {
                    x: 100,
                    y: 300,
                    width: 24,
                    height: 32,
                },
                axis: ScrollAxis::Vertical,
            },
        };
        review
            .edits
            .apply(
                &review.store,
                Annotation::Line {
                    from: (1.0, 1.0),
                    to: (4.0, 4.0),
                    rgba: [0, 0, 255, 255],
                    width: 1.0,
                },
            )
            .unwrap();
        assert!(review.retake_prefix(1).is_err());
        assert_eq!(review.store.strip_count(), 3);
        assert!(review.edits.undo());
        let anchor = review.retake_prefix(2).unwrap();
        assert_eq!(anchor.rgba, source(7).rgba);
        assert_eq!(review.store.strip_count(), 2);
        assert_eq!(review.edits.applied_len(), 0);
        assert!(review.partial);
        assert_eq!(review.preview.width, 24);
        assert_eq!(review.preview.height, 39);
        drop(review);
        assert_eq!(std::fs::read_dir(&temp).unwrap().count(), 0);
        std::fs::remove_dir_all(temp).unwrap();
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
