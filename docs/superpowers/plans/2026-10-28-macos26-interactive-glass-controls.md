# Interactive macOS Liquid Glass Controls Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the existing opt-in macOS NSPanel Liquid Glass capsule functional through native, accessible AppKit buttons for preview theme switching and safe probe shutdown, without touching captured pixels or production eframe windows.

**Architecture:** Retain one material wrapper chosen by `shot-core::glass_toolbar::select_glass_material` (macOS 26 glass; older vibrancy; Reduce Transparency opaque), but replace its static NSTextField with two native NSButton controls. A main-thread Objective-C target/action object queues commands in a `Cell` while the probe AppKit event loop drains them immediately after `sendEvent`. Changes to preview visuals are performed by `ProbeEgui`; close exits through the existing safe WGPU-before-view teardown. The native target is retained by the toolbar, because NSControl's `target` is weak.

**Tech Stack:** Rust 2024, objc2 0.6.5 / objc2-app-kit 0.3.2, egui-wgpu 0.37, wgpu 30 Metal, macOS 14–26, GitHub Actions macos-26.

## Global Constraints

- Only `apps/shot-macos/src/bin/ns-panel-probe` and pure `shot-core` tests change; normal screenshot/selection/Pin/PNG and window implementations stay unchanged.
- macOS 26 native material, macOS 14–25 visual effect and reduced-transparency opaque view retain the exact behavior established in PR #36.
- Leave the underlying Metal canvas opaque: glass is only for a small bottom-center control layer. Use system-native colors and a single glass parent rather than nested glass.
- Keep AppKit work on the main thread and NSView/WGPU lifetime ordering safe: detach native controls before dropping the WGPU Surface.
- No fake screenshot `Save`/`Copy` actions: this stage demonstrates two real, testable probe actions only.
- `--check` stays headless and starts no AppKit or GPU, so CI cannot establish appearance, click accessibility, button focus, interactive retina behavior, or actual Liquid Glass presentation.

---

### Task 1 — RED/GREEN preview theme contract (cross-platform)

**Files:** `crates/shot-core/tests/glass_toolbar.rs`, `crates/shot-core/src/glass_toolbar.rs`

**Interfaces:** `PreviewTheme::{Dark,Light}`, `PreviewTheme::toggle(self)->Self`, `PreviewTheme::toggle_label(self)->&'static str` (label describes action, not current appearance).

- [ ] RED: add test asserting Dark → Light → Dark and button labels `浅色预览` and `深色预览`. Run `cargo test -p shot-core --test glass_toolbar --locked` in CI and confirm expected missing-symbol compile failure; do not mislabel formatting failures as RED.
- [ ] GREEN: add minimal `PreviewTheme` implementation; run Linux core test, strict Clippy.

### Task 2 — Native actionable toolbar

**Files:** `apps/shot-macos/src/bin/ns-panel-probe/glass_toolbar.rs`, `apps/shot-macos/Cargo.toml`

**Interfaces:**
- `GlassToolbarAction::{ToggleTheme,CloseProbe}` (a shared pure command enum suitable for `Cell<Option<...>>`).
- `ProbeGlassToolbar::take_action(&self)->Option<GlassToolbarAction>`
- `ProbeGlassToolbar::set_preview_theme(&self, theme: PreviewTheme)` updates the theme button label.
- Retain an ObjC `GlassActionTarget` with `#[thread_kind=MainThreadOnly]`; target/action callbacks write pending action, never mutate Metal or NSPanel directly.
- Place two real `NSButton` controls within `NSGlassEffectView.contentView` / legacy content / opaque content (same 236×44-point pill), with semantic visible titles and native keyboard/accessibility behavior.

- [ ] RED: introduce test/usage expecting `take_action` and `set_preview_theme`, making the macOS feature test fail against the static-toolbar API.
- [ ] GREEN: replace the static label, add native buttons and selectors with a retained target; update `objc2-app-kit` feature list with `NSButton` and `NSButtonCell` only as needed.
- [ ] Verify macOS opt-in tests, binary `--check`, strict Clippy, and original default `cargo test --workspace`.

### Task 3 — Connect to existing event loop, docs and gate

**Files:** `apps/shot-macos/src/bin/ns-panel-probe.rs`, `apps/shot-macos/src/bin/ns-panel-probe/panel_egui.rs`, `docs/macos-window-strategy.md`, `docs/REMAINING.md`, `docs/STATUS.md`.

- [ ] RED: add a headless egui test showing preview visuals change Dark→Light→Dark without GPU/window. Reuse `PreviewTheme`; test does not instantiate AppKit.
- [ ] GREEN: add `ProbeEgui::set_preview_theme`, drain native queued actions after `NSApplication.sendEvent`; Toggle changes egui visuals and button's next-action label and requests repaint; Close hides the panel then follows existing WGPU/surface/view teardown.
- [ ] Full latest PR CI must pass: Linux core tests, macOS Swift helper/IPC, feature opt-in tests, probe headless binary/Clippy, default workspace, Release, Lite/Full/Add-on bundles.
- [ ] Update Issue #5 while keeping it open. Real macOS 26 and 14–25 UI tests and keyboard/VoiceOver/Reduce Transparency/Retina/Spaces/IME and production capture/pin integration remain outstanding.
- [ ] Merge only after latest CI success.

## Manual macOS 26 + macOS 14–25 verification

```sh
cargo run --locked -p shot-macos --features native-panel-probe --bin ns-panel-probe -- --check
cargo run --locked -p shot-macos --features native-panel-probe --bin ns-panel-probe -- --show
```

Record screencasts of glass vs vibrancy vs Reduce Transparency, the two button keyboard/mouse actions, theme changes, dark/light system appearance, 1x/2x monitor movement, resize, minimize/restore, X close and Escape, VoiceOver action announcements, app/resource exit. CI does not run `--show` and does not prove any of these device gates.
