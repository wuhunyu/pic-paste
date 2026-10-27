# Native NSPanel Close and Resize Lifecycle — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the isolated AppKit NSPanel/WGPU probe exit on panel close, track view resizing and Retina backing-scale changes, reconfigure its Metal drawable, and release the drawable before its NSView/NSPanel owner.

**Architecture:** Keep the current production eframe/winit UI unchanged. Introduce a platform-neutral, test-first surface lifecycle state machine in shot-core. In the opt-in macOS probe, process AppKit events on the main thread with a bounded idle wait rather than entering an unreturning `NSApplication.run()`, check the content view's actual geometry, reconfigure only when physical dimensions change, redraw, and exit when the native panel is no longer visible. The probe is still not the egui/window input bridge.

**Tech Stack:** Rust 2024, shot-core, objc2 0.6.5, objc2-app-kit/foundation 0.3.2, WGPU 30 Metal, macOS 14+, GitHub Actions macos-26.

## Global Constraints

- Normal PicPaste screenshot/pin app and release bundles remain unchanged; feature `native-panel-probe` opt-in.
- No reinterpretation of winit's NSWindow into NSPanel; all native operations stay on AppKit main thread.
- No PNG format changes, network calls, automatic scrolling, or changes to Lite/Full/OCR add-on.
- Do not claim native visible close/resize/Spaces behavior passed until a real interactive Mac tests `--show`.
- A real `NSView` must outlive its WGPU Surface. Core transitions can be tested on Linux.

---

### Task 1: Test-first close/resize/Retina state

**Files**
- Modify: `crates/shot-core/tests/native_panel_surface.rs`
- Modify: `crates/shot-core/src/native_panel.rs`

**Interfaces**
- `PanelSurfaceLifecycle::new(configured: PanelSurfaceSize) -> Self`
- `observe(&mut self, width_pt: f64, height_pt: f64, scale: f64) -> Result<PanelSurfaceUpdate, PlatformError>`
- `close(&mut self) -> PanelSurfaceUpdate`
- `configured_size(&self) -> Option<PanelSurfaceSize>`
- `enum PanelSurfaceUpdate { Unchanged, Reconfigure(PanelSurfaceSize), Suspended, Closed }`

- [x] RED: add unit tests expecting same-size/Retina resize/repeated changes, zero-point suspension/restoration, invalid geometry preserving last good state, and close preventing future reconfiguration.
- [x] Run `cargo test -p shot-core --test native_panel_surface --locked` in GitHub CI and confirm expected **missing API** error.
- [x] GREEN: add state machine to `native_panel.rs`; invalid input is rejected without state mutation; zero-size content suspends only for finite, nonnegative dimensions and valid scale; post-close transitions are inert.
- [x] Verify Linux test + strict Clippy, macOS workspace CI, before moving to Task 2.

### Task 2: Native event pump, redraw and release

**Files**
- Modify: `apps/shot-macos/src/bin/ns-panel-probe.rs`
- Modify: `apps/shot-macos/Cargo.toml`

**Interfaces**
- Add `render_frame(surface: &wgpu::Surface<'_>, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<(), String>`, extracting existing solid-clear render/present logic.
- In `show()`, initialize using actual NSView bounds and backing scale. Create `PanelSurfaceLifecycle` *after* configuring the initial surface and painting the first frame.
- After showing the panel, pump `NSApplication::nextEventMatchingMask_untilDate_inMode_dequeue` in default run-loop mode with a bounded expiry; forward events through `sendEvent`; exit as soon as `panel.isVisible() == false`.
- Observe actual content-view bounds and `panel.backingScaleFactor()` after dispatch. Only `Reconfigure` may call `surface.configure` and render a replacement frame. Suspended/Unchanged must avoid reconfigure; Closed must terminate.
- Add `NSWindowStyleMask::Resizable` for visual test; preserve main-thread window ownership.
- Ensure the event pump exits before dropping queue/device/surface, then release content_view and panel. If polling or drawing fails, propagate the error only *after* cleanup.
- [ ] Update supported AppKit/Foundation Cargo features (`NSEvent`, `NSDate`, `NSRunLoop`, `NSObjCRuntime`) only as needed.
- [ ] Run macOS feature test, `--check`, strict Clippy, Release and Lite/Full/Add-on CI; fix compiler mismatch without weakening tests.

### Task 3: Documentation and review

**Files**
- Modify: `docs/macos-window-strategy.md`, `docs/STATUS.md`, `docs/REMAINING.md`
- Create: `docs/superpowers/plans/2026-10-27-native-panel-lifecycle.md`
- Update: Issue #5 (not closed)

- [ ] Document native close button, resize, 1x/2x monitor movement, repeated resize, minimize/restore, and process-exit device checklist.
- [ ] Record the specific full Linux/macOS CI run and explicit **GUI test not executed** warning.
- [ ] Merge only after latest full PR CI success. Continue to defer actual egui keyboard/mouse/IME and production overlay replacement.

## Acceptance Boundaries

**Automated:** pure resize/close transitions; macOS native calls compile and `--check` stays headless; no change to production app behavior.

**Needs real hardware:** panel appears, X close returns success without Ctrl+C, Metal surface stays visible through resizing, dynamic 1x/2x crossing, minimize/restore, fullscreen Spaces/focus/TCC and actual cleanup. Record a screen capture and macOS/GPU device info in Issue #5 before claiming these gates.
