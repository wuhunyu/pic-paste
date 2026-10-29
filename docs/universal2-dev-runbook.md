# macOS Universal 2: unsigned development build and architecture verification

This is a **P6-T2 build-tooling subtask**, NOT signed release delivery and
NOT macOS 14/26 physical GUI acceptance.

## Outputs and constraints

- dist-universal2/PicPaste-macos-universal2-Lite-UNSIGNED-DEV.zip
- dist-universal2/PicPaste-macos-universal2-Full-UNSIGNED-DEV.zip
- Adjacent .sha256 checksum files.
- Lite contains a genuine arm64 + x86_64 Contents/MacOS/PicPaste with no
  built-in OCR helper. Full contains the same Universal 2 app plus universal
  Contents/Helpers/PicPasteOCR (offline Apple Vision OCR and QR).
- No NSPanel companion is included; stable eframe remains default.
- Ad-hoc signatures are verified for development integrity only. ZIPs are
  **not** Developer ID-signed, notarized, or public GitHub Releases.

## Build on Apple Silicon with Xcode

Requirements: macOS 14+ Xcode, stable Rust, Python 3.11+, Swift, Apple lipo,
codesign, and ditto.

From repo root:

~~~bash
python3 -m unittest discover -s tests -p test_universal_macho.py -v
bash -n scripts/build-macos-universal-dev.sh
bash scripts/build-macos-universal-dev.sh
lipo -archs dist-universal2/Full/PicPaste.app/Contents/MacOS/PicPaste
lipo -archs dist-universal2/Full/PicPaste.app/Contents/Helpers/PicPasteOCR
shasum -a 256 -c dist-universal2/PicPaste-macos-universal2-Lite-UNSIGNED-DEV.zip.sha256
shasum -a 256 -c dist-universal2/PicPaste-macos-universal2-Full-UNSIGNED-DEV.zip.sha256
~~~

The builder installs the aarch64-apple-darwin and x86_64-apple-darwin Rust
targets if necessary (network may be needed for installation), builds both
Rust architecture slices, compiles Swift Vision OCR for both with minimum
macOS 14, then uses lipo and checks every input + output **before packaging**.

For the macOS CI runner, it is possible to reuse ARM64 binaries produced
**from the exact same source SHA**, as a matching pair:

~~~bash
PICPASTE_UNIVERSAL_ARM64_BINARY="$PWD/target/release/shot-macos" \
PICPASTE_UNIVERSAL_ARM64_OCR_BINARY="$PWD/target/release/PicPasteOCR" \
bash scripts/build-macos-universal-dev.sh
~~~

These variables must be supplied together. Missing, symlinked or wrongly
labelled inputs fail before existing universal app replacement. Do not reuse
Canary-feature binaries or artifacts from different commits.

The PICPASTE_PACKAGE_ARCH=universal2 override in package-macos.sh rejects
thin binaries or a Full package with a thin OCR helper **before deleting the
previous app**. Explicit arm64/x86_64 overrides also verify actual CPU types.
Without the override, native-architecture Lite/Full/Canary packaging remains
unchanged.

## CI evidence

- Existing Rust CI runs 11 synthetic and fail-closed Mach-O unit tests and
  validates the new shell script syntax on Ubuntu.
- The separate macOS Universal 2 Dev QA workflow builds actual arm64/x86_64
  executable slices, assembles both development apps, validates the embedded
  main/OCR architectures, codesign structure, ZIP hashes, and rejects a
  deliberately mislabeled thin file without deleting the valid package.
- This expensive workflow triggers on relevant build/packaging inputs
  (or manually), not on every unrelated PR.
- Headless CI cannot prove x86_64 launch on physical Intel hardware, macOS
  Screen Recording permission prompts, Dock icon, performance, IME, Spaces
  or notarization.

## Remaining release work (hard blockers)

1. Real macOS14 and 26 hardware tests, including Apple Silicon and Intel,
   matching final source SHA and real GUI/performance, scroll and OCR evidence.
2. Apple certificate ownership, Developer ID and hardened runtime signatures
   of host and nested helpers using the same Team ID.
3. Notary Accepted, stapling, Gatekeeper clean-download execution, and a
   correctly notarized standalone Universal 2 OCR Add-on.
4. Release gate and human provenance approval, signed rollback package,
   immutable git tag and public GitHub Release.

Do NOT rename these developer archives to production versioned ZIPs, remove
quarantine or bypass Gatekeeper, or publish before all gates pass.

## Rollback

Revert this tooling PR without touching installed applications. With no
PICPASTE_PACKAGE_ARCH set, the original package-macos.sh still builds
native host-architecture Lite/Full or explicit Canary packages.
User screenshots and settings are not modified by these scripts.
