# PicPaste

Offline screenshot and pinned-image utility. Rust 2024 + **egui/eframe 0.37.0** + WGPU; native ScreenCaptureKit capture on macOS.

> **Development prototype, not an audited or production-ready app.** GitHub Actions compile/test/lint does not grant screen recording permissions or verify multi-display, Spaces, or mouse-passthrough behavior.

## macOS preview

The current prototype supports:

- Global hotkey **Control + Option + 1** to capture the primary display with ScreenCaptureKit.
- Drag a region inside the captured preview, or accept the full display.
- Save a PNG to `~/Pictures/PicPaste/`, copy an image to the system clipboard, or pin it in a separate always-on-top window.
- Adjustable PNG corner transparency (default 16 physical pixels).
- Global hotkey **Control + Option + 2** to pin the clipboard image, or recover existing passthrough pins.
- Multiple pinned viewports, opacity controls and a recovery action.
- Native macOS status menu with capture, pin, recover, show and quit actions.

**Known limitations:** selection is currently inside the main preview rather than a full-screen overlay; active pinned windows may be present in subsequent screen captures; native macOS window behavior and high-DPI coordinate calculations need real-device acceptance. Annotation history and compositing are core modules, but a full toolbar with all tools is not yet integrated. OCR recognition, live scrolling capture, Windows integration and production signing/notarization are not yet delivered.

## Build

Use a recent Xcode/SDK (the `screencapturekit 11` Swift/Metal dependency requires new SDK types) with Rust Stable. Deployment target is macOS 14.

```sh
export MACOSX_DEPLOYMENT_TARGET=14.0
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release -p shot-macos
bash scripts/package-macos.sh
```

`dist/PicPaste-macos-*-UNSIGNED-DEV.zip` is an **unnotarized development package**, suitable only for controlled testing. Developer ID distribution requires a real bundle ID, code-signing identity, and notarization; these are deliberately not faked in CI.

## Workspace

| Crate | Responsibility |
| --- | --- |
| `shot-core` | Pixel geometry, selection, pin interaction and state machines |
| `shot-image` | PNG-only export, image compositing, rounded alpha |
| `shot-platform` | Platform-native capture adapter |
| `shot-scroll` | Scroll matching, canvas bounds and state |
| `shot-ocr-protocol` | Bounded, versioned offline IPC request format |
| `apps/shot-macos` | macOS UI, hotkeys and pinned viewports |
| `apps/shot-ocr-helper` | Future independently packaged offline OCR helper |

See [development ledger](docs/STATUS.md) for completed vs pending work and original v1.2 implementation plan for acceptance requirements. All image file exports are **PNG only**.
