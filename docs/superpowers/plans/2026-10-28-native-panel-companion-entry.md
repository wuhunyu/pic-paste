# macOS Native NSPanel Companion Launch Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Provide an explicitly opt-in, crash-isolated entry in the PicPaste settings window that launches the real NSPanel/Metal/egui/Liquid Glass companion for manual acceptance while keeping every production capture, selection, pin, and PNG action on the existing eframe backend.

**Architecture:** Both Rust binaries already share one Cargo package; the native companion `ns-panel-probe` has its own AppKit event loop and must not run inside eframe. An optional feature-gated production module locates only a same-directory executable named `ns-panel-probe`, refuses symlinks/nonexecutable files, and spawns it with the fixed `--show` argument. The main app retains the child handle, reuses it on repeated clicks, and terminates it on exit. Errors are surfaced in the main window, not silently reinterpreted as successful NSPanel rendering.

**Tech Stack:** Rust 2024, eframe 0.37, existing `native-panel-probe` feature, macOS GitHub Actions (macos-26).

## Global Constraints

- Do not migrate the production screenshot/selection editor or pins in this PR. No risk to the existing hotkeys, ScreenCaptureKit, OCR or PNG/Alpha handling.
- Only show the native experiment entry if `shot-macos` is explicitly compiled with `--features native-panel-probe`.
- The companion is a separate `NSPanel` process: never run a second AppKit event loop inside the eframe process or reinterpret a winit NSWindow as NSPanel.
- A missing/rejected helper must leave the current capture backend functioning and must not attempt a shell/PATH fallback or launch a user-selected binary.
- The companion has no screenshot/selection parity. Report it as a manual visual/input test entry only, never as production NSPanel integration.
- The `--check` CI path remains headless. A green CI does not prove glass appearance, input/IME candidate positioning, Spaces, focus, TCC or performance on real macOS 14+/26 machines.

---

### Task 1 — Test-first same-directory companion resolution and process lifetime

**Files:** `apps/shot-macos/src/main.rs` (RED tests and cfg module), `apps/shot-macos/src/native_probe_launcher.rs` (implementation).

**Interfaces:**
- `pub(crate) fn sibling_probe_path(executable: &Path) -> Result<PathBuf, String>`
- `pub(crate) fn validate_probe_path(path: &Path) -> Result<(), String>`
- `pub(crate) enum ProbeLaunchResult { Started, AlreadyRunning }`
- `pub(crate) struct NativeProbeLauncher { child: Option<Child> }`
- `NativeProbeLauncher::launch(&mut self) -> Result<ProbeLaunchResult, String>`; `launch_from(&mut self, executable: &Path)` for real filesystem regression tests.

- [x] RED: add feature-gated tests asserting absolute sibling path resolution, refusal of relative paths, absent/nonexecutable/symlink helpers and no duplicate child on a repeated click.
- [x] Run: `cargo test -p shot-macos --features native-panel-probe --bin shot-macos --locked` on macOS CI. Expected initial missing module or missing symbol compilation failure.
- [x] GREEN: implement `native_probe_launcher.rs`; derive the binary name from `current_exe`, reject symlinks, verify the execute bit, use `Command::new(...).arg("--show").spawn()` (never `sh -c`); retain the `Child` while the demo runs and kill/wait on Drop.
- [ ] Verify original tests, new launch tests and strict Clippy on macOS; no global hotkey or capture code changes.

### Task 2 — Feature-gated settings button and CI

**Files:** `apps/shot-macos/src/app.rs`, `.github/workflows/ci.yml`, `docs/macos-window-strategy.md`, `docs/REMAINING.md`.

**Interfaces:** 
- New `PicPasteApp.native_probe: NativeProbeLauncher` field compiled only with `native-panel-probe`.
- A collapsed `原生 NSPanel 实验（不替换截图）` section on the normal settings screen. The only button starts the companion and reports Started/AlreadyRunning/missing executable without claiming presentation success.

- [x] Add `#[cfg(feature = "native-panel-probe")]` field/default and settings-only button; preserve `overlay::capture_builder` and its capture hotkeys unchanged.
- [x] Extend macOS CI with `cargo test -p shot-macos --features native-panel-probe --bin shot-macos --locked` and `cargo clippy -p shot-macos --features native-panel-probe --bin shot-macos --locked -- -D warnings`. Existing probe binary and default bundle build gates remain.
- [x] Record precise manual use without representing a dev preview as a signed release.

### Task 3 — Acceptance and rollback

- [ ] Verify **one final PR head** across Linux core, macOS feature-gated launcher and companion tests, default workspace+Release, strict Clippy, Lite/Full/Add-on artifacts.
- [ ] On physical macOS GUI, build both sibling binaries: `cargo build -p shot-macos --features native-panel-probe --bins --locked` and run `cargo run -p shot-macos --features native-panel-probe --bin shot-macos --locked`; open the experimental section and test both startup and repeat clicks. Move the NSPanel across 1×/2× displays, test mouse/keyboard/CJK IME/VoiceOver/Reduce Transparency/Spaces.
- [ ] Test removing `target/debug/ns-panel-probe` (helper missing) and launching the default app without the feature: existing screenshot shortcuts, overlay and PNG must still work.
- [ ] Do not close Issue #5 or switch production capture/pins to NSPanel without physical tests and complete editor parity. Rollback is recompiling without `native-panel-probe` or reverting this isolated PR.

