# P6-T2 Developer ID and Notary: signed candidate only

This runbook describes a **human-operated private signing stage**, NOT a
public 1.0.0 Release, tag, or proof of successful physical GUI acceptance.
Do not place Apple keys, images, recognized text, raw OCR/scroll samples,
personal paths, or private screenshots in the repository or GitHub artifacts.

## Entry requirements (all mandatory)

- Final PicPaste candidate source SHA **including this signing implementation**
  frozen and identical to the tested build. A clean source checkout with the
  exact approved SHA is mandatory; PR head CI and merge SHA are NOT identical.
- Two real physical QA reports: macOS 14 and macOS 26, 10 scenarios each;
  verified physical Intel and Apple Silicon startup; actual P95 and resource
  measurements; at least 50 scroll scenarios and at least 400 genuine OCR
  samples. Original videos/sample source provenance reviewed in person.
- Existing scripts/release_evidence_gate.py returns 0 on all genuine inputs.
  The output evidence/release-gate.json contains source_sha of this checkout.
- An independent person has examined the *underlying* private evidence and
  signed the review decision administratively. No open Sev-1 or Sev-2.
- Authorized Apple Developer ID Application certificate in the trusted Mac
  Keychain, with actual Team ID matching the host and standalone OCR Add-on.
- A valid notarytool keychain profile already enrolled on that Mac, with Apple
  notary access. All build/Swift/Rust targets and network dependencies available.

**Important:** release_signing_policy.py checks declarations and cryptographic
hash links, NOT actual source authenticity, identities, screenshots, or physical
device behavior. Manually attested JSON can be fabricated; it is not proof by
itself.

## Create the private approval only after genuine evidence review

From the **final source commit** run:

~~~bash
python3 scripts/release_evidence_gate.py \
  --mac14 evidence/macos14/native-acceptance.json \
  --mac26 evidence/macos26/native-acceptance.json \
  --ocr evidence/ocr/real-ocr-samples.csv \
  --scroll evidence/scroll-scenarios.csv \
  --source-sha "$(git rev-parse HEAD)" \
  --output evidence/release-gate.json
shasum -a 256 evidence/release-gate.json
~~~

An actual reviewer must privately create evidence/release-approval.json.
**The values below are schema documentation, NOT a genuine approval.** Do
not submit this example or synthetic unit-test fixtures for signing.

~~~json
{
  "schema": "picpaste.release-approval/1",
  "source_sha": "REPLACE_WITH_REAL_40_CHARACTER_COMMIT_SHA",
  "gate_sha256": "REPLACE_WITH_SHA256_OF_UNMODIFIED_RELEASE_GATE_JSON",
  "reviewer": "REPLACE_WITH_ACTUAL_REVIEWER_IDENTIFIER",
  "reviewed_at": "2026-10-29T11:00:00+08:00",
  "reviewed_original_device_video": false,
  "reviewed_original_ocr_and_scroll_samples": false,
  "verified_mac14": false,
  "verified_mac26": false,
  "verified_arm64_hardware": false,
  "verified_intel_hardware": false,
  "open_sev1": -1,
  "open_sev2": -1
}
~~~

Change the false values to true **only after the corresponding real tests
and reviews have actually been completed**. Set open_sev1 and open_sev2 to
zero only when verified. The gate rejects any incomplete, duplicate, stale,
tampered, symlinked, imbalanced or mismatched evidence file and rejects
approval from a different source SHA. The two JSON files stay in the
gitignored evidence folder, not in source control.

## Controlled signing procedure

On the trusted Mac, configure the keychain outside the repository:

~~~bash
security find-identity -v -p codesigning
xcrun notarytool store-credentials picpaste-notary
xcrun notarytool history --keychain-profile picpaste-notary
~~~

Set these non-secret identifiers in the release operator's trusted session;
the real Keychain contains private credentials (never commit/export them):

~~~bash
export PICPASTE_SIGN_IDENTITY='Developer ID Application: YOUR ACTUAL CERTIFICATE (YOURTEAMID)'
export PICPASTE_APPLE_TEAM_ID='YOURTEAMID'
export PICPASTE_NOTARY_PROFILE='picpaste-notary'
export PICPASTE_RELEASE_SOURCE_SHA="$(git rev-parse HEAD)"
export PICPASTE_RELEASE_GATE="$PWD/evidence/release-gate.json"
export PICPASTE_RELEASE_APPROVAL="$PWD/evidence/release-approval.json"
bash scripts/release-macos-signed.sh
~~~

Use actual 10-character Apple Team IDs and matching real certificate names.
Do NOT execute the example placeholders. The script refuses non-macOS,
uncommitted source changes, a wrong source SHA, stale/missing QA evidence,
absent reviewer declarations, untrusted Team IDs, missing notary credentials,
non-1.0.0 candidates and enabled experimental NSPanel Canary.

It rebuilds Universal 2 Rust and Swift from the approved checkout (does
not accept previously built thin binaries), signs **embedded OCR first,
main executable second, outer .app last**, checks Developer ID, same Team ID,
hardened runtime, then submits the app ZIP to Apple Notary. It requires
Notary **Accepted**, staples the .app, checks the stapled ticket with
stapler, assesses the .app with spctl, then re-archives the stapled app.

It also signs the standalone Universal 2 OCR executable using the **same
Team**, checks its protocol, and notarizes its ZIP. A raw executable cannot
be stapled individually; the standalone ZIP must still pass clean download
and actual installation tests, rather than being advertised as stapled.

## Outputs and release boundary

Only after **all three** submissions return Accepted, the script atomically
publishes a private candidate folder:

~~~text
dist/release-candidate-<source-sha-prefix>/
  Lite/PicPaste.app
  Full/PicPaste.app
  OCR-AddOn/PicPasteOCR
  PicPaste-macos-universal2-Lite-1.0.0-CANDIDATE.zip
  PicPaste-macos-universal2-Full-1.0.0-CANDIDATE.zip
  PicPaste-OCR-AddOn-macos-universal2-1.0.0-CANDIDATE.zip
  notary-Lite.json
  notary-Full.json
  notary-addon.json
  release-manifest.json
  SHA256SUMS.txt
~~~

A repeated attempt refuses to overwrite an existing candidate directory.
Failure cleans the private staging directory and never publishes partial
public assets. Nothing creates an immutable Git tag or GitHub Release.
The manifest explicitly marks clean-download Gatekeeper tests and the
**P6-T3 final manual distribution approval as still required**.

## Next steps before a real v1.0.0 GitHub Release

On real downloaded (quarantined, not locally built) candidates, test physical
macOS 14/26 on Apple Silicon and Intel, Gatekeeper-first-launch behavior,
Screen Recording TCC reauthorization from the new Bundle ID, old-version
upgrade, user data/PNG preservation, rollback, Lite/Full and separately
installed signed OCR Add-on. Do not use xattr -dr or disable Gatekeeper.
Store actual device evidence and signed rollback SHA privately. Only with
fresh human review and zero Sev-1/Sev-2 may P6-T3 publish a public release.

Quick code regression tests require no Apple credentials:

~~~bash
python3 -m unittest discover -s tests -p 'test_release_signing_policy.py' -v
bash -n scripts/release-macos-signed.sh
~~~

**CI is not signing:** public PR runners validate only the policy and shell
syntax. They do not receive real QA evidence or Apple signing secrets.


## Extra pre-publication audit (P6-T3 preparation)

The signer now creates a SHA-linked manifest containing version, bundle ID,
SHA256 of the preexisting QA gate and independent human review declaration.
Before placing output into dist/release-candidate-<source-sha>/ it invokes
scripts/release_candidate_audit.py --verify-macos. This audits every ZIP,
archive-vs-signed-app binary hash, notary receipt and Apple signature using
the same actual Team ID. Failure leaves no completed candidate directory.
For independent reruns and the final **physical** rollout/rollback checklist,
see [release-candidate-audit.md](release-candidate-audit.md).
An approved original QA report is **not** final first-download Gatekeeper or
rollback approval, and no GitHub Release is created.
