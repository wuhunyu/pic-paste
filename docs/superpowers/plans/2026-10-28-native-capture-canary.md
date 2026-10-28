# Opt-in Native Capture Canary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the already implemented experimental NSPanel selection/annotation helper into a guarded *automatic* capture handoff for development builds, with a reliable return to the existing eframe editor and a reproducible canary package.

**Architecture:** Do not replace the existing renderer. The parent owns ScreenCaptureKit capture, revision checks, AnnotationDocument, PNG-only save/copy/pins and OCR. An opt-in feature/build switch starts the separate AppKit helper *after* a successful immutable capture, temporarily suppresses the normal fullscreen eframe editor, validates a single PPSEL001/PPANN001 reply, and restores legacy UI on any error. Cancelling the automatically launched native editor closes the capture, whereas cancelling the existing manual probe still leaves the legacy editor unchanged. Native GUI validation must be collected on a real Mac.

**Tech Stack:** Rust 2024, shot-core, macOS eframe/winit + native AppKit NSPanel/WGPU Metal, bash, GitHub Actions macos-26, Python acceptance scripts.

## Global Constraints

- macOS 14+ deployment target, macOS 26 system Liquid Glass with older/accessibility fallback.
- No Electron/WebView, no online dependencies at runtime; exported files are PNG-only with Alpha.
- Existing screenshot capture, normal eframe editing, pins, scroll screenshot and OCR must remain functional without the optional helper.
- Default UI and release ZIPs remain unchanged. Only `--features native-panel-probe` adds the canary UI; package with `PICPASTE_NATIVE_CANARY=1` for a deliberately labeled development ZIP.
- No false device acceptance: CI passes headless checks only. Issue #5 and #10 remain OPEN.
- Pass existing Linux core and full macOS workspace, strict Clippy, release and Lite/Full/Add-on package checks.

---

### Task 1: Test the reversible handoff state machine (RED/GREEN)

**Files:**
- Create: `crates/shot-core/tests/native_capture_canary.rs`
- Create: `crates/shot-core/src/native_capture_canary.rs`
- Modify: `crates/shot-core/src/lib.rs`

**Interfaces:**
- `CanaryPhase::begin(opted_in: bool, started_id: Option<u64>) -> CanaryPhase`
- `CanaryPhase::hides_legacy(self) -> bool`
- `CanaryPhase::complete(&mut self, received_id: u64, reply: CanaryReply) -> CanaryTransition`
- `CanaryPhase::reset(&mut self)`
- `CanaryReply::{Applied, Cancelled, Rejected}`
- `CanaryTransition::{IgnoreStale, ShowLegacy, DismissCapture}`

- [ ] **Step 1: Write failing tests** for default/failed-launch fallback, accepted reply, native cancellation, errors, stale result and reset. Expected: no `shot_core::native_capture_canary` module.
- [ ] **Step 2: Verify RED** by running `cargo test -p shot-core --test native_capture_canary --locked` in CI (expected unresolved import).
- [ ] **Step 3: Implement phase transitions**: only explicit opt-in + a nonzero successful helper request enters native; mismatched IDs leave phase unchanged and ignored; accepted/error replies return legacy; valid native cancellation dismisses capture; reset always restores legacy.
- [ ] **Step 4: Verify GREEN**: `cargo test -p shot-core --test native_capture_canary --locked` succeeds.
- [ ] **Step 5: Commit** the test and state machine.

### Task 2: Integrate automatic canary capture without changing default flow

**Files:**
- Modify: `apps/shot-macos/src/app.rs`
- Test: `apps/shot-macos/src/app.rs` and `crates/shot-core/tests/native_capture_canary.rs`

**Interfaces:**
- `PicPasteApp::launch_native_selection(ctx: &egui::Context) -> Result<ProbeLaunchResult, String>` reuses the existing validated snapshot and revision-aware process launcher.
- `native_canary_enabled: bool` defaults to false on each launch.
- `native_canary_phase: CanaryPhase` tracks whether the legacy viewport should be hidden.

- [ ] **Step 1: RED first**: add test contracts for stale/failed/cancelled automatic handoff to Task 1 state machine and run the PR checks.
- [ ] **Step 2: Implement** one launch path shared by manual button and automatic capture. Right after `pending_frame` is turned into `SelectionView`, launch native only if enabled; keep the eframe editor ready; suppress its fullscreen viewport while the verified helper is active.
- [ ] **Step 3: Implement response handling**: use existing authenticated display/capture/edit revision validation; accepted reply reopens existing eframe editor, automatic native cancellation dismisses capture, malformed/crash results reopen editor unmodified. Stale replies must not dismiss or modify a new capture.
- [ ] **Step 4: Add visible rollback**: settings checkbox to opt in for current run; tray `Show` during active canary stops helper and reveals legacy editor. Start of next capture resets state.
- [ ] **Step 5: Verify** `cargo test -p shot-macos --features native-panel-probe --bin shot-macos --locked`, strict Clippy and default macOS build; commit.

### Task 3: Package optional companion and update acceptance evidence

**Files:**
- Modify: `scripts/package-macos.sh`
- Modify: `.github/workflows/ci.yml`
- Modify: `docs/native-macos-acceptance.md`
- Modify: `docs/STATUS.md`
- Modify: `docs/REMAINING.md`

**Interfaces:**
- `PICPASTE_NATIVE_CANARY=1` requires existing executable `target/release/ns-panel-probe`, copies into app `Contents/MacOS`, ad-hoc signs it before signing bundle, and emits clearly named `*-NATIVE-CANARY-UNSIGNED-DEV.zip`. No flag → byte-for-byte equivalent standard packaging behavior.

- [ ] **Step 1: RED first**: extend macOS CI packaging smoke checks for absent companion fail-closed and canary ZIP presence.
- [ ] **Step 2: Build** `cargo build --release -p shot-macos --features native-panel-probe --bins --locked` *after* standard Lite/Full archives, then assemble separate Lite/Full canary ZIPs.
- [ ] **Step 3: Assert** signed sibling executable exists, signature verifies and artifact uploads include canary archives; standard Lite/Full and OCR Add-on keep working.
- [ ] **Step 4: Document** practical test path and explicit rollback via menu, real device evidence (NSPanel, TCC, Retina/Spaces/IME/PNG/Alpha/P95) and absence of GUI validation.
- [ ] **Step 5: Commit**, inspect PR CI, merge only after all required jobs succeed.

## Acceptance boundary

This delivers an **opt-in staged capture routing mechanism**, not a production NSPanel replacement. Actual GUI, Retina, TCC, keyboard/IME, multi-screen and performance checks remain in #5/#10 until real Mac records are supplied. Long screenshot stays manual; Windows waits for macOS acceptance.
