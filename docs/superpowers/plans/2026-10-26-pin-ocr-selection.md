# Pin OCR Line Selection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users actively invoke the existing offline Apple Vision OCR helper on a pinned image, select a recognized line by clicking the pin, and copy text; keep moving and mouse passthrough mutually exclusive with OCR selection.

**Architecture:** Reuse `shot_core::pins::PinMode`, `shot_core::input::DragLatch` mode logic, `shot_ocr_protocol::hit_test::hit_line_in_capture`, and `apps/shot-macos/src/ocr.rs`. Keep OCR opt-in and async. The root `PicPasteApp` retains authoritative OCR state for each immutable pin, processes `PinEvent` messages from deferred egui viewports, and rejects replies for closed pins or no-longer-active selection modes. A pure pin-to-image coordinate mapping helper prevents wrong OCR hitboxes when users zoom or resize pins.

**Tech Stack:** Rust 2024, egui/eframe 0.37, local Swift Vision SHOC helper, arboard 3.7.0. No new dependency; no network.

## Global Constraints

- OCR is offline, explicitly user triggered, and unavailable Lite helper never blocks moving, pinning or PNG.
- Recognized line geometry is tied to the pin's immutable source image; no repeat inference on hover/drag.
- Only a line box can be selected with the current Vision provider; **word/character boxes are not to be fabricated**.
- One OCR worker per pin, bounded to existing helper 20s timeout; late worker replies must be ignored when pin is closed/selection cancelled.
- Move, text select and mouse passthrough are mutually exclusive pin modes; Shift+drag in selection mode can temporarily move.
- Clipboard copying happens on an explicit action, never automatically during OCR or pointer hover.
- PNG-only export and existing screenshot editor OCR behavior must remain unchanged.
- macOS physical TCC, Spaces and multi-pin performance remain separate acceptance gates.

---

### Task 1: Pin coordinate mapping (TDD)

**Files:** `crates/shot-core/src/pin_view.rs` (create), `crates/shot-core/src/lib.rs` (modify), `crates/shot-core/tests/pin_view.rs` (create).

**Interface:** `pub struct PinImageView { image_width: u32, image_height: u32, left: f32, top: f32, width: f32, height: f32 }`; `pub fn map_pin_click(view: PinImageView, point: (f32,f32)) -> Option<(f32,f32)>`.

- [x] Write tests for source-pixel placement, retina/zoom, toolbar margin rejection and invalid dimensions.
- [x] Confirm RED with `cargo test -p shot-core --test pin_view --locked` (expected unresolved import).
- [x] Implement bounded finite transform; do not up/downsample pixels.
- [x] Confirm GREEN with core tests and clippy; commit.

### Task 2: Pin mode and event loop

**Files:** `apps/shot-macos/src/pin.rs` (modify), `apps/shot-macos/src/app.rs` (modify).

- [x] Add `PinEvent` (`Recognize`, `Move`, `PickLine`, `CopyLine`) with source pixel positions and monotonically unique pin IDs.
- [x] Replace pin's independent passthrough bool with `PinMode` enum; pin viewport mouse passthrough derives only from `PinMode::Passthrough`, and root recovery switches to Move. Click never initiates drag in TextSelect mode; Shift+drag enables temporary Move.
- [x] Add explicit OCR button inside each pin, one async worker/pin, and isolated `WorkerResult::PinOcr`.
- [x] On successful OCR, store `Arc<OcrDocument>` and line index, draw only selected-line quadrilateral; do not fake missing word boxes.
- [x] Connect `PinEvent::PickLine` to `hit_line_in_capture(doc,(0,0),point)` and explicit copy via `selected_text`; avoid unnecessary repeated OCR.
- [x] Reuse existing copy/error status UI and Lite fallback, ensure unchanged `SelectionView` OCR functionality.

### Task 3: CI, docs and merge

**Files:** `README.md`, `docs/STATUS.md`, `docs/TASKS.md`.

- [x] Verify `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, macOS Release build, Swift helper IPC and Lite/Full package on GitHub Actions.
- [x] Update P2-T2 / P3-T2 to partial, explicitly leave word-level/QR/Add-on and physical multi-monitor tests open.
- [ ] Merge full-green PR and run main CI. Issue #6/#8 update with exact SHA and CI evidence.
