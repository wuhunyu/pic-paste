# PicPaste

An offline, macOS-first screenshot and pinned-image utility written in Rust 2024, egui/eframe + wgpu and native ScreenCaptureKit. No Electron or WebView. **Development preview, not a signed/notarized public release.** Windows integration is planned after the macOS acceptance gates.

> **Progress truth:** [Current status](docs/STATUS.md) · [Original 20-task acceptance tracker](docs/TASKS.md) · [Open work](https://github.com/wuhunyu/pic-paste/issues). This README describes the code on `main`, not work in an unmerged PR. As of 2026-10-26, live preview (#18), self-window exclusions (#20), trusted-prefix trimming (#21), pinned-image offline OCR line selection (#22), and **manual slow-scroll failure recovery (#23)** are merged with green PR CI. Physical mixed-DPI, Spaces, Screen Recording TCC, click-through preview and performance tests remain **unverified**.

## Implemented macOS development workflow

1. **Screenshot:** `Ctrl+Option+1` captures the selected display. The settings window can switch to the next detected monitor. The selection/editor opens in a **separate borderless native eframe/winit viewport** positioned using the selected screen's logical bounds; it is **not** a dedicated AppKit NSPanel. Drag to select, click a recognized window candidate, or adjust selection with eight handles; move an existing selection. The 9×9 source-pixel magnifier samples exact RGBA values and can copy HEX color. Mixed-DPI/Spaces and permission behavior still require physical-device verification.
2. **Annotations:** line, arrow, brush, rectangle, ellipse, pixelate, blur and **offline English/Chinese text** (choose Text, enter text/font size, click to place). Non-destructive Undo/Redo. Text rasterizes with available local system fonts; missing glyphs/system-font availability are not yet verified across clean macOS installations.
3. **Output:** screenshot file exports are **PNG-only**. Save PNG or `⌘S` quick-save; copy image to the native clipboard, or create a pin. The adjustable 0–64 px transparent rounded-corner alpha is preserved in PNG output; a bounded thumbnail previews the export. Verify color fidelity, alpha and clipboard pasting in real Mac applications before release.
4. **Pins:** `Ctrl+Option+2` pins clipboard images. Multiple independent always-on-top image viewports support opacity, scaling and mouse passthrough. Choose **识别文字 (Full)** inside a pin to request local Apple Vision OCR, then click a recognized **line** and select **复制选中行**. Move, Text Select and Passthrough are mutually exclusive; Shift+drag temporarily moves a pin in Text Select mode. `Ctrl+Option+3` recovers interactivity after passthrough. Multi-pin GPU/resource budgets, real drag/selection routing and native Spaces compatibility have not passed device acceptance.
5. **Manual long screenshots:** select a scroll region, choose vertical or horizontal capture, then scroll the target page or document manually. The worker stores matching strips on disk, rejects uncertain displacement and supports last-segment undo and **keep-first-N trusted segments** tail trimming in review; post-capture review requires explicit Save PNG or Discard. Capture size limits are 60,000 px per long edge / 200 MP. A newest-frame **live preview** (PR #18, merged) is produced at up to 420×600 pixels in a separate pointer-passthrough viewport without allocating a full long-image RGBA preview. Before starting, optionally set **开头（上／左）** and **末尾（下／右）** fixed-band exclusions in physical pixels (#24): vertical excludes sticky headers/footers, horizontal excludes sidebars; those bands are absent from both matching and the final PNG. Invalid bands are rejected before capture; 0/0 preserves existing behavior. This is a manual setting, **not automatic header detection**. **Automatic scrolling is intentionally out of scope.** If three changed frames cannot be reliably matched (for example, after scrolling too fast), the worker **pauses without advancing its trusted anchor**. Backtrack manually to an overlapping scene, select **长截图慢速重试（从暂停继续）** from the macOS menu bar, then scroll slowly in the original direction. The existing trusted strips are reused; you can repeat recovery or finish and save only the trustworthy portion. Automatic dynamic-region masking, true middle-segment repair and the 50-scene accuracy acceptance remain incomplete. The capture adapter excludes PicPaste-owned windows by PID (#20); real-device live-preview positioning, click-through behavior and complete exclusion must still be tested.
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

No verified physical macOS multi-monitor/Spaces/TCC/clipboard/performance report; manual scrolling is the explicit product behavior (no auto scroll); no QR or pin-level **word/character** OCR selection; no separate signed OCR Add-on; no x86_64 macOS release or Developer ID notarization; no Windows native implementation. `packaging/macos/Info.plist` retains development bundle ID `com.example.picpaste`.

See [task-by-task acceptance and evidence](docs/TASKS.md). **CI-green proves build and automated tests only—not real screen capture or sustained performance on a user's device.**
