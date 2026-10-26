# PicPaste implementation ledger — 2026-10-26

## Acceptance rule

Owner permits proceeding without macOS manual testing **after compilation succeeds**.
This does not mean that an uncompiled source tree passes the gate.
GitHub Actions currently exposes **no workflow runs or commit statuses** for the latest connector-created commits; compilation is not verified.

## Completed source work (committed)

- P0-T1 partial: Rust workspace, crate boundaries, egui app smoke window, macOS CI workflow.
- Shared core: physical-pixel region geometry including negative origins and reverse drag, explicit screenshot/scroll session transitions.
- P1 foundations: PNG RGBA encoding, supersampled rounded-corner alpha operation, non-destructive annotation command history, undo/redo tests.
- P2 foundations: multi-pin in-memory state, shared immutable image buffers, geometry validation, passthrough-recovery state and pointer-down input latch tests.
- P3 foundations: bounded, versioned SHOC OCR request transport, invalid-length/version/truncation tests. No recognition provider is integrated.
- P4 foundations: conservative vertical grayscale overlap matcher and strict 60,000-pixel / 200MP canvas limits with unit tests.

## Not implemented

- P0-T2: native macOS status menu, hotkey service and event lifecycle.
- P0-T3: ScreenCaptureKit and real screen coordinate mapping / selection overlay.
- P0-T4: native floating windows, always-on-top and mouse hit-test passthrough.
- P1: complete annotation rasterization, editing UI, clipboard, safe atomic PNG file writing.
- P2: native multi-window behavior, zoom/opacity UI and texture cache.
- P3: signed helper execution, Vision OCR, hit testing and Lite/Full packaging.
- P4: actual scroll capture workers, ROI controls, horizontal matching, streaming tile store / recovery and export.
- P5: release, signing, notarization, benchmark samples and rollback verification.
- P6: Windows native implementation and independent validation.

## Unverified work

The Linux container has no Rust toolchain and cannot reach GitHub directly for clone or dependency retrieval. The GitHub connector can modify files but cannot run Rust tests. No macOS GitHub Actions workflow run was returned as of this update. Consequently **not one cargo check, format, test or Clippy execution may be marked passed**, and no binary may be described as usable.

## Next gate

Enable and run the existing macOS CI workflow; produce and commit Cargo.lock; fix compiler and formatter failures. Then add native platform work and only mark each task complete once its compile gate passes. macOS manual functionality and performance tests may remain deferred by owner approval.

Commands required:

```sh
cargo generate-lockfile
cargo fmt --all
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```


## Subsequent implementation batch (2026-10-26)

- Added `shot-scroll::session` with manual/auto mode, confidence-failure fallback state, canvas-boundary rejection and cancellation state.
- Added `shot-image::stream_png` with PNG row streaming and cancellation propagation.
- Added `scripts/bench_report.py` with p50/p95/max report computation, deterministic Python regression tests and macOS CI test step.
- The GitHub connector can commit repository content but exposes no **workflow dispatch** operation. GitHub queried for commit-associated workflow runs returned no runs; therefore compiler/test outcomes are unknown.
- These new modules are algorithmic foundations and do not substitute for native capture, annotation rendering, live scroll capture, OCR integration or packaging.
