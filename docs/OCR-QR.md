# Offline Vision text units and QR — development preview

Updated 2026-10-27. Scope: [Issue #8](https://github.com/wuhunyu/pic-paste/issues/8), [PR #28](https://github.com/wuhunyu/pic-paste/pull/28). This document describes feature code; it does not certify macOS 14 physical-device or 400-image OCR accuracy.

## Screenshot workflow

After capture and optional pixel selection, choose one explicit action in the editor:

- **识别词 (Accurate)**: ask the signed local Apple Vision helper for accurate full-line OCR; supply actual `VNRecognizedText.boundingBox(for:)` on natural word ranges. In the highlighted OCR UI choose `选词` to click one available provider word box and copy its text, or `选整行` to copy the whole recognized line.
- **识别字 (Fast)**: perform a separate fast Vision request with one native character-range box per returned Unicode character (the recognized text may differ from Accurate). Use `选字` to click and copy exactly one returned character. If the provider cannot return a box, no guessed bounding box is shown. `选整行` remains available.
- **识别二维码 (离线)**: ask the same local Helper to scan only QR symbology; list all valid decoded payload strings and their pixel bounds. Click a `复制二维码` action to copy exact payload. **Never auto-open untrusted URLs.**

Changing screenshot pixels, crop or capture revision invalidates previous OCR/QR results; asynchronous results for an older revision are discarded.

## Pinned image workflow

Every pinned viewport offers `识别词`, `识别字`, and `识别二维码` with a user click, plus mode-specific `选整行` vs `选词/字`, and explicit `复制所选文字` / `复制内容`. Leaving Text Select cancels the pending OCR selection; pointer hovering never initiates inference.

## Offline boundary and constraints

- All recognition is local to the ad-hoc-signed Full developer bundle: `Contents/Helpers/PicPasteOCR`. The executable must pass `codesign --verify --strict` before launch. Lite has no Helper, displays a clear missing-feature error, and keeps screenshot/pins/PNG functional.
- SHOC v1 request: magic, version, width, height and RGBA pixels; max **80 MiB input**, **8 MiB framed JSON response**, **20 seconds** per one-shot Helper.
- The `--characters` and `--qr` flags are strictly whitelisted command arguments. The text response preserves `OcrDocument.lines[].words`; QR has a separate validated `qr_document` response variant, max 100 payloads / 16 KiB per string, no mixed text-and-QR envelope.
- Vision Accurate gives word precision and may return an entire Chinese phrase for a subrange; Fast provides character-range rectangles, but accuracy can differ. Geometry is directly from the provider; **no invented character positions** or OCR on mouse movement.
- All image save/export functionality remains **PNG-only**. QR scanning does not save alternate formats or connect to networks.

## Current verification and remaining gates

Automated: Rust response bounds, UTF-8 unit selection and polygon hit-testing; macOS workspace tests, Clippy, Swift Helper compile/IPC, empty response and local generated QR sample decoding. These become checked only after a successful complete PR CI.

Still outstanding in Issue #8: independent signed OCR Add-on install/upgrade/removal and Developer Team ID trust, at least 400 mixed Chinese/English samples with CER thresholds, macOS 14+ physical-device text selection/pin focus and real clipboard testing. CI-green is not a physical-device acceptance report.
