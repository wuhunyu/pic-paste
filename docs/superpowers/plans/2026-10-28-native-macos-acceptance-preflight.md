# Reproducible Native macOS Acceptance Preflight Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a non-invasive, machine-readable real-Mac preflight + explicit physical-acceptance evidence workflow to reduce the remaining NSPanel/Liquid Glass/Retina/TCC/IME verification gap without pretending CI covers actual GUI interaction.

**Architecture:** A pure shot-core acceptance report model checks AppKit point geometry, mixed backing scales, native material selection, and records screen-recording TCC authorization as a preflight observation only. The existing opt-in `ns-panel-probe` gets a safe `--preflight` subcommand that reads real NSScreen frames, NSProcessInfo OS version, NSWorkspace Reduce Transparency, runtime NSGlassEffectView availability and calls **CGPreflightScreenCaptureAccess** (never request permission), emitting one JSON record; no NSPanel, WGPU, screenshot, network, temp file or privilege changes. A Python standard-library helper validates this JSON and generates/evaluates an evidence checklist where every physical case starts `not_run`. GitHub macos-26 CI exercises the no-GUI command and JSON shape; no visual-test pass is inferred.

**Tech Stack:** Rust 2024, shot-core, objc2 AppKit/Foundation, CoreGraphics preflight FFI on macOS only, Python 3 stdlib, GitHub Actions.

## Global Constraints

- Default `shot-macos` behavior, eframe screenshot/editor, native `--select`/PPANN001/PPSEL001, PNG-only output, clipboard, pins, OCR and manual scroll remain unchanged.
- The new diagnostic command is exclusively in the existing `native-panel-probe` gated executable; `--preflight` must not initialize NSApplication, create UI/Metal, capture pixels or request TCC. It reports existing authorization only.
- The AppKit NSScreen frame uses **bottom-left global logical points**, not ScreenCaptureKit's top-left origin or physical source pixels. Never scale a global origin with the display's Retina backing factor.
- Do not include screen content, usernames, file paths, clipboard, serial numbers or device identifiers in the JSON; only macOS version, scaling/geometry and boolean diagnostics.
- JSON records include `schema="picpaste.native-preflight/1"`, `material` one of native/vibrancy/opaque, and `interactive_gui_observed=false` unconditionally.
- Treat an empty `screens` list as a headless-runner observation, **not** a pass or an error. Invalid/NaN screen geometry is rejected.
- Explicit manual checks require evidence and remain `not_run` by default. CI cannot mark a true physical Mac 14/26, 1×/2× mixed Retina, Spaces, Chinese IME, TCC, glass accessibility or production latency target as passing.

---

### Task 1: Pure report model — RED → GREEN

**Files:** Create `crates/shot-core/tests/native_acceptance.rs` then `crates/shot-core/src/native_acceptance.rs`; modify `crates/shot-core/src/lib.rs`.

**Interfaces:**
- `NativeScreenDiagnostic::new(origin_pt:(f64,f64),size_pt:(f64,f64),backing_scale:f64)->Result<Self,String>`
- `NativePreflightReport::new(os_version:(u32,u32,u32),reduce_transparency:bool,glass_class_available:bool,tcc_screen_recording_authorized:bool,screens:Vec<NativeScreenDiagnostic>)->Result<Self,String>`
- `report.material()`, `report.has_negative_origin()`, `report.has_mixed_scales()`, `report.to_json_line()`.
- Material is determined by the existing `select_glass_material` policy; `interactive_gui_observed` is serialized as literal false.

- [ ] Add test of OS 26 glass, older/Reduce Transparency fallback, mixed 1×/2×, negative AppKit origin, empty headless screens and malformed NaN/inf/zero scale/dimensions. Assert JSON contract values, including no real-GUI assertion.
- [ ] Push and run Linux CI: verify expected error is unresolved `native_acceptance` module (not a format problem).
- [ ] Implement the bounded pure model and JSON output with no additional runtime dependency. Run core tests and strict lints; retain report for macOS executable.

### Task 2: Real macOS read-only preflight

**Files:** Modify `apps/shot-macos/src/bin/ns-panel-probe.rs`; create `apps/shot-macos/src/bin/ns-panel-probe/preflight.rs`.

**Interfaces:**
- New `ProbeCommand::Preflight` accepted only as the exact `["--preflight"]` argument list; `--select/--show` remain unchanged.
- `preflight::run` obtains MainThreadMarker, NSProcessInfo OS version, NSWorkspace Reduce Transparency, Objective-C NSGlassEffectView runtime availability, NSScreen.screens logical frame/backingScale, and CoreGraphics `CGPreflightScreenCaptureAccess` without invoking its prompt function. Exactly one JSON line to stdout.
- On non-macOS, return a stable unsupported-platform error; no screenshot or GPU initialization.
- Add unit test for CLI argument exclusivity, as well as macOS no-GUI CLI invocation in CI.

- [ ] RED: CLI parse test must fail for `ProbeCommand::Preflight`.
- [ ] GREEN: implement scoped command and AppKit/CG preflight, checking all geometry, run macOS compilation and strict Clippy.
- [ ] Set CI to run binary once with `--preflight` and validate its single JSON output; ensure TCC false and empty screens are **not** failures.

### Task 3: Explicit on-device evidence report generator

**Files:** Create `scripts/native_acceptance.py`, `tests/test_native_acceptance.py` and `docs/native-macos-acceptance.md`.

**Interfaces:**
- `python3 scripts/native_acceptance.py prepare --preflight <path> --output <path>`: parse/validate `picpaste.native-preflight/1`; generate one JSON report containing the exact environment report and explicit physical scenarios all in `not_run` state, each with blank evidence.
- `python3 scripts/native_acceptance.py verify --report <path>`: fail if any required scenario is not `passed`, lacks a nonempty evidence note, has a duplicate/unknown case, or if there are zero observable screens.
- Physical scenarios: actual NSPanel open/close/resize/focus; macOS 26 native material/14–25 fallback/Reduce Transparency; 1×/2× negative-origin monitors and 8 grips; PPANN001 draw/undo/redo/PNG pixel inspection; Chinese IME; Spaces/fullscreen; TCC deny/grant/revoke and recovery; 30 cold starts/100 hotkey P95 and idle CPU/RSS/pin frame budget.

- [ ] RED: Write Python unit tests for untrusted preflight JSON schema/types/nonfinite values, empty-screen runner report, missing/extra case and evidence, valid report acceptance, and never invent `passed`.
- [ ] GREEN: implement generator/evaluator using stdlib only, no system modifications. Run `python3 -m unittest discover -s tests -p 'test_*.py'` locally or via CI.
- [ ] Document how to capture on a physical Mac and explicitly distinguish automated verification from physical acceptance.

### Task 4: CI, documentation and merge

**Files:** Modify `.github/workflows/ci.yml`, `README.md`, `docs/STATUS.md`, `docs/REMAINING.md` and `docs/TASKS.md`.
- [ ] The macOS CI run checks `ns-panel-probe --preflight` JSON via `scripts/native_acceptance.py prepare`; this must pass with zero screens and TCC denied, without claiming hardware acceptance. Upload its `native-preflight.json` as a diagnostic artifact.
- [ ] Run complete same-head CI (Linux core, macOS launcher+NSPanel, workspace strict Clippy, release & Lite/Full/Add-on packages); squash merge only if successful.
- [ ] Update Issue #5/#10 with the reproducible physical-device command and open checkboxes. Keep both open; no fake metrics or screenshots.

**Acceptance:** A developer running the opt-in binary on a real Mac can produce a schema-validated environment report and a physical-QA checklist. CI catches schema/API regressions but **cannot** certify screen recording grant, interactive Liquid Glass appearance, real display scaling or performance. The default production screenshot backend stays unchanged.
