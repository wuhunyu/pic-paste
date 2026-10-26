# Unicode text annotations (Issue #6)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Finish the previously unsupported text annotation command, including macOS Chinese typography and a clickable text editing tool.

**Architecture:** Use existing locked ab_glyph 0.2.32 and fonts supplied by the local operating system, lazily loaded only on first text draw. Keep PNG compositor deterministic once a font is loaded, preserving the source image, Undo/Redo and RGBA antialiasing.

**Tech Stack:** Rust 2024, shot-image, ab_glyph 0.2.32, egui/eframe 0.37, macOS system fonts.

## Global Constraints
- Offline only; no bundled font assets or new network calls.
- macOS Chinese system glyphs and no silent empty text.
- Bound text length and font size, preserve PNG and Undo/Redo.

### Task 1: Unicode rasterizer
**Files:** Create crates/shot-image/tests/text_annotation.rs; modify crates/shot-image/src/composite.rs, crates/shot-image/src/advanced_composite.rs, crates/shot-image/Cargo.toml, Cargo.lock.
- [x] Write failing English/CJK and invalid-size tests.
- [ ] Verify red with cargo test -p shot-image --locked.
- [ ] Implement system font loading and alpha-blended glyphs, then run green tests and Clippy.

### Task 2: Editor text tool
**Files:** crates/shot-image/src/annotation_tools.rs, apps/shot-macos/src/app.rs.
- [ ] Add Tool::Text, size input and click-to-place UI, keep brush/selection gestures unchanged.
- [ ] Verify macOS CI fmt/tests/Clippy/release/Swift helper/Lite+Full packages.
- [ ] Merge passing PR. Physical font and clipboard tests remain explicit device gates.
