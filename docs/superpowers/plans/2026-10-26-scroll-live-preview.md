# Live scroll preview using a bounded newest-frame mailbox

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Display bounded, continuously updated long screenshot preview during manual scroll capture while letting mouse and wheel events reach the underlying app.

**Architecture:** A single-frame Arc<Mutex<Option<RgbaFrame>>> mailbox replaces intermediate thumbnails, avoiding unbounded preview queues. The capture worker creates an at-most 420x600 disk-backed preview after each trusted stitch, and the UI presents the latest in a small transparent-pointer always-on-top native eframe viewport.

**Tech Stack:** Rust 2024, shot-scroll TileStore, egui/eframe 0.37, macOS 14+.

## Global Constraints
- All images user-exported as PNG, the live preview is transient memory only.
- No full long-image RGBA allocation for preview. Bounded most recent image, no backlog.
- Preview cannot intercept scrolling or focus the capture target.
- All ScreenCaptureKit/tile I/O stays on worker thread.
- Actual macOS window passthrough/Spaces behavior requires real device verification.

### Task 1: Bounded mailbox, test-first
**Files:** create crates/shot-scroll/tests/live_preview.rs, crates/shot-scroll/src/live_preview.rs; update crates/shot-scroll/src/lib.rs.
- [x] Write failing tests for one-shot take, replacing stale frames, shared copies.
- [ ] Verify RED, implement mailbox, run core tests and Clippy.

### Task 2: Producer/consumer and floating preview
**Files:** apps/shot-macos/src/scroll.rs and apps/shot-macos/src/app.rs.
- [ ] Publish initial and each trusted appended tile-store preview with upper 420x600 limit; trigger UI repaint.
- [ ] Render latest in pointer-passthrough nonactivating native viewport while capture is active.
- [ ] Finish/cancel drops viewport and scratch, normal review remains.
- [ ] Run full macOS CI, then merge. Real device acceptance is separate.
