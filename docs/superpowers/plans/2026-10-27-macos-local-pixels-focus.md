# macOS Display-Local Capture and Viewport Focus Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Correct mixed-DPI/negative-origin capture coordinate provenance and explicitly control native screenshot/pin viewport focus, while preserving offline screenshot, PNG, OCR, long capture, and user-managed pins.

**Architecture:** The `CapturedFrame` image and `origin_px` become **target-display-local physical pixel coordinates**; only `OverlayPlacement` uses desktop logical point bounds. `shot-core` validates crop bounds/overflow before any pixel copying. eframe 0.37 viewports keep their existing winit/egui renderer, with explicit initial focus/transparent-no-shadow behavior; the AppKit NSPanel decision is documented separately rather than falsely claiming that a winit NSWindow is an NSPanel.

**Tech Stack:** Rust 2024, eframe/egui 0.37, screencapturekit 11 on macOS 14+, GitHub Actions macOS + Ubuntu.

## Global Constraints
- Keep UI and file operations fully offline, PNG-only (including transparent rounded Alpha), no Electron or WebView.
- Keep ScreenCaptureKit enumeration and image capture on worker threads; never call it in egui viewport callbacks.
- Global desktop **points** may be negative and must not be multiplied by the current display's Retina scale.
- Every full-display capture has `origin_px=(0,0)`; crop offsets are strictly display-local **physical pixels**, computed with checked integer addition.
- Screenshot overlay requests initial keyboard activation to make Escape and selection usable. Independent pinned images do not initially activate or steal focus; their mode-specific mouse passthrough remains unchanged.
- A winit-created `NSWindow` is not an AppKit `NSPanel`; do not change runtime class or introduce unverified native objective-C window pointer mutations.
- No claims of true mixed-DPI/Spaces/TCC correctness, frame timing, focus semantics or NSPanel equivalence without a macOS 14+ real-device matrix.

---

## File Responsibilities

- `crates/shot-core/tests/capture_pixel_space.rs`: failing display-local origin, negative logical bounds, crop and invalid/overflow tests.
- `crates/shot-core/src/platform.rs`: `CapturedFrame::for_display` and `region_origin_px` with an explicit coordinate contract.
- `crates/shot-platform/src/macos/capture.rs`: remove invalid global-origin × display-scale calculation; keep pixels display-local and reuse validated crop offset.
- `apps/shot-macos/src/overlay.rs`: focusable screenshot viewport, fixed no-shadow/no-background drag, unit tests.
- `apps/shot-macos/src/pin.rs`: testable `pin_builder` with no initial focus stealing, appropriate shadow/mouse mode; preserve application-managed drag.
- `docs/macos-window-strategy.md`: NSPanel renderer/lifecycle decision and exact later macOS true-device acceptance checklist.
- `README.md`, `docs/STATUS.md`, `docs/REMAINING.md`, `docs/TASKS.md` and Issue #5: code versus device acceptance states.

### Task 1: Display-local source coordinates, RED then GREEN

**Interfaces:**
```rust
impl CapturedFrame {
    pub fn for_display(display_id: DisplayId, image: RgbaFrame) -> Self;
    pub fn region_origin_px(&self, rect: RectPx) -> Result<(i32, i32), PlatformError>;
}
```

- [ ] **Test first** in `capture_pixel_space.rs`: a 2× Retina monitor with logical desktop bounds `(-1440,-100)` still yields physical pixel origin `(0,0)` for full capture; valid `RectPx{x:2,y:1,width:2,height:1}` yields `(2,1)`; reject negative/out-of-bounds/empty region and offset integer overflow without mutation.
- [ ] **Verify RED** on CI Ubuntu: `cargo test -p shot-core --test capture_pixel_space --locked`, expect unresolved `CapturedFrame::for_display`.
- [ ] **Implement core constructor** and checked region offset; test image is small and generated in memory, no actual TCC permission required. Negative *desktop-point* overlay placement remains separate from local pixel geometry.
- [ ] **Wire macOS adapter**: replace `(frame.origin.x * filter.point_pixel_scale())` global-origin calculation with `CapturedFrame::for_display`; `capture_region` uses `full.region_origin_px(region)?` before cropping.
- [ ] **Verify GREEN** Linux core suite and macOS full workspace/Clippy/Release. No reading screen necessary for pure tests.

### Task 2: Screenshot and pin focus policy, RED then GREEN

**Interfaces:** `overlay::capture_builder(OverlayPlacement)->ViewportBuilder`; `pin::pin_builder(id:u64,size:egui::Vec2,mode:PinMode)->ViewportBuilder`.

- [ ] **Test first**: capture builder expects `active=Some(true)`, `mouse_passthrough=Some(false)`, `movable_by_window_background=Some(false)`; pin builder for Move/TextSelect/Passthrough expects `active=Some(false)`, `has_shadow=Some(false)`, `movable_by_window_background=Some(false)`, and passthrough enabled only in the dedicated mode.
- [ ] **Verify RED** with macOS CI; expect failed focus assertion and missing `pin_builder` API.
- [ ] **Implement minimal builders** while preserving `with_always_on_top()`, `with_decorations(false)`, existing window sizes, explicit drag via `ViewportCommand::StartDrag`, and current OCR UI/close event handling.
- [ ] **Verify GREEN** with macOS `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo build --release -p shot-macos --locked`.

### Task 3: Architecture decision and delivery evidence

- [ ] Document that eframe/winit currently generates `NSWindow` viewports, not `NSPanel`. Switching to a dedicated NSPanel requires an AppKit main-thread host, renderer surface/window lifecycle and keyboard/mouse event bridge, not just calling `setLevel` on an existing window. Keep Issue #5 NSPanel acceptance open pending physical-device reproduction or explicit requirements waiver.
- [ ] Link [Apple SCDisplay frame documentation](https://developer.apple.com/documentation/screencapturekit/scdisplay/frame) (points) and [eframe 0.37 window/viewport APIs](https://docs.rs/eframe/0.37.0/eframe/struct.NativeOptions.html) as design references.
- [ ] Update README/status/remaining/tasks and Issue #5, clearly distinguishing unit/CI evidence from physical macOS 14+ heterogeneous display, negative-origin, Spaces, TCC and focus behavior.
- [ ] Run PR CI including Linux core, macOS Swift IPC, Rust workspace tests, strict Clippy, release and Lite/Full/Add-on packaging. Fix failures before squash merge.
- [ ] Merge only when latest PR CI is green, then synchronize the main ledger/Issue with commit and run references. Keep Issue #5 open until real device tests.

## Acceptance Matrix

| Scenario | Expected / evidence |
|---|---|
| Full screenshot on negative-origin 2× display | `CapturedFrame.origin_px=(0,0)`; `OverlayPlacement` retains negative logical desktop points |
| Crop `(x=2,y=1,w=2,h=1)` | Origin `(2,1)` in display-local physical pixels |
| Negative/empty/out-of-bounds/overflow crop | Error before pixel copy; no coordinate wrap |
| Open screenshot viewport | Request focus, accepts keyboard/selection; remains non-fullscreen |
| Open pinned image | Initially nonactive, always on top; no accidental background drag |
| Pin Passthrough mode | Mouse passthrough true only in that mode, still recoverable |
| True AppKit NSPanel | **Not delivered**; architecture and acceptance gap remain documented |
| Mixed DPI/Spaces/TCC/30 cold starts/100 hotkeys | Real macOS device evidence still pending; CI-green is not a pass |
