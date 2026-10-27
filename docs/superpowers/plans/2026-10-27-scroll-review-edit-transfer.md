# Manual Long Screenshot Review Editing and Transfer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (\`- [ ]\`) syntax for tracking.

**Goal:** Let a user annotate a trusted long screenshot at source-pixel resolution, export the complete edited PNG, and copy/pin a bounded native-resolution selection without materializing the full long canvas.

**Architecture:** Every annotation produces an immutable disk-backed replacement patch of only its affected source-pixel rectangle, composed atop prior patches in command order. PNG export and review thumbnail sampling apply patches during existing row streaming; undo/redo changes only the active patch prefix. Clipboard and GPU pins use a user-selected, bounded RGBA crop. This preserves 60,000-pixel and 200-MP image support for PNG without claiming that giant clipboard textures are safe.

**Tech Stack:** Rust 2024, \`shot-scroll\` and \`shot-image\`, \`png 0.18.1\`, egui/eframe + wgpu, macOS 14+, GitHub Actions.

## Global Constraints

- PNG-only; no JPEG, lossy previews in saved output, Electron, WebView, online OCR or automatic scrolling.
- Original trusted strips remain immutable; temporary patches are private files in the TileStore-owned directory, which cleans on drop.
- Never materialize the complete long RGBA canvas: annotation patches limited to 4,000,000 pixels; clipboard/pin selections limited to 4,000,000 pixels and 4,096 pixels per side.
- Each patch is a *replacement* of the source-pixel rectangle rendered from current history; no double alpha blending or loss of previous edits at intersections.
- Save failure retains the review and all edits; explicit discard deletes temporary strips and edits.
- Unsupported gigantic annotation bounds must visibly error without mutating history; no unannounced downsampling.
- Preserve 0–64 physical-pixel transparent corners at final export, and apply the same radius to copied/pinned selection.
- This PR does not add middle-strip repairs, dynamic-region detection, native AppKit NSPanel, x86_64, signing, or physical-device acceptance.

---

### Task 1: Disk-backed edit stack

**Files:**
- Create: \`crates/shot-scroll/src/edits.rs\`
- Modify: \`crates/shot-scroll/src/lib.rs\`
- Modify: \`crates/shot-scroll/src/tile_store.rs\`
- Test: \`crates/shot-scroll/tests/review_edits.rs\`

**Interfaces:**
- \`EditStack::apply(&mut self, store: &TileStore, command: Annotation) -> io::Result<()>\`
- \`EditStack::{undo,redo,len,applied_len,clear}\`
- \`TileStore::{preview_with_edits,encode_png_with_edits,save_png_edited_no_clobber,render_region_edited}\`
- \`EditStack::apply_to_row(&self, y, row, patch_readers)\` uses open-once file handles for streaming.

- [ ] **Step 1: Write a failing integration test before implementation.** Build synthetic vertically and horizontally stitched frame data, apply line then overlapping pixelate or blur, and compare exact native-color PNG output against \`shot_image::composite::composite\` on a small reconstructed frame. Exercise undo, redo, branching history, atomic no-clobber save, cleanup, and invalid >4MP commands.
- [ ] **Step 2: Run RED.** \`cargo test -p shot-scroll --locked --test review_edits\` fails for missing \`edits\` module/API. Capture the expected compile error in CI.
- [ ] **Step 3: Implement minimum.** Bound each annotation to a conservative clipped rectangle, read only that region, overlay prior patch files, translate annotation coordinates relative to region, render using existing compositing, and atomically write an immutable patch. PNG/thumbnail scanlines combine source rows and patch replacement rows in command order. Preserve old zero-edit methods.
- [ ] **Step 4: Run GREEN.** \`cargo test -p shot-scroll --locked\` and \`cargo clippy -p shot-scroll --all-targets --locked -- -D warnings\` both pass.

### Task 2: Bounded copy/pin and review coordinate mapping

**Files:**
- Create: \`crates/shot-scroll/src/review_transfer.rs\`
- Test: \`crates/shot-scroll/tests/review_transfer.rs\`
- Modify: \`crates/shot-scroll/src/lib.rs\`

**Interfaces:**
- \`default_transfer_rect(canvas: (u32,u32)) -> RectPx\`
- \`validate_transfer_rect(rect: RectPx, canvas: (u32,u32)) -> io::Result<()>\`
- \`map_preview_point(local:(f32,f32),displayed:(f32,f32),canvas:(u32,u32)) -> Option<(i32,i32)>\`

- [ ] **Step 1: Write failing tests** for full small screenshot default, 60k pixel tall default, coordinate scaling/clamping, 4MP and 4096-edge rejects.
- [ ] **Step 2: Run RED.** \`cargo test -p shot-scroll --locked --test review_transfer\` must fail because API does not exist.
- [ ] **Step 3: Implement and rerun GREEN.** Use checked u64 area arithmetic and preserve full native-pixel dimensions; no silent resample.

### Task 3: macOS review interaction

**Files:**
- Modify: \`apps/shot-macos/src/scroll.rs\`
- Modify: \`apps/shot-macos/src/app.rs\`
- Test: \`apps/shot-macos/src/scroll.rs\` native unit tests, and integration tests from Tasks 1–2.

**Interfaces:**
- \`ScrollReview\` owns \`edits: EditStack\` and raw bounded \`preview\`.
- \`ScrollReview\` supports background edit/undo/redo and preview refresh.
- \`WorkerResult::ScrollPrepared(PrepareAction,Result<(ScrollReview,RgbaFrame),(String,ScrollReview)>)\` delivers bounded crops to the existing native clipboard / pinned image workflow; the review survives.
- \`scroll::export_review\` calls the edited row-streaming export with the selected corner radius.

- [ ] **Step 1: RED tests** verify default transfer rect and image-coordinate mapping in core (Task 2); macOS test checks edit-preview update/undo workflow on a tiny fixture.
- [ ] **Step 2: UI implementation:** Render existing annotation tool controls above review image; click for text, drag for strokes, click-and-drag for selection, undo/redo. Expose numeric x/y/w/h for precise selection. Highlight selection. When review is saved or copied, use *real* edited source pixels; clipboard/pin is limited to validated region. Keep existing PNG save/trim/discard actions. Disable actions while a background edit/export is running.
- [ ] **Step 3: Run macOS CI** including full workspace tests, Clippy, Release, Swift OCR IPC and Lite/Full developer ZIPs, checking zero-device acceptance disclosure.

### Task 4: Review, merge and record evidence

**Files:**
- Modify: \`README.md\`, \`docs/STATUS.md\`, \`docs/TASKS.md\`
- Update: GitHub Issue #7 with tested functionality, limits and remaining open gates.

- [ ] **Step 1:** Check diff for full-image allocations, dirty state after failed export, pixel mismatch at overlapping annotations, invalid bounds and clipboard failure.
- [ ] **Step 2:** Open PR, wait for successful full PR CI, merge to \`main\`, then verify the new \`main\` CI.
- [ ] **Step 3:** Record exactly what has CI evidence and what still requires physical macOS/50-scene acceptance.

## Self-review of scope

This completes long-image annotation → streaming PNG and bounded region → clipboard/pin. It intentionally does *not* claim an entire 60,000-pixel PNG can be put into a single native clipboard/GPU texture: the user must select a valid full-resolution region for those two actions. The disk-backed patches allow modifications across the entire long scroll canvas as long as each individual annotation's local area stays within the 4 MP cap.
