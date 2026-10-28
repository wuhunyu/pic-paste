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

## PR #35 — Isolated egui Metal UI and basic AppKit input (2026-10-27)

A new `native-panel-probe`-only egui-wgpu renderer draws an interactive button and single-line text field into the **real AppKit NSPanel's existing Metal surface**, with explicit font-texture upload and GPU buffer updates. Native NSView-local Cocoa positions are converted to egui's top-left **logical** points (never desktop-global physical pixels); basic mouse presses/drags, scroll events, focus changes and direct ASCII key events are queued for the next paint. The system NSPanel still owns titlebar/close/resize events, and WGPU resources are dropped before its native view. The normal eframe screenshot and pin backends are unchanged.

**Not yet a production-ready event bridge:** `NSTextInputClient` composition, Chinese/Japanese/Korean IME marked text and candidate placement, complex keyboard layouts, clipboard, accessibility, non-US scroll directions, frame pacing, GPU performance, live focus and Spaces behavior are not proven. Headless macOS CI tests a pure egui pass, compiled event code and `--check` only; the interactive `--show` path must be tested on macOS 14+ before claiming real presentation or clicks. Keep Issue #5 open.

## 2026-10-27 — Native Liquid Glass policy (PR #36)

**Decision recorded in Issue #5:** macOS 26 uses **AppKit native `NSGlassEffectView`**, *only* for floating controls; do not blur screenshot/crop/annotation/pin pixels. Apple explicitly recommends glass at the top control hierarchy with the actual control in `contentView`, not glass placed as an unrelated sibling behind UI elements. This probe uses a single floating label/capsule (no `NSGlassEffectContainerView` necessary until multiple glass shapes are present).

**Runtime selection:** macOS >=26 **and** `NSGlassEffectView` Objective-C runtime class exists → Liquid Glass; macOS 14–25 or missing class → `NSVisualEffectView` (Popover, WithinWindow); Accessibility Reduce Transparency → opaque rounded `NSBox` with dynamic window background and label text. Native system colors adapt to light/dark mode. The accessibility preference is sampled once at probe startup, so restart the probe after toggling it.

**Geometry:** One bottom-center 236×44-point pill, 20 pt from bottom, with at least 16 pt side room and a minimum 268×140-point content region. Invalid/zero sizes hide it. `NSPanel.contentView` continues to own WGPU Metal; the AppKit toolbar is an overlay child that detaches before the surface releases its host. New clicks in the capsule region do not enter egui; release events still enter egui to prevent stuck drags.

**Explicit limits:** This is a **noninteractive status-label demonstration** in `native-panel-probe`, not the final action toolbar, not a finished style of the production app and not a tested macOS 26 GUI screenshot. No Retina, Spaces, accessibility preference switching, real Material composition or idle/performance claim can be derived from headless CI. Complete real macOS 26 and macOS 14+ GUI/dark/light/reduce-transparency tests and native IME before any opt-in production integration.

References: https://developer.apple.com/videos/play/wwdc2025/310/ and https://developer.apple.com/documentation/appkit/nsglasseffectview

## PR #37 — Interactive native glass controls (2026-10-28)

The optional `ns-panel-probe` now uses two actual native `NSButton` controls inside a **single** existing AppKit material capsule. “浅色预览/深色预览” switches only the egui demo canvas via `Context::set_theme`, and “关闭” exits the probe through the existing WGPU-before-NSView teardown. Button target/action selectors queue commands using a main-thread-only retained target (AppKit controls hold their target weakly). The same controls live in `NSGlassEffectView.contentView` on macOS 26, `NSVisualEffectView` on 14–25 and opaque `NSBox` for Reduce Transparency; no production screenshots or saved pixels are modified.

**Automated boundary:** Pure Rust toggle-label tests and macOS compile, headless egui style transitions, `--check`, workspace/release/package gates can be validated by CI. CI does **not** execute a mouse click, keyboard activation, VoiceOver action, focus transition, actual translucency, live Reduce Transparency switching or Retina/Spaces/Metal presentation. Test the two actions and macOS 14/26 fallback on a real Mac before integrating a real screenshot “复制/保存/标注” toolbar. Keep Issue #5 open.

## 2026-10-28 — native IME and safe opt-in companion bridge (#38–#40)

PR #38 implements `NSTextInputClient` / `NSTextInputContext` for marked CJK text and candidate caret rectangles in the independent Metal/egui NSPanel, and PR #39 adds balanced native-consumed key routing. Both changes have passing Linux/macOS CI; neither is physical GUI acceptance.

PR #40 adds a developer-only entry in the existing PicPaste settings screen (compiled with `--features native-panel-probe`). The entry launches the **separate process** `ns-panel-probe --show`, only from a verified same-directory, non-symlink, executable file. Multiple clicks reuse a live child; parent exit cleans it up. Missing/invalid helpers return a visible error and **do not affect ScreenCaptureKit, the real screenshot editor, pins, OCR, PNG or production eframe windows**.

Manual opt-in development verification (not part of normal Lite/Full builds):

```sh
cargo build --locked -p shot-macos --features native-panel-probe --bins
cargo run --locked -p shot-macos --features native-panel-probe --bin shot-macos
```

In the main settings UI expand **原生 NSPanel 实验（不替换截图）** and click **启动独立 NSPanel 实验窗口**. Check macOS 14+/26 display presentation, Liquid Glass/vibrancy/Reduce Transparency, CJK candidate placement, keyboard/mouse, 1x/2x, Spaces/focus and teardown. The helper is not bundled by default; without it the UI must show an error and regular screenshot features must still work. This is a **test access seam** rather than native screenshot UI migration. Gate any actual capture/pin NSPanel rollout on GUI acceptance and editor-feature parity.


## PR #41 — True frozen RGBA screenshot preview in isolated native NSPanel (2026-10-28)

With the `native-panel-probe` feature explicitly enabled, the current production **eframe** screenshot editor exposes a developer-only **NSPanel 冻结画面预览（实验）** button. It sends the already captured display-local unpremultiplied RGBA pixels to the same-directory, checked `ns-panel-probe --snapshot` child via a private anonymous stdin pipe. The `PPSNAP01` envelope includes a bounded version, the physical image dimensions and length (≤8192 edge, ≤128 MiB), target display ID and **independent global logical-point origin/size**. No temporary image file, shell command, PATH discovery, data mutation or PNG recapture is involved. The parent retains the child and stops it before the next screenshot; the transfer runs in a writer thread rather than the UI loop.

The child validates the entire frame before AppKit/WGPU initialization and paints a read-only image texture fitted to its **experimental** NSPanel. The logical-point placement is carried by the transport but **does not yet position the native panel over the target display**. This is a real-frame renderer/data bridge, **not** a fullscreen screenshot selection editor, a native annotation toolbar, a production window backend switch or performance validation. Existing screenshot selection, pin, OCR, long-capture and PNG-only paths are unchanged, including the macOS 14–25 and Reduce Transparency toolbar fallbacks from earlier probes.

Manual test in an interactive macOS 14+/26 session:

```sh
cargo build --locked -p shot-macos --features native-panel-probe --bins
cargo run --locked -p shot-macos --features native-panel-probe --bin shot-macos
```

Capture normally, click **NSPanel 冻结画面预览（实验）**, check actual displayed source pixels (including alpha, dark/light, 1×/2×) and close/reopen; then capture again and ensure the previous separate process exits. On a mixed-DPI setup include negative display origins and Spaces. Also verify the default build without the feature, ordinary screenshots, saved PNG, clipboard and multi-pin behavior. Device findings, sample screenshots, hardware/OS and memory observations are still required in Issues #5/#10; a green headless CI is not evidence of those results. The next gated milestone is a native selection rectangle and trustworthy handoff into the existing editor, not automatic rollout.


## PR #42 — Experimental native selection aligned to the captured display (2026-10-28)

PR #42 adds a separate, explicitly feature-gated `ns-panel-probe --select` mode on the frozen `PPSNAP01` RGBA input. AppKit `NSScreen.screens` provides the screen containing the menu bar; the native selection window's **bottom-left** origin is calculated as `appkit_origin_y = primary_screen_max_y - (SCDisplay.frame.origin_y + SCDisplay.frame.height)`, while the **x** coordinate remains the CoreGraphics logical-point x. Match the resulting rectangle to an attached NSScreen (1-point tolerance) before showing a **borderless floating NSPanel** with `CanJoinAllSpaces | FullScreenAuxiliary`; no screen-global point is multiplied by a Retina scaling factor. A wrong/removed display prevents launching the overlay rather than placing it over an arbitrary monitor.

The NSPanel paints the immutable source pixels over the logical content rectangle, draws a yellow live drag rectangle, and uses a pure Rust mapping to clamp/round back to **display-local physical source pixels**. On drag release, it exits the native loop and writes a single exact-length `PPSEL001` stdout reply; Escape returns an explicit cancellation. The parent reads on a worker thread and applies the selection only if both the original capture revision and the display ID/physical bounds still match the active eframe editor. Crash or malformed data never overwrites a live selection. The existing screenshot editor remains visible as a safety fallback; native controls do not save PNGs or mutate original RGBA pixels.

Manual verification remains required on macOS 14+/26 with 1× and 2×/negative-origin displays: check borderless activation, pointer drag, Escape, monitor unplug, Spaces/fullscreen and Stage Manager behavior, change monitor between captures, inspect the same selected pixel rectangle in the existing editor, and repeat ordinary PNG/save/clipboard/pin/OCR. The CI macOS runner only tests compile, pure geometry and headless input/output, **not** real GUI/retina/Space/TCC/performance behavior. This is not a full native editor migration and must not close Issue #5.


## PR #43 — Eight-grip native selection adjustment (developer feature)

The existing feature-gated `ns-panel-probe --select` is now a two-phase selector. After initial mouse drag, the rectangle stays inside the native NSPanel. Eight visible grips resize the four edges and four corners with the opposite sides anchored and a positive one-source-pixel minimum. Dragging the interior moves the rectangle and preserves its exact physical dimensions; moving and resizing clamp to the capture's display-local RGBA boundary. Geometry lives in pure `shot-core::native_selection_edit` and does not multiply negative/global desktop origins by per-monitor scaling. Active gestures are not yet confirmed results, and an invalid/click-only gesture cannot wipe a committed selection.

The egui-wgpu Metal overlay draws the yellow selection stroke, eight white handles, size in physical pixels and an egui floating action capsule. Enter or **确认 (Enter)** explicitly completes the gesture and emits the existing single validated `PPSEL001` packet to the eframe editor; Escape, close or **取消 (Esc)** yields cancellation. **Release no longer closes the panel.** The screenshot source remains immutable and the default build still uses eframe, including PNG-alpha, pin and OCR. The selection action capsule is not a native NSGlassEffectView annotation toolbar.

Headless CI verifies pixel geometry, action gating, macOS builds and existing packages, but cannot prove physical UI behavior. Device acceptance must cover macOS 14+/26, negative-origin displays, 1×/2× mixed DPI, Spaces/fullscreen, focus/IME/keyboard, Reduce Transparency and CPU/GPU/RSS. Native annotation tools and Liquid Glass editing controls remain a separately gated following task; Issue #5/#10 remain open.
