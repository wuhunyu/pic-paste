# Smart window selection for macOS capture

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** In Issue #5, identify selectable window rectangles on a captured display and offer click-to-select, without introducing any new permission requirements.

**Architecture:** On the existing ScreenCaptureKit capture worker, collect SCWindow metadata from the same SCShareableContent snapshot as the selected display. Convert logical global frames into physical display-local pixel rectangles in pure shot-core. The capture editor receives immutable window rectangles; hover previews a window candidate and click selects it, while existing drag/resize remains available.

**Tech Stack:** Rust 2024, shot-core, screencapturekit 11.0, egui/eframe 0.37, macOS 14+.

## Global constraints
- All images saved only as PNG; no Electron/network/WebView.
- Do not trust window metadata, filter non-finite/offscreen/desktop-layer/tiny frames.
- Respect display origin and independent per-screen scale on mixed-DPI monitors.
- Preserve existing drag/resize/magnifier selection.
- CI is code validation, not actual Screen Recording/Spaces acceptance.

### Task 1: Candidate geometry, TDD
**Files:** Create `crates/shot-core/tests/window_candidates.rs`, `crates/shot-core/src/window_candidates.rs`; modify `crates/shot-core/src/lib.rs`.
- [x] Write tests for mixed-DPI secondary display, partially offscreen frames, invalid metadata, layer filtering, duplicate IDs and overlap.
- [ ] Verify expected RED with `cargo test -p shot-core --test window_candidates --locked`.
- [ ] Implement robust finite validated intersection, floor/ceil/clamp mapping and stable overlap order.
- [ ] Verify GREEN via core CI and strict Clippy.

### Task 2: ScreenCaptureKit metadata and selection UX
**Files:** modify `crates/shot-platform/src/macos/capture.rs`, `apps/shot-macos/src/app.rs`.
- [ ] Return capture frame, native overlay position and filtered candidate rectangles as one worker result.
- [ ] Add editor hover outline and click-to-select; preserve freeform drag, OCR and existing handles.
- [ ] Test macOS workspace, release, OCR helper, Lite+Full bundles.
- [ ] Merge passing PR; keep Issue #5 open for AppKit NSPanel and interactive mixed-DPI/Spaces tests.
