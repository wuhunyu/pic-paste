//! macOS manual long screenshot worker. Neither file I/O nor ScreenCaptureKit
//! runs on the egui/AppKit event thread.
use eframe::egui;
use shot_core::platform::{DisplayId, ScreenCapture, ScrollAxis};
use shot_core::{RectPx, RgbaFrame};
use shot_platform::macos::MacCapture;
use shot_scroll::stitch::match_scroll_frame;
use shot_scroll::live_preview::LatestPreview;
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

pub struct ScrollCaptureControl {
    command: Arc<AtomicU8>,
    live_preview: LatestPreview,
}

impl ScrollCaptureControl {
    pub fn take_live_preview(&self) -> Option<RgbaFrame> {
        self.live_preview.take()
    }

    pub fn finish(&self) {
        let _ = self
            .command
            .compare_exchange(RUNNING, FINISH, Ordering::SeqCst, Ordering::SeqCst);
    }

    pub fn cancel(&self) {
        self.command.store(CANCEL, Ordering::SeqCst);
    }
}

impl Drop for ScrollCaptureControl {
    fn drop(&mut self) {
        if self.command.load(Ordering::SeqCst) == RUNNING {
            self.cancel();
        }
    }
}

/// A trusted, still-editable draft. Temporary strips survive until
/// the user explicitly saves or discards this review.
pub struct ScrollReview {
    pub store: TileStore,
    pub preview: RgbaFrame,
    pub partial: bool,
}

pub enum ScrollEvent {
    Progress {
        segments: usize,
        width: u32,
        height: u32,
    },
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
pub fn export_review(review: &ScrollReview) -> Result<PathBuf, String> {
    let path = output_path(review.partial)?;
    review
        .store
        .save_png_no_clobber(&path)
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
    let mut mismatches = 0u32;
    let mut partial = false;
    while command.load(Ordering::SeqCst) == RUNNING {
        thread::sleep(Duration::from_millis(260));
        if command.load(Ordering::SeqCst) != RUNNING {
            break;
        }
        let next = capturer
            .capture_region(display, roi)
            .map_err(|error| format!("continuous capture failed: {error:?}"))?;
        if next.image.rgba == previous.rgba {
            // The target has not moved. Do not add duplicate rows/columns.
            continue;
        }
        if let Some(displacement) = match_scroll_frame(&previous, &next.image, axis) {
            match store.append(&next.image, displacement) {
                Ok(()) => {
                    previous = next.image;
                    mismatches = 0;
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
        } else {
            mismatches += 1;
            if mismatches >= 3 {
                // Do not advance the trusted anchor after ambiguous motion.
                partial = true;
                break;
            }
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
        preview,
        partial,
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
