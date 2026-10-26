# PicPaste development ledger · 2026-10-26

## Confirmed successful checkpoints

- Main merged prototype: [CI #111](https://github.com/wuhunyu/pic-paste/actions/runs/40270735124) — **green** on commit `ddf121c125230829f15651819222d0fbfdd1d435`.
- Earlier PR #1 [merged](https://github.com/wuhunyu/pic-paste/pull/1): native ScreenCaptureKit capture, selected PNG, image pinning, hotkeys and status menu.
- Earlier PR #2 [merged](https://github.com/wuhunyu/pic-paste/pull/2): draw/pixelate/blur, independent pin zoom, manual vertical/horizontal scroll capture and local Apple Vision Full helper; [CI #109](https://github.com/wuhunyu/pic-paste/actions/runs/40270117959) also compiled Swift, exercised real SHOC pipe IPC, and assembled Lite/Full.
- Current [PR #3](https://github.com/wuhunyu/pic-paste/pull/3) is **in progress**, not an accepted release. It adds bounded long screenshot review, OCR polygon clicking and the fullscreen editor viewport. Check PR-head CI before marking these complete.

## Current implementation by stage

| Stage | Implemented in repository | Still required before completion |
| --- | --- | --- |
| P0 workspace | Rust workspace, tracked `Cargo.lock`, macOS CI fmt/test/clippy/Release + dev bundle | Real macOS 14 host, cold/warm startup and idle RSS/CPU |
| P0 screenshot | ScreenCaptureKit primary display; display ID and image physical pixels; fullscreen root editor in PR #3 | Dedicated AppKit NSPanel, TCC/Spaces/Retina/DPI real-device correctness (independent borderless native viewport introduced in PR #13) |
| P0 menu/hotkeys | Status bar, Ctrl+Option+1/2/3/4, recover passthrough | Hotkey settings/conflict, 30 restart tests, performance |
| P1 annotation | Line/rectangle/ellipse/arrow/brush/pixelate/blur, undo/redo; PR #3 adds eight selection handles, drag-to-move, an exact physical-pixel 9x9 magnifier and HEX copy | Text with Chinese fonts, automatic window candidates, real-device mixed-DPI magnifier verification |
| P1 PNG | PNG-only atomic no-clobber export, rounded transparent alpha and clipboard | Paste tests in common native apps, color profile fidelity |
| P2 pins | Multiple independent egui viewports, zoom, opacity, passthrough/recovery | Native Spaces and drag/scroll/selection routing tests, texture budget |
| P3 OCR | Optional offline Apple Vision helper in Full, SHOC framing, 20s timeout, Lite missing-helper degrade; PR #3 adds OCR line click and capture/edit/selection ticket validation to discard stale responses | Pin text mode, real 400-image CER benchmark, word boxes/provider cache, Add-on signed installer |
| P4 scroll | Manual vertical/horizontal sampling, confidence reject, disk tile strips, streamed PNG, size limits; thumbnail and review in PR #3 | True real-time preview, automatic scrolling, seam edit, sticky-header mask, scene accuracy ≥95% |
| P5 packaging | ARM64 ad-hoc-signed Lite/Full development zips and CI SHA256 | x86_64, Developer ID, notarization, device release matrix, rollback drills |
| P6 Windows | Shared Rust interfaces only | Windows platform adaptation **after** macOS release |

## Risk / acceptance evidence

- macOS CI cannot grant interactive Screen Recording, Accessibility or Automation permissions; no TCC or frame-latency claims are proven by these builds.
- The deployment target is macOS 14, but the dependency toolchain currently uses a macOS 26 SDK; minimum supported runtime must be tested.
- OCR is offline. Full's bundled helper is launched on demand; application must keep taking screenshots when helper is absent.
- Long screenshot uses a private scratch directory and remains vulnerable to animated content and low-information matches: ambiguous motion must not be appended.
- All image file output paths must have a `.png` extension. Temporary `.part` / raw `.rgba` files are scratch only and never user-exportable.
- Developer-only bundles have ad-hoc signatures; they are **not** valid publicly notarized releases.

## Commands

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --release -p shot-macos --locked
python3 -m unittest discover -s tests -p 'test_*.py'
```

Do not mark PR #3 completed until its own Actions jobs have passed all required gates. P0–P6 is not considered finished while the real-device and release quality checks remain outstanding.
