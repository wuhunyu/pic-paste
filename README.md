# PicPaste

An offline, macOS-first screenshot and pinned-image utility written in Rust 2024, egui/eframe + wgpu and native ScreenCaptureKit. No Electron or WebView. **Development preview, not a signed/notarized public release.** Windows integration is planned after the macOS acceptance gates.

> **Progress truth:** [Current status](docs/STATUS.md) · [Remaining work / active Issue ledger](docs/REMAINING.md) · [Original 20-task tracker](docs/TASKS.md) · [Open Issues](https://github.com/wuhunyu/pic-paste/issues). This README describes the code on `main`, not work in an unmerged PR. As of 2026-10-27, manual screenshot preview (#18), self-window exclusion (#20), trusted-prefix trimming (#21), pin OCR line picking (#22), slow-scroll recovery (#23), fixed-band exclusion (#24), rounded long PNG (#25), bounded review annotation/copy/pin (#26), and animation-tolerant overlap / trusted middle retake (#27) are merged with green PR CI. Physical mixed-DPI, Spaces, Screen Recording TCC, native clipboard/pins and performance acceptance remain **unverified**.

## Implemented macOS development workflow

1. **Screenshot:** `Ctrl+Option+1` captures the selected display. The settings window can switch to the next detected monitor. The selection/editor opens in a **separate borderless native eframe/winit viewport** positioned using the selected screen's logical bounds; it is **not** a dedicated AppKit NSPanel. Drag to select, click a recognized window candidate, or adjust selection with eight handles; move an existing selection. The 9×9 source-pixel magnifier samples exact RGBA values and can copy HEX color. Mixed-DPI/Spaces and permission behavior still require physical-device verification.
2. **Annotations:** line, arrow, brush, rectangle, ellipse, pixelate, blur and **offline English/Chinese text** (choose Text, enter text/font size, click to place). Non-destructive Undo/Redo. Text rasterizes with available local system fonts; missing glyphs/system-font availability are not yet verified across clean macOS installations.
3. **Output:** screenshot file exports are **PNG-only**. Save PNG or `⌘S` quick-save; copy image to the native clipboard, or create a pin. The adjustable 0–64 px transparent rounded-corner alpha is preserved in PNG output; a bounded thumbnail previews the export. Verify color fidelity, alpha and clipboard pasting in real Mac applications before release.
4. **Pins:** `Ctrl+Option+2` pins clipboard images. Multiple independent always-on-top image viewports support opacity, scaling and mouse passthrough. Choose **识别文字 (Full)** inside a pin to request local Apple Vision OCR, then click a recognized **line** and select **复制选中行**. Move, Text Select and Passthrough are mutually exclusive; Shift+drag temporarily moves a pin in Text Select mode. `Ctrl+Option+3` recovers interactivity after passthrough. Multi-pin GPU/resource budgets, real drag/selection routing and native Spaces compatibility have not passed device acceptance.
5. **Manual long screenshots:** select a scroll region, choose vertical or horizontal capture, then scroll the target page or document manually. The worker stores matching strips on disk, rejects uncertain displacement and supports last-segment undo and **keep-first-N trusted segments** tail trimming in review; post-capture review requires explicit Save PNG or Discard. Capture size limits are 60,000 px per long edge / 200 MP. A newest-frame **live preview** (PR #18, merged) is produced at up to 420×600 pixels in a separate pointer-passthrough viewport without allocating a full long-image RGBA preview. Before starting, optionally set **开头（上／左）** and **末尾（下／右）** fixed-band exclusions in physical pixels (#24): vertical excludes sticky headers/footers, horizontal excludes sidebars; those bands are absent from both matching and the final PNG. Invalid bands are rejected before capture; 0/0 preserves existing behavior. This is a manual setting, **not automatic header detection**. **Automatic scrolling is intentionally out of scope.** If three changed frames cannot be reliably matched (for example, after scrolling too fast), the worker **pauses without advancing its trusted anchor**. Backtrack manually to an overlapping scene, select **长截图慢速重试（从暂停继续）** from the macOS menu bar, then scroll slowly in the original direction. The existing trusted strips are reused; you can repeat recovery or finish and save only the trustworthy portion. PR #26 adds disk-backed non-destructive long-image annotations and Undo/Redo to the review; the final annotated PNG is still streamed at full source resolution. A precise selected region can be copied to the native clipboard or pinned, subject to 4 MP total / 4096 pixels per side (no downsampling). PR #27 tolerates a *small* dynamically changing region during physical-pixel overlap matching without removing or falsifying those source pixels in the final PNG. If a reviewed middle seam is wrong, choose its one-based segment number and **retake from that segment**: all subsequent old strips are discarded; the original trusted-prefix viewport is reconstructed from disk, and the user scrolls manually from that anchor. No arbitrary deletion while retaining unverifiable later strips, automatic animation erasure in exported PNG, or 50-scene physical-device acceptance is claimed. The capture adapter excludes PicPaste-owned windows by PID (#20); real-device live-preview positioning, click-through behavior and complete exclusion must still be tested.
6. **Offline OCR (Full only):** explicitly request Apple Vision accurate recognition through the signed-at-build local Swift helper and SHOC IPC. Supports copying the full recognized text and clicking a recognized **line** on either the screenshot preview or an existing pinned image (#22). The helper has bounded input and timeout, and missing helper in Lite degrades without breaking screenshot/PNG functions. **Word/character-level boxes, QR decoding, 400-image quality benchmark and independent Add-on installer are not delivered.**

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
| `packaging/macos/PicPasteOCR.swift` | On-demand, offline Apple Vision helper for the Full variant |
| `scripts/package-macos.sh` | Developer-only Lite/Full app assembly, ad-hoc codesigning and ZIP SHA256 |

## Current limitations

There are **7 open Issues (#5–#11)** with an up-to-date [remaining-work ledger](docs/REMAINING.md). No verified physical macOS multi-monitor/Spaces/TCC/clipboard/performance report; manual scrolling is the explicit product behavior (no auto scroll); no QR or pin-level **word/character** OCR selection; no separate signed OCR Add-on; no x86_64 macOS release or Developer ID notarization; no Windows native implementation. `packaging/macos/Info.plist` retains development bundle ID `com.example.picpaste`.

See [task-by-task acceptance and evidence](docs/TASKS.md). **CI-green proves build and automated tests only—not real screen capture or sustained performance on a user's device.**

## 2026-10-27 incremental delivery — rounded long screenshots

PR [#25](https://github.com/wuhunyu/pic-paste/pull/25) is merged into `main`. Manual long-screenshot review now uses the same 0–64 physical-pixel rounded-alpha setting as regular PNG capture: its bounded thumbnail updates when the radius changes, and the final source-resolution PNG applies the mask per output scanline without building a contiguous long RGBA image. Zero-radius, PNG-only, no-overwrite, trusted-tile cleanup, and disk-backed export remain unchanged. Linux/macOS PR CI [#40432104187](https://github.com/wuhunyu/pic-paste/actions/runs/40432104187) succeeded; physical-device alpha, visual parity, and 50-scene acceptance remain open.

## 2026-10-27 — annotated long screenshots and bounded transfer (PR #26)

PR [#26](https://github.com/wuhunyu/pic-paste/pull/26) is merged on `main`. The long screenshot review can add offline source-pixel line, arrow, brush, rectangle, ellipse, pixelate, blur, and text annotations, Undo/Redo them, and export full-size PNG with rounded alpha. Each operation writes a private disk patch capped at 4 MP; export and bounded previews apply these patches while streaming source rows. Native-resolution selections can be copied or pinned with a 4-MP/4096-pixel per-edge hard cap; invalid selections are explicitly rejected. Trusted source tiles remain unchanged; successful save or explicit discard drops private temp data. [PR CI #40435034912](https://github.com/wuhunyu/pic-paste/actions/runs/40435034912) passed Linux/macOS tests, Clippy, Release and Lite/Full packaging. Full-image clipboard/pin of a 60k-edge canvas, dynamic-content filtering, true middle-strip repair, real device interaction and performance targets are not delivered or verified.

## 2026-10-27 — dynamic tolerance and safe middle-retake (PR #27)

Merged [PR #27](https://github.com/wuhunyu/pic-paste/pull/27) adds eight-lane displacement proposals and an independent stable-pixel consensus (up to 25% changed sample tolerance, with unique-shift ambiguity rejection). Repeated wallpaper, animation-only and oversized dynamic regions remain untrusted. Pixels are excluded **only from matching**, never from saved PNG. The review can select an erroneous segment (2..N), retain exactly the previous trusted prefix, delete the selected segment and its entire suffix, and return to manual capture on the same original physical display/ROI/axis with a reconstructed native viewport anchor. Cancellation or worker errors preserve a reviewable prefix; preview is rebuilt after partial progress. The last-viewport reconstruction opens only intersecting strips, not every strip at once, and the live preview reuses its GPU texture. Full Linux/macOS PR [CI #40440066887](https://github.com/wuhunyu/pic-paste/actions/runs/40440066887) verified automated build/tests. Physical-scene acceptance and in-place safe suffix splicing remain open.
