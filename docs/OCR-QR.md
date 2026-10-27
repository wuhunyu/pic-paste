# Offline Vision text units and QR — development preview

Updated 2026-10-27. Scope: [Issue #8](https://github.com/wuhunyu/pic-paste/issues/8), merged [PR #28](https://github.com/wuhunyu/pic-paste/pull/28), and standalone Add-on work. This document describes feature code; it does not certify macOS 14 physical-device or 400-image OCR accuracy.

## Screenshot workflow

After capture and optional pixel selection, choose one explicit action in the editor:

- **识别词 (Accurate)**: ask the signed local Apple Vision helper for accurate full-line OCR; supply actual `VNRecognizedText.boundingBox(for:)` on natural word ranges. In the highlighted OCR UI choose `选词` to click one available provider word box and copy its text, or `选整行` to copy the whole recognized line.
- **识别字 (Fast)**: perform a separate fast Vision request with one native character-range box per returned Unicode character (the recognized text may differ from Accurate). Use `选字` to click and copy exactly one returned character. If the provider cannot return a box, no guessed bounding box is shown. `选整行` remains available.
- **识别二维码 (离线)**: ask the same local Helper to scan only QR symbology; list all valid decoded payload strings and their pixel bounds. Click a `复制二维码` action to copy exact payload. **Never auto-open untrusted URLs.**

Changing screenshot pixels, crop or capture revision invalidates previous OCR/QR results; asynchronous results for an older revision are discarded.

## Pinned image workflow

Every pinned viewport offers `识别词`, `识别字`, and `识别二维码` with a user click, plus mode-specific `选整行` vs `选词/字`, and explicit `复制所选文字` / `复制内容`. Leaving Text Select cancels the pending OCR selection; pointer hovering never initiates inference.

## Offline boundary and constraints

- **Full:** the built-in `Contents/Helpers/PicPasteOCR` is preferred. Its code signature is verified before launch; if the embedded helper is corrupt, OCR fails closed instead of switching to a potentially unrelated external executable.
- **Lite:** without the embedded helper, the app examines one fixed path: `~/Library/Application Support/PicPaste/AddOns/OCR/PicPasteOCR`. It rejects symlinks and malformed files; requires a strict code-signature verification, matching **nonempty** Apple Team IDs for the host app and helper, then runs a bounded version-only handshake before OCR/QR. If missing/untrusted/incompatible, only OCR/QR is unavailable; screenshot, PNG, pins and long capture continue working.
- **Ad-hoc development builds** have no Apple Team ID. Two ad-hoc signatures are not a trust relationship: standalone Add-ons are deliberately rejected by the installer and runtime until both the host and Add-on carry properly matching Developer ID signatures.
- SHOC v1 request: magic, version, width, height and RGBA pixels; max **80 MiB input**, **8 MiB framed JSON response**, **20 seconds** per one-shot Helper.
- The `--characters`, `--qr` and `--addon-info` flags are strictly whitelisted command arguments. `--addon-info` returns exactly `PICPASTE-OCR-ADDON/1 SHOC/1\n` without reading a SHOC image or running Vision. The text response preserves `OcrDocument.lines[].words`; QR has a separate validated `qr_document` response variant, max 100 payloads / 16 KiB per string, no mixed text-and-QR envelope.
- Vision Accurate gives word precision and may return an entire Chinese phrase for a subrange; Fast provides character-range rectangles, but accuracy can differ. Geometry is directly from the provider; **no invented character positions** or OCR on mouse movement.
- All image save/export functionality remains **PNG-only**. QR scanning does not save alternate formats or connect to networks.

## Current verification and remaining gates

Automated: Rust response bounds, UTF-8 unit selection and polygon hit-testing; macOS workspace tests, Clippy, Swift Helper compile/IPC, empty response and local generated QR sample decoding. PR #29 full CI [#40456135637](https://github.com/wuhunyu/pic-paste/actions/runs/40456135637) passed these automated gates, including standalone development packaging and ad-hoc trust refusal.

Still outstanding in Issue #8: real Developer ID–signed standalone Add-on installation and user interaction on macOS 14+, successful notarization/clean-machine Gatekeeper distribution (also #9), at least 400 mixed Chinese/English samples with CER thresholds, cancellation/Helper crashes and real clipboard testing. CI-green is not a physical-device acceptance report.

## Standalone Add-on installation — source and signing workflow

The separate Add-on is a **single local Swift executable** with a signed, embedded API handshake; there is no unsigned external manifest or network provider. The installed location is fixed and user-owned. The installer never modifies `PicPaste.app` and needs no sudo.

```sh
# Build the helper locally first (the existing macOS CI also builds it).
# Developer ID is required for installable packages; ad-hoc is dev only.
PICPASTE_SIGN_IDENTITY="Developer ID Application: Your Organization (TEAMID)" \
  bash scripts/package-ocr-addon.sh

# Extract the signed PicPasteOCR executable from the ZIP, then:
bash scripts/manage-ocr-addon.sh install "/Applications/PicPaste.app" "/path/to/PicPasteOCR"
bash scripts/manage-ocr-addon.sh status "/Applications/PicPaste.app"
bash scripts/manage-ocr-addon.sh uninstall
```

The installer stages its copy under `~/Library/Application Support/PicPaste/AddOns/OCR/`, verifies strict signatures and matching nonempty Developer Team IDs, executes only the signed `--addon-info` handshake, replaces the old version atomically and restores the old executable on failed verification. The runtime repeats the same trust and compatibility checks at each explicit OCR request; no `PATH` or environment override chooses executable code.

The separate ad-hoc development artifact `PicPaste-OCR-AddOn-macos-*-UNSIGNED-DEV.zip` is intentionally **not installable**. It is a packaging/signature smoke-test fixture, not a signed distribution. Real signed, notarized end-user packages require Apple credentials and clean-device checks, which remain outside CI evidence.
