> **Current checkpoint (2026-10-29):** `main` at [`2ba61d8`](https://github.com/wuhunyu/pic-paste/commit/2ba61d81c67637242eed5fdc932bc751177ac071) merges PR #55. [Main CI #40806711107](https://github.com/wuhunyu/pic-paste/actions/runs/40806711107) is successful. 47 PRs merged; **6 open Issues (#6–#11)**. Issue #5 was owner-closed on Oct 29, although outstanding physical NSPanel acceptance remains tracked in #10. **The 1.0.0 Bundle ID/icon/version changes under review are candidate metadata only, not a signed/notarized public release.** See [release preparation](docs/release-runbook.md).

> **2026-10-29 confirmed baseline:** `main` at `4083926` merges PR #52 (44 merged PRs in total). Post-merge [CI #40678378483](https://github.com/wuhunyu/pic-paste/actions/runs/40678378483) passed `core` and `rust` jobs. Native NSPanel automatic screenshot routing remains an opt-in developer canary; no physical macOS GUI acceptance or signed/notarized release is claimed.

# PicPaste

> **原生 NSPanel（仅实验 Feature）：** PR #45–#52 已在独立原生浮层实现八手柄选区、系统 Liquid Glass 控件、画笔/直线/箭头/矩形/椭圆/马赛克/模糊/Unicode 文字与本地 Undo/Redo；确认后通过有界 `PPANN001` 进入既有 eframe 标注历史及 PNG 保存/撤销/贴图链路。使用 `--features native-panel-probe` 才可访问；正式 Lite/Full 默认仍用 eframe/winit，不代表 macOS GUI、Retina、Spaces、TCC、IME 或性能已经真机验收。

An offline, macOS-first screenshot and pinned-image utility written in Rust 2024, egui/eframe + wgpu and native ScreenCaptureKit. No Electron or WebView. **Development preview, not a signed/notarized public release.** Windows integration is planned after the macOS acceptance gates.

> **Progress truth:** [Current status](docs/STATUS.md) · [Remaining work / active Issue ledger](docs/REMAINING.md) · [Original 20-task tracker](docs/TASKS.md) · [Open Issues](https://github.com/wuhunyu/pic-paste/issues). This README describes the code on `main`, not work in an unmerged PR. As of 2026-10-28, manual screenshot preview (#18), self-window exclusion (#20), trusted-prefix trimming (#21), pin OCR line picking (#22), slow-scroll recovery (#23), fixed-band exclusion (#24), rounded long PNG (#25), bounded review annotation/copy/pin (#26), animation-tolerant overlap / trusted middle retake (#27), Vision word/character OCR plus offline QR (#28), trusted standalone Add-on code (#29), and experimental native annotation persistence (#45), offline Mac preflight (#46), native ellipse (#47), local Undo/Redo (#48) and bounded line/pixelate/blur (#49) and native keyboard routing (#50) and bounded native effect pixel previews (#51) are integrated. Physical mixed-DPI, Spaces, Screen Recording TCC, native clipboard/pins and performance acceptance remain **unverified**.

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


### 原生 Liquid Glass 标注工具栏（开发实验，PR #44）

通过 `cargo build --locked -p shot-macos --features native-panel-probe --bins` 构建，再执行 `cargo run --locked -p shot-macos --features native-panel-probe --bin shot-macos`。普通截图后选择 **NSPanel 原生框选（实验）**，底部将出现真正的 AppKit 原生工具栏：**选区、画笔、箭头、矩形、文字、撤销、取消**。选中画笔/箭头/矩形后在冻结截图上拖动，文字使用独立输入框（可使用系统输入法）然后在画面中单击放置；撤销只撤销最后一条标注，选区仍可八方向调整并用 Enter/确认返回现有编辑器。

**重要：本阶段标注只是原生 NSPanel 进程的本地预览，点击确认后不会同步到 eframe，也不会保存进 PNG。** 只有原有选区的 `PPSEL001` 结果会返回。需要保存带标注的截图，请继续使用现有 eframe 标注工具，等待下一阶段为标注建立有界、可验证的跨进程传输。macOS 26 使用系统 `NSGlassEffectView`；macOS 14–25 使用 `NSVisualEffectView`；辅助功能“降低透明度”优先使用不透明系统控件。所有路径都不改变截图原始像素。


### 原生标注回传与 PNG 保存（实验功能，PR #45）

使用 `cargo build --locked -p shot-macos --features native-panel-probe --bins`，再执行 `cargo run --locked -p shot-macos --features native-panel-probe --bin shot-macos`。正常截图后点击 **NSPanel 原生框选（实验）**，用系统材质工具栏绘制画笔、箭头、矩形或文字。只有点击 **确认（Enter）** 后，选区与实际标注才通过私有管道返回当前 eframe 编辑器，出现在预览中，并纳入现有撤销/重做、PNG 保存、复制和贴图链路。Esc/取消不会修改原图或历史标注。

协议仍兼容旧版 `PPSEL001`：没有标注或取消时，保持原来的 33 字节格式；有标注的确认结果使用新的版本化 `PPANN001`。消息有 2,100,000 字节内容上限，最多 128 条标注、每条画笔 2048 个点、文字 256 UTF-8 字节；主程序核对原始物理像素尺寸、显示器、选区、全部标注及**截图与编辑版本**，并在变更用户历史前先完成整幅候选图合成。非法/损坏/过期回复整批拒绝。此实现仅在可选原生实验功能中启用；真实 macOS 14+/26 GUI、混合 Retina、Spaces、TCC、Liquid Glass 和性能仍需真机验收。

### 原生 Mac 真机环境预检与验收证据（PR #46）

在 macOS 14+ 用 `cargo build --locked -p shot-macos --features native-panel-probe --bins` 构建，然后运行：

```bash
target/debug/ns-panel-probe --preflight > native-preflight.json
python3 scripts/native_acceptance.py prepare --preflight native-preflight.json --output native-acceptance.json
# 逐项完成真实 GUI/IME/TCC/Retina/PNG/性能测试，手动填写真实数据和证据
python3 scripts/native_acceptance.py verify --report native-acceptance.json
```

`--preflight` **不弹权限请求、不截屏、不创建窗口**，仅输出当前 Mac 的系统版本、逐屏 AppKit 逻辑点/缩放、系统玻璃与权限状态。报告默认所有物理测试 `not_run`，且 `release_ready=false`。完整分步骤说明、P95 目标与跨 macOS 版本限制参见 [真机验收规程](docs/native-macos-acceptance.md)。GitHub Runner 的无 GUI 预检通过不能代替真实 Mac 测试。

### 实验原生直线/马赛克/模糊（PR #49）

开启 `native-panel-probe` 功能后，NSPanel 工具栏在原有 560pt 宽的系统材质胶囊内采用双排按钮，增加直线、马赛克和模糊操作；源 RGBA 始终不可变。马赛克、模糊的**原生层仅框选并标记作用区域，并非像素级效果预览**。确认后经 `PPANN001` tag 6/7/8 到原 eframe 编辑器，通过原有 `AnnotationDocument` 使用 12px 块马赛克、6px 半径模糊或 3px 直线精确合成，再保存 PNG；可以在编辑器撤销/重做。每项区域效果最多 4,000,000 物理像素，不允许越界、0 面积或整数溢出。保持旧版 tag 1–5、`PPSEL001`、PNG-only、离线和默认 eframe 路由不变。仍需真实 macOS 14+/26 操作与像素结果验收。

### 实验 NSPanel 键盘操作（PR #50）

仅在 `--features native-panel-probe` 的实验 NSPanel 中：`⌘Z` 撤销已提交原生标注，`⇧⌘Z` 重做，Enter 确认有效选区，Escape 取消并退出面板。在**原生文字输入框激活**时，这些快捷键由 egui / macOS 输入法接管，尤其是 Escape 不能直接关闭整个截图窗口。Cmd+Z 按 `NSEvent.charactersIgnoringModifiers` 匹配当前键盘布局的字母，而不是固定美式键盘物理键位；快捷键的 KeyDown、KeyUp 成对消费，按住不会反复撤销。Cmd+Option+Z、Cmd+Control+Z 不覆盖。CI 只能验证路由纯逻辑及 macOS 编译，真实中文/日韩 IME、候选框、非美式键盘和焦点切换仍需实机验收。生产默认 UI、原图/协议及 PNG-only 不变。

### 原生马赛克/模糊像素预览（实验 PR #51）

可选 NSPanel 现在对**已提交、无重叠、且全部为马赛克/模糊的标注**显示真实像素效果；和最终 PNG 共用 `shot-image::composite` 的 12px 马赛克块、6px 模糊半径算法，源局部坐标和 RGBA/Alpha 一致。仅从源截图拷贝必要的 ROI 到独立 egui 小纹理；原始截图和 `PPANN001`/ `PPSEL001` 不修改。每次仅在提交/撤销/重做后更新缓存，拖拽未完成时仍为区域框。

为了控制资源，额外保留的 CPU 源截图不得超过 **32 MiB**，所有可预览效果区域合计不得超过 **1,000,000 像素**；超过预算、标注重叠、与线条/文字混用或没有 CPU 源时，**明确退回区域框**，确认后在既有 eframe PNG 编辑器里查看最终效果。原本允许的单次 4MP 效果编辑和 PNG 保存不受预览 1MP 预算限制。默认 eframe 截图路径、Mac 原生系统玻璃控件以及离线运行不变。实时拖拽期间不计算像素效果，尚未进行真实 Mac Retina/颜色/Alpha、Spaces/TCC、IME/资源测试。

## 2026-10-28 — 可回退的原生截图试运行入口（PR #52）

仅在 `--features native-panel-probe` 开发构建中，设置页可以**当次运行**显式选择“下一次截图优先使用原生 NSPanel”。启动成功后，目标显示器冻结画面由同目录的独立 AppKit/Metal companion 展示并编辑；严格校验 `PPSEL001` / `PPANN001`、截图/编辑 revision 后，结果仍交回稳定 eframe 的 PNG-only 保存、Undo/Redo、剪贴板与贴图流程。原生启动/回包错误回到稳定编辑器，原生取消则关闭此次自动截图；菜单栏“显示窗口”可强制退出试运行并恢复编辑器，重启默认关闭。默认 Lite/Full 包**没有**这个开关或 companion。

使用 `cargo build --release -p shot-macos --features native-panel-probe --bins --locked`，然后 `PICPASTE_NATIVE_CANARY=1 PICPASTE_VARIANT=Lite bash scripts/package-macos.sh` 或 `PICPASTE_VARIANT=Full` 生成单独的 **NATIVE-CANARY-UNSIGNED-DEV** ZIP。仅供开发测试；需要 [真实 Mac 验收](docs/native-macos-acceptance.md)，不代表已经完成默认 NSPanel 替换、macOS 14–26 Retina/Spaces/TCC/IME/性能、正式签名或公证。

### macOS Universal 2 (development-only)

For actual arm64 + x86_64 Lite/Full **UNSIGNED-DEV** packages on a macOS
build host, use `bash scripts/build-macos-universal-dev.sh`. The dedicated
[Universal 2 runbook](docs/universal2-dev-runbook.md) explains architecture
verification, ZIP checks and the path-filtered macOS workflow. This does
**not** produce a notarized release or prove Intel/Mac14 real-device QA;
existing native-architecture and optional Canary development packages remain
unchanged.


### macOS Developer ID signing (manual, private release candidate)

The [signed candidate runbook](docs/signed-release-candidate-runbook.md)
describes a manually operated, QA-evidence-bound Developer ID / Notary
workflow using `scripts/release-macos-signed.sh`. It produces **private
CANDIDATE ZIPs only**, never a public GitHub Release. Signing is blocked
without real macOS 14/26 and Intel/Apple Silicon device reviews, approved
source SHA, a valid Developer ID Team, Keychain notarization credentials and
a human provenance declaration. CI checks code syntax and failure cases
without Apple secrets or claiming real QA completion.


### Signed candidate audit (P6-T3 preparation)

The [candidate integrity and rollback hold guide](docs/release-candidate-audit.md)
explains the additional fail-closed check before private signed candidate
archives are retained. It validates exact three ZIP hashes, captured Notary
status, bundle contents and SHA linkage to real QA evidence and manual review.
Mac native mode also checks the Developer ID Team, hardened runtime and
stapled .app ticket. **Neither CI nor this audit authorizes a public
release**; real downloaded Gatekeeper, upgrades, rollback and final approval
remain required.


### Physical macOS release rehearsal (operator-only)

The [physical Gatekeeper/rollback rehearsal guide](docs/physical-release-rehearsal.md)
explains the non-destructive, read-only checks and the 7 manually observed
cases required on macOS14/26 and Apple Silicon/Intel. The associated CLI
collects source SHA, same-Team Developer ID/Notary evidence, ZIP checksums,
quarantine flags and PNG/config preservation hashes. **It never installs,
opens, signs or publishes any app automatically.** Complete physical evidence
and a separately authorized release decision are still required.


### One-click macOS ARM64 test installers (ad-hoc, not notarized)

The latest successful [Rust CI workflow](https://github.com/wuhunyu/pic-paste/actions/workflows/ci.yml) uploads an extra `picpaste-macos-test-dmg-ARM64` Actions artifact with Lite and Full **`*-UNNOTARIZED-TEST.dmg`** disk images. Mount a DMG and drag `PicPaste.app` onto the `Applications` shortcut. The stable eframe backend is used and experimental native NSPanel Canary is excluded; Full has offline OCR embedded. macOS users may need to authorize the unnotarized test app in **System Settings → Privacy & Security → Open Anyway**, then grant Screen Recording separately. See [Chinese testing instructions](docs/unsigned-macos-test-install.md). **Do not disable Gatekeeper or publish these artifacts as v1.0.0.**
