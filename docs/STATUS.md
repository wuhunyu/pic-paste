# Implementation status (2026-10-26)

## Repository foundation

- P0-T1: **in progress**. Rust workspace, core geometry/state machine, PNG encoder and CI scaffold committed.
- Native macOS 14+ ScreenCaptureKit capture: **not implemented**.
- Global hotkeys, AppKit menu bar, transparent pinned windows and mouse passthrough: **not implemented**.
- Screenshot/annotation workflow, OCR, scrolling capture and Windows adapters: **not implemented**.
- The UI is a smoke test, not a functional screenshot application.

## Verification

The environment used to initialize the repository is Linux with no Rust toolchain and no direct GitHub network DNS access. It cannot compile these sources or run macOS TCC, Spaces, Retina or passthrough tests. GitHub Actions on macOS must provide the first compilation signal; treat until then as **unverified**. The lockfile must be generated and committed after toolchain verification.

## Next sequential gates

1. Resolve any compiler/API mismatches surfaced by GitHub CI for eframe 0.37.0; run format, Clippy and tests on macOS.
2. Implement P0-T2 global hotkeys, native menu bar and lifecycle.
3. Implement P0-T3 ScreenCaptureKit frame capture and coordinate mapping.
4. Implement P0-T4 pinned NSWindow/NSPanel and real latency measurements.
5. Do not advance into P1 until the P0 platform acceptance gates pass.

Refer to the v1.2 development package index and 08-implementation-plan.md for authoritative tasks.
