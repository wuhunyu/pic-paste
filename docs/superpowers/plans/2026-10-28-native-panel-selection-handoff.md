# Native NSPanel Selection and Safe Handoff Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a developer-only full-target-display AppKit NSPanel selection mode, with mouse-drag selection returning a verified physical pixel rectangle to the existing eframe screenshot editor, without changing any default production behavior.

**Architecture:** Preserve the immutable `PPSNAP01` RGBA input pipe introduced in PR #41. A new `--select` mode creates a borderless native NSPanel at the target display's CoreGraphics-to-AppKit logical-point converted position, paints its real frozen screenshot, and converts egui mouse drags into a bounded display-local physical pixel rectangle. Return exactly one versioned `PPSEL001` binary selection/cancellation reply via *dedicated stdout*; PicPaste reads it on a worker thread, checks display, capture revision and bounds, then applies the selection to its unchanged eframe editor. Do not replace the production screenshot viewport or assume device-level GUI acceptance.

**Tech Stack:** Rust 2024, shot-core, egui/eframe 0.37, objc2-app-kit 0.3.2 NSPanel/NSScreen, WGPU Metal 30, GitHub Actions macOS.

## Global Constraints

- Default macOS executable is unchanged; `--features native-panel-probe` is mandatory for the new UI and helper. Only the explicit button launches `--select`.
- Never transmute winit-created NSWindow into NSPanel; separate process owns native AppKit event loop and Metal surface, releasing GPU before NSView.
- User screenshot raw pixels, OCR, clipboard, PNG alpha/rounding, annotations and pins remain in the existing eframe editor; no automatic saving or disk screenshot artifacts.
- Preserve independently expressed CoreGraphics *global logical points* and screenshot *display-local physical pixels*. Do not scale absolute desktop origins by Retina factor.
- Input `PPSNAP01` enforces 8192 max edge and 128 MiB. Selection reply has exact 33-byte `PPSEL001` envelope (8 magic, 1 status, 8 display ID, four 4-byte rect components) and no trailing bytes.
- The native selection result must be checked against the active capture revision and current display ID and must be fully within the captured RGBA dimensions. Non-matching, malformed, empty or stale results must never mutate the screen selection.
- Escape and normal panel closure must return cancellation, not fabricated rectangular selections. A helper that crashes or exits without a full reply produces a visible error and preserves the editor.
- Native UI positioning, mixed-scale, Spaces, TCC and keyboard behavior must be validated on a real macOS 14+/26 device. Green macOS CI only verifies compilation, pure-logic tests and headless path.
- Continue to keep native editor migration and physical-device performance tests open in Issues #5/#10.

---

### Task 1: Pure selection state, global AppKit position and bounded response wire format

**Files:**
- Create `crates/shot-core/src/native_selection.rs`
- Modify `crates/shot-core/src/lib.rs`
- Test `crates/shot-core/tests/native_selection.rs`

**Interfaces:**
- `NativeSelection::new(placement: OverlayPlacement, pixels: (u32,u32)) -> Result<Self,String>`
- `press(&mut self, point_pt:(f32,f32)) -> bool`, `drag(&mut self, point_pt:(f32,f32))`, `release(&mut self,point_pt:(f32,f32))->Option<RectPx>`, `preview_rect_points(&self)->Option<(f32,f32,f32,f32)>`.
- `AppKitSelectionFrame::from_quartz(placement: OverlayPlacement, primary_top_y: f64)->Result<Self,String>`.
- `NativeSelectionReply::{Selected{display:DisplayId,rect:RectPx},Cancelled{display:DisplayId}}`; `write_to<W:Write>`, `read_from<R:Read>`, `validated_in(&self, display:DisplayId, pixels:(u32,u32))->Result<Option<RectPx>,String>`.

- [ ] **RED:** Write tests for reverse-direction drag, subpixel clamping, zero-size rejection, 1x/2x, outside drags, negative monitor origin and primary AppKit top conversion; verify `cargo test -p shot-core --test native_selection --locked` fails with an unresolved import.
- [ ] **GREEN:** Implement exact 33-byte little-endian `PPSEL001` response; round min endpoint down and max up to avoid losing one-pixel selections, clamp to physical RGBA edges; reject status/magic/trailing data; make validation display and dimension aware. Run `cargo fmt --all -- --check`, `cargo test -p shot-core --test native_selection --locked` and `cargo clippy -p shot-core --all-targets --locked -- -D warnings`; commit.

### Task 2: Native NSPanel target placement and egui drag selection

**Files:**
- Modify `apps/shot-macos/Cargo.toml` (enable `NSScreen`)
- Modify `apps/shot-macos/src/bin/ns-panel-probe.rs`
- Modify `apps/shot-macos/src/bin/ns-panel-probe/panel_egui.rs`

**Interfaces:**
- `ProbeCommand::Select` parsed exclusively from `--select`.
- `ProbeEgui::new(device,format,image,selection:Option<NativeSelection>)`; `selection_result(&self)->Option<RectPx>`.
- Existing native `--show`, `--snapshot`, `--check` remain valid.

- [ ] **RED:** Write a headless CLI test that requires `--select` and a headless egui selection-paint contract. CI must fail at missing variant/interface, not formatting.
- [ ] **GREEN:** In `--select`, read validated `PPSNAP01` stdin before AppKit, convert its target CG-origin/size using `NSScreen::screens(mtm).firstObject().frame()`, validate against an attached screen rectangle; create borderless floating NSPanel with CanJoinAllSpaces and FullScreenAuxiliary, with full captured logical-point dimensions. Render its RGBA stretched exactly over content logical points, draw native selection stroke and respond to egui drag release. Disable experimental glass button bar for this mode. Escape returns cancel, completed drag returns the selected physical region; in both cases write one `PPSEL001` reply to stdout without any diagnostic logs on stdout. Run macOS headless tests and strict Clippy; commit.

### Task 3: Asynchronous selection reply and fallback eframe editor

**Files:**
- Modify `apps/shot-macos/src/native_probe_launcher.rs`
- Modify `apps/shot-macos/src/main.rs`
- Modify `apps/shot-macos/src/app.rs`

**Interfaces:**
- `launch_select(&mut self,snapshot:NativeSnapshot,tx:Sender<Result<NativeSelectionReply,String>>,wake:egui::Context)->Result<ProbeLaunchResult,String>`
- `launch_select_from(&mut self,executable:&Path,snapshot:NativeSnapshot,tx:Sender<Result<NativeSelectionReply,String>>,wake:egui::Context)->Result<ProbeLaunchResult,String>`
- `PicPasteApp.native_selection_rx` and `native_selection_revision`.

- [ ] **RED:** Add an exact same-directory executable shell-fixture test that reads `PPSNAP01` on stdin and sends a `PPSEL001` reply on stdout; test malformed reply and no-second-process behavior. Observe failures on missing launcher entry.
- [ ] **GREEN:** Use `Command::new(verified_sibling).arg("--select").stdin(Stdio::piped()).stdout(Stdio::piped())`, send raw bytes and consume reply on independent workers. In the existing editor show `NSPanel 原生框选（实验）` only in feature-gated screenshot mode, keep default eframe overlay as fallback. Store originating capture revision, restore editor focus and apply the returned rectangle only when selected display and image dimensions still match; show a visible status for cancellation and errors. Add a repaint wakeup when the reader returns; commit.

### Task 4: Documentation, full CI and merge gating

**Files:**
- Modify `README.md`, `docs/STATUS.md`, `docs/REMAINING.md`, `docs/TASKS.md`, `docs/macos-window-strategy.md`
- Use existing `.github/workflows/ci.yml` (Linux, macOS feature, workspace, strict Clippy, Release, Lite/Full/Add-on).

- [ ] Document developer build (`cargo build --locked -p shot-macos --features native-panel-probe --bins`), start screenshot via existing UI, click native selection button, drag, verify same physical rectangle in eframe editor; test Escape, target unplug, negative origin, mixed Retina and focus restoration.
- [ ] Verify PR-head `cargo fmt`/test/strict-Clippy/Release/package checks in GitHub Actions; do not state any GUI/device acceptance from CI.
- [ ] Merge only when all CI gates are green and update Issue #5 with separate physical GUI validation checklist. Default production window remains eframe until feature parity is proven.

**Acceptance:** Optional native NSPanel can show a real display-local screenshot at the intended target display logical rectangle; drag selection reports an exact validated pixel rectangle back to the existing editor, preserving all current screenshot, OCR, pin, and PNG functionality.
