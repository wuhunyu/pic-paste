# PicPaste

A small, offline screenshot and pinned-image utility. **macOS-first**, Rust 2024 + egui/eframe 0.37 + wgpu, with a native ScreenCaptureKit capture adapter. Windows support comes after macOS release acceptance.

> **Development preview only.** A green macOS GitHub Actions build means the code compiles, passes automated tests and can be packaged. It does **not** establish real-device Screen Recording permission behavior, mixed-DPI/Spaces positioning, input passthrough reliability or performance limits.

## Available macOS preview workflow

- **Ctrl+Option+1:** snapshot the primary display and open the screenshot selection/editor. The screenshot is captured **before** opening the fullscreen main-editor viewport. Drag to select; PR #3 adds eight edge/corner handles, drag-to-move, 9x9 pixel magnifier, and exact HEX eyedropper copy. Esc cancels.
- Annotate using **line, arrow, brush, rectangle, ellipse, pixelate, blur**; undo/redo is non-destructive. Text annotation is not yet implemented.
- **Save PNG**, **Copy image**, or **Pin** the selected/composited screenshot. The corner radius is configurable from 0–64 physical pixels; image file formats are always **PNG only**, with alpha preserved.
- **Ctrl+Option+2:** create a pin from the clipboard image. Supports multiple independent always-on-top panes, per-pin opacity, scale and optional mouse passthrough. **Ctrl+Option+3** disables all passthrough as a recovery shortcut.
- **Manual scrolling screenshots:** select a single scroll region in the screenshot editor and choose vertical (↓) or horizontal (→). The main window hides while you scroll the target application. Use **Ctrl+Option+4** or the menu bar to finish, or choose Cancel from the menu. The implementation rejects uncertain overlaps and respects a 60,000px long-edge / 200MP area limit.
- **Scroll review (in development PR #3):** finish returns a bounded thumbnail, accepts removal of the last trusted segment, and waits for explicit **Save PNG** or **Discard**. It keeps raw tile strips on disk rather than allocating a full RGBA long image; exported PNG is streamed row-by-row.
- **OCR (Full package):** explicit '识别文字' action invokes a separate local Apple Vision Accurate helper. Copy all recognition text; PR #3 additionally supports choosing an OCR line on the capture preview by clicking its Vision-detected polygon. **Lite** contains no OCR helper. The OCR helper is never required for normal screenshot, pin or scroll operations.

### Preview limitations

- The fullscreen screenshot editor uses the main eframe viewport. It is **not yet** a separate AppKit NSPanel, and only the primary display is currently captured. Mixed-DPI/negative-origin and fullscreen Spaces behavior remain real-device acceptance gates.
- OCR line hit-testing is being integrated for screenshot previews; *pin text-selection mode*, exact word/character hit testing without provider word boxes, and independent offline OCR Add-on installation remain incomplete.
- Manual scroll capture and post-capture review are implemented at the code level, but automatic scroll injection, live continually updated preview, adjustable sticky-header masks, middle-segment editing and empirical 50-scene accuracy verification are pending.
- Window candidate recognition, true native cross-display magnifier geometry, text annotations, configurable global hotkeys and Windows support are not delivered. The basic 9x9 screenshot-local magnifier/eyedropper in PR #3 is not a substitute for mixed-DPI real-device acceptance.
- Release artifacts are ad-hoc signed **development-only** and are not Developer-ID signed or Apple-notarized. The advertised macOS 14 deployment target has not yet been confirmed on an actual macOS 14 machine.

## Build and tests (macOS)

Requires a recent Xcode/macOS SDK and Rust stable. The current `screencapturekit 11` dependency builds against the macOS 26 SDK on Actions, although `MACOSX_DEPLOYMENT_TARGET` is 14.0.

```sh
export MACOSX_DEPLOYMENT_TARGET=14.0
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --release -p shot-macos --locked
swiftc -O -target arm64-apple-macos14.0 -framework Vision \
  -framework CoreGraphics -framework Foundation -framework CryptoKit \
  packaging/macos/PicPasteOCR.swift -o target/release/PicPasteOCR
python3 scripts/check_ocr_helper.py target/release/PicPasteOCR

PICPASTE_VARIANT=Lite bash scripts/package-macos.sh
PICPASTE_VARIANT=Full bash scripts/package-macos.sh
```

GitHub Actions publishes `picpaste-macos-dev-ARM64` with **Lite and Full** unsigned-for-distribution (ad-hoc signed) bundles. Look in the relevant [Actions run](https://github.com/wuhunyu/pic-paste/actions) → Artifacts. The last fully verified merged baseline is **[main CI #111](https://github.com/wuhunyu/pic-paste/actions/runs/40270735124)**; PR #3 must pass its own complete CI before merge.

## Workspace

| Component | Purpose |
| --- | --- |
| `shot-core` | Pixel geometry, input and selection states, screenshot lifecycle |
| `shot-image` | PNG-only crop/export, alpha rounding, non-destructive annotations |
| `shot-platform` | Native macOS ScreenCaptureKit capture adapter |
| `shot-scroll` | Conservative overlap matching, disk-backed RGBA tiles, streaming PNG and bounded thumbnails |
| `shot-ocr-protocol` | Bounded, versioned OCR pipe protocol, validated geometry, line hit tests |
| `apps/shot-macos` | Main native UI, keyboard shortcuts, status menu, pins and scroll review |
| `packaging/macos/PicPasteOCR.swift` | Optional isolated system-Vision offline OCR helper in Full package |

For precise delivery and acceptance gaps, see [development status](docs/STATUS.md). The original v1.2 specification remains the authority for P0–P6 goals; green CI is not a substitute for its measured device acceptance criteria.
