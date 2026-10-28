# Native NSPanel NSTextInputClient / CJK IME Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development and superpowers:systematic-debugging when implementing this plan. Run each gate in order.

**Goal:** Make the opt-in AppKit NSPanel + Metal + egui proof accept native Chinese/Japanese/Korean marked-text composition and place candidate UI near the egui text caret, without changing production windows or screenshot pixels.

**Architecture:** Pure `shot-core::ime::ImeComposition` owns the UTF-16-to-Unicode preedit state and produces immutable `ImeChange` messages. A main-thread, retained Objective-C `NSTextInputClient` inside `ProbeImeBridge` receives `NSTextInputContext` callbacks, buffers changes, and computes candidate rectangles from the actual egui caret and the NSView/window coordinate transforms. The existing `ProbeEgui` event loop forwards exactly one `egui::Event::Ime` or `egui::Event::Text`, preserving normal key/shortcut delivery.

**Tech Stack:** Rust workspace, `objc2=0.6.5`, `objc2-app-kit=0.3.2`, `objc2-foundation=0.3.2`, `egui=0.37.0`, `wgpu=30.0.1`, GitHub Actions macOS runners.

## Global Constraints

- All new Cocoa/unsafe code is behind the optional `native-panel-probe` feature and `target_os = "macos"`.
- Production `eframe/winit` screenshot/selection/pin windows, PNG export, shortcut bindings and offline execution are unchanged.
- The probe CLI `--check` is headless and cannot be used to claim real input, cursor geometry or macOS Liquid Glass has been verified.
- Retain the input client while its context is alive and release the context and host view before destroying the panel's WGPU surface.
- A failed CI or physical-Mac acceptance is a hard stop for closing Issue #5.

---

### Task 1: UTF-16 composition state

**Files:**
- Create: `crates/shot-core/src/ime.rs`
- Modify: `crates/shot-core/src/lib.rs`
- Test: `crates/shot-core/tests/ime_composition.rs`

**Interfaces:**
- `ImeComposition::preedit(text, selection_start_utf16, selection_len_utf16) -> ImeChange::Preedit`
- `ImeComposition::commit(text) -> ImeChange::Commit`
- `ImeComposition::cancel() -> Option<ImeChange>`
- `marked_utf16_len`, `selected_range_utf16` for NSTextInputClient query selectors

- [x] **Step 1:** Add RED tests for Chinese preedit → commit, emoji surrogate boundary, empty dismiss, focus loss and direct text.
- [x] **Step 2:** Verify the RED CI fails with `unresolved import shot_core::ime` (run `40610916828`).
- [x] **Step 3:** Implement the pure UTF-16 conversion and state reset. Fix the empty-preedit case exposed by test `empty_preedit_and_focus_loss_clear_without_fabricating_commits`.
- [ ] **Step 4:** Verify the latest `cargo test -p shot-core -p shot-image -p shot-scroll -p shot-ocr-protocol --locked` CI job is green.

### Task 2: Native client + exact egui event routing

**Files:**
- Create: `apps/shot-macos/src/bin/ns-panel-probe/ime.rs`
- Modify: `apps/shot-macos/Cargo.toml`
- Modify: `apps/shot-macos/src/bin/ns-panel-probe/panel_egui.rs`
- Modify: `apps/shot-macos/src/bin/ns-panel-probe.rs`

**Interfaces:**
- `ProbeImeBridge::new(mtm, host)`, `set_active(bool)`, `handle_key(&NSEvent)`, `drain() -> Vec<ImeChange>`, `set_cursor(Option<egui::Rect>)`
- `egui_event_for_ime_change(ImeChange) -> Option<egui::Event>`
- `ProbeEgui::wants_text_input()`, `ime_cursor_rect()`, `push_ime_change(ImeChange)`

- [x] **Step 1:** Define the minimal native client action queue contract test on macOS.
- [x] **Step 2:** Add retained main-thread `NSTextInputClient` selector callbacks and one `NSTextInputContext` per probe.
- [x] **Step 3:** Convert NSTextInputClient marked and committed strings to egui preedit and commit once; exclude direct ASCII injection while the native context is active.
- [x] **Step 4:** Use `output.platform_output.ime.cursor_rect` and `NSView.convertRect_toView` / `NSWindow.convertRectToScreen` for candidate geometry; clear state on focus loss.
- [ ] **Step 5:** Verify `cargo fmt --all -- --check` and `cargo test -p shot-macos --features native-panel-probe --bin ns-panel-probe --locked` in macOS CI, then run `cargo clippy -p shot-macos --features native-panel-probe --bin ns-panel-probe --locked -- -D warnings`.

### Task 3: Automated and physical acceptance

**Files:**
- Modify: `docs/REMAINING.md`, `docs/STATUS.md`
- Reference: GitHub Issue #5 and Issue #10

- [ ] **Step 1:** Verify all CI jobs from the same final PR head: Linux core, macOS opt-in probe tests, workspace tests, strict Clippy, macOS package Lite/Full/Add-on.
- [ ] **Step 2:** On macOS 14+, run:

```bash
cargo run -p shot-macos --features native-panel-probe --bin ns-panel-probe -- --check
cargo run -p shot-macos --features native-panel-probe --bin ns-panel-probe -- --show
```

- [ ] **Step 3:** In the focused text field, exercise Pinyin `nihao → 你好`, Japanese/Korean marked-text replacement and candidate selection, ASCII and Option dead keys, Backspace/Enter, mouse selection, focus switches, Escape/Close, and screenshots. Check for double insertion and leftover preedit.
- [ ] **Step 4:** Repeat at 1x/2x Retina scales, mixed DPI, Spaces/fullscreen and with Reduce Transparency; capture device/OS/keyboard locale/input method/screenshots/failure steps in Issue #5.
- [ ] **Step 5:** Only consider migrating official capture/pin panels after the physical acceptance gates. Do not close Issue #5 with compile-only evidence.

**Rollback:** The opt-in `native-panel-probe` feature can remain disabled (the default), leaving the production `eframe/winit` app unaffected. If needed, revert the experimental PR without altering `data/`, screenshot PNGs, or main-window behavior.
