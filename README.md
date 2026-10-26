# PicPaste

Lightweight offline screenshot and pinning application, macOS first, developed in Rust.

> **Project status:** P0 prototype / engineering foundation. The native capture, hotkeys, pin windows, OCR and scrolling capture are **not implemented yet**. Do not treat this repository as a usable release.

## Design constraints

- macOS 14+ first; Windows follows after macOS validation.
- Rust 2024, egui/eframe, native OS adapters; no Electron or WebView.
- Saved screenshots and exports are PNG only, preserving rounded-corner transparency.
- Offline operation; OCR is optional and cannot block basic screenshots.
- Scroll capture defaults to manual scrolling with recovery on mismatch.

## Developer workflow

```sh
cargo generate-lockfile
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p shot-macos
```

Run native integration and performance gates on real macOS hardware before advancing past P0. The development specification is under `docs/spec/`.
