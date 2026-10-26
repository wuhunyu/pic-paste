# Manual Long Screenshot Fixed Band Exclusion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users safely exclude page headers/footers or fixed sidebars from *manual* long screenshots before frame matching and on-disk stitching, avoiding false matches and repeated chrome in the PNG.

**Architecture:** Prefer adjusting the physical-pixel capture ROI **before** ScreenCaptureKit takes the first or subsequent frame, rather than masking matching scores or copying extra RGBA frames. A pure `FixedBands::content_roi` geometry function calculates a validated, cropped sub-region for vertical (top/bottom) or horizontal (left/right) motion. Both `previous` and `TileStore` continue using full pixels of the **same resulting cropped region**; conservative match and slow-scroll recovery behavior are unchanged. Two user-supplied physical-pixel offsets are validated before hiding the screenshot editor.

**Tech Stack:** Rust 2024, `shot-core::RectPx`, `shot-core::platform::ScrollAxis`, `shot-scroll`, egui 0.37, ScreenCaptureKit 11, GitHub Actions macOS runner.

## Global Constraints

- No automatic scrolling. Users control all motion; PicPaste only captures/compares and appends trusted content.
- PNG-only output, 60,000 px / 200 MP / 4,096 strips; no full-image RGBA buffer. No added dependency or online service.
- By default `leading=0, trailing=0` and existing manual screenshot behavior must remain unchanged.
- User-specified fixed bands are completely **excluded from the exported PNG**, not merely hidden during overlap matching. Make this obvious in the UI.
- All offsets in **physical pixels** within the chosen manual scroll ROI. Negative display coordinates must remain correct.
- Invalid crop (remaining axis <24px, cross-axis <24px, origin addition overflow) must fail before capture starts, without hiding UI or creating temporary files.
- Cropping is stable for every frame across recovery attempts. Do not change crop values during one capture.
- Moving main content, sticky headers, dynamic animation, fixed footers, repeated patterns, sideways scroll and low-information areas must never force an unverified append.
- Do not claim that dynamic animation/headers are automatically detected. This is **manual fixed-band exclusion**; dynamic-content heuristics and 50 real-device acceptance scenes remain open.

---

### Task 1: Cropped ROI geometry with TDD

**Files:** Create `crates/shot-scroll/src/fixed_bands.rs`; modify `crates/shot-scroll/src/lib.rs`; create `crates/shot-scroll/tests/fixed_bands.rs`.

**Interface:**
```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FixedBands { pub leading: u32, pub trailing: u32 }
impl FixedBands {
    pub fn content_roi(self, original: RectPx, axis: ScrollAxis) -> Result<RectPx, &'static str>;
}
```

- [x] Write failing tests showing actual input/output ROI geometry at negative origins; vertical/horizontal scroll scene matching and TileStore export without sticky chrome; identical/flat content remains untrusted.
- [x] Run `cargo test -p shot-scroll --test fixed_bands --locked` via GitHub Actions and verify **RED** specifically because `shot_scroll::fixed_bands` does not exist.
- [x] Implement the following bounded ROI transform (use unchanged `RectPx` for `FixedBands::default()`):

```rust
use shot_core::{RectPx, platform::ScrollAxis};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FixedBands { pub leading: u32, pub trailing: u32 }
impl FixedBands {
    pub fn content_roi(self, original: RectPx, axis: ScrollAxis) -> Result<RectPx, &'static str> {
        if original.width < 24 || original.height < 24 { return Err("scroll ROI must be at least 24x24 pixels"); }
        let extent = match axis {
            ScrollAxis::Vertical => original.height,
            ScrollAxis::Horizontal => original.width,
        };
        let remaining = extent.checked_sub(self.leading)
            .and_then(|n| n.checked_sub(self.trailing))
            .filter(|n| *n >= 24).ok_or("fixed bands leave less than 24 pixels of scroll content")?;
        let offset = i32::try_from(self.leading).map_err(|_| "fixed band offset overflow")?;
        let mut cropped = original;
        match axis {
            ScrollAxis::Vertical => {
                cropped.y = cropped.y.checked_add(offset).ok_or("scroll Y origin overflow")?;
                cropped.height = remaining;
            }
            ScrollAxis::Horizontal => {
                cropped.x = cropped.x.checked_add(offset).ok_or("scroll X origin overflow")?;
                cropped.width = remaining;
            }
        }
        Ok(cropped)
    }
}
```

- [ ] Run `cargo test -p shot-scroll --test fixed_bands --locked`, `cargo fmt --all -- --check` and `cargo clippy -p shot-scroll --all-targets --locked -- -D warnings`. Confirm 7 tests GREEN and existing matcher tests still GREEN.
- [ ] Commit verified pure geometry and pixel-aware integration tests.

### Task 2: Wire native manual capture controls

**Files:** Modify `apps/shot-macos/src/app.rs`. Do not change `apps/shot-macos/src/scroll.rs`: it already captures every frame from the ROI provided by `start_manual_scroll`.

**Interface:** `scroll_fixed_bands: FixedBands` in `PicPasteApp`, default `FixedBands::default()`. Right before `scroll::launch`, calculate `let selected = match self.scroll_fixed_bands.content_roi(screen.selected_region(), axis) { Ok(region) => region, Err(message) => {self.status=format!("长截图固定区域设置无效：{message}"); return;} }` and keep the editor visible on failure.

- [ ] Write a regression test for ROI rejection and negative display coordinates in Task 1 before changing `app.rs`; it is a requirement of `FixedBands::content_roi`.
- [ ] Add two `egui::DragValue` inputs within screenshot tool UI: `开头(上/左)` and `末尾(下/右)`. Show the note `这些固定区域不会进入长图 PNG；纵向忽略上/下，横向忽略左/右`. Inputs are physical pixels, initially 0.
- [ ] Validate `content_roi` before launching a worker, hiding root, dismissing overlay or writing any tiles; display an actionable error if the content is less than 24 physical pixels.
- [ ] Launch existing capture with validated *cropped* region; verify it keeps the same region through slow-scroll recovery and preserves existing 260 ms sampling/review/export/finish/cancel semantics.
- [ ] Verify macOS `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, Release build, Swift OCR IPC, Lite and Full developer packages through GitHub Actions.

### Task 3: Update roadmap and complete release gate

**Files:** `README.md`, `docs/STATUS.md`, `docs/TASKS.md`, [Issue #7](https://github.com/wuhunyu/pic-paste/issues/7).

- [ ] Document that manual fixed bands are available, and sticky bands are **not** retained even once in final PNG. Do not claim automatic sticky detection or dynamic-content repair.
- [ ] Update P4-T1 and P4-T3 in status/acceptance tracker. Track real Mac sample tests for vertical sticky header/footer, horizontal toolbar/sidebar, wrong crop boundaries, dynamic animation, reverse scrolling, negative multi-display origin and error messages.
- [ ] Merge only when PR-specific Linux+macOS CI is full-green. Check post-merge `main` CI and keep physical-device 50-scene acceptance open.

**Acceptance rationale:** Removing fixed chrome from the actual physical ROI is safer than ignoring arbitrary changing pixels in matching scores, and automatically ensures the on-disk PNG contains only the scrollable area. It does **not** solve animated content within the chosen ROI, nor arbitrary middle-segment edit/resume.

## Evidence and limitations

- RED due to missing `shot_scroll::fixed_bands`: [CI #40316689699](https://github.com/wuhunyu/pic-paste/actions/runs/40316689699); formatting corrected before RED confirmation.
- Pure geometry plus vertical/horizontal sticky-region tests passed the Linux core job [CI #40316842815](https://github.com/wuhunyu/pic-paste/actions/runs/40316842815) before the job was superseded by new changes.
- Physical Mac sticky bars, dynamic areas, focus and mouse passthrough remain unverified; slow-scroll recovery remains the fallback.
