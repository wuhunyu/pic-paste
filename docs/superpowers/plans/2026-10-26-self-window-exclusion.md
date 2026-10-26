# Prevent PicPaste Screenshot Self-Capture — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop ScreenCaptureKit screenshot and scrolling capture from photographing PicPaste's own live preview and pinned-image windows, without excluding other apps or requiring additional permissions.

**Architecture:** `SCShareableContent::get()` already provides the display and available windows per capture. Pass the same shareable window snapshot into `MacCapture::capture_native`, filter window ownership by our actual process PID, and supply `&[&SCWindow]` to the existing `SCContentFilter::create().with_display().with_excluding_windows()` builder. Reuse the same predicate to omit self-owned windows from smart window candidates.

**Tech Stack:** Rust 2024, `screencapturekit = 11.0.0` on macOS 14+, `shot-core` pure tests, GitHub Actions macOS 26 runner (runtime device behavior unverified).

## Global Constraints

- Capture filter must not hide windows from unrelated processes or those with unknown owners.
- No new dependencies, network use, Electron/WebView or changes to PNG-only policy.
- ScreenCaptureKit calls remain on worker threads; no new TCC capability is requested.
- This guards against PicPaste-origin windows that ScreenCaptureKit reports. It does not prove all popovers or system overlays are excluded on real macOS hardware.
- Do not mark real-device live preview behavior tested based on a green CI run.

---

### Task 1: Ownership predicate, tests first

**Files:** `crates/shot-core/tests/capture_exclusions.rs` (new); `crates/shot-core/src/window_candidates.rs` (modify)

**Interface:** `pub fn is_self_capture_window(owner_pid: Option<i32>, current_pid: i32) -> bool`

- [x] Step 1 — Add tests: exact PID matches; other PIDs, unknown or invalid PIDs do not; multiple same-process windows are excluded.
- [ ] Step 2 — Verify RED: `cargo test -p shot-core --test capture_exclusions --locked` reports unresolved import.
- [ ] Step 3 — Implement predicate: `current_pid > 0 && owner_pid == Some(current_pid)`; no fallback to window title or bundle ID.
- [ ] Step 4 — Verify GREEN: core test + strict core Clippy.
- [ ] Step 5 — Commit implementation.

### Task 2: Filter actual ScreenCaptureKit windows

**Files:** `crates/shot-platform/src/macos/capture.rs` (modify)

**Interfaces:** `capture_native(display: &SCDisplay, windows: &[SCWindow]) -> Result<CapturedFrame, PlatformError>` with `windows.iter().filter(...).collect::<Vec<&SCWindow>>()`; caller passes the `content.windows()` snapshot. Apply to `capture_primary`, `capture_with_placement`, `capture_display`; `capture_region` delegates to `capture_display` already.

- [ ] Step 1 — Add macOS unit test(s) of window ownership mapping (no TCC needed) using the pure helper.
- [ ] Step 2 — Modify capture adapter, no unwrap of missing `owning_application()`. Both self-window exclusion and window-candidate filtering must use the same predicate.
- [ ] Step 3 — Run `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo build --release -p shot-macos --locked`, Swift OCR IPC and Lite/Full packaging via macOS CI.
- [ ] Step 4 — Merge only after green full CI and update #5 / #7 / README / STATUS. Device tests: screen with two active pins + scroll preview while taking 10 consecutive frames and verify no PicPaste window appears in captured output.
