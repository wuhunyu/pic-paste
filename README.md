# PicPaste

> **Native frozen screenshot bridge (PR #41, experimental):** with `--features native-panel-probe`, an opt-in button in the *existing* screenshot editor sends the already captured raw RGBA pixels to a same-directory `ns-panel-probe --snapshot` process via a bounded, versioned anonymous stdin pipe. That independent AppKit NSPanel renders a **read-only fitted frozen-image preview** through egui/WGPU Metal; no image is saved, recaptured, annotated or changed by the preview. The default Lite/Full builds do not include this entry or helper. This is **not a native production selection/editor migration**; real macOS 14+/26 GUI/Retina/Spaces/TCC, input and performance testing remain open.

An offline, macOS-first screenshot and pinned-image utility written in Rust 2024, egui/eframe + wgpu and native ScreenCaptureKit. No Electron or WebView. **Development preview, not a signed/notarized public release.** Windows integration is planned after the macOS acceptance gates.

> **Progress truth:** [Current status](docs/STATUS.md) · [Remaining work / active Issue ledger](docs/REMAINING.md) · [Original 20-task tracker](docs/TASKS.md) · [Open Issues](https://github.com/wuhunyu/pic-paste/issues). This README describes the code on `main`, not work in an unmerged PR. As of 2026-10-27, manual screenshot preview (#18), self-window exclusion (#20), trusted-prefix trimming (#21), pin OCR line picking (#22), slow-scroll recovery (#23), fixed-band exclusion (#24), rounded long PNG (#25), bounded review annotation/copy/pin (#26), animation-tolerant overlap / trusted middle retake (#27), Vision word/character OCR plus offline QR (#28), and trusted standalone Add-on code (#29) are merged with green PR CI. Physical mixed-DPI, Spaces, Screen Recording TCC, native clipboard/pins and performance acceptance remain **unverified**.

## Implemented macOS development workflow

1. **Screenshot:** `Ctrl+Option+1` captures the selected display. The settings window can switch to the next detected monitor. The selection/editor opens in a **separate borderless native eframe/winit viewport** positioned using the selected screen's logical-point bounds; screenshots and region offsets are **target-display-local physical pixels** (#31), never desktop points multiplied by Retina scale. The selection viewport requests initial keyboard focus. It is **not** a dedicated AppKit NSPanel. Drag to select, click a recognized window candidate, or adjust selection with eight handles; move an existing selection. The 9×9 source-pixel magnifier samples exact RGBA values and can copy HEX color. Mixed-DPI/Spaces and permission behavior still require physical-device verification.
2. **Annotations:** line, arrow, brush, rectangle, ellipse, pixelate, blur and **offline English/Chinese text** (choose Text, enter text/font size, click to place). Non-destructive Undo/Redo. Text rasterizes with available local system fonts; missing glyphs/system-font availability are not yet verified across clean macOS installations.
3. **Output:** screenshot file exports are **PNG-only**. Save PNG or `⌘S` quick-save; copy image to the native clipboard, or create a pin. The adjustable 0–64 px transparent rounded-corner alpha is preserved in PNG output; a bounded thumbnail previews the export. Verify color fidelity, alpha and clipboard pasting in real Mac applications before release.
4. **Pins:** `Ctrl+Option+2` pins clipboard images. Multiple independent always-on-top image viewports support opacity, scaling and mouse passthrough. The pin viewport initially remains nonactive, disables its transparent-window shadow and disallows background dragging (explicit Shift+drag still moves a pin). Choose **识别词**, **识别字**, or **识别二维码** in a pinned image to request local Apple Vision, then select a real line/word/character box or copy an explicitly decoded QR payload (#28). Move, Text Select and Passthrough are mutually exclusive; Shift+drag temporarily moves a pin in Text Select mode. `Ctrl+Option+3` recovers interactivity after passthrough. The bounded pin-texture ledger (PR #30) limits 12 windows, 64 MiB estimated distinct RGBA textures and 4 MP per image; identical pinned pixels reuse a TextureHandle. True 5-pin frame P95, RSS/CPU, drag/selection routing and native Spaces compatibility have not passed device acceptance.
5. **Manual long screenshots:** select a scroll region, choose vertical or horizontal capture, then scroll the target page or document manually. The worker stores matching strips on disk, rejects uncertain displacement and supports last-segment undo and **keep-first-N trusted segments** tail trimming in review; post-capture review requires explicit Save PNG or Discard. Capture size limits are 60,000 px per long edge / 200 MP. A newest-frame **live preview** (PR #18, merged) is produced at up to 420×600 pixels in a separate pointer-passthrough viewport without allocating a full long-image RGBA preview. Before starting, optionally set **开头（上／左）** and **末尾（下／右）** fixed-band exclusions in physical pixels (#24): vertical excludes sticky headers/footers, horizontal excludes sidebars; those bands are absent from both matching and the final PNG. Invalid bands are rejected before capture; 0/0 preserves existing behavior. This is a manual setting, **not automatic header detection**. **Automatic scrolling is intentionally out of scope.** If three changed frames cannot be reliably matched (for example, after scrolling too fast), the worker **pauses without advancing its trusted anchor**. Backtrack manually to an overlapping scene, select **长截图慢速重试（从暂停继续）** from the macOS menu bar, then scroll slowly in the original direction. The existing trusted strips are reused; you can repeat recovery or finish and save only the trustworthy portion. PR #26 adds disk-backed non-destructive long-image annotations and Undo/Redo to the review; the final annotated PNG is still streamed at full source resolution. A precise selected region can be copied to the native clipboard or pinned, subject to 4 MP total / 4096 pixels per side (no downsampling). PR #27 tolerates a *small* dynamically changing region during physical-pixel overlap matching without removing or falsifying those source pixels in the final PNG. If a reviewed middle seam is wrong, choose its one-based segment number and **retake from that segment**: all subsequent old strips are discarded; the original trusted-prefix viewport is reconstructed from disk, and the user scrolls manually from that anchor. No arbitrary deletion while retaining unverifiable later strips, automatic animation erasure in exported PNG, or 50-scene physical-device acceptance is claimed. The capture adapter excludes PicPaste-owned windows by PID (#20); real-device live-preview positioning, click-through behavior and complete exclusion must still be tested.
6. **Offline OCR and QR:** PR #28 adds accurate word boxes, separate fast character boxes, and explicitly triggered offline QR decode/clipboard copy in the screenshot and pin interfaces. Full uses its bundled signed Apple Vision Helper first; Lite can use a *separately installed* signed executable only if it has the same nonempty Apple Team ID as the installed app and an exact API 1 / SHOC 1 capability reply. The new Add-on packaging and installer are a development preview; ad-hoc builds cannot establish an external trust identity. Missing, damaged or incompatible OCR never blocks screenshot/pin/PNG. **400-image OCR quality evaluation, notarized Developer ID release and macOS physical-device acceptance remain incomplete.**

### Configurable hotkeys

Open **全局快捷键设置** in the settings window. You can assign each of the four actions to a distinct `Ctrl+Option+1…9` combination, apply/save, or restore defaults. Settings are saved under `~/Library/Application Support/PicPaste/hotkeys.conf`. Registration failures attempt to roll back to the previous bindings. Custom modifier combinations and real-world conflict/restart testing are still pending.

### macOS build and test

Current CI uses a macOS 26 runner SDK and `MACOSX_DEPLOYMENT_TARGET=14.0`. **The minimum supported macOS 14 runtime has not been verified on a physical macOS 14 host.** On a compatible macOS development machine:

```sh
export MACOSX_DEPLOYMENT_TARGET=14.0
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --release -p shot-macos --locked

swiftc -O -target arm64-apple-macos14.0 \
  -framework Vision -framework CoreGraphics -framework Foundation \
  -framework CryptoKit packaging/macos/PicPasteOCR.swift \
  -o target/release/PicPasteOCR
python3 scripts/check_ocr_helper.py target/release/PicPasteOCR

PICPASTE_VARIANT=Lite bash scripts/package-macos.sh
PICPASTE_VARIANT=Full bash scripts/package-macos.sh
```

The relevant [GitHub Actions runs](https://github.com/wuhunyu/pic-paste/actions) publish the `picpaste-macos-dev-ARM64` artifact with Lite/Full **ad-hoc-signed development ZIPs** and SHA256 digests. These artifacts are **not** Developer ID–signed, notarized, or suitable as an official distribution. No GitHub Release exists as of this ledger.

## Workspace

| Path | Responsibility |
| --- | --- |
| `crates/shot-core` | Source pixel geometry, capture and selection states, display/window candidate mapping and shortcut validation |
| `crates/shot-image` | Non-destructive annotation rendering (including local Unicode fonts), PNG-only crop/export, alpha rounding and preview |
| `crates/shot-platform` | Native macOS ScreenCaptureKit capture and window information |
| `crates/shot-scroll` | Conservative overlap matching, on-disk tile strips, bounded live preview, streaming PNG and size limits |
| `crates/shot-ocr-protocol` | Bounded/versioned SHOC protocol, OCR geometry and line hit tests |
| `apps/shot-macos` | Native menu/settings, per-monitor editor viewport, pins, hotkeys, offline OCR client, manual long-screenshot UI |
| `packaging/macos/PicPasteOCR.swift` | On-demand offline Vision/QR Helper and signed capability handshake for Full or a trusted external Add-on |
| `apps/shot-macos/src/ocr_addon.rs` | Fixed-path standalone Add-on discovery, signature/Team ID and API compatibility enforcement |
| `scripts/manage-ocr-addon.sh`, `scripts/package-ocr-addon.sh` | User-installed, matching-Team standalone Add-on and separate code-signed ZIP packaging |
| `scripts/package-macos.sh` | Developer-only Lite/Full app assembly, ad-hoc codesigning and ZIP SHA256 |

## Current limitations

There are **7 open Issues (#5–#11)** with an up-to-date [remaining-work ledger](docs/REMAINING.md). No verified physical macOS multi-monitor/Spaces/TCC/clipboard/performance report; manual scrolling is the explicit product behavior (no auto scroll); QR and native word/character OCR code is merged (#28), while separately installable matching-Team Add-on code still requires signed-app macOS validation; no x86_64 macOS release or Developer ID notarization; no 400-image CER evidence or Windows native implementation. `packaging/macos/Info.plist` retains development bundle ID `com.example.picpaste`.

See [task-by-task acceptance and evidence](docs/TASKS.md). **CI-green proves build and automated tests only—not real screen capture or sustained performance on a user's device.**

## 2026-10-27 incremental delivery — rounded long screenshots

PR [#25](https://github.com/wuhunyu/pic-paste/pull/25) is merged into `main`. Manual long-screenshot review now uses the same 0–64 physical-pixel rounded-alpha setting as regular PNG capture: its bounded thumbnail updates when the radius changes, and the final source-resolution PNG applies the mask per output scanline without building a contiguous long RGBA image. Zero-radius, PNG-only, no-overwrite, trusted-tile cleanup, and disk-backed export remain unchanged. Linux/macOS PR CI [#40432104187](https://github.com/wuhunyu/pic-paste/actions/runs/40432104187) succeeded; physical-device alpha, visual parity, and 50-scene acceptance remain open.

## 2026-10-27 — annotated long screenshots and bounded transfer (PR #26)

PR [#26](https://github.com/wuhunyu/pic-paste/pull/26) is merged on `main`. The long screenshot review can add offline source-pixel line, arrow, brush, rectangle, ellipse, pixelate, blur, and text annotations, Undo/Redo them, and export full-size PNG with rounded alpha. Each operation writes a private disk patch capped at 4 MP; export and bounded previews apply these patches while streaming source rows. Native-resolution selections can be copied or pinned with a 4-MP/4096-pixel per-edge hard cap; invalid selections are explicitly rejected. Trusted source tiles remain unchanged; successful save or explicit discard drops private temp data. [PR CI #40435034912](https://github.com/wuhunyu/pic-paste/actions/runs/40435034912) passed Linux/macOS tests, Clippy, Release and Lite/Full packaging. Full-image clipboard/pin of a 60k-edge canvas, dynamic-content filtering, true middle-strip repair, real device interaction and performance targets are not delivered or verified.

## 2026-10-27 — dynamic tolerance and safe middle-retake (PR #27)

Merged [PR #27](https://github.com/wuhunyu/pic-paste/pull/27) adds eight-lane displacement proposals and an independent stable-pixel consensus (up to 25% changed sample tolerance, with unique-shift ambiguity rejection). Repeated wallpaper, animation-only and oversized dynamic regions remain untrusted. Pixels are excluded **only from matching**, never from saved PNG. The review can select an erroneous segment (2..N), retain exactly the previous trusted prefix, delete the selected segment and its entire suffix, and return to manual capture on the same original physical display/ROI/axis with a reconstructed native viewport anchor. Cancellation or worker errors preserve a reviewable prefix; preview is rebuilt after partial progress. The last-viewport reconstruction opens only intersecting strips, not every strip at once, and the live preview reuses its GPU texture. Full Linux/macOS PR [CI #40440066887](https://github.com/wuhunyu/pic-paste/actions/runs/40440066887) verified automated build/tests. Physical-scene acceptance and in-place safe suffix splicing remain open.


## 2026-10-27 — standalone offline OCR Add-on (PR #29 merged; dev preview)

Lite can discover only `~/Library/Application Support/PicPaste/AddOns/OCR/PicPasteOCR` when no embedded Helper exists. Before execution, PicPaste checks the signed app and the external executable with Apple's `codesign`, rejects symlinks and **any missing/mismatched Team ID**, and requires the exact `--addon-info` reply `PICPASTE-OCR-ADDON/1 SHOC/1`. Full continues to prefer its own built-in Helper. The user-run installer replaces an Add-on atomically with rollback and supports uninstall/status without sudo or network.

```sh
# Only real Apple Developer ID credentials produce an installable Add-on.
PICPASTE_SIGN_IDENTITY="Developer ID Application: Your Organization (TEAMID)" \
  bash scripts/package-ocr-addon.sh

# After extracting PicPasteOCR from its signed ZIP:
bash scripts/manage-ocr-addon.sh install "/Applications/PicPaste.app" "/path/to/PicPasteOCR"
bash scripts/manage-ocr-addon.sh status "/Applications/PicPaste.app"
bash scripts/manage-ocr-addon.sh uninstall
```

When `PICPASTE_SIGN_IDENTITY` is absent, packaging produces `*-UNSIGNED-DEV.zip` with an ad-hoc signature. This artifact deliberately **fails external install/trust checks**. A proper Developer ID–signed PicPaste app plus a helper signed by the *same Apple Team* is required. The remaining notarization, macOS 14+ Gatekeeper and 400-image OCR/real-device gates are in [Issue #9](https://github.com/wuhunyu/pic-paste/issues/9) / [Issue #10](https://github.com/wuhunyu/pic-paste/issues/10). See [full OCR Add-on instructions](docs/OCR-QR.md).

PR [#29](https://github.com/wuhunyu/pic-paste/pull/29) was merged with full [PR CI #40456135637](https://github.com/wuhunyu/pic-paste/actions/runs/40456135637) success. Signing with a real Apple Developer ID and testing the installed external Add-on on physical macOS remain separate work.

## 2026-10-27 — bounded multi-pin GPU resources (PR #30)

[PR #30](https://github.com/wuhunyu/pic-paste/pull/30) adds a checked texture-residency ledger, bounded uploads and immediate reference-ledger release when a pin closes. Admission is capped at **12 live pins**, **4 million source pixels and 4096 physical pixels per edge** per image, and **64 MiB of estimated unique RGBA8 texture payload**. Five distinct 1920×1080 images require 41,472,000 estimated bytes and fit; a ninth distinct 1080p image is rejected while eight remain visible. Rejected images do not silently evict existing pin windows.

When a new pin has exactly the same dimensions and **all RGBA bytes** as an existing visible pin, it shares the existing `Arc<RgbaFrame>` and `egui::TextureHandle` rather than uploading a duplicate. Per-window mode, opacity, zoom, OCR selection and close state remain independent. The final pin closure releases the texture ledger's unique allocation and drops the handles; GPU backend reclamation can be asynchronous. A close action wakes the hidden root window for cleanup.

The settings view reports **estimated** unique GPU RGBA bytes, not measured VRAM/RSS. A giant clipboard/long-screenshot pin is rejected before egui texture upload without affecting PNG export or image copying. Real macOS 14+ Retina/Spaces, 5×1080p frame P95 ≤16.7ms, idle CPU/RSS and clipboard/Alpha acceptance remain open in [Issue #6](https://github.com/wuhunyu/pic-paste/issues/6) / [Issue #10](https://github.com/wuhunyu/pic-paste/issues/10).

[PR #30](https://github.com/wuhunyu/pic-paste/pull/30) is merged at [`0657fd5`](https://github.com/wuhunyu/pic-paste/commit/0657fd52dcb3f1250a2a5db8aae2ff63993ecd5e); [full PR CI #40458668058](https://github.com/wuhunyu/pic-paste/actions/runs/40458668058) passed Linux/macOS tests, Clippy, Release, Lite/Full and OCR Add-on packaging. Physical 5-pin GPU frame P95/RSS/CPU measurements remain unverified.

## 2026-10-27 — macOS display-local pixels and window focus (#31)

[PR #31](https://github.com/wuhunyu/pic-paste/pull/31) separates native screenshot pixel origins from global desktop logical placement: the complete ScreenCaptureKit display image is always `(0,0)` local pixels, and bounded crops use validated integer offsets. A secondary Retina monitor's negative logical desktop origin is retained for the editor viewport, **not** multiplied by its own scale and written into pixel-space data. Unit tests cover negative/out-of-range/empty crop regions and integer overflow. Screenshot viewports explicitly request keyboard activation; independent pin windows start nonactivating, avoid transparent shadows and keep passthrough exclusively mode-controlled.

This is code and CI-level hardening, **not a physically verified macOS 14+ heterogeneous-monitor fix**. The runtime remains eframe/winit `NSWindow`, **not** AppKit `NSPanel`. See [macOS window architecture decision](docs/macos-window-strategy.md) for the staged design, device matrix and acceptance boundaries.

[PR #31](https://github.com/wuhunyu/pic-paste/pull/31) merged as [`73122e0`](https://github.com/wuhunyu/pic-paste/commit/73122e009c4bef66b39c4ef0f0f60def4680c76e) after complete [PR CI #40461875462](https://github.com/wuhunyu/pic-paste/actions/runs/40461875462) success (Linux/macOS tests, strict Clippy, Swift IPC, release, Lite/Full/Add-on packaging). Dedicated NSPanel and macOS 14+ physical multi-DPI, Spaces, TCC, focus and P95 acceptance remain open.

## 2026-10-27 — experimental AppKit NSPanel + WGPU Metal proof (#32)

[PR #32](https://github.com/wuhunyu/pic-paste/pull/32) adds a **separately compiled, opt-in** companion program. It creates a real AppKit `NSPanel` containing a retained `NSView`, constructs a WGPU 30 Metal surface using that view's AppKit window handle, and attempts one solid navy-color render and present. It does **not** replace the production eframe/winit NSWindow screenshot or pin views.

```sh
# Pure command parser / geometry check: no GUI, GPU, TCC or ScreenCaptureKit.
cargo run --locked -p shot-macos --features native-panel-probe --bin ns-panel-probe -- --check

# Explicit interactive test on a personal macOS 14+ Mac with a visible desktop.
# If the panel opens, observe the navy-colored Metal surface.
# Use Ctrl+C in the invoking terminal to stop the probe.
cargo run --locked -p shot-macos --features native-panel-probe --bin ns-panel-probe -- --show
```

The interactive `--show` path has **not been exercised on a real Mac** by automated CI. It is a *smoke test*, not an event-loop, IME, keyboard, close/cleanup, multiple-Spaces or real-display acceptance. In particular Ctrl+C force-terminates the process and does not prove graceful WGPU teardown. The default PicPaste application, Lite/Full archives, PNG-only Alpha export, scrolling and offline OCR/QR remain unchanged. [Architecture and acceptance checklist](docs/macos-window-strategy.md).

### Experimental native close/resize follow-up (#34)

The `ns-panel-probe --show` experiment now attempts AppKit event dispatch, native X-button exit, Metal redraw on content-view resizing or backing-scale changes and cleanup before releasing the native view. This does **not** yet replace the normal screenshot UI or prove anything about real macOS Spaces or input/IME behavior. After building with `native-panel-probe`, use the window's **X close button** (not only terminal Ctrl+C) and try moving/resizing across 1x and 2x monitors. Record OS, GPU, video and failures in [Issue #5](https://github.com/wuhunyu/pic-paste/issues/5).

### Experimental macOS 26 Liquid Glass capsule (PR #36)

The optional AppKit `ns-panel-probe` now includes one **native, bottom-centered material capsule** above the WGPU/egui Metal canvas. On macOS 26+ (if available) it uses `NSGlassEffectView.contentView`; macOS 14–25 uses `NSVisualEffectView`. macOS Accessibility **Reduce Transparency** chooses an opaque system-colored `NSBox`. The label is deliberately **noninteractive** (a material preview, not finished screenshot-tool buttons). The capsule is fixed at 236×44 logical points and hides if the panel becomes too small.

```sh
cargo run --locked -p shot-macos --features native-panel-probe --bin ns-panel-probe -- --check
cargo run --locked -p shot-macos --features native-panel-probe --bin ns-panel-probe -- --show
```

`--check` does not create AppKit windows or access the GPU. Hosted macOS CI does not run `--show` and **cannot prove** material appearance or real click/focus/Spaces/Retina behavior. Reopen the probe after changing Reduce Transparency (live preference observation is not implemented). Original screenshot/pin windows, scrolling, OCR/QR, PNG-only Alpha output and packaging remain unaffected. See [Issue #5](https://github.com/wuhunyu/pic-paste/issues/5) and [window strategy](docs/macos-window-strategy.md).

> **2026-10-28 native macOS progress:** PR [#38](https://github.com/wuhunyu/pic-paste/pull/38) added opt-in NSPanel CJK text composition, and PR [#39](https://github.com/wuhunyu/pic-paste/pull/39) fixed AppKit-consumed key routing; both are merged. [#40](https://github.com/wuhunyu/pic-paste/pull/40) adds an **explicitly feature-gated** option in the main settings window to launch the separate NSPanel/Liquid Glass test program. It does **not** switch actual screenshot/editor or pin windows to NSPanel. Real macOS GUI, CJK candidate placement, Retina/Spaces, TCC, performance and accessibility acceptance remain open.



### Experimental native NSPanel screen selection (developer feature)

Build both native binaries with `cargo build --locked -p shot-macos --features native-panel-probe --bins`, then run `cargo run --locked -p shot-macos --features native-panel-probe --bin shot-macos`. Capture normally and click **NSPanel 原生框选（实验）**. A separate **borderless native AppKit NSPanel** displays an immutable full-display RGBA screenshot on the selected display, lets you drag a region and transfers one validated physical-pixel rectangle back to the existing eframe editor. Escape cancels and leaves the screenshot unchanged. This mode is **opt-in** and needs an interactive macOS desktop; it is not the default production screenshot backend or a complete native annotation editor. macOS 14+/26, Spaces, TCC, mixed Retina scaling, keyboard focus and actual GPU/GUI performance still need real-device acceptance. No temporary screenshot files, PNG changes or new network services are involved.


### 八方向原生选区调整（开发实验，PR #43）

仅在通过 `--features native-panel-probe` 编译后，普通截图页面的“NSPanel 原生框选（实验）”按钮才会打开独立的 AppKit NSPanel。拖动鼠标建立选区后，**不会立即返回**：可拖动选区内部移动、拖动八个白色控制点调整大小，显示物理像素尺寸，最后点击“确认 (Enter)”或按回车才通过 `PPSEL001` 将选区安全回传到现有 eframe 编辑器。点“取消 (Esc)”或按 Escape 将保留原截图并取消原生框选。按默认方式启动的截图、PNG、OCR、长图和贴图均不受影响。此工具栏目前是 **egui 实验操作区，不是原生 Liquid Glass 标注工具栏**，macOS 26 真机、Retina/多屏、Spaces、焦点与性能仍须实际验证。

开发运行：

```sh
cargo build --locked -p shot-macos --features native-panel-probe --bins
cargo run --locked -p shot-macos --features native-panel-probe --bin shot-macos
```
