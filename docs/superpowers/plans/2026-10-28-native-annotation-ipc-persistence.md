# Native Annotation IPC and PNG Persistence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transfer validated NSPanel markup commands to the existing eframe editor so native Brush, Arrow, Rectangle and Unicode Text survive panel exit, can be undone/redone, and are included in the existing PNG crop/save pipeline.

**Architecture:** Preserve the legacy fixed 33-byte `PPSEL001` stdout format for no-mark and cancellation replies. Add an opt-in binary `PPANN001` reply containing the same selected display/rectangle, physical source dimensions and a bounded vector of physical-source-pixel `NativeMark` commands. The parent worker decodes strictly off the UI thread; the main event loop rejects malformed/stale/display-mismatched/size-mismatched replies and atomically validates/converts all commands into the established `shot-image::AnnotationDocument`, then refreshes the existing composite preview. No new PNG renderer, temp file or network service.

**Tech stack:** Rust 2024, shot-core protocol and markup, shot-image AnnotationDocument/composite, macOS AppKit NSPanel/egui/WGPU, GitHub Actions.

## Global Constraints

- Feature flag `native-panel-probe` only. Default eframe capture, selection, PNG-only alpha, clipboard, OCR, pins and manual long-screenshot unchanged.
- No edits to capture RGBA source. All native coordinates are the source-display-local physical pixels; never scale global desktop origin with Retina density.
- `PPSEL001` remains exactly 33 bytes and the old launcher receives it. `PPANN001` is versioned, one-shot and length-bounded (maximum 2,100,000-byte body), contains capture display ID/source width/source height, and rejects unknown tags, truncation, trailing bytes, impossible counts, empty/collapsed shapes, invalid UTF-8, out-of-range coordinates and invalid rectangles.
- Limits: at most 128 marks, 2048 brush points per command and 256 UTF-8 bytes per Text. All marks are validated before applying *any* command; never partially import. Accepted marks map to the existing cyan RGB (39,178,231), stroke 3.0 and text size 26.0.
- Only explicit confirm returns annotations; cancel/Esc/crash retains old selection and old history. Reject results after capture **or edit** revision changes; discard output from an older killed child. Never claim successful import on rejected replies.
- No real-device visual acceptance claim from GitHub Actions; macOS 14–26, mixed Retina, TCC, Spaces, IME, Liquid Glass and performance remain physical QA.

---

### Task 1: Wire protocol — tests first (shot-core)

**Files:** Create `crates/shot-core/tests/native_annotation_ipc.rs`; create `crates/shot-core/src/native_annotation.rs`; modify `crates/shot-core/src/lib.rs`.

**Interfaces:** `NativeEditReply::{Legacy(NativeSelectionReply), Annotated {selection:NativeSelectionReply,pixels:(u32,u32),marks:Vec<NativeMark>}}`; `write_to<W:Write>`, `read_from<R:Read>`, `validated_in(expected_display,pixels)`. `PPANN001` uses 8-byte magic, little-endian 4-byte payload length, then 8-byte display id, image dimensions (2×u32), rectangle (x/y i32, width/height u32), u16 mark count and typed records; Brush tag=1 with u16 count/point pairs, Arrow=2, Rectangle=3, Text=4 with u16 byte length. A cancelled selection or selected reply with zero marks uses unchanged `PPSEL001`.

- [ ] Write RED tests for byte-for-byte legacy compatibility, Unicode and all 4 mark types round trip, max bounds, wrong dimensions/display, invalid/trailing/truncated/unknown tag, corrupted UTF-8/zero-length, and failed writes.
- [ ] Run GitHub Linux CI to witness missing import error (not only fmt mismatch).
- [ ] Implement bounded codec and validation in shot-core; cargo fmt/core tests/strict Clippy green.

### Task 2: Convert native marks into existing undoable PNG annotations (shot-image)

**Files:** Create `crates/shot-image/tests/native_annotation_import.rs` and `crates/shot-image/src/native_import.rs`; modify `crates/shot-image/src/lib.rs`.

**Interfaces:** `import_native_marks(document:&mut AnnotationDocument, marks:&[NativeMark]) -> Result<usize,String>`; ensure source pixel bounds and markup limits before applying any command. Use current Annotation::{Brush,Arrow,Rectangle,Text} exactly, in original full-frame source coordinates, with stroke rgba [39,178,231,255] and width 3.0.

- [ ] RED test import/undo/redo, real composite & PNG round-trip, immutable source, negative display local scaling (handled by protocol), invalid second mark does not apply first.
- [ ] GREEN convert all marks with strict validation and then apply; cargo fmt/core-image tests/strict Clippy green.

### Task 3: Native child/parent wiring and stale-reply defenses

**Files:** Modify `apps/shot-macos/src/bin/ns-panel-probe/panel_egui.rs`, `apps/shot-macos/src/bin/ns-panel-probe.rs`, `apps/shot-macos/src/native_probe_launcher.rs`, `apps/shot-macos/src/app.rs`, `apps/shot-macos/src/main.rs`.

- [ ] RED native launcher test: fake sibling sends PPANN001 marks; bounded reader returns them and correctly rejects trailing/malformed stream; selected old PPSEL001 still works. Add app headless test for conversion into screenshot history.
- [ ] Child keeps pending output in memory until explicit selection confirm, exports only committed `NativeMarkup::visible` after selection result is accepted. If none, emit PPSEL001; Cancel always PPSEL001 cancellation.
- [ ] Parent worker uses NativeEditReply decoder; UI checks current `capture_revision` and `edit_revision`, display, physical pixels and selection bounds, validates/imports all commands then calls existing `refresh_preview`. Errors preserve the entire prior document and selected region.
- [ ] Verify macOS helper/main unit tests, full workspace tests, strict Clippy, Release and Lite/Full/Add-on developer packaging.

### Task 4: Docs and merge gate

**Files:** Modify `README.md`, `docs/STATUS.md`, `docs/REMAINING.md`, `docs/TASKS.md`, `docs/macos-window-strategy.md`.
- [ ] Describe native flow and genuine PNG persistence, limits and fail-closed behavior; identify macOS hardware/appearance/IME test gaps.
- [ ] Verify **latest PR-head** complete CI success, then squash merge to main; note new protocol and still-open hardware validation on Issue #5/#10.

**Acceptance:** Markup drawn in the experimental native NSPanel is imported into the current screenshot's AnnotationDocument and rendered into existing PNG save/copy/pin output, with undo/redo. Malformed, cancelled or stale IPC never changes existing screenshot data. Full production backend replacement and real macOS GUI/performance acceptance remain separate.
