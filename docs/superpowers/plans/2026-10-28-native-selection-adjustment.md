# Native NSPanel Selection Adjustment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users refine a native NSPanel screenshot selection with eight draggable handles or move the selected rectangle before explicitly confirming or cancelling the result.

**Architecture:** Keep the already-merged `PPSNAP01` input and `PPSEL001` output unchanged. A new pure `shot-core` edit-state module manages the selection in capture-local physical pixels, exposed to a feature-gated egui overlay on the separate AppKit NSPanel. A release commits the tentative geometry inside the child; only explicit **确认/Enter** sends the selected rect to the existing eframe editor. **Esc/取消** sends the existing cancellation packet. No screenshot bytes are modified.

**Tech Stack:** Rust 2024; shot-core; egui 0.37/WGPU 30; AppKit NSPanel via objc2 0.3; GitHub Actions macOS and Linux.

## Global Constraints

- Never enable native NSPanel in the default `shot-macos` build. `--features native-panel-probe` and the existing explicit `NSPanel 原生框选（实验）` trigger remain mandatory.
- Preserve the existing one-shot `PPSEL001` (33 bytes) validated stdout response, `PPSNAP01` bounded RGBA stdin, display ID, current capture revision validation, and separate helper process ownership.
- All coordinates exposed to the parent are display-local physical source pixels; never multiply the global desktop logical origin by Retina backing scale. Mouse gesture coordinates use panel-local logical points.
- Every selected region is positive-area and fully within the source image. Eight handle drags must keep the opposite side fixed and clamp to a one-physical-pixel minimum without crossing/flipping.
- Moving must preserve exact pixel dimensions and clamp within source bounds, even near opposite display edges. Floating-point nonfinite/malformed coordinates must fail closed.
- Keep the native screenshot preview immutable and unchanged after editing. The egui controls affect only selection state, not source pixels; actual Liquid Glass and native annotation toolbar parity are separate future work.
- Escape and close cancel; drag release alone must **not** emit the result. Explicit confirmation is necessary. No stale/partial result may overwrite the parent editor.
- macOS real GUI (macOS 14–26, multiple screens/Retina, focus, Spaces, TCC, frame rate, accessibility) remains a **device-only** acceptance gate; no GitHub Actions headless test claims to prove it.

---

### Task 1: Pure physical-pixel selection editor (TDD)

**Files**
- Create: `crates/shot-core/tests/native_selection_edit.rs`
- Create: `crates/shot-core/src/native_selection_edit.rs`
- Modify: `crates/shot-core/src/lib.rs`

**Interfaces**
- `NativeSelectionEdit::new(placement: OverlayPlacement, pixels:(u32,u32))->Result<Self,String>`
- `begin(&mut self, pt:(f32,f32))->Option<SelectionGrip>`; `update(&mut self, pt:(f32,f32))`; `end(&mut self, pt:(f32,f32))->Option<RectPx>`; `abort(&mut self)`; `reset(&mut self)`.
- `confirmed(&self)->Option<RectPx>` returns only committed, currently non-dragged positive-area regions.
- `current_rect`, `preview_rect_points`, `grip_centers_points`, `hit_test` return deterministic display-local geometry.

- [ ] RED: Write tests first for initial/renewed drag, 1×/2×, all eight grip locations, west/north/east/south edges, opposite side anchoring, inwards cross-edge clamping, move and bound overflow, click-without-drag, abort and confirmed-before-end, nonfinite point rejection. Run core CI to witness missing module error (not rustfmt failure).
- [ ] GREEN: Implement a single `RectPx` model with a temporary `Gesture { anchor, point, original, grip }`, applying snap-to-pixel rounding and positive-area clamp; do not change the `NativeSelection` legacy one-shot module or the `NativeSelectionReply` wire format.
- [ ] Run Linux CI `cargo fmt --all -- --check`, `cargo test -p shot-core --test native_selection_edit --locked`, `cargo clippy -p shot-core --all-targets --locked -- -D warnings` until green; commit.

### Task 2: Native egui eight-grip overlay and explicit confirmation (TDD)

**Files**
- Modify: `apps/shot-macos/src/bin/ns-panel-probe/panel_egui.rs`
- Modify: `apps/shot-macos/src/bin/ns-panel-probe.rs`

**Interfaces**
- Change the `--select` path only to accept `NativeSelectionEdit` instead of one-shot `NativeSelection`.
- `ProbeEgui::confirm_selection() -> bool` records one committed selected rectangle and requests native panel closure.
- `ProbeEgui::selection_finished() -> bool` handles both Confirm and Cancel without emitting a selected rectangle on drag release; cancellation remains `PPSEL001` status 0.
- `show_native_selection` draws the 8 visible handles, selection geometry, a small floating egui **确认/取消** control surface and the current pixel size; no drawing into the source screenshot texture.

- [ ] RED: Add macOS headless tests for no completion after first drag, confirmation without an area being rejected, confirm once a committed selection exists, and explicit Cancel; verify missing APIs cause expected failures.
- [ ] GREEN: Integrate model in egui's `drag_started/dragged/drag_stopped` pipeline. Preserve mouse image freeze, pointer routing, platform IME and native Metal lifecycle. Handle Enter only when a committed positive-area selection exists; cancel on Escape, close, or UI cancel. Return exactly one existing reply.
- [ ] Run macOS feature tests `cargo test -p shot-macos --features native-panel-probe --bin ns-panel-probe --locked` and strict workspace lints until green.

### Task 3: Scope documentation and CI merge gate

**Files**
- Modify: `README.md`, `docs/STATUS.md`, `docs/REMAINING.md`, `docs/TASKS.md`, `docs/macos-window-strategy.md`

- [ ] Document explicit build and user flow: screenshot > NSPanel 原生框选（实验） > draw > move/resize via eight grips > Confirm or Enter > existing eframe annotation editor, and Escape/Cancel.
- [ ] Confirm complete PR-head CI: Linux core, macOS native helper, all workspace tests, strict Clippy, Release, Swift Vision OCR IPC and Lite/Full/Add-on developer packages; physical UI still unverified.
- [ ] Only squash merge into `main` if all checks pass, then record the merge and real-device validation gaps on Issue #5.

**Acceptance:** The feature-gated NSPanel selector supports multiple local edits before sending a single source-resolution physical-pixel result. The stable eframe selection, annotation, PNG, pins, long screenshot and OCR paths are unchanged. Further native annotation tools and macOS 26 Liquid Glass editing chrome remain their own separately verifiable milestones.
