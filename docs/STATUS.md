# PicPaste implementation status · 2026-10-26

Source specification: [Rust screenshot development package v1.2](../README.md). Current branch: `feat/macos-capture-png-pin-20261026`.

## Verified automatic build gates

- Main baseline: [CI #31](https://github.com/wuhunyu/pic-paste/actions/runs/40263680159) — green.
- Pixel crop and atomic PNG save: [CI #34](https://github.com/wuhunyu/pic-paste/actions/runs/40264536444) — green; preceding TDD red #33 documented unresolved imports.
- ScreenCaptureKit native adapter on macOS-26 SDK: [CI #36](https://github.com/wuhunyu/pic-paste/actions/runs/40264846742) — green.
- Screenshot selection UI, global hotkeys, pasted and independent pinned viewports: [CI #38](https://github.com/wuhunyu/pic-paste/actions/runs/40265276885) — green.
- Later menu and packaging builds: pending final separate CI status. Do not claim green before results.

## Stage ledger

| Stage | Code status | Acceptance gap |
| --- | --- | --- |
| P0-T1 Workspace | Build/test/lint green; `Cargo.lock` not yet tracked | Commit lock and CI check-only formatting |
| P0-T2 Status bar/hotkeys | Implemented mac menu, hotkeys and dispatch | Menu bar real-world run and hotkey conflicts |
| P0-T3 Screen capture | Native ScreenCaptureKit, pixel crop and preview box | Full-screen overlay, multi-display and mixed-DPI checks |
| P0-T4 Pins | Multiple native egui top-level viewports, always-on-top, passthrough/recover UI | Spaces, always-on-top, mouse passthrough and latency measurements |
| P1 Selection/annotation | Inline selection, command history, line/rect/pixelate compositor, undo/redo unit tests | In-app annotation UI, ellipse/arrow/blur/text, eyedropper/magnifier |
| P1 PNG | PNG-only export, non-clobber atomic save, corner alpha, clipboard interface | Mainstream target-app paste and color management |
| P2 Pins | Independent viewport, shared image Arc, opacity | Zoom, texture cache and drag/input-router real-world tests |
| P3 OCR | Versioned bounded SHOC transport | Vision provider, helper lifecycle, text selection, Lite/Full |
| P4 Scroll | Session, bounds, vertical matcher, PNG row writer | Live/manual/auto scroll capture, horizontal, tile store, review |
| P5 Delivery | Unsigned development .app script | Developer ID sign, notarize, measurements and release gates |
| P6 Windows | Shared interfaces only | Native Windows capture/overlay/windowing/OCR/packaging |

## Security and acceptance

- No telemetry or cloud dependencies are intentionally added; image data is kept local.
- Image paths are always `.png`; no public end-user format selector.
- Screen Recording permission on macOS is still required and CI cannot provide it for interactive capture.
- The macOS 26 SDK is a **build-time** requirement imposed by a Swift/Metal dependency; the deployment target remains 14.0. The minimum runtime version is not independently proven without real-device testing.
- Current CI auto-formats its checkout before running fmt check. This is tracked tech debt and should be replaced by a check-only formatting gate once formatting is committed.
- `Cargo.lock` is generated during CI but is not yet committed, so dependency resolution is not fully reproducible.
- Unsigned development artifacts are **not** suitable for distribution as a public release.

## Reproducible commands

```sh
cargo generate-lockfile
cargo fmt --all
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release -p shot-macos
bash scripts/package-macos.sh
```

Nothing in this ledger means P0–P6 is complete or macOS real-device behavior has been validated.
