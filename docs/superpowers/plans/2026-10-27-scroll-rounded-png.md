# Long Screenshot Rounded PNG Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (\`- [ ]\`) syntax for tracking.

**Goal:** Apply PicPaste's existing configurable physical-pixel transparent corner radius to trusted manual long-screenshot PNG exports, without allocating the full stitched RGBA image, and render matching rounded previews.

**Architecture:** Reuse the screenshot rounding mask at scanline granularity. Stream trusted rows from \`TileStore\` into PNG encoder and mask alpha in place; retain the existing zero-radius APIs. In the UI, copy the already-bounded raw review preview when the radius changes, scale the physical corner radius to the preview, and retain its original bytes for further trims.

**Tech Stack:** Rust 2024, shot-image, shot-scroll, egui/eframe, png 0.18.1, macOS CI.

## Global Constraints

- File exports remain PNG-only; never upsample the review thumbnail.
- Do not allocate an RGBA buffer of the full long canvas: maximum 60,000 pixels along one edge, 200 MP, disk-backed strips.
- Keep atomic/no-clobber output and \`TileStore\` temporary cleanup.
- Current radius slider is 0–64 physical pixels; zero must be exact legacy behavior.
- Preview must reflect selected radius, including when changing the slider after capture or trimming.
- No automatic scrolling, Windows scope, signature/notarization or claim of physical macOS acceptance in this PR.

---

### Task 1: Shared row-level alpha rounding

**Files:**
- Modify: \`crates/shot-image/src/lib.rs\`
- Test: \`crates/shot-image/src/lib.rs\` module tests

**Interfaces:**
- Consume \`&mut [u8]\` of exactly one RGBA scanline, canvas width/height, Y and radius.
- Produce \`pub fn rounded_alpha_row(row: &mut [u8], width: u32, height: u32, y: u32, requested_radius: u32)\`.

- [ ] **Step 1: Write a failing test.**

\`\`\`rust
#[test]
fn row_rounding_matches_existing_full_frame_and_preserves_rgb() {
    let mut expected = RgbaFrame::new(8, 6, vec![200; 8 * 6 * 4]).unwrap();
    rounded_alpha(&mut expected, 3);
    let mut rows = vec![200; 8 * 6 * 4];
    for (y, row) in rows.chunks_exact_mut(8 * 4).enumerate() {
        rounded_alpha_row(row, 8, 6, y as u32, 3);
    }
    assert_eq!(rows, expected.rgba);
    assert!(rows[3] < 200);
    assert_eq!(rows[0], 200);
}
\`\`\`

- [ ] **Step 2: Verify RED.** Run \`cargo test -p shot-image --locked row_rounding_matches_existing_full_frame_and_preserves_rgb\`. Expected: unresolved \`rounded_alpha_row\` symbol.
- [ ] **Step 3: Implement.** Extract the existing 4×4 corner coverage calculation into \`rounded_alpha_row\`; have \`rounded_alpha\` iterate contiguous scanlines and invoke the shared function so both paths are byte-identical. Cap radius at \`min(width,height)/2\`; preserve original RGB and multiply only alpha.
- [ ] **Step 4: Verify GREEN.** Run \`cargo test -p shot-image --locked\`. Expected: all tests pass.

### Task 2: Disk-backed rounded PNG and integration tests

**Files:**
- Modify: \`crates/shot-scroll/src/tile_store.rs\`
- Modify: \`crates/shot-scroll/Cargo.toml\` (add existing png 0.18.1 as dev-dependency)
- Test: create \`crates/shot-scroll/tests/rounded_export.rs\`

**Interfaces:**
- Produce \`pub fn encode_png_rounded<W: Write>(&self, output: W, radius: u32) -> io::Result<()>\`.
- Produce \`pub fn save_png_rounded_no_clobber(&self, destination: &Path, radius: u32) -> io::Result<()>\`.
- Keep \`encode_png(output)\` and \`save_png_no_clobber(path)\` as zero-radius wrappers.

- [ ] **Step 1: Write failing tests.** Build two-strip vertical and horizontal stores, decode the emitted PNG with \`png::Decoder\` and compare exact pixels to an expected small \`RgbaFrame\` passed through \`shot_image::rounded_alpha\`. Check both radius 0 and radius 3; save a rounded PNG and confirm the no-overwrite error and absence of \`.part\` leaks.
- [ ] **Step 2: Verify RED.** Run \`cargo test -p shot-scroll --locked --test rounded_export\`. Expected: \`encode_png_rounded\` and \`save_png_rounded_no_clobber\` not found.
- [ ] **Step 3: Implement.** Route the new encoding method through existing \`shot_image::stream_png::encode_png_rows\`: use existing open strip handles to fill each row, then invoke \`shot_image::rounded_alpha_row\` before writing it. Reuse the previous atomic file path and hard-link publication with the supplied radius. Zero-radius old methods delegate to new ones.
- [ ] **Step 4: Verify GREEN.** Run \`cargo test -p shot-scroll --locked\`; check horizontal/vertical stitch tests as well.

### Task 3: Review UI and evidence

**Files:**
- Modify: \`apps/shot-macos/src/scroll.rs\` (\`export_review(review: &ScrollReview, radius: u32)\`)
- Modify: \`apps/shot-macos/src/app.rs\`
- Modify: \`README.md\`, \`docs/STATUS.md\`, \`docs/TASKS.md\` and Issue #7 after CI

**Interfaces:**
- The physical radius at save-button click is passed into \`save_png_rounded_no_clobber\` in a background thread.
- Raw \`review.preview\` stays unchanged; when radius changes, clone only that bounded image, apply a radius scaled by \`preview.width/canvas.width\`, and upload the preview texture.

- [ ] **Step 1: Add test-first UI helper for pixel-space preview radius and bounds**, including zero radius and a 60,000-pixel long edge (rounding may yield zero preview pixels). Run the affected test and verify RED before implementation.
- [ ] **Step 2: Wire review preview to the existing slider and save thread.** The preview cache must invalidate when radius changes and on a successful trim; reject no save actions while another review operation is processing.
- [ ] **Step 3: Verify complete macOS CI.** \`cargo fmt --all -- --check\`; \`cargo test --workspace --locked\`; \`cargo clippy --workspace --all-targets --locked -- -D warnings\`; release build; OCR Swift IPC and Lite/Full development ZIPs. Linux core tests must also pass.
- [ ] **Step 4: Review PR diff; merge only when PR CI is green, then verify main CI and update status/evidence.** Do not claim real-macOS acceptance.

## Self-review

The new stream must preserve: exact physical pixels except alpha in four corners, no full-canvas allocation, atomically published PNG, no-clobber, tail trimming, Live Preview, OCR and Lite/Full packaging. UI thumbnail rounding is visual only; exported PNG is always recomputed from trusted source strips.
