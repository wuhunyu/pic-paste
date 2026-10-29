# P6-T3 Physical Gatekeeper and rollback rehearsal

This is an **operator-only physical device acceptance procedure**, NOT a public
PicPaste v1.0.0 release. The accompanying Python test suite uses synthetic
fixtures and cannot authenticate real device videos, Apple credentials or
manual observations.

## Prerequisites

- A PRIVATE Developer-ID-signed, notarized, stapled release candidate for the
  exact frozen source SHA, audited via scripts/release_candidate_audit.py.
- Download all three Lite/Full/standalone-OCR CANDIDATE ZIPs using the same
  browser workflow as end users. Obtain SHA256 values from the original trusted
  private manifest, **not from a checksum beside an untrusted replacement ZIP**.
- Extract Lite and Full into separate folders using Finder/Archive Utility,
  without opening either app first. They share a Bundle ID. Preserve
  com.apple.quarantine on all ZIPs and both .apps.
- Use a dedicated, non-production macOS test account. Create a test fixture
  folder containing at least one PNG and a small configuration test file.
  No real user data should be copied into the fixture.
- Do not disable Gatekeeper, delete quarantine attributes, or import Apple
  certificate private keys to GitHub Actions.

## 1. Preflight on a real Mac before first Finder launch

This script is read-only apart from writing the NOT-RUN receipt. It checks
host OS and CPU, downloaded ZIP hashes/quarantine, app quarantine, Developer
ID Application + matching Team ID, hardened runtime, arm64/x86_64 Universal 2
slices, app stapler tickets and local spctl assessment. It never installs or
launches an app and refuses to run prepare in CI.

~~~bash
python3 scripts/physical_release_rehearsal.py prepare \
  --host-label mac14-arm64 \
  --source-sha "$PICPASTE_RELEASE_SOURCE_SHA" \
  --team-id "$PICPASTE_APPLE_TEAM_ID" \
  --lite-zip "$HOME/Downloads/PicPaste-macos-universal2-Lite-1.0.0-CANDIDATE.zip" \
  --full-zip "$HOME/Downloads/PicPaste-macos-universal2-Full-1.0.0-CANDIDATE.zip" \
  --addon-zip "$HOME/Downloads/PicPaste-OCR-AddOn-macos-universal2-1.0.0-CANDIDATE.zip" \
  --lite-sha256 "$LITE_SHA256_FROM_TRUSTED_MANIFEST" \
  --full-sha256 "$FULL_SHA256_FROM_TRUSTED_MANIFEST" \
  --addon-sha256 "$ADDON_SHA256_FROM_TRUSTED_MANIFEST" \
  --lite-app "$HOME/Downloads/PicPaste-Lite/PicPaste.app" \
  --full-app "$HOME/Downloads/PicPaste-Full/PicPaste.app" \
  --addon-exe "$HOME/Downloads/PicPasteOCR" \
  --data-root "$HOME/PicPaste-TestFixture" \
  --output evidence/physical/mac14-arm64/report.json
~~~

App and ZIP quarantine must be present BEFORE Finder first launch. A successful
spctl assessment does **not** prove a first-launch prompt or Screen Recording
TCC experience; all seven manual cases start with status NOT-RUN.

## 2. Record seven genuinely observed cases

Every case requires an actual nonempty evidence file beneath the report
directory, a human description, and explicit pass/fail. No user screenshots,
OCR text or private file paths are written into the JSON by the tool.

| Case | Manual test |
|---|---|
| quarantined_first_launch | Finder launch without bypassing Gatekeeper |
| tcc_screen_recording | Deny, grant, revoke and restart Screen Recording |
| capture_png_alpha | Genuine screenshot, transparency and PNG preservation |
| pin_clipboard_and_ime | Pinned windows, clipboard, CJK IME and focus |
| lite_full_switch | Lite/Full install-switch with prior preferences preserved |
| standalone_signed_ocr | Same-Team trusted Add-on, OCR and QR, uninstall |
| data_preserving_rollback | Genuine rollback or data-preserving reinstall |

Example for just one observed case:

~~~bash
python3 scripts/physical_release_rehearsal.py mark \
  --report evidence/physical/mac14-arm64/report.json \
  --case quarantined_first_launch --status passed \
  --evidence first-launch.mov \
  --notes "Finder first launch succeeded with Gatekeeper and quarantine intact."
~~~

Do not mark a case passed if only the tool's automated preflight succeeded.
To record a failed scenario, use --status failed and preserve its evidence.

## 3. Verify preserved data after a real rollback or reinstall

The read-only fingerprint includes all relative file names and file SHA256
digests from the dedicated fixture, while the report stores only the aggregate
digest and counts. It never restores or deletes data. Actual app preferences
and user-generated PNGs outside the fixture also require manual confirmation.

~~~bash
python3 scripts/physical_release_rehearsal.py postcheck \
  --report evidence/physical/mac14-arm64/report.json \
  --data-root "$HOME/PicPaste-TestFixture"
python3 scripts/physical_release_rehearsal.py verify \
  --report evidence/physical/mac14-arm64/report.json
~~~

## 4. Four OS x CPU combinations

Release evidence must cover **macOS 14 + 26** and both **Apple Silicon +
Intel**. Missing Intel hardware is a genuine blocking dependency; CI runners
or emulated machines are not substitutes for physical device acceptance.

~~~bash
python3 scripts/physical_release_rehearsal.py matrix --reports \
  evidence/physical/mac14-arm64/report.json \
  evidence/physical/mac14-intel/report.json \
  evidence/physical/mac26-arm64/report.json \
  evidence/physical/mac26-intel/report.json
~~~

The matrix checks the same frozen source SHA, Apple Team, identical three ZIP
hashes, distinct host labels, all manual evidence references, and matching
before/after PNG and config fingerprints. Success is **eligible for an
independent human review only**; ready_for_public_release is always false.
Manual proof origin cannot be verified by any JSON schema.

If no prior trusted signed version exists, document and exercise a
**data-preserving removal/reinstall** rather than claiming a rollback to
an imaginary signed baseline. Keep all real videos private and gitignored.

## 5. CI limitation

~~~bash
python3 -m unittest discover -s tests -p test_physical_release_rehearsal.py -v
python3 scripts/physical_release_rehearsal.py --help
~~~

These tests use synthetic data, **not actual hardware**. Apple Developer
credentials, macOS 14/26 physical tests, initial TCC/IME/Retina/Spaces, true
400-image OCR, 50 genuine scroll scenes and a separate manual distribution
decision remain hard blockers. No tag or public GitHub Release is created.
