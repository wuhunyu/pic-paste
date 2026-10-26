# Scroll Review Trusted Prefix Trim — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow a user to discard the untrustworthy tail of a long screenshot from a selected segment, rather than repeatedly deleting only the last segment.

**Architecture:** Reuse `TileStore::undo_last` and on-disk strips to implement `keep_first(count)` without contiguous RGBA allocation. The editor offers an explicit `保留前 N 段` control during post-capture review; a worker applies the trim, regenerates the bounded thumbnail and returns the existing `ScrollEdited` event. Do **not** delete a middle strip while retaining later strips, since that would break image continuity.

**Tech Stack:** Rust 2024, shot-scroll TileStore, egui/eframe 0.37, GitHub Actions macOS 26.

## Global Constraints
- PNG-only file export and 60,000 px / 200 MP canvas limits stay unchanged.
- A trim count outside `[1, strip_count]` errors without mutation; a no-op returns 0 removed.
- Keep files contiguous and maintain the exact row/column stride in vertical and horizontal workflows.
- Remove each discarded raw strip from the private session directory; user-visible PNG is created only by explicit Save.
- An I/O error during a multi-step trim can leave fewer strips than originally requested but must keep the remaining TileStore internally valid.
- Real-device review and the separate interrupted-capture/retry feature remain unverified.

---

### Task 1 — Disk strip prefix trim (test-first)

**Files:** `crates/shot-scroll/tests/tile_store.rs` (modify), `crates/shot-scroll/src/tile_store.rs` (modify).

**Interface:** `pub fn keep_first(&mut self, keep_count: usize) -> io::Result<usize>`.

- [x] Step 1 — Add tests for multi-strip vertical exact rows and PNG, horizontal trim with subsequent append, and invalid/no-op input with file safety.
- [ ] Step 2 — Run RED: `cargo test -p shot-scroll --test tile_store --locked` must fail on missing `keep_first`.
- [ ] Step 3 — Reject invalid keep counts before any mutation; repeatedly call `undo_last()` until `strip_count() == keep_count`; report removed strip count.
- [ ] Step 4 — Run GREEN: core tests, Clippy, PNG test and temporary-file cleanup.
- [ ] Step 5 — Commit.

### Task 2 — Expose safe batch trim in macOS review

**Files:** `apps/shot-macos/src/app.rs` (modify).

**Interface:** `fn trim_scroll_review(&mut self, ctx: &egui::Context, keep_count: usize)` / `scroll_keep_segments: usize` state.

- [ ] Step 1 — Add a segment-count selector beside review buttons with range `1..=store.strip_count()` and a disabled action when no tail would be removed.
- [ ] Step 2 — Use the existing worker result and `TileStore::preview(720,900)` to regenerate the review thumbnail off the UI thread.
- [ ] Step 3 — Update review status to say trusted prefix, not arbitrary middle segment deletion. Keep existing last-segment undo as a separate shortcut.
- [ ] Step 4 — Verify fmt, workspace tests, strict Clippy, macOS release build, Swift OCR IPC and Lite/Full packaging.
- [ ] Step 5 — Merge after complete CI and record the remaining true middle-segment correction, resume and real-device testing under Issue #7.
