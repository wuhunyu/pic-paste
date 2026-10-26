# macOS Display Selection & Geometry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Begin Issue #5 with deterministic multi-display selection, correct per-display physical-pixel mappings, and a user-accessible capture action while preserving the existing macOS screenshot workflow.

**Architecture:** Keep display selection and mapping as tested pure Rust in `shot-core::display`. Enumerate ScreenCaptureKit displays only on the screenshot worker thread. Capture an explicit display ID via `ScreenCapture::capture_display` instead of assuming the first discovered screen is the only option. The eframe editor remains a preview; a native AppKit overlay and full Spaces acceptance are separate follow-on changes under Issue #5.

**Tech Stack:** Rust 2024, egui/eframe 0.37, screencapturekit 11.0, GitHub Actions macos-26 and Ubuntu.

## Global Constraints

- macOS minimum target 14.0; actual runtime compatibility requires real-machine verification.
- Screenshot and all file exports remain PNG only; no network requests, Electron, or WebView.
- The screen capture / SCShareableContent enumeration must remain off the UI event thread.
- No claims of mixed-DPI correct native overlay before AppKit/Spaces tests.
- Existing 170 ms hide-then-capture behavior remains until a native exclusion strategy is tested.
- Run `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo build --release -p shot-macos --locked`.
- No real-device validation is available on CI; record the limitation.

---

### Task 1: Deterministic display choice and local pixel mapping

**Files:**
- Create: `crates/shot-core/src/display.rs`
- Modify: `crates/shot-core/src/lib.rs` (export the new module)
- Test: `crates/shot-core/tests/display.rs`

**Interfaces:**
- `DisplaySelection::current() -> Option<DisplayId>`
- `DisplaySelection::resolve(&mut self, &[DisplayId]) -> Result<DisplayId, PlatformError>`
- `DisplaySelection::advance(&mut self, &[DisplayId]) -> Result<DisplayId, PlatformError>`
- `DisplayMapping::new((f64,f64),(f64,f64),(u32,u32)) -> Result<Self, PlatformError>`
- `DisplayMapping::pixel_at((f64,f64)) -> Option<(u32,u32)>`

- [x] **Step 1: Write the failing test** — `tests/display.rs` tests monitor enumeration, cable removal, wrapping, negative origins, mixed pixel scale and invalid geometry.
- [ ] **Step 2: Run red** — `cargo test -p shot-core --test display --locked`; expected unresolved `shot_core::display` until implementation.
- [ ] **Step 3: Implement minimal module** — validate finite positive logical frame; map points by subtracting logical origin and multiplying by pixel-width/height ratios. Do not derive a global physical origin by multiplying screen origin by one display's scale.
- [ ] **Step 4: Run green** — same test; expect 6 tests passing, plus all core tests and Clippy.
- [ ] **Step 5: Commit** — `feat(core): select capture displays and map native pixels`.

### Task 2: Wire selection into the macOS screenshot workflow

**Files:**
- Modify: `apps/shot-macos/src/app.rs`
- Modify: `README.md`
- Test: `crates/shot-core/tests/display.rs`

**Interfaces:**
- Screenshot worker receives `DisplaySelection` and `advance: bool`.
- Worker returns selected display in `WorkerResult::Captured` upon success.
- UI displays selected display ID and a deliberate `截取下一块屏幕` action; standard hotkey captures the same selected display.

- [ ] **Step 1: Add selection tests** — ensure advance cycles, a missing display reselects safely, and an empty discovery list does not corrupt state.
- [ ] **Step 2: Verify red** — compile/test against the interface added by the test.
- [ ] **Step 3: Integrate** — put enumeration and ScreenCaptureKit calls on the worker, maintain the selection only on successful captures, and show an explicit error if no display is available.
- [ ] **Step 4: Verify green** — macOS full CI (core + macOS release + helper IPC + Lite/Full packaging).
- [ ] **Step 5: Commit** — `feat(macos): capture chosen display and cycle monitors`.

### Task 3: Remaining Issue #5 acceptance sequence (independently reviewed)

**Files:**
- Create: `crates/shot-platform/src/macos/window.rs`, `crates/shot-platform/src/macos/display.rs`, `apps/shot-macos/src/overlay.rs`
- Modify: `apps/shot-macos/src/app.rs`, `apps/shot-macos/src/main.rs`, `apps/shot-macos/src/pin.rs`
- Test: macOS integration tests requiring Screen Recording permission and multiple screens.

**Interfaces:** One native overlay window per target display (logical-point frame); screenshot texture and selection remain display-local physical pixels. AppKit windows, if adopted, are instantiated and manipulated only on the main thread.

- [ ] **Step 1:** Write coordinate/frame-to-pixel and capture-state tests for a secondary negative-origin, 2× Retina monitor and stale capture results.
- [ ] **Step 2:** Run red; verify test fails for missing native overlay placement.
- [ ] **Step 3:** Create the AppKit/eframe bridge after checking the exact 0.37 APIs on macOS, never invent Cocoa methods.
- [ ] **Step 4:** Run macOS CI and real-device Screen Recording, Spaces, mixed-DPI and passthrough tests; measure capture P95 against 200 ms.
- [ ] **Step 5:** Commit verified overlay/window changes and update Issue #5. Do not mark Issue #5 done until real-device gates pass.
