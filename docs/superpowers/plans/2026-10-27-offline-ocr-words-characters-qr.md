# Offline Vision word/character selection and QR decoding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add genuine Apple Vision word and character bounds to the existing screenshot/pin picker and independent explicit offline QR detection without network or fabricated geometry.

**Architecture:** Keep existing SHOC version=1 RGBA request and helper path. Extend the opt-in helper command with `--characters` (Vision `.fast`) and `--qr` (Vision QR-only barcode request). Default text request keeps `.accurate` and now returns `OcrLine.words` from Apple's own `boundingBox(for:)` on linguistic word ranges; fast mode returns individually indexed character rectangles using the same validated `OcrWord` wire type. Add a separately tagged `qr_document` envelope variant, validated strictly in Rust. All results require explicit user clicks; preserve existing per-image OCR ticket/PinOcrGate and no OCR on pointer hover.

**Tech stack:** Rust 2024, `shot-ocr-protocol`, eframe 0.37, Swift Foundation/CoreGraphics/Vision macOS 14+, GitHub Actions.

## Global constraints

- PNG-only image output; no QR auto-opening URLs, network access, new dependencies or per-hover OCR inference.
- No synthetic subdivision of Vision `.accurate` word rectangles into character rectangles; `.fast` emits native character boxes independently and may change recognition accuracy.
- Keep SHOC protocol v1 for default OCR; `qr_document` must be an exclusive response envelope alternative, with bounds validation and no unbounded allocations.
- OCR only runs on explicit request and is bound to current screenshot crop revision or live pinned source; stale results must be discarded.
- QR outputs decoded text only, explicitly copied by user; no automatic navigation to untrusted URLs.
- Preserve 80 MiB RGBA input, 8 MiB JSON output and 20s local helper execution timeout; Helper still requires codesign.
- CI shows protocol/build/limited smoke results, not 400 mixed images or real macOS 14/clipboard/accuracy acceptance.
- macOS ARM64 Lite/Full only; independent signed Add-on is a later Issue #8 task.

---

### Task 1: Protocol contract and exact hit-test selection

**Files:**
- Create: `crates/shot-ocr-protocol/tests/unit_selection.rs`
- Create: `crates/shot-ocr-protocol/tests/qr_response.rs`
- Modify: `crates/shot-ocr-protocol/src/hit_test.rs`
- Modify: `crates/shot-ocr-protocol/src/response.rs`
- Modify: `crates/shot-ocr-protocol/src/lib.rs`

**Interfaces:**
- `TextPickMode::{Line,Unit}`; `TextSelection::{Line(usize),Unit{line:usize,unit:usize}}`
- `hit_text_selection(&OcrDocument,(u32,u32),(f32,f32),TextPickMode)->Option<TextSelection>`
- `selected_selection_text(&OcrDocument,TextSelection)->Option<&str>`
- `selection_polygon(&OcrDocument,TextSelection)->Option<&[[f32;2];4]>`
- `QrDocument { image_hash, pixel_width, pixel_height, provider, codes: Vec<QrCode> }`
- `OcrResponse::QrDocument(QrDocument)`, wire envelope `qr_document`

- [ ] **Step 1 RED:** Add fixtures for word/Chinese character hit-testing with offset, no fabricated boxes, invalid positions, safe selected text; add QR JSON valid, empty, malformed polygon, oversize (>100) and missing payload cases.
- [ ] **Step 2 RED command:** `cargo test -p shot-ocr-protocol --locked`; initial failure must be missing requested API. Confirm in GitHub Linux CI.
- [ ] **Step 3 GREEN:** Implement unit selection using existing `hit_line_in_capture` and `hit_word_in_capture`; strictly validate QR envelope and return variant without changing legacy text fixture.
- [ ] **Step 4:** `cargo test -p shot-ocr-protocol --locked`, Clippy `-D warnings`.

### Task 2: Mac Vision native extraction (word, char, QR)

**Files:**
- Modify: `packaging/macos/PicPasteOCR.swift`
- Modify: `apps/shot-macos/src/ocr.rs`
- Modify: `scripts/check_ocr_helper.py`
- Test: `tests/test_ocr_helper.py` or native IPC smoke in `scripts/check_ocr_helper.py`

**Interfaces:**
- `ocr::recognize(image)` keeps accurate text, words via `enumerateSubstrings(options:.byWords)`.
- `ocr::recognize_characters(image)` sends `--characters`; returns text lines with one Unicode character per `words` entry, geometric boxes sourced from Vision fast recognition.
- `ocr::recognize_qr(image)` sends `--qr`, returns `QrDocument` with decoded payload and pixel polygon.

- [ ] **Step 1 RED:** Extend macOS IPC smoke to request `--characters` and `--qr`; assert distinct document/envelope response and input limits. Baseline helper ignores flags and has no `qr_document` output.
- [ ] **Step 2 GREEN:** Validate single allowed argv; derive `VNRecognizedText.boundingBox(for:)` rectangles, cap units per line at 4096; in QR mode call `VNDetectBarcodesRequest` with `.qr`, return only nonempty payloads up to 100 codes. Keep response framing and hash.
- [ ] **Step 3 GREEN:** Reuse bounded signed helper launch, timeout and modes; reject response variant mismatch.
- [ ] **Step 4:** full macOS CI Swift compile and SHOC native smoke.

### Task 3: Explicit screenshot/pin UI selection and QR actions

**Files:**
- Modify: `apps/shot-macos/src/app.rs`
- Modify: `apps/shot-macos/src/pin.rs`
- Test: `apps/shot-macos/src/app.rs` state tests (when feasible).

**Interfaces:**
- Screen: explicit accurate word-recognition button, fast character button, QR button; choose `TextPickMode::{Unit,Line}` and draw polygon for selected unit, copy only exact recognized string; no word->character approximation.
- Pin: choose accurate word or fast char; display and copy selected text using existing `PinOcrGate`; QR separate explicit action and copy payload; avoid stale OCR results when user leaves TextSelect.
- `WorkerResult::Qr` and `WorkerResult::PinQr`; QR async result follows same crop/image identity gate.

- [ ] **Step 1 RED:** Add state/selection tests for exact picked unit text and no selection after invalidation.
- [ ] **Step 2 GREEN:** Implement mode-aware helper requests in background thread, precise bounds on source pixels, QR result view and explicit clipboard copy. Keep screenshots/pins usable in Lite when no OCR helper.
- [ ] **Step 3 GREEN:** Full macOS workspace tests + Clippy + release/build package.

### Task 4: Evidence and synchronization

**Files:** `README.md`, `docs/STATUS.md`, `docs/TASKS.md`, `docs/REMAINING.md`; GitHub Issue #8.

- [ ] **Step 1:** Check no text copied on implicit hover, no untrusted QR URL opening, all result geometry bounded and per-click intentional.
- [ ] **Step 2:** PR complete CI green, merge, confirm `main` CI status.
- [ ] **Step 3:** Mark word/char/QR **code** complete only after verified; leave Add-on, 400-sample CER and physical macOS tests open.

## Out-of-scope truth

Genuine Vision `.accurate` word box precision and `.fast` character boxes can vary by language and OS. This feature provides provider-supplied geometry, **not** a measured OCR quality guarantee. QR discovery decodes text offline but never opens websites or executes URLs.
