# Signed standalone OCR Add-on Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Support a separately packaged offline macOS Vision OCR/QR Helper for Lite without weakening signed Full or basic screenshot safety.

**Architecture:** Keep the existing signed Full helper as first choice. Only when that embedded helper is absent, resolve a fixed per-user Add-on path and require an intact code signature, an identical non-empty Apple Developer Team ID to the app, and an exact versioned capability handshake. Install/uninstall by a user-invoked script, staging/validating in a private directory before atomic replacement. Developer ad-hoc packages are testable but deliberately cannot be installed as trusted standalone Add-ons.

**Tech Stack:** Rust 2024, macOS codesign, Apple Vision Swift helper, Bash, Python unittest for script/contract tests, GitHub Actions macOS runner.

## Global Constraints

- macOS 14+; Rust + egui/eframe + wgpu; no Electron/WebView, no network, PNG-only image output.
- Use existing SHOC v1 pixel request, 80MiB input / 8MiB response / 20s OCR execution timeout.
- Offline helper executes only after explicit user action; never auto-open QR payload URLs.
- Production trust is an exact non-empty Team ID match with the signed host app, and strict signature validation on the installed helper. Never accept arbitrary ad-hoc helper code in Lite.
- OCR Add-on must never be required by screenshot, long screenshot, pins, PNG save or clipboard.
- Real Developer ID certification, notarization, Gatekeeper, and macOS 14+ physical performance acceptance are tracked in #9/#10, not silently claimed by CI.

---

## File responsibility map

- `apps/shot-macos/src/ocr_addon.rs`: fixed-path discovery, signature and Team ID checks, exact protocol handshake, unit tests on malformed/missing/incompatible identities.
- `apps/shot-macos/src/ocr.rs`: choose bundled signed helper first, use verified standalone helper only if bundled is absent.
- `apps/shot-macos/src/main.rs`: register runtime Add-on module.
- `packaging/macos/PicPasteOCR.swift`: signed `--addon-info` capability output without reading image or performing recognition.
- `scripts/manage-ocr-addon.sh`: user-invoked install/uninstall/status, same Team ID, safe staged atomic replacement, private Add-on path, no elevated privileges.
- `scripts/package-ocr-addon.sh`: separately sign/package helper with optional Developer ID; dev ad-hoc builds clearly labeled and not installable.
- `tests/test_ocr_addon_packaging.py`: test installer behavior and unsafe input rejection with controlled fixtures.
- `scripts/check_ocr_helper.py`: positive `--addon-info` check against real Swift helper.
- `.github/workflows/ci.yml`: exercise tests and produce separate ad-hoc development archive.
- `docs/OCR-QR.md`, `README.md`: boundary, usage and unverified gates.

### Task 1: Add-on discovery and trusted identity boundary

**Interfaces:** `ocr_addon::external_helper(app_executable: &Path) -> Result<PathBuf, String>`; `team_identifier(text: &str) -> Option<&str>`; `compatible(reply: &[u8]) -> bool`.

- [x] Write Rust unit tests for missing/adhoc/mismatched Team ID and incompatible handshake; expected positive only with identical non-empty Team ID and exact `PICPASTE-OCR-ADDON/1 SHOC/1\n`.
- [ ] Confirm test is RED from undefined functions, then add module.
- [x] Implement fixed location `~/Library/Application Support/PicPaste/AddOns/OCR/PicPasteOCR`, reject symlinked helper/dir, require `codesign --verify --strict` and `codesign --display --verbose=4` for both host app and Add-on.
- [x] Implement a bounded 2-second `--addon-info` check before accepting external candidate; reject unknown API or SHOC version.
- [x] Connect `ocr.rs` to verified Add-on fallback only if bundled helper is absent; test; commit.

Verification: `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, macOS CI.

### Task 2: Versioned signed Swift capability handshake

**Interfaces:** `PicPasteOCR --addon-info` prints exactly `PICPASTE-OCR-ADDON/1 SHOC/1\n` and exits 0, independent of any SHOC image request.

- [x] Extend `scripts/check_ocr_helper.py` with a real-process test asserting byte-for-byte capability output; observe RED on current helper before modification.
- [x] Add `addonInfo` to the whitelisted Swift helper argument mode; respond without image processing and preserve prior SHOC v1 for OCR/QR.
- [x] Run real Swift compilation and IPC tests on the macOS runner; commit.

Verification: `swiftc -O -target arm64-apple-macos14.0 -framework Vision -framework CoreGraphics -framework Foundation -framework CryptoKit packaging/macos/PicPasteOCR.swift -o target/release/PicPasteOCR`; `python3 scripts/check_ocr_helper.py target/release/PicPasteOCR`.

### Task 3: Independent package and safe installer

**Interfaces:** `bash scripts/package-ocr-addon.sh`; `bash scripts/manage-ocr-addon.sh install <PicPaste.app> <PicPasteOCR signed binary>`; `... uninstall`; `... status <PicPaste.app>`.

- [x] Test missing args, symlink or ad-hoc signed binary refusal, Team ID mismatch, stale/incompatible handshake, and fail-safe reinstall/uninstall.
- [x] Package a separately signed helper (Developer ID identity from `PICPASTE_SIGN_IDENTITY`, else ad-hoc dev only) as `PicPaste-OCR-AddOn-macos-<arch>-<type>.zip` and SHA256.
- [x] Installer verifies source and its staged copy, requires same nonempty Team ID as the app, validates handshake, writes only the fixed private home path under umask 077, atomically swaps using rollback on errors; never changes app bundle.
- [x] Uninstall removes only the owned Add-on helper; status reports installed/missing/invalid. No sudo, no network, no app mutation.
- [x] Add macOS CI packaging and test; commit.

Verification: `python3 -m unittest discover -s tests -p 'test_*.py'`, macOS `codesign --verify --strict` packaging smoke; ad-hoc install should fail closed.

### Task 4: Documentation, CI and delivery

- [x] Update README/OCR-QR/Issue #8 with version/API and installation usage, distinguishing feature code from real Developer ID and physical-device acceptance.
- [x] Run GitHub Actions full Rust + Swift + Python + Lite/Full + independent Add-on packaging. Resolve any CI failures before merge.
- [x] Open PR referencing #8; merge only after full CI succeeds, synchronize remaining-task ledger, and keep #8 open for 400-image CER, cancellation and real-device tests.

## Acceptance checks

1. Full keeps its own helper and rejects a corrupt embedded signature instead of silently selecting external code.
2. Lite without Add-on still performs screenshot/pin/PNG; OCR produces a readable error.
3. Lite with **Team ID-matched, strict verified, compatible** Add-on can execute the existing OCR/QR offline IPC.
4. Missing, corrupted, symlinked, foreign-Team, ad-hoc and incompatible Add-ons are never executed.
5. Installation leaves no partial trusted helper and can be repeated; uninstall does not touch Full or screenshots.
6. CI passes; no claim of successful Developer ID distribution, notarization, or real macOS 14 acceptance without external certificate/device evidence.

## Completed automation evidence

- PR [#29](https://github.com/wuhunyu/pic-paste/pull/29) squash-merged at `3ec7a56` after full [CI #40456135637](https://github.com/wuhunyu/pic-paste/actions/runs/40456135637) success.
- Independent development artifact has an ad-hoc signature and intentionally fails standalone trust validation. This does not establish the signed, notarized Developer ID happy path.
- Real macOS 14+, Apple credentials and end-user installation are still required before the matching-Team positive install acceptance can be checked.
