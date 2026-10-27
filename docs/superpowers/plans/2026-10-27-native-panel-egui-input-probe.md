# Native NSPanel egui Rendering and Basic Input Probe Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render a real interactive egui panel inside the isolated AppKit `NSPanel`/WGPU Metal probe, while preserving production PicPaste windows and hard requirements.

**Architecture:** The existing standalone `ns-panel-probe --show` keeps ownership of its `NSPanel`, `NSView`, Metal surface, and AppKit event pump. A new `panel_egui` module uses `egui::Context`, `egui_wgpu::Renderer`, and a queued `RawInput` to draw an egui counter button and text field. Cocoa events are translated only for the probe window: pointer coordinates go through the real NSView transform into top-left logical points; mouse/scroll, key and printable text are forwarded to egui. The compiler and CI do not execute GUI `--show` in headless macOS runners.

**Tech Stack:** Rust 2024, egui/egui-wgpu 0.37.0, wgpu 30.0.1, objc2-app-kit 0.3.2, macOS 14+, Rust CI on Linux/macOS.

## Global Constraints

- The new code is feature-gated by `native-panel-probe`, built as an explicit companion binary. Do not alter the production eframe/winit screenshot and pin viewports.
- Keep offline-only, PNG-only, no Electron or WebView, manual scrolling, and existing OCR/QR, packaging and Windows sequencing unchanged.
- A real AppKit `NSPanel` and retained `NSView` must outlive WGPU's `Surface`; all Cocoa methods are called on the main thread.
- Reject nonfinite window geometry; do not multiply global desktop coordinates by Retina scale.
- This PR is **not** a complete `NSTextInputClient`/CJK IME bridge: marked-text composition, candidate windows, keyboard layout edge cases, screen-reader/accessibility and cursor shapes remain separate native-device gates.
- A CI compile and headless `--check` cannot prove an interactive Mac display, key focus, mouse click or actual Metal presentation.

---

### Task 1 — Test-first Cocoa-to-egui pointer space

**Files:** `crates/shot-core/src/native_panel.rs`, `crates/shot-core/tests/native_panel_surface.rs`

**Interfaces:** `PanelPointerPoint { x: f32, y: f32 }`; `PanelPointerPoint::from_view(x: f64, y: f64, origin_x: f64, origin_y: f64, width_pt: f64, height_pt: f64, flipped: bool) -> Result<Self, PlatformError>`.

- [ ] RED: add tests for bottom-left conversion `(40,70)` in `200×120` → top-left `(40,50)`, flipped view unchanged, nonzero bounds origin, outside-view drags, and rejection of NaN/Infinity/zero geometry.
- [ ] Verify `cargo fmt --all -- --check` passes, and `cargo test -p shot-core --test native_panel_surface --locked` fails with the intended missing symbol.
- [ ] GREEN: add pure coordinate converter in `shot-core`; inputs stay in logical points and remain valid outside bounds for drag release.
- [ ] Verify Linux core tests and strict Clippy.

### Task 2 — Paint egui UI through Metal, no production changes

**Files:** `apps/shot-macos/Cargo.toml`, `Cargo.lock`, `apps/shot-macos/src/bin/panel_egui.rs`, `apps/shot-macos/src/bin/ns-panel-probe.rs`

**Interfaces:** `ProbeEgui::new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self`; `ProbeEgui::paint(surface, device, queue, size, backing_scale, logical_width, logical_height, focused) -> Result<(), String>`.

- [ ] RED: introduce a macOS-bin test asserting the egui-only widget pass produces nonempty shapes without requiring a GPU, initially failing on the missing `ProbeEgui` type.
- [ ] Add optional pinned `egui = "=0.37.0"` (`default_fonts`) and `egui-wgpu = "=0.37.0"` dependencies behind `native-panel-probe`, and add the already-resolved package names to `shot-macos` in Cargo.lock.
- [ ] Implement `ProbeEgui` to run one egui pass for an interactive counter and single-line ASCII typing probe; upload texture deltas, call renderer update_buffers, issue WGPU render pass with `forget_lifetime()`, present and free removed textures.
- [ ] Paint the first frame, then repaint on captured input or view size/backing-scale changes; preserve 50-ms event wakeup and close/suspend safety.
- [ ] Verify macOS feature tests, headless `--check`, strict Clippy and entire workspace.

### Task 3 — Basic AppKit input bridge; real IME stays deferred

**Files:** `apps/shot-macos/src/bin/panel_egui.rs`, `apps/shot-macos/src/bin/ns-panel-probe.rs`, `docs/macos-window-strategy.md`

- [ ] Deliver Cocoa mouse move, left/right click, drag, scroll and keyDown/keyUp (Escape, Return, Tab, Backspace, arrows) to egui `RawInput` with screen coordinates converted by `NSView.convertPoint_fromView`.
- [ ] Deliver printable direct-key text only when not under Cmd/Ctrl and not interpreting marked text. Forward AppKit events normally so native titlebar close/resize keep working. Support window focus changes and avoid forwarding events originating in another window.
- [ ] Test non-GUI conversion on Linux and sample egui pass without GPU on macOS CI. Actual typing, alternate keyboard layouts, Korean/Japanese/Chinese IME, drag, click and focus require interactive Mac testing.
- [ ] Run latest full Linux+macOS CI; only merge after all required jobs including Release/Lite/Full/Add-on succeed.
- [ ] Document code/CI evidence separately from real-device results; leave Issue #5 open and mark native `NSTextInputClient` IME/production overlay migration outstanding.

## Manual macOS 14+ acceptance

```sh
cargo run --locked -p shot-macos --features native-panel-probe --bin ns-panel-probe -- --check
cargo run --locked -p shot-macos --features native-panel-probe --bin ns-panel-probe -- --show
```

Verify: actual egui controls appear, button counts clicks, pointer follows position after move/resize, scroll works, ASCII field accepts letters and Backspace, Escape and X close do not deadlock; move the panel between 1× and Retina 2× displays; minimize/restore; inspect leaked resources. Record OS, GPU, screen recording and failures. Do NOT mark IME or production replacement complete from these checks.
