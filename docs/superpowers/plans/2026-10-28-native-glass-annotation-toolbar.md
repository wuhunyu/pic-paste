# macOS Native Liquid Glass Annotation Toolbar — Local Preview Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** Give feature-gated NSPanel an actual system Liquid Glass annotation tool capsule controlling local brush, arrow, rectangle, text and undo over immutable screenshots.

**Architecture:** Extend the retained AppKit ProbeGlassToolbar with selection-mode native NSButtons and typed GlassToolbarAction dispatch. New pure Rust shot-core::native_markup captures bounded source-local physical-pixel drawing commands. Existing egui Metal canvas renders those marks as a separate preview layer while preserving the existing, exactly 33-byte PPSEL001 selection handoff. **This PR is explicitly local preview only: marks are not yet persisted to the parent editor or PNG**, which requires a separate authenticated annotation IPC milestone.

**Tech stack:** Rust 2024, objc2 NSGlassEffectView / NSVisualEffectView / NSBox, egui 0.37, WGPU 30, GitHub Actions macOS.

## Global Constraints

- Default production eframe editor, PNG/alpha, clipboard, OCR, pins and scroll are untouched; the native-panel-probe feature is opt-in only.
- macOS 26 + NSGlassEffectView supported => native glass; macOS 14-25 => vibrancy; Reduce Transparency => opaque. Never apply material filters to captured screenshot pixels.
- Retain Cocoa NSButton targets and buttons. Use one native capsule; capture Native toolbar mouse-down before egui, preserve native event loop and WGPU teardown.
- Drawing positions are local physical source pixels, independent of global desktop origin and backed by valid finite local logical coordinates. At most 128 local commands, 2048 points per brush and 256 UTF-8 text bytes; reject out-of-bounds.
- Keep NSPanel eight handles, selection movement, explicit confirmation and Escape cancellation working with existing PPSEL001 exact 33-byte output. Undo affects only local marks.
- Real macOS GUI tests / IME / Spaces / multi Retina / accessibility / TCC / performance are device-only and do not follow from green CI.

---

### Task 1: Pure annotation state and tests

**Files:** Create crates/shot-core/tests/native_markup.rs (first), crates/shot-core/src/native_markup.rs; modify crates/shot-core/src/lib.rs.

**Interfaces:** NativeMarkupTool enum (Select, Brush, Arrow, Rectangle, Text), NativeMark enum (Brush, Arrow, Rectangle, Text); NativeMarkup::new(placement,pixels), set_tool, begin, drag, end, place_text, undo, visible, draft.

- [ ] RED: Test pixel scaling, brush points, reverse arrows/rectangles, invalid positions, text length/non-empty Unicode, bounded counts, undo and selection tool no-op. Run Linux CI and observe missing-module error.
- [ ] GREEN: Implement a simple bounded model without screenshot byte ownership. Verify cargo fmt, core tests and -D warnings in CI.

### Task 2: Native action dispatch with material compatibility

**Files:** Modify crates/shot-core/src/glass_toolbar.rs and tests/glass_toolbar.rs; apps/shot-macos/src/bin/ns-panel-probe/glass_toolbar.rs; apps/shot-macos/src/bin/ns-panel-probe.rs.

**Interfaces:** Add GlassToolbarAction variants Select/Brush/Arrow/Rectangle/Text/Undo/Cancel, preserve demo actions; ProbeGlassToolbar::new_selection(mtm, host) makes a 560pt native capsule with seven buttons and same material selection rule; update_selected_tool changes selected appearance.
- [ ] RED: Add tests for toolbar action variants and selection layout; verify missing API error before implementation.
- [ ] GREEN: Implement retained Cocoa target actions and toolbar hit testing; keep stdout exclusively for PPSEL001 in --select. Run macOS headless tests and strict Clippy.

### Task 3: Local image annotation overlay and undo

**Files:** Modify apps/shot-macos/src/bin/ns-panel-probe/panel_egui.rs and apps/shot-macos/src/bin/ns-panel-probe.rs.

- [ ] RED: Headless UI actions tests for new tool, mark commit, undo, Unicode text field and native selection still requiring explicit confirm.
- [ ] GREEN: Render native markup as egui Painter primitives above immutable screenshot texture. Text tool uses IME-capable egui TextEdit, then image click. Keep old selection-only command output unchanged.
- [ ] Verify complete same-head Linux/macOS tests, Clippy, Release, Vision IPC and Lite/Full/Add-on packaging.

### Task 4: Docs/acceptance and merge

**Files:** README.md; docs/STATUS.md; docs/REMAINING.md; docs/TASKS.md; docs/macos-window-strategy.md.
- [ ] Document build/run and local-only limitation prominently, plus precise next milestone for bounded annotation IPC and persistence in existing eframe AnnotationDocument.
- [ ] Merge only with full PR-head CI success; update Issue #5 and keep issue #5/#10 open for real Mac device acceptance and annotation persistence.

**Acceptance:** Real AppKit-native material toolbar with working local brush, arrow, rectangle, text and undo controls in feature-gated NSPanel; immutable screenshot and production eframe remain untouched. No claim that native marks have been saved or synchronized yet.
