# Changelog

This file describes PicPaste source milestones and does **not** certify a signed,
notarized or publicly distributed version.

## Unreleased — macOS 1.0.0 candidate (2026-10-29)

### Implemented in source / automatic CI

- Rust screenshot core, selection/annotations, PNG-only save/clipboard and
  native capture with stable eframe fallback; optional experimental NSPanel
  companion is **development-only** and excluded from the intended release.
- Manual vertical/horizontal scrolling and conservative trusted stitching,
  recoverable segments, transparently rounded PNG and pinned images.
- Offline Apple Vision OCR, word/character selection, QR text and a separately
  packaged same-Team trusted OCR Add-on.
- arm64 + x86_64 Universal 2 **development** Lite/Full ZIPs and a private
  Developer ID/Notary candidate builder that refuses missing QA approvals.
- Candidate integrity audit checking ZIP/member hashes, Apple Team, notarization
  records and approved QA provenance hashes.
- Physical release rehearsal tool for SHA256/quarantine/signature preflight,
  operator-observed first launch/TCC/PNG/OCR and data-preserving rollback
  evidence on macOS 14 and 26 / Apple Silicon and Intel.

### Not yet accepted or published

- Real macOS 14/26 GUI, permissions, Retina, IME and hardware performance
  acceptance; 50 real scroll scenarios and 400 real OCR samples.
- Verified real Developer ID signing, Apple notarization, first-download
  Gatekeeper behavior, upgraded settings/PNG preservation and rollback drills.
- Independent manual final sign-off, immutable v1.0.0 tag and public
  GitHub Release. The repository currently remains **unreleased**.

Do not download ad-hoc UNSIGNED-DEV archives expecting a trusted public app.
Actual release notes will be finalized only after signed asset and physical
acceptance evidence is approved.
