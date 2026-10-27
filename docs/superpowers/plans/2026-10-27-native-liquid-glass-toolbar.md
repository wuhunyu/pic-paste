# macOS 26 Native Liquid Glass Toolbar Probe Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Verify a floating, genuinely native macOS 26 Liquid Glass toolbar *inside the existing optional NSPanel/egui Metal probe*, with macOS 14–25 vibrancy and Reduce Transparency solid-color fallbacks, without changing PicPaste production screenshot/pin windows.

**Architecture:** Overlay one compact AppKit-managed pill **above** the probe's existing WGPU-backed content view, and embed a native noneditable status label in `NSGlassEffectView.contentView`. The material and layout policies are platform-independent pure Rust helpers with tests. On runtime macOS 26+, when the class exists and the accessibility setting permits translucency, use `NSGlassEffectView`; otherwise use `NSVisualEffectView` or opaque `NSBox`. Keep glass away from screenshot source pixels, target ROI, PNG export and the production eframe/winit path.

**Tech Stack:** Rust 2024; shot-core; objc2-app-kit 0.3.2; objc2-foundation 0.3.2; native NSPanel/WGPU 30; macOS 14+ runtime, macOS 26 SDK/CI.

## Global Constraints

- All native UI code remains under the existing `native-panel-probe` feature and opt-in `ns-panel-probe --show`; `--check` stays headless.
- A single glass capsule at the lower center is the signature visual. Restrained blue WGPU canvas and system dynamic text; no CSS blur, SwiftUI, Electron/WebView or new third-party dependencies.
- The actual screenshot canvas, selection, annotations, pin surface and PNG output remain crisp/unmodified.
- Use `NSGlassEffectView.contentView` for label hierarchy; do **not** place glass behind a sibling egui label (Apple WWDC25 AppKit guidance). One capsule needs no `NSGlassEffectContainerView`.
- Runtime selection: macOS >=26 **and** class available -> native glass; macOS 14–25/class unavailable -> vibrancy; Reduce Transparency -> solid nontranslucent rounded pill.
- Window resize, zero-size/hide, Retina backing scale and close must not leak or reparent the WGPU view. Existing WGPU Surface retains its original NSView owner.
- Code/CI is **not** interactive macOS 26 validation. Do not claim TCC, Spaces, Mac appearance or performance are proven.
- Accessibility transparency setting is sampled on probe creation; live preference change notification remains a future integration requirement.

## Layout direction

- One bottom-centered rounded pill (236×44 logical points; 20 pt bottom inset; >=16pt side margins), not glass all over the canvas.
- Dynamic native system label; glass itself supplies material without duplicated blur or dark overlay. The label reads "PicPaste · Liquid Glass" on macOS 26, "PicPaste · Vibrancy" on legacy, and "PicPaste · 高对比" for Reduce Transparency.
- Keep the existing egui button/typing controls in the Metal layer. The status pill demonstrates AppKit composition; it is a noninteractive label and **not** a working toolbar action.

### Task 1: TDD material decision and layout (portable)

**Files:** Create `crates/shot-core/src/glass_toolbar.rs`; Modify `crates/shot-core/src/lib.rs`; Create `crates/shot-core/tests/glass_toolbar.rs`.

**Interfaces**
- `enum GlassMaterial { Native, Vibrancy, Opaque }`
- `fn select_glass_material(mac_major: u32, class_available: bool, reduce_transparency: bool) -> GlassMaterial`
- `struct FloatingBarFrame { x: f64, y: f64, width: f64, height: f64 }`
- `FloatingBarFrame::for_content_size(width_pt: f64, height_pt: f64) -> Option<Self>` returns None for nonfinite, too small or zero dimensions
- `FloatingBarFrame::contains(x: f64, y: f64) -> bool` for native pointer filtering

- [ ] RED: write tests for macOS 26 native, macOS 14/25 fallback, unavailable class, Reduce Transparency opaque priority, centered 640×360 rectangle x=202/y=20/w=236/h=44, small/nonfinite hidden, edge containment.
- [ ] Verify expected failing `cargo test -p shot-core --test glass_toolbar --locked` (missing module/API), after formatting.
- [ ] GREEN: implement and expose helpers without AppKit dependencies; use finite numbers and careful clamping; run Linux tests, format and strict Clippy; commit.

### Task 2: Native AppKit glass view and fallbacks

**Files:** Create `apps/shot-macos/src/bin/ns-panel-probe/glass_toolbar.rs`; Modify `apps/shot-macos/src/bin/ns-panel-probe.rs`; Modify `apps/shot-macos/Cargo.toml`.

**Interfaces**
- `struct ProbeGlassToolbar { view: Retained<NSView>, mode: GlassMaterial }`
- `ProbeGlassToolbar::new(mtm: MainThreadMarker, host: &NSView) -> Self`
- `ProbeGlassToolbar::relayout(&self, bounds: NSRect)`; `detach(&self)`
- `ProbeGlassToolbar::intercepts_event(&self, event: &NSEvent, host: &NSView) -> bool` for pointer-down and hover avoidance (but preserve outside-to-inside drag release).

- [ ] Add AppKit feature flags `NSGlassEffectView`, `NSVisualEffectView`, `NSTextField`, `NSControl`, `NSBox`, `NSWorkspace`, `NSAccessibility`; Foundation `NSProcessInfo`. Do not add crates or mutate Cargo.lock versions.
- [ ] Runtime check `NSProcessInfo::processInfo().operatingSystemVersion().majorVersion`, `AnyClass::get("NSGlassEffectView")`, `NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceTransparency()` and choose mode.
- [ ] Native path: create `NSGlassEffectView`, 22 pt radius, assign **native `NSView` content container** with `NSTextField` label using `setContentView`. Keep the view above the original Metal NSView and do not replace NSPanel.contentView.
- [ ] Legacy: `NSVisualEffectView` material Popover, WithinWindow; accessible: `NSBox` custom fill using `NSColor::windowBackgroundColor()`, 22 pt radius, zero border. Native label is non-editable/selectable and uses system text color.
- [ ] On content changes, relayout/hide when too small; before owner shutdown, remove toolbar from host, drop toolbar, then follow existing WGPU surface-before-NSView release order.
- [ ] Run macOS CI feature tests, native --check, strict Clippy. Fix actual compiler signatures, never weaken Clippy.

### Task 3: Verification and documentation

**Files:** Update `docs/macos-window-strategy.md`, `docs/STATUS.md`, `docs/REMAINING.md`, `README.md`; maintain Issue #5 and PR discussion.

- [ ] Document native Liquid Glass on 26+, visual effect on 14–25, high-contrast solid on Reduce Transparency, exact opt-in CLI, noninteractive glass label, no production integration.
- [ ] Explicit manual macOS 26 and macOS 14+ acceptance: light/dark, accessibility setting, 1x/2x monitors, resize/mini/restore/close, interaction hit testing and GPU frame correctness, Spaces, TCC and idle CPU.
- [ ] Full PR CI must pass Linux/macOS feature tests, --check, workspace test/lint, Release, Lite/Full/Add-on archives; only then merge PR. Keep Issue #5 open.
