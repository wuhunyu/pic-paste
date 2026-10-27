# macOS Window Strategy — Issue #5

**Decision date:** 2026-10-27 · **Status:** Interim engineering decision, **not** a requirements waiver and **not** NSPanel completion.

## What is implemented

PicPaste uses eframe 0.37 / winit native macOS `NSWindow` viewports and WGPU. The root settings window, per-display selection viewport, per-pin viewport and long-screenshot preview are managed by one egui event loop. Native ScreenCaptureKit is called in workers; only snapshot results and display-local pixels enter the UI. The original selected display's global desktop bounds are kept in **logical points** for viewport placement. The RGBA image, selection geometry and crop offsets are **physical pixels local to that display**, with a complete frame origin of `(0,0)`.

The #31 code requests initial keyboard focus for the screenshot selection viewport (needed for Escape and keyboard edit), starts pinned windows non-activating, disables the shadow that can ghost behind a transparent pin, disables native background dragging to preserve mode-specific drag handling, and enables mouse passthrough only in the pin's explicit Passthrough mode. These settings are **requests to winit**; they are not proof of real macOS focus or Spaces behavior.

## NSPanel decision

**Keep eframe/winit as a temporary selection and pin window backend, not as a replacement for required NSPanel.** Its viewport builder returns platform-native `NSWindow`s managed by the winit event loop; setting `always_on_top` or `with_active` does not change the Cocoa class to `NSPanel`. Reinterpreting/mutating an already-owned `NSWindow` pointer as a different AppKit class is unsafe, particularly with WGPU's Metal surface, event routing and async lifetime. A dedicated `NSPanel` would require a separately owned AppKit window/NSView **on the main thread**, a renderer surface hosted in that view, and explicit pointer/keyboard/IME/drag/close/Spaces lifecycle bridging with the egui UI.

Acceptance gate: on a real macOS 14+ computer, first determine whether current eframe behavior satisfies screenshot overlay capture and pin/focus constraints. If it fails because of native window semantics, implement/test a dedicated AppKit panel backend (capture and pins as separate feature gates). Do not close Issue #5 as “NSPanel done” until such a backend is integrated and tested, or a separately approved requirement change explicitly accepts the eframe window instead.

## Coordinate contract

| Value | Space | Example on secondary Retina screen |
|---|---|---|
| SCDisplay.frame / OverlayPlacement | Desktop-global **logical points**, may be negative | `(-1440,-100)` and `(1440,900)` |
| Full CapturedFrame.origin_px | Target-display-local **physical pixels** | `(0,0)` |
| CapturedFrame.image | Physical pixel data, no global offset | e.g. `2880×1800` at 2× |
| RectPx / selection / crop | Target-display-local physical pixel rectangle | `x=200,y=100,w=100,h=80` |
| Crop CapturedFrame.origin_px | Pixel origin inside target display | `(200,100)` |

Never compute a global physical pixel origin by multiplying `SCDisplay.frame.origin` by that display's Retina scale. The input and output spaces belong to different coordinate systems. Never convert a full 60k-px long screenshot to one GPU pin; existing 4 MP bounded transfer remains unchanged.

## Real-device acceptance matrix (still open)

Use at least macOS 14 and 15+ where available, with an internal Retina panel and a second external monitor. Record hardware, OS build, display arrangement/origins and scale, capture permissions and relevant screen recordings.

1. **Display geometry:** secondary monitor to left and above primary (negative origin), 1×/2× mixed scaling, switch monitor before capture; map 8 selection handles, window candidate, magnifier/HEX and cropped PNG against physical pixels.
2. **Hot-plug:** disconnect/reconnect selected monitor during idle/capture and after overlay opens; fail visibly without an invisible stuck root, never capture the wrong display.
3. **Spaces / fullscreen:** capture from regular desktops, Stage Manager and fullscreen app Spaces; verify viewport appears above the correct target, close/escape restores focus, no reordering into an unwanted fullscreen Space.
4. **Pin interaction:** five 1080p windows, Shift+drag, native drag, OCR text selection, entering passthrough and hotkey recovery; close last shared texture; no input interception while passthrough is on.
5. **TCC and shortcut conflict:** fresh Screen Recording deny/grant/revoke, restart and re-enable; system-reserved hotkey conflict preserves old bindings; errors are visible. Do not infer TCC success from CI.
6. **Self-window exclusion:** ten consecutive captures with both pins and live scroll preview intersecting target ROI: zero app-window ghosts/overlays in the saved PNG.
7. **Performance evidence:** 30 cold starts P95 ≤1.5s; 100 hotkeys to selecting viewport P95 ≤200ms; with five 1080p pins frame P95 ≤16.7ms, record CPU/RSS and individual failures. `cargo test` and the hosted runner cannot substantiate these metrics.

## References

- Apple [SCDisplay.frame](https://developer.apple.com/documentation/screencapturekit/scdisplay/frame) and [SCScreenshotConfiguration.sourceRect](https://developer.apple.com/documentation/screencapturekit/scscreenshotconfiguration/sourcerect): display frames and source rectangles are logical points.
- [eframe 0.37 NativeOptions](https://docs.rs/eframe/0.37.0/eframe/struct.NativeOptions.html) and [egui ViewportBuilder](https://docs.rs/egui/0.37.0/egui/viewport/struct.ViewportBuilder.html): portable viewport ownership, initial activation, transparency, shadow and passthrough options.

## NSPanel Metal surface proof — PR #32 (2026-10-27)

The native window backend investigation now includes an **independent, gated Rust companion executable** (`apps/shot-macos/src/bin/ns-panel-probe.rs`), not a window reclassification and not an integration into `shot-macos`'s eframe process. Enabled solely with `--features native-panel-probe`, it obtains a `MainThreadMarker`, creates a real AppKit `NSPanel`, retains its `NSView` to outlive a WGPU Metal-only `Surface`, chooses a compatible adapter and submits a single clear frame through WGPU 30. A pure `--check` path exercises the CLI and physical backing dimensions without allocating windows/GPU or requesting TCC permission.

The native panel's pixel size is computed by `PanelSurfaceSize::from_points` from **content points × backing scale** with finite/positive validation and an 8192-pixel edge cap. Its global desktop position is never converted to physical pixels with one screen's scale. The probe's `NSPanel` backing view and surface must stay alive together; it is forbidden to reinterpret a winit-created `NSWindow` as this class.

**Manual macOS 14+ test (not yet passed):** clone repo, execute `cargo run --locked -p shot-macos --features native-panel-probe --bin ns-panel-probe -- --show` on an interactive desktop, verify a genuinely titled/floating NSPanel and solid navy Metal frame (record screenshot/video, physical display and OS/GPU); move the panel across 1× and 2× monitors, observe window activation and fullscreen Spaces; use Ctrl+C in the terminal to end. **Ctrl+C is a process stop, not a graceful close test**. Record errors and screenshots in [Issue #5](https://github.com/wuhunyu/pic-paste/issues/5). CI executes only the pure CLI check and compiler/unit validation.

**Separate implementation gates:** (1) real physical surface presentation and lifetime, (2) NSPanel close delegate and event-loop shutdown/resize, (3) egui-wgpu paint + native key/mouse/IME/drag events, (4) opt-in production capture overlay replacement with rollback, (5) optional pin integration, (6) multi-monitor/Spaces/TCC/performance evidence. Do **not** mark a native NSPanel product implementation completed based on the probe's successful build.

## PR #34 — Native NSPanel close, resize and backing-scale lifecycle (2026-10-27)

The opt-in probe now has a test-first `PanelSurfaceLifecycle` that distinguishes unchanged drawable size, resize or Retina scale reconfiguration, zero-size/minimized suspension, and terminal close. Instead of entering `NSApplication.run()` permanently, the companion program processes AppKit events on the main thread with a bounded wait and checks the real `NSView.bounds()` and `NSPanel.backingScaleFactor()`. Upon the X close button it exits its event loop, releases WGPU queue/device/surface **before** releasing the retained view/panel; resize configures a new Metal swapchain and paints a fresh solid frame. Production eframe/winit windows are untouched.

**This remains unverified at device level.** Only native compilation, headless `--check`, Rust unit tests and CI can be credited; actual interactive close, min/restore, live resizing, 1x/2x screen migration, focus, and GPU presentation must be run on a macOS 14+ Mac. It is **not** egui-wgpu integration or an input/IME bridge. [PR #34](https://github.com/wuhunyu/pic-paste/pull/34) and Issue #5 remain the engineering gates.
