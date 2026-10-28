# Native NSPanel Frozen Screenshot Bridge — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In an explicitly opted-in developer build, display *real already-captured screenshot RGBA pixels* in the existing independent AppKit NSPanel/Metal/egui companion, without changing the production screenshot editor, PNG output, pins, or OCR.

**Architecture:** Stream one bounded, versioned binary envelope from PicPaste to its strictly same-directory companion over anonymous standard input, then render the immutable frame through the existing egui-wgpu panel surface. The parent retains the child and sends the payload asynchronously to avoid blocking the UI. The experimental screenshot toolbar button is only visible in the selected screenshot editor and only when `native-panel-probe` is compiled. No disk screenshot data is left behind. This is a read-only frozen-image integration gate, **not** a production native selection editor migration.

**Tech Stack:** Rust 2024, shot-core, eframe/egui 0.37, AppKit NSPanel, WGPU 30 Metal, GitHub Actions macOS-26.

## Global Constraints

- Keep the default `shot-macos` executable, ScreenCaptureKit capture worker, legacy eframe selection/annotations, PNG-only alpha, pins, OCR, and scrolling untouched.
- The native panel is a separate owned process, never a reinterpret-cast of winit's `NSWindow`; retain the WGPU-before-NSView teardown.
- Never pass screenshot bytes in shell arguments, unbounded files, or a world-readable temporary path; use piped stdin only and an explicit fixed `--snapshot` argument.
- Fail closed on nonfinite logical points, bad RGBA lengths, non-positive dimensions, dimensions exceeding 8192, and payloads exceeding 128 MiB *before allocation*.
- Retain target display's **logical point** placement separately from its full-display **local physical pixel** RGBA origin. Do not multiply global desktop origin by scale.
- The experimental view must never change or save captured pixels; toolbar glass material stays above, not within, the screenshot image.
- Headless `--check` and Rust CI cannot prove AppKit presentation or TCC, mixed DPI, Spaces, input, frame pacing, accessibility, or macOS 26 Liquid Glass. Issue #5/#10 remain open.
- Preserve exact helper-executable validation and parent process cleanup; any new snapshot process must be terminated before a new capture.

---

### Task 1 — Versioned bounded native screenshot protocol

**Files**
- Create `crates/shot-core/src/native_snapshot.rs`
- Modify `crates/shot-core/src/lib.rs`
- Test `crates/shot-core/tests/native_snapshot.rs`

**Interfaces**
- `NativeSnapshot::new(placement: OverlayPlacement, image: RgbaFrame) -> Result<Self,String>`
- `NativeSnapshot::validate_dimensions(width:u32,height:u32) -> Result<usize,String>`
- `NativeSnapshot::write_to<W: Write>(&self, sink:&mut W) -> Result<(),String>`
- `NativeSnapshot::read_from<R:Read>(source:&mut R) -> Result<Self,String>`

- [ ] Write tests **first**, including a negative logical monitor origin, exact RGBA round-trip, malformed magic, truncated payload, invalid dimensions/length, and rejection of a payload larger than 128 MiB.
- [ ] Observe initial Linux CI RED because `shot_core::native_snapshot` is missing.
- [ ] Implement explicit little-endian header and bounded read, with no unsafe code or whole-image staging buffer; use:
  ```rust
  let image = RgbaFrame::new(width, height, rgba).map_err(|error| format!("{error:?}"))?;
  let placement = OverlayPlacement::new(DisplayId(display), (origin_x, origin_y), (width_pt, height_pt))
      .map_err(|error| format!("{error:?}"))?;
  ```
- [ ] Verify `cargo test -p shot-core --test native_snapshot --locked` and strict core Clippy, then commit.

### Task 2 — Gated CLI and real frame in existing egui/Metal panel

**Files**
- Modify `apps/shot-macos/src/bin/ns-panel-probe.rs`
- Modify `apps/shot-macos/src/bin/ns-panel-probe/panel_egui.rs`

**Interfaces**
- `ProbeCommand::Snapshot`, triggered solely by `--snapshot` and reading a single `NativeSnapshot` from stdin before AppKit is initialized.
- `native::show(snapshot: Option<NativeSnapshot>)`, `ProbeEgui::new(device,format,snapshot: Option<RgbaFrame>)`.

- [ ] First add headless CLI/test assertions that `--snapshot` parses, malformed stream is rejected, and a 2×2 immutable RGBA snapshot generates egui image paint shapes.
- [ ] Observe expected RED CI on the missing CLI variant or changed renderer API.
- [ ] Load an egui RGBA texture and paint it fitted inside the panel content, without modifying screenshot bytes or changing the demo mode.
- [ ] Verify `--check` remains GPU/window-free; run native feature tests and strict macOS Clippy; commit.

### Task 3 — App-to-companion IPC and explicit screenshot UI entry

**Files**
- Modify `apps/shot-macos/src/native_probe_launcher.rs`
- Modify `apps/shot-macos/src/main.rs` (new gated child transport tests)
- Modify `apps/shot-macos/src/app.rs`

**Interfaces**
- `NativeProbeLauncher::launch_snapshot(snapshot: NativeSnapshot) -> Result<ProbeLaunchResult,String>`
- `NativeProbeLauncher::launch_snapshot_from(executable:&Path,snapshot:NativeSnapshot) -> Result<ProbeLaunchResult,String>`

- [ ] Add failing macOS tests for a fixed sibling executable and no `PATH`/symlink fallback; test no duplicate child, and a nonblocking piped stdin snapshot payload.
- [ ] Observe RED on new missing launcher methods.
- [ ] Implement `Command::new(validated_sibling).arg("--snapshot").stdin(Stdio::piped())` and a single writer thread; store `Child` for `stop_for_capture` and `Drop` cleanup.
- [ ] Add `原生 NSPanel 冻结画面预览（实验）` to the existing screenshot editor only, inside `#[cfg(feature="native-panel-probe")]`. Validate dimensions *before cloning* source RGBA. Report 'process launched, GUI not yet verified' rather than a false successful-render claim.
- [ ] Verify default screenshots, hotkeys, PNG, OCR and pins keep their current pathways; commit.

### Task 4 — Documentation, CI evidence and acceptance boundary

**Files**
- Modify `README.md`
- Modify `docs/macos-window-strategy.md`
- Modify `docs/STATUS.md`, `docs/REMAINING.md`, `docs/TASKS.md`
- Use existing `.github/workflows/ci.yml` native-feature and normal package matrix.

- [ ] Document `cargo build -p shot-macos --features native-panel-probe --bins --locked`, launch main with feature, capture normally, click frozen-preview button, inspect pixel geometry and window lifecycle; also cover malformed stdin, 1x/2x and Spaces device-only tests.
- [ ] Record the exact PR-head full Linux/macOS CI run including Rust unit/integration, strict Clippy, default Release, Swift Vision IPC and Lite/Full/Add-on packages.
- [ ] Merge with expected SHA only if all CI gates are green. A failing or missing GUI device result must remain open under Issue #5/#10 rather than being represented as acceptance.

**Acceptance:** A developer-only, raw-RGBA frozen screenshot reaches a separate genuine NSPanel Metal renderer without saving or modifying production images. Native drag/select/annotation parity and macOS physical GUI tests explicitly remain future work.
