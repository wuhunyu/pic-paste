# Native NSPanel Bounded Pixel Preview Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the opt-in NSPanel's outline-only committed pixelate/blur markers with genuine native pixel-effect previews when rendering is deterministic and bounded, while falling back honestly for complex or large edits.

**Architecture:** Extract a single bounded, source-local ROI in `shot-image`; run the *existing PNG compositor* with translated origin so RGBA and alpha exactly match the final parent editor. Cache each committed native redaction as an egui texture, aligned to source-local logical points, and regenerate only when undo/redo or mark history changes. Retain source CPU pixels only for modest snapshots; all over-limit/overlap/mixed-mark histories remain outline-only rather than showing a misleading preview.

**Tech Stack:** Rust 2024, shot-core NativeMark/PPANN001, shot-image composite, egui 0.37/WGPU Metal, AppKit NSPanel, GitHub Linux+macOS Actions.

## Global Constraints

- Opt-in `--features native-panel-probe` only; default eframe/winit, PNG-only, offline capture/OSS-free transport, PPANN001/PPSEL001 and source RGBA remain unchanged.
- Source CPU retention capped at **32 MiB** (3840×2160 fits; 5120×2880 does not). Total cached effect pixels capped at **1,000,000** across marks. A 4MP operation is still valid for final PNG but may use outline-only preview.
- Pixel-preview eligibility: all committed marks are Pixelate/Blur, ROI geometry valid and pairwise disjoint; no mixed vector/text marks and no overlapping effects. These cases fall back to outlines.
- A draft gesture always remains an outline. Pixel textures are generated on committed changes only; cache is invalidated on undo/redo, not recomputed for every drag frame.
- Pixelate uses existing block-size 12 and blur uses existing radius 6 in `shot-image`; patch ROI origin normalized to (0,0) for exact edge sampling and block alignment. Preserve source alpha and crop geometry.
- Display CPU/GPU use no more than 1MP of effect texture. On error, outline-only fallback; never report a false WYSIWYG preview.
- CI cannot verify physical macOS 14/26 Liquid Glass, IME, TCC, Spaces, Retina layout or perf; keep Issues #5/#6/#10 open.

---

### Task 1: Pure exact ROI compositor

**Files:** `crates/shot-image/tests/native_effect_preview.rs`, `crates/shot-image/src/native_effect_preview.rs`, `crates/shot-image/src/native_import.rs`, `crates/shot-image/src/lib.rs`.

**Interfaces:**
```rust
pub const MAX_NATIVE_PREVIEW_SOURCE_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_NATIVE_PREVIEW_PIXELS: u64 = 1_000_000;
pub fn preview_source_supported(width: u32, height: u32) -> bool;
pub fn can_preview_native_effects(marks: &[NativeMark], pixels: (u32,u32)) -> bool;
pub struct NativeEffectPatch { pub left: u32, pub top: u32, pub image: RgbaFrame }
pub fn render_native_effect_patch(source: &RgbaFrame, mark: &NativeMark) -> Result<NativeEffectPatch,String>;
```

- [ ] RED test: exact source-local patch pixel/alpha equality to a cropped full-image `composite()` output for Pixelate and Blur, including odd ROI offsets; ensure source immutability.
- [ ] RED test: two disjoint effects supported, overlapping/mixed/oversized/invalid marks refused; 4K CPU-retention supported and 5K rejected.
- [ ] Verify RED on GitHub Linux Core: test fails due to missing `native_effect_preview` module, not test formatting.
- [ ] GREEN: validate marks before allocation, copy only bounded ROI, render with shared block-size/radius constants from `native_import`. Run shot-image tests and strict Clippy; commit.

### Task 2: Cached native textures and truthful UI

**Files:** `apps/shot-macos/src/bin/ns-panel-probe/effect_preview.rs` (new), `apps/shot-macos/src/bin/ns-panel-probe/panel_egui.rs`, `apps/shot-macos/src/bin/ns-panel-probe.rs`.

- [ ] RED: headless egui test creates cache, verifies exactly one render on first committed mark and no redundant texture churn on same marks; changes to marks/undo invalidate. Test fallback when source absent or marks overlap.
- [ ] GREEN: store original source only when ≤32MiB and the selection mode is active; `EffectTextureCache::sync()` keyed by `Vec<NativeMark>` and source dimensions creates `TextureOptions::NEAREST` patches only in approved cases. Two sync checks before and after `run_ui` prevent stale paint on undo and trigger a follow-up repaint on commit. Paint cache textures over snapshot, then skip committed outlines in exact mode while retaining draft outlines.
- [ ] UI status: exact pixel preview only for eligible committed effects; otherwise explicitly state reason, not generic claim. Never render a fake GPU blur.
- [ ] Verify macOS native probe feature test, full workspace tests, strict Clippy; commit.

### Task 3: Evidence and merge

**Files:** `README.md`, `docs/STATUS.md`, `docs/REMAINING.md`, `docs/TASKS.md`, `docs/native-macos-acceptance.md`.

- [ ] Record exact preview eligibility, memory ceilings, draft-only outlines and fallback for overlap/large images; add Mac physical pixel/alpha acceptance cases.
- [ ] Verify latest same-head Linux and macOS CI success (release plus Lite/Full/Add-on packaging); merge squash with expected head SHA and record issues #5/#6.
- [ ] Check resulting main CI, do not claim real GUI acceptance.
