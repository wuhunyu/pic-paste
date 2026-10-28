# Native NSPanel line, pixelate and blur Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add source-local straight-line and bounded pixelate/blur tools to the opt-in NSPanel experiment, hand them back through validated PPANN001, and render them through existing non-destructive PNG history.

**Architecture:** Extend the shared native markup command union, keep full-display source RGBA immutable, transport newly tagged bounded mark packets to the existing parent screenshot editor, then let `shot-image` render Pixelate/Blur using its existing CPU compositors. The NSPanel preview draws clear highlighted *regions*, not a pixel-accurate blur/pixelation; the resulting effect becomes visible in the parent editor/PNG after confirm. Keep the legacy PPSEL001 wire and existing PPANN001 tags byte-for-byte compatible.

**Tech Stack:** Rust 2024, shot-core, shot-image, egui-wgpu, native AppKit NSGlassEffectView toolbar, GitHub macOS-26 CI.

## Global Constraints

- macOS 14+ minimum target; macOS 26 Liquid Glass/older vibrancy/Reduce Transparency fallbacks remain.
- Opt-in `native-panel-probe` only; production eframe/winit backend unchanged.
- PNG-only, 0 online calls, original RGBA immutable, all marks validated before history mutation.
- `MAX_MARKS=128`; pixelate/blur effect ROI at most 4,000,000 physical pixels, no arithmetic overflow; fixed `block_size=12` and `radius=6`.
- Source-local physical-pixel coordinates; never scale the desktop-global origin. Old tags 1–5 and PPSEL001 unchanged.
- CI passing does NOT equal real device acceptance for focus, TCC, IME, Retina, Spaces, performance.

---

### Task 1: Core gestures and bounds

**Files:** `crates/shot-core/src/native_markup.rs`, `crates/shot-core/tests/native_line_redaction.rs`

**Interfaces:** Add `NativeMarkupTool::{Line,Pixelate,Blur}`, `NativeMark::Line {from,to}`, `NativeMark::{Pixelate,Blur} {left,top,width,height}`, `MAX_NATIVE_EFFECT_PIXELS: u64`; use existing `begin/drag/draft/end/undo/redo`.

- [ ] **Step 1: RED test** — include three behaviors:
```rust
marks.set_tool(NativeMarkupTool::Line);
assert!(marks.begin((20.0, 10.0)));
assert!(marks.end((5.0, 6.0)));
assert_eq!(marks.visible()[0], NativeMark::Line{from:(40,20),to:(10,12)});
marks.set_tool(NativeMarkupTool::Pixelate);
assert!(marks.begin((30.0, 20.0)));
assert!(marks.end((10.0, 5.0)));
assert_eq!(marks.visible()[1], NativeMark::Pixelate{left:20,top:10,width:41,height:31});
```
  A 4MP-over-limit drag must not commit or clear previously available redo history.
- [ ] **Step 2: Check RED** — `cargo test -p shot-core --test native_line_redaction --locked` fails due to missing variants.
- [ ] **Step 3: GREEN** — validate draft and committed ROI dimensions with `u64::from(width)*u64::from(height) <= 4_000_000`; reverse drag normalization, zero-length line/rectangle rejected; use the source-local point conversion already present. Check `MAX_MARKS` and history semantics unchanged.
- [ ] **Step 4: Test** — same command and `cargo clippy -p shot-core --all-targets --locked -- -D warnings`; commit.

### Task 2: Versioned IPC and parent import

**Files:** `crates/shot-core/src/native_annotation.rs`, `crates/shot-core/tests/native_line_redaction.rs`, `crates/shot-image/src/native_import.rs`, `crates/shot-image/tests/native_line_redaction_import.rs`

**Interfaces:** PPANN001 tags `6`=Line, `7`=Pixelate, `8`=Blur. Line payload 16 bytes, ROI payload 16 bytes. Parent imports `Annotation::Line`, `Annotation::Pixelate {block_size:12}` or `Annotation::Blur {radius:6}`.

- [ ] **Step 1: RED tests** — roundtrip new tags, reject out-of-frame, zero dimensions, overflow and >4MP effect regions; test invalid second mark leaves existing history intact. Decode corrupted tags/coordinates must fail closed.
```rust
let reply=NativeEditReply::Annotated{selection,pixels:(3000,2000),marks:vec![
  NativeMark::Pixelate{left:4,top:5,width:50,height:40}
]};
let mut bytes=Vec::new();
reply.write_to(&mut bytes).unwrap();
assert_eq!(bytes[46],7);
assert_eq!(NativeEditReply::read_from(&mut bytes.as_slice()).unwrap(), reply);
```
- [ ] **Step 2: Check RED** — `cargo test -p shot-core --test native_line_redaction --locked` and `cargo test -p shot-image --test native_line_redaction_import --locked`.
- [ ] **Step 3: GREEN** — extend `validate_marks`, `write_to`, `read_from` and `import_native_marks` using prior geometry contract. No new wire magic; no partial write of bad input.
- [ ] **Step 4: Test** — full `cargo test -p shot-core -p shot-image --locked` and Clippy; commit.

### Task 3: Native UI and truthful preview

**Files:** `crates/shot-core/src/glass_toolbar.rs`, `crates/shot-core/tests/glass_toolbar.rs`, `apps/shot-macos/src/bin/ns-panel-probe/glass_toolbar.rs`, `apps/shot-macos/src/bin/ns-panel-probe/panel_egui.rs`, `apps/shot-macos/src/bin/ns-panel-probe.rs`.

**Interfaces:** Add AppKit `chooseLine:`, `choosePixelate:`, `chooseBlur:`. Keep 560pt-wide toolbar with **12 buttons in two rows of 6** (88pt tall); keep its minimum host width 580pt, drop-in older material fallback. `paint_native_marks` shows line and translucent, labeled ROI (not a fake WYSIWYG pixel effect).

- [ ] **Step 1: RED test** — toolbar geometry `FloatingBarFrame::for_annotation_size(1200.0,800.0).unwrap().height == 88.0`, three new tool action variants dispatch, headless egui paint shapes.
- [ ] **Step 2: Confirm RED** — `cargo test -p shot-core --test glass_toolbar --locked` fails for height and new action variants.
- [ ] **Step 3: GREEN** — update geometry and AppKit layout, add new selectors and checked choices. Add a separate `paint_native_marks` match arm for each new mark; add popup text: native effect region preview only; final pixelate/blur in parent after confirming.
- [ ] **Step 4: Verify** — macOS CI `cargo test -p shot-macos --features native-panel-probe --bin ns-panel-probe --locked` and `cargo clippy -p shot-macos --features native-panel-probe --bin ns-panel-probe --locked -- -D warnings`; commit.

### Task 4: Status and merge

**Files:** `README.md`, `docs/STATUS.md`, `docs/REMAINING.md`, `docs/TASKS.md`.

- [ ] **Step 1:** Describe functionality, fixed effect parameters, bounds and region-outline-only native preview; update #5/#6 issue evidence without closing real-device gates.
- [ ] **Step 2:** Verify `cargo fmt --all -- --check`, full Linux tests, macOS native feature and full macOS CI (strict Clippy, Release, Lite/Full/Add-on).
- [ ] **Step 3:** Confirm the latest same-head CI success, then merge PR with expected head SHA and check main CI.
- [ ] **Step 4:** Preserve honest status: no real macOS GUI/Retina/Spaces/TCC/IME/latency verification.
