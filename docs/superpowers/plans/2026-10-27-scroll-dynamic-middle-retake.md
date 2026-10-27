# Dynamic-Content-Tolerant Scroll Matching and Trusted Middle Retake Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Detect manual scroll displacement despite small animated regions, and safely repair a bad middle segment by re-capturing the tail from the last verified prefix, without splicing unverified later strips.

**Architecture:** Coarse stage votes for physical-pixel shift across eight cross-axis lanes instead of one whole-line fingerprint. Verification accepts only a unique candidate with ≥75% independently matching pixels, ignoring changed dynamic samples for matching only; exported pixels remain the user's actual screenshot. Middle repair trims the selected suspect segment and every later segment, reconstructs the last full trusted capture viewport from disk-backed tiles, and resumes manual capture using the original display, ROI, axis, and the unchanged confidence/recovery policy. Abort/error retains a reviewable trusted prefix.

**Tech:** Rust 2024, shot-scroll, shot-macos, ScreenCaptureKit, GitHub Actions Linux/macOS.

## Global constraints

- Manual scrolling only; never inject scroll events or invent pixels to hide animation.
- PNG-only, transparent alpha retained, full-size streaming canvas ≤60,000 px along long edge and ≤200 MP.
- No arbitrary removal of one strip while retaining its later suffix; retake always discards the suffix from the suspect index onward.
- Original display / ROI / axis immutable across retake; do not recrop sticky headers a second time.
- No whole long-canvas RGBA allocations; last-anchor allocation limited to original capture ROI size.
- No false claim that CI validates 50 real scenes or performance / Apple TCC behavior.
- Preserve existing annotation history when retake rejected; require no active annotations before geometry-changing retake.

---

### Task 1: Dynamic-region match tolerance

**Files:** `crates/shot-scroll/src/stitch.rs`; `crates/shot-scroll/tests/rgba_match.rs`.

**Interface:** `match_scroll_frame(previous: &RgbaFrame, next: &RgbaFrame, axis: ScrollAxis)->Option<u32>` remains unchanged.

- [ ] **Step 1 RED:** Add tests for an actual scroll + 20% animated cross-lane band (both axes), a 16/96-pixel animated full-width banner, a 50% animated band, animation-only without scrolling, and repeating wallpaper. Expected shifts 11/9/9 for the first three positive cases; otherwise None.
- [ ] **Step 2:** Verify RED in `cargo test -p shot-scroll --locked --test rgba_match`: old full-line hash or untrimmed mean pixel error incorrectly rejects positive animation scenarios.
- [ ] **Step 3 GREEN:** Split cross-axis into eight lanes; propose shifts via per-lane line hashes ignoring high-frequency repeated rows. Verify candidates using 3-channel per-pixel difference, threshold ≤12, ≥75% good pixel coverage, ≤8 mean good error, positive 35%-overlap rule, and a strict runner-up ambiguity guard. Reject low-information, ambiguous or too-dynamic scenes; never write a tile on None.
- [ ] **Step 4:** `cargo test -p shot-scroll --locked` and `cargo clippy -p shot-scroll --all-targets --locked -- -D warnings` must pass.

### Task 2: Recoverable native viewport / segment location

**Files:** `crates/shot-scroll/src/tile_store.rs`; `crates/shot-scroll/tests/tile_store.rs`.

**Interfaces:**
- `TileStore::segment_span(&self, segment_one_based:usize)->Option<(u32,u32)>`.
- `TileStore::last_viewport(&self)->io::Result<RgbaFrame>` materializes only the original ROI dimensions after any legal trusted-prefix trim.

- [ ] **Step 1 RED:** Add vertical/horizontal test with at least three strips, trim suffix, extract last ROI, append new matched frame and compare every resulting row to the expected contiguous source.
- [ ] **Step 2 GREEN:** Reuse open strip handles for every extracted row and copy the last native viewport; compute segment coordinates with checked/validated spans.
- [ ] **Step 3:** Confirm no orphan temp strips remain and full PNG/preview existing tests still pass.

### Task 3: macOS middle-retake state machine and UI

**Files:** `apps/shot-macos/src/scroll.rs`, `apps/shot-macos/src/app.rs`, macOS scroll module tests.

**Interfaces:**
- `ScrollTarget {display:DisplayId,roi:RectPx,axis:ScrollAxis}` persisted in ScrollReview.
- `ScrollReview::retake_prefix(&mut self,keep_count:usize)->Result<RgbaFrame,String>` returns the old trusted last viewport and keeps a partial review on error.
- `scroll::launch_retake(review,keep_count,sender,ctx)->ScrollCaptureControl` captures in worker; `ScrollEvent::RetakeFailed{review,error}` returns source on failure.
- Reuse existing progress, pause/retry, finish/cancel and live-preview events; a cancelled retake retains its prefix instead of deleting it.

- [ ] **Step 1 RED:** macOS unit test rejects retake while an annotation is visible, verifies no source mutation on rejection; after Undo retakes exactly the chosen prefix, drops redo patch files, and confirms expected last full ROI.
- [ ] **Step 2 GREEN:** Refactor existing capture worker into a shared loop accepting a trusted review and native previous viewport, with explicit fresh-vs-retake cancellation handling and preserve-on-error for resumed sessions.
- [ ] **Step 3 UI:** Add one-based suspect-segment selector from 2..=segment count, show source pixel start/end, explicit button “从该段重新手动采集”. It discards the suffix and hides main viewport, resumes from fixed original ROI; user must backtrack manually until matched. Avoid retake if edits actively applied.
- [ ] **Step 4:** Full macOS CI fmt/workspace test/Clippy/Release + Swift OCR helper + Lite/Full zip must pass.

### Task 4: Merge and ground status

**Files:** `README.md`, `docs/TASKS.md`, `docs/STATUS.md`; GitHub Issue #7.

- [ ] Review diff for wrong-shift false positives, first-segment edge cases, cancellation preserving data, UI using same physical ROI, partial warnings, and dynamic pixels unmodified.
- [ ] Merge only after complete PR CI green. Verify and cite new main CI; explain that dynamic filtering applies to matching, not PNG pixel erasure.
- [ ] Keep true middle in-place splice, auto dynamic erasure, and 50-scene device acceptance explicitly out of scope; this is safe middle-retake from a trusted prefix.

## Acceptance evidence

CI tests prove only bounded synthetic match fixtures, ROI/strip reconstruction, and Rust state transition assertions; they cannot prove success on macOS 14+ capture permissions, actual animated pages, long run stability, clipboard/GPU/pin performance, or 50 real websites.
