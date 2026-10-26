# Configurable global hotkeys — Issue #5

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace hard-coded Ctrl+Option+1/2/3/4 digit bindings with validated, persistent and UI-editable shortcuts (Ctrl+Option+digit 1..9); handle registration conflicts without losing the existing working set.

**Architecture:** Pure `shot-core::shortcuts` owns validation and versioned parsing. `apps/shot-macos/src/hotkey.rs` owns macOS registration, rollback and on-disk settings, and the settings window owns pending edits. No new dependencies.

**Tech Stack:** Rust 2024, global-hotkey 0.8.0, egui/eframe 0.37, macOS 14+, GitHub Actions.

## Global Constraints
- No Electron/WebView/network; all image saves stay PNG-only.
- Four actions must always have distinct keys, while Ctrl+Option remains fixed in this increment.
- Registration must run on the macOS UI thread.
- Existing bindings remain functional after conflicting rebind attempts.
- Never overwrite saved configuration when registration failed.
- CI proves compilation and pure logic only; actual OS hotkey conflict/restart requires device tests.

---

### Task 1: Validated shortcuts (test first)
**Files:** Create `crates/shot-core/src/shortcuts.rs`; Modify `crates/shot-core/src/lib.rs`; Test `crates/shot-core/tests/hotkey_bindings.rs`.

- [x] Write unit tests for defaults, unique digits, transactional edits, versioned serialization, corrupted settings, and shortcut labels.
- [ ] Confirm red via `cargo test -p shot-core --test hotkey_bindings --locked`.
- [ ] Implement minimal `HotkeyBindings`, `ShortcutAction`, `ShortcutError`.
- [ ] Confirm green: core unit tests and clippy.

### Task 2: macOS hotkey transaction and UI
**Files:** Modify `apps/shot-macos/src/hotkey.rs`, `apps/shot-macos/src/app.rs`, `apps/shot-macos/src/menu.rs`; tests in hotkey module.

- [ ] Map `Code::Digit1..Digit9`, validate bindings before register; keep pressed-only event filtering.
- [ ] Re-register on the UI thread with rollback when the OS rejects a new combination; the prior hotkeys must still be usable.
- [ ] Load config from `~/Library/Application Support/PicPaste/hotkeys.conf`; use atomic, no-clobber temp write and rename; report malformed settings, fall back to defaults.
- [ ] Add four ComboBox controls in settings, `应用快捷键` and `恢复默认`, dynamic main-window labels.
- [ ] Run macOS workspace test/Clippy/Release/Swift OCR IPC/Lite+Full CI.
- [ ] Update Issue #5; do not mark completed without real-device conflict/restart/Spaces tests.
