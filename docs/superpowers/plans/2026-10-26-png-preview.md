# Live rounded PNG preview and quick-save (Issue #6)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Show a faithful transparent-rounded PNG preview for the current crop without allocating the full cropped RGBA frame; support Cmd+S quick-save in the screenshot editor.

**Architecture:** Reusable pure `shot-image::preview::png_selection_preview` maps output pixels to source pixels and uses the exact PNG export alpha-rounding function on a bounded thumbnail. The macOS editor caches its texture by capture/edit revision, crop and radius to avoid wasted idle work.

**Tech Stack:** Rust 2024, shot-image, egui/eframe 0.37; no new dependencies.

## Constraints
- Only PNG output, PNG Alpha and native clipboard image types; never file-type selectors.
- Max preview edge 512 px; validate source dimensions and crop boundaries.
- No full-resolution crop allocated just to generate a UI thumbnail.
- Mac Cmd+S triggers the same output pipeline as the Save PNG button.
- Unit tests and CI do not substitute for color profile/clipboard real-device tests.

### Task 1: Preview renderer
**Files:** create `crates/shot-image/src/preview.rs`, `crates/shot-image/tests/png_preview.rs`, modify `crates/shot-image/src/lib.rs`.
- [x] Write failing tests for bounded output, crop, alpha, and invalid selection.
- [ ] Confirm RED.
- [ ] Add bounded sampling and alpha rounding; run core tests and Clippy.

### Task 2: Editor integration
**Files:** `apps/shot-macos/src/app.rs`
- [ ] Cache preview by `(capture_revision, edit_revision, selected_rect, radius)`.
- [ ] Display optional folded PNG transparency preview.
- [ ] Add Cmd+S quick-save shortcut and UI label.
- [ ] Run macOS tests/lints/release/OCR IPC and development bundle packaging.
- [ ] Merge passing branch to main; retain Issue #6 real-device tests.
