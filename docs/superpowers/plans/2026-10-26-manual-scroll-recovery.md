# Manual Scroll Failure Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a user recover a manual long screenshot when scrolling too fast or when overlap is ambiguous, without throwing away trusted strips or introducing automatic scrolling.

**Architecture:** Add a pure mismatch/retry state machine in `shot-scroll`, and use one atomic capture-control command shared with the macOS worker to park capture after three untrusted changing frames. Keep the same `TileStore` and last trusted `previous` image while paused; only an explicit menu action resumes sampling, and a verified overlap clears the partial-warning state. The existing always-on-top mouse-passthrough live preview shows guidance; the tray menu exposes “继续手动长截图（慢速重试）” so the user can act while the root window is hidden.

**Tech Stack:** Rust 2024, ScreenCaptureKit 11, egui/eframe, tray-icon, no new dependencies.

## Global Constraints

- **No automatic scroll injection, timer-driven scrolling or movement control.** User controls horizontal/vertical scrolling; PicPaste only captures and stitches trusted pixels.
- Preserve PNG-only output and original 60,000px/200MP, 4096-segment, memory limits.
- Never advance previous trusted frame or append TileStore data on mismatch, reversed scroll, animation or uncertain overlap.
- After three consecutive *changed but untrusted* frames: enter **paused recovery** rather than silently finalizing a partial PNG. Identical frames must not count as failures or implicitly reset the counter.
- Keep tray menu accessible when root window is hidden. Paused preview must remain click-through so it cannot block scroll input.
- User can backtrack manually to a common scene, then use tray **Continue** and scroll slowly. Retry is repeatable. No automatic gap fabrication or joining unverified middle segments.
- Finish during unresolved pause/retry keeps a `partial` review warning and only trusted strips; Cancel cleans all private tile files.
- Bad/missing TCC permission or tile I/O errors remain explicit failures; do not swallow them as overlap mismatches.
- macOS interactive mouse/window behavior, fixed headers, animation and ≥50 test scenarios still require real-device acceptance.

---

### Task 1: Trusted-overlap recovery state (TDD)

**Files:**
- Create: `crates/shot-scroll/src/recovery.rs`
- Modify: `crates/shot-scroll/src/lib.rs`
- Test: `crates/shot-scroll/tests/manual_recovery.rs`

**Interfaces:**
- `FrameObservation::{Matched, Unchanged, Untrusted}`
- `RecoveryDecision::{Continue, Pause, IgnoreWhilePaused}`
- `RecoveryTracker::{observe, resume, paused, needs_partial_review, consecutive_failures}`

- [x] Step 1: Add failing Rust tests for the third mismatch, unchanged-frame neutrality, repeated retry, safe pause, and clearing the partial warning only after confirmed overlap.
- [ ] Step 2: Run `cargo test -p shot-scroll --test manual_recovery --locked` via GitHub CI; expected RED `unresolved import shot_scroll::recovery`.
- [ ] Step 3: Create the following minimal production type/state machine:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameObservation { Matched, Unchanged, Untrusted }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryDecision { Continue, Pause, IgnoreWhilePaused }
#[derive(Default)]
pub struct RecoveryTracker {
    failures: u8,
    paused: bool,
    awaiting_verified_overlap: bool,
}
impl RecoveryTracker {
    pub fn paused(&self) -> bool { self.paused }
    pub fn consecutive_failures(&self) -> u8 { self.failures }
    pub fn needs_partial_review(&self) -> bool {
        self.paused || self.awaiting_verified_overlap
    }
    pub fn resume(&mut self) -> bool {
        if !self.paused { return false; }
        self.paused = false;
        self.failures = 0;
        self.awaiting_verified_overlap = true;
        true
    }
    pub fn observe(&mut self, observation: FrameObservation) -> RecoveryDecision {
        if self.paused { return RecoveryDecision::IgnoreWhilePaused; }
        match observation {
            FrameObservation::Matched => {
                self.failures = 0;
                self.awaiting_verified_overlap = false;
                RecoveryDecision::Continue
            }
            FrameObservation::Unchanged => RecoveryDecision::Continue,
            FrameObservation::Untrusted => {
                self.failures = self.failures.saturating_add(1);
                if self.failures >= 3 {
                    self.paused = true;
                    RecoveryDecision::Pause
                } else {
                    RecoveryDecision::Continue
                }
            }
        }
    }
}
```

- [ ] Step 4: Run `cargo test -p shot-scroll --test manual_recovery --locked`, `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings`. Expect GREEN on supported CI runners.
- [ ] Step 5: Commit verified recovery tracker before changing macOS worker.

### Task 2: Pause/Retry/Finish command and manual menu

**Files:**
- Modify: `apps/shot-macos/src/scroll.rs`
- Modify: `apps/shot-macos/src/menu.rs`
- Modify: `apps/shot-macos/src/app.rs`
- Test: `#[cfg(test)]` in `apps/shot-macos/src/scroll.rs`

**Interfaces:** Keep `ScrollCaptureControl::{finish,cancel,take_live_preview}` and add `resume() -> bool`, `is_paused() -> bool`. Add `ScrollEvent::{RecoveryNeeded { segments, width, height }, RecoveryResumed}` and `MenuAction::ResumeScroll`.

- [ ] Step 1: Write macOS unit tests for control atomics: finish succeeds while paused, cancel cannot be overwritten by finish, and retry is only legal while paused. Tests must fail before adding the new API.
- [ ] Step 2: Add `PAUSED=3`, `RETRY=4` and an atomic compare/exchange loop to `finish()` so Finish works from RUNNING, PAUSED, or RETRY but never overwrites CANCEL.
- [ ] Step 3: Replace `mismatches += 1; if mismatches >= 3 { partial=true; break }` in `capture_loop` with `RecoveryTracker::observe(Untrusted)`. If `Pause`, CAS RUNNING→PAUSED, send `RecoveryNeeded`, keep `TileStore` and `previous`. While paused, stop taking screenshots. On RETRY, CAS RETRY→RUNNING, call `tracker.resume()`, send `RecoveryResumed`, and resume at the **same trusted anchor**.
- [ ] Step 4: Do not allow changed source pixels to mutate TileStore after command != RUNNING; do not count identical frames as mismatches. Set `review.partial` when the session is finished while `tracker.needs_partial_review()`.
- [ ] Step 5: Add tray menu item “继续手动长截图（慢速重试）”→`MenuAction::ResumeScroll`. In `app.rs`, add `resume_manual_scroll` and status messages. In live preview, use `ScrollCaptureControl::is_paused()` to show clear instructions to roll back manually and resume via the tray; preview remains `with_mouse_passthrough(true)`.
- [ ] Step 6: Run full macOS fmt/test/Clippy/Release/Swift OCR IPC/Lite+Full workflow; fix any failures before merge.

### Task 3: Documentation, acceptance, merge

**Files:**
- Modify: `README.md`
- Modify: `docs/STATUS.md`
- Modify: `docs/TASKS.md`
- Modify: GitHub Issue [#7](https://github.com/wuhunyu/pic-paste/issues/7)

- [ ] Step 1: Remove “auto scroll missing” as a defect/roadmap obligation. Keep “manual scroll; automatic capture and trusted stitching.” Add failure guidance and recovery protocol.
- [ ] Step 2: Update P4-T3 to “手动失败恢复/固定区域过滤/中间段纠错” and state code vs real-device acceptance separately.
- [ ] Step 3: Record reproducible cases: jump >65% viewport, scroll backward, changing header, animated body, no-motion, repeated failures, finish-paused, cancel-paused, both axes, PNG export.
- [ ] Step 4: Merge only full-green PR; verify current `main` CI. Do not claim 50-scene acceptance or true pixel-perfect macOS performance from CI.
