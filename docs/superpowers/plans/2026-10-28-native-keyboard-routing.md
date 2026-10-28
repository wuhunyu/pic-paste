# Native NSPanel keyboard routing implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement safe, layout-independent macOS keyboard shortcuts in the experimental NSPanel without stealing keys from native text input or the parent application.

**Architecture:** Add a pure `shot-core` router that maps panel-local key input into Undo/Redo/Confirm/Cancel and remembers a consumed press until its matching release. The AppKit panel event loop consults the router before IME/egui dispatch. Events not consumed by the panel shortcut router retain existing Cocoa/IME/egui processing and the default production eframe backend remains untouched.

**Tech Stack:** Rust 2024, shot-core, objc2-app-kit NSEvent, egui/WGPU, GitHub Actions Linux/macOS.

## Global Constraints

- Minimum macOS 14.0; experimental `native-panel-probe` only, no default production backend switch.
- Use `NSEvent.charactersIgnoringModifiers()` and modifier flags instead of US-QWERTY hardware keycode 6 to recognize Cmd+Z; use documented NSEvent key codes 36/76 (Enter) and 53 (Escape).
- Do not intercept any mark shortcut, Escape or Enter when egui's text input is active; IME must handle marked-text cancellation first.
- Ignore Cmd+Option+Z and Cmd+Control+Z; no repeat-triggered duplicate actions; consume matching key-up.
- Only mutate committed annotation history on explicit undo/redo, never original captured RGBA; PPANN001, PPSEL001 and PNG-only unchanged.
- CI no-GUI tests do not prove Retina, TCC, IME, Spaces, Cocoa focus, Apple Silicon latency or visual quality.

---

### Task 1: Pure native key decision and release suppression

**Files:** Create `crates/shot-core/tests/native_key_routing.rs`; create `crates/shot-core/src/native_key_routing.rs`; modify `crates/shot-core/src/lib.rs`.

**Interfaces:** `NativeShortcutRouter::key_down(NativeKeyDown<'_>)->NativeKeyDecision`; `NativeShortcutRouter::key_up(u16)->NativeKeyDecision`; `reset()`. Action values: `Undo`, `Redo`, `Confirm`, `Cancel`; decisions: `Pass`, `Consumed`, `Action(...)`.

- [ ] **Step 1 RED:** test Cmd+z/Shift+Cmd+Z, case-insensitive UTF-8 key string, modifier isolation, keyboard repeat suppression, matching key-up suppression, text-active/other-window pass-through, Escape/Enter selection and demo behavior.
- [ ] **Step 2 VERIFY RED:** GitHub Linux Core `cargo test -p shot-core --test native_key_routing --locked` fails because the new module is not yet defined.
- [ ] **Step 3 GREEN:** implement pure router, without AppKit/unsafe and with only one outstanding consumed physical keycode.
- [ ] **Step 4 VERIFY:** Linux Core tests and strict Clippy pass; commit changes.

### Task 2: AppKit event loop

**Files:** Modify `apps/shot-macos/src/bin/ns-panel-probe.rs`.

**Interfaces:** Convert only KeyDown NSEvents to `NativeKeyDown` with `charactersIgnoringModifiers`; on KeyUp call `key_up`. Call router only for the current panel. Skip egui/IME/Cocoa forwarding for consumed events and route actions directly into existing `ProbeEgui::undo_markup/redo_markup/confirm_selection/cancel_selection`, hiding NSPanel only after Confirm/Cancel. Reset router when panel loses focus.

- [ ] **Step 1 RED:** add a headless native probe compilation test referencing the pure router in `ns-panel-probe.rs` and verify new library API compiles only after implementation; pure router tests verify routing on input without a Cocoa GUI.
- [ ] **Step 2 GREEN:** integrate without changing the event handling of mouse, resize, IME or toolbar buttons.
- [ ] **Step 3 VERIFY:** `cargo test -p shot-macos --features native-panel-probe --bin ns-panel-probe --locked`; `cargo clippy --workspace --all-targets --locked -- -D warnings` on macOS CI.

### Task 3: Docs, merge, acceptance

**Files:** Update `README.md`, `docs/STATUS.md`, `docs/REMAINING.md`, `docs/native-macos-acceptance.md`.

- [ ] Document shortcuts, text-field isolation, initial US/Mac vs alternate keyboard layout behavior, caveat on real IME/keyboard layouts and production backend.
- [ ] Validate latest same-head full CI (Linux/macOS tests + strict Clippy, Release, Lite/Full/Add-on packaging).
- [ ] After full green CI, squash-merge with expected head SHA and add issue #5 update; inspect resulting main CI.
- [ ] Preserve true status of real Mac tests (no fake 1×/2×/Spaces/TCC/IME performance sign-off).
