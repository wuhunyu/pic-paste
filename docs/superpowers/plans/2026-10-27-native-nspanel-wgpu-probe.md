# Isolated NSPanel + WGPU Metal Surface Probe Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove that a genuine AppKit `NSPanel` can own an `NSView` used by the project's pinned wgpu 30 Metal renderer without changing production screenshot/pin windows.

**Architecture:** First add a tested physical-surface geometry policy to `shot-core`. Then build an opt-in macOS-only companion executable, `ns-panel-probe`, using objc2's real `NSPanel` type, the panel's retained content `NSView`, a `raw-window-handle` AppKit handle and wgpu 30's explicit Metal surface; configure, clear and present exactly one frame. Normal PicPaste `shot-macos` and its eframe event loop are unchanged. The probe is **not** the production NSPanel/egui event bridge and is not auto-run on headless CI.

**Tech stack:** Rust 2024, `shot-core`, objc2 0.6.5 / objc2-app-kit 0.3.2 / objc2-foundation 0.3.2, raw-window-handle 0.6.2, wgpu 30.0.1 (Metal), pollster 1.0.1. All crates already exist in `Cargo.lock`.

## Global constraints
- macOS minimum deployment target 14.0. No Electron/WebView, no network, no image format changes; screenshots and long exports remain PNG-only.
- No runtime subclass conversion from winit `NSWindow` to `NSPanel`, and no replacement of the main window/overlay/pins in this PR.
- The experimental program must be opt-in by Cargo feature `native-panel-probe`. `shot-macos` default build, build artifacts and app UI stay unchanged.
- All AppKit references are created and used on the main thread, retained through WGPU surface lifetime. Surface and device must be dropped **before** the native view and panel.
- No physical NSPanel/Spaces/focus result can be claimed from headless GitHub CI; a real interactive Mac with GPU is required to validate visible presentation, close, focus and window levels.
- Compile and test on latest stable macOS CI; run Linux core tests, strict Clippy and Release without feature and probe compile with feature, then execute only the pure `--check` probe on hosted CI.

---

## Task 1: Surface sizing contract (RED -> GREEN)

**Files**
- Create: `crates/shot-core/tests/native_panel_surface.rs`
- Create: `crates/shot-core/src/native_panel.rs`
- Modify: `crates/shot-core/src/lib.rs`

**Interface**
```rust
pub struct PanelSurfaceSize { pub width: u32, pub height: u32 }
impl PanelSurfaceSize {
    pub fn from_points(width_pt: f64, height_pt: f64, scale: f64) -> Result<Self, PlatformError>;
}
```

- [x] Write failing core tests for 640×360pt@2x → 1280×720 physical pixels, 640×360pt@1x, fractional 1.5x scaling, zero/negative/NaN/Infinity/oversized (>8192 px) and multiplication overflow.
- [x] Run `cargo test -p shot-core --test native_panel_surface --locked` on GitHub CI and confirm the expected `unresolved import shot_core::native_panel` RED, not an infrastructure error.
- [x] Implement `PanelSurfaceSize::from_points` using checked finite positive dimensions, rounded backing scale (no desktop-coordinate multiplication), and 8192px cap.
- [x] Export module and confirm GREEN on Linux core CI plus Clippy.

## Task 2: Real NSPanel and WGPU Metal surface (opt-in binary)

**Files**
- Create: `apps/shot-macos/src/bin/ns-panel-probe.rs`
- Modify: `apps/shot-macos/Cargo.toml`, `Cargo.lock`
- Modify: `.github/workflows/ci.yml`

**Interfaces**
```text
cargo run -p shot-macos --features native-panel-probe --bin ns-panel-probe -- --check
cargo run -p shot-macos --features native-panel-probe --bin ns-panel-probe -- --show
```

- [x] Write tests in the companion binary for `ProbeCommand::parse` (`--check`, `--show`, reject other flags) and a pure `--check` which does not touch AppKit, GPUs, screens or TCC.
- [x] Verify RED in macOS CI if the command/feature is missing; add an optional Cargo feature/dependency set pinned to existing lock versions.
- [x] `--show`: get `MainThreadMarker`, create actual `NSPanel` with `initWithContentRect_styleMask_backing_defer`; retain panel/content view, mark content view layer-backed, construct an `AppKitWindowHandle` pointing at NSView and an unsafe WGPU `Surface` with an explicit `METAL` instance. Request compatible adapter/device; reject non-Metal backend; configure and submit one clear render pass; present, show panel and run AppKit event loop.
- [x] Correct ownership: no unsafe lifetime extension beyond panel/view; close only after surface disposal; document manual termination and no headless GUI execution. No unsafe ObjC pointer reclassification.
- [x] Add CI step `cargo test -p shot-macos --features native-panel-probe --bin ns-panel-probe --locked`, `cargo build -p shot-macos --features native-panel-probe --bin ns-panel-probe --locked`, and `cargo run ... -- --check` (no GUI). Keep normal packaging unchanged.
- [ ] Run latest full PR CI; correct any compiler/API differences from logs without weakening tests.

## Task 3: Evidence, merge and next gate

**Files**
- Modify: `docs/macos-window-strategy.md`, `docs/TASKS.md`, `docs/REMAINING.md`, `docs/STATUS.md`, `README.md`
- Update: GitHub Issue #5

- [x] Document what the compiled probe proves (NSPanel+NSView+WGPU Metal surface code), what it does not prove (visual presentation, egui input bridge, production screenshot/clip windows, Spaces/focus/TCC/performance).
- [x] Supply real-Mac manual acceptance procedure: `--show`, verify displayed solid-color panel, screenshot evidence, Escape/close/last surface teardown, 1×/2× displays, fullscreen Spaces and window activation, then record OS/GPU/scale and failures in Issue #5.
- [ ] PR only merges when full Linux/macOS CI succeeds. Update Issue #5 without closing; leave main UI and current screenshots intact.
- [x] Future distinct PR: egui-wgpu rendering + input/IME bridge on NSPanel; subsequent PR switches one capture overlay behind opt-in toggle after physical panel validation. Only then consider pins.

## Risk matrix

| Risk | Mitigation |
|---|---|
| Wrong AppKit subclass/winit owner | Create a standalone real `NSPanel`, never mutate winit window class |
| Surface outlives `NSView` | Scoped variables and drop WGPU surface before view/panel |
| Headless GitHub Actions | Compile/bin unit test and `--check` only; `--show` manual |
| Rendering backend accidentally Vulkan | WGPU instance METAL-only + adapter backend assertion |
| DPI / resize mismatch | `PanelSurfaceSize` verified independent of desktop global origin |
| Production regression | Optional feature/companion binary, no change to `shot-macos` default runtime |
| Hidden GUI acceptance gap | Issue #5 remains open; require actual device screen recording/evidence |

## Validation evidence as of 2026-10-27

- RED #40465652103: expected missing `shot_core::native_panel`.
- RED #40465947486: expected missing `ProbeCommand`.
- Candidate GREEN: [CI #40467547957](https://github.com/wuhunyu/pic-paste/actions/runs/40467547957) validates opt-in native AppKit/WGPU code, CLI and existing workspace. **Real GUI mode --show is not exercised in hosted CI**.
- [ ] Native close/resize/input-loop/IME/Spaces and per-display GPU presentation need actual device work.
