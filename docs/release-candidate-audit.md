# P6-T3 preparation: signed candidate integrity and rollback hold

**Purpose:** fail closed between Apple Notary acceptance and human-authorized
public distribution. This does NOT publish a GitHub Release or tag, does not
create a rollback archive, and does not replace real Gatekeeper/upgrade tests.

The signed-candidate builder (scripts/release-macos-signed.sh) now audits
every **private** output directory before moving it into the stable
dist/release-candidate-<sha>/ location.

## Automated audit coverage

scripts/release_candidate_audit.py verifies:

- release-manifest.json: full frozen source SHA, expected Team ID,
  1.0.0 candidate identity, Bundle ID, and **SHA256 of both** the genuine
  quality gate JSON and separate human reviewer approval JSON. It re-runs
  the existing approval policy and requires that the manifest still declares
  distribution NOT approved and Gatekeeper testing outstanding.
- SHA256SUMS.txt: exactly three expected archive entries, no duplicates,
  strict shasum -a 256 format, and full byte-by-byte SHA256 agreement.
  A changed file cannot be silently re-labelled as approved.
- notary-Lite.json, notary-Full.json, notary-addon.json: all three
  must claim Accepted and have different nonempty submission IDs.
  This checks the captured data structure; authenticity of submission
  records still depends on the trusted Apple operator/session.
- Each ZIP: CRC integrity; no absolute/traversal paths, unsafe symlinks,
  duplicate members or experimental NSPanel companion; exact Lite/Full/OCR
  expected content. Embedded Info.plist must report the approved Bundle ID,
  version, icon and macOS floor.
- Each archived executable, icon and Info.plist must have the **same SHA256
  bytes as the staged app/add-on files**. Lite cannot secretly embed OCR;
  Full must embed nonempty OCR; standalone OCR must be present and match.
- When --verify-macos is supplied on a real Mac: codesign --verify,
  expected Developer ID + matching Team + Hardened Runtime for app/main/OCR,
  stapler ticket validation and Gatekeeper local assessment for Lite/Full.
  A Linux CI run refuses --verify-macos, rather than marking native
  signing checks as passed.

The audit JSON **always** returns ready_for_public_release: false and
physical_download_verification_completed: false even if every check passes.
A synthetically generated ZIP and fake review declarations can pass **static
unit tests**; these are NOT trusted Apple signatures or genuine QA.

## How to run it

On the trusted release Mac, AFTER scripts/release-macos-signed.sh has
completed with the accepted human review, all final outputs live under
dist/release-candidate-<commit-prefix>/. Use the precise real paths:

~~~bash
python3 scripts/release_candidate_audit.py \
  --candidate "dist/release-candidate-$(git rev-parse --short=12 HEAD)" \
  --source-sha "$(git rev-parse HEAD)" \
  --team-id "$PICPASTE_APPLE_TEAM_ID" \
  --gate "$PWD/evidence/release-gate.json" \
  --approval "$PWD/evidence/release-approval.json" \
  --verify-macos
~~~

The signing script invokes **exactly this native audit before any final
candidate directory is published**; if the audit fails, the temporary
staging folder is removed by the existing cleanup handler. No code signing
key, Keychain password, raw screenshot or private QA video is uploaded by CI.

On Linux or a pull request, run **only** synthetic negative tests:

~~~bash
python3 -m unittest discover -s tests -p test_release_candidate_audit.py -v
python3 -m unittest discover -s tests -p test_release_signing_policy.py -v
bash -n scripts/release-macos-signed.sh
~~~

## Mandatory physical rollout/rollback gate — still OPEN

Before preparing a public v1.0.0 release, an authorized reviewer must:

1. Download the quarantined final candidate archives on **actual macOS 14
   and 26**, covering Apple Silicon and Intel; confirm Gatekeeper first
   launch, actual Developer ID trust, TCC Screen Recording authorization,
   Retina, Spaces, IME and screenshot reliability.
2. Validate Lite→Full and signed standalone OCR Add-on installation with
   the SAME Team ID, all legitimate user settings and PNG files preserved.
3. Exercise upgrades from the previous Bundle ID and explicit recovery/
   uninstall path. For a first-ever signed version, document and *test*
   data-preserving removal/reinstall instead of inventing a prior stable
   signed package. Record 1 rollback scenario per supported OS/architecture.
4. Collect real screenshots/video, measured acceptance reports, ZIP SHA256,
   reviewer identity, observations and an approved contingency for Gatekeeper,
   damaged archives or OCR installation. Keep these private.
5. Confirm zero Sev1/Sev2 and final human release approval. Only the separate
   P6-T3 publication action may then tag and upload immutable signed assets.

**Never** remove quarantine with xattr -dr, disable Gatekeeper, or accept a
CI runner as proof of normal first-download macOS behavior. The initial
device acceptance approval predates signed package creation; the **final
signed-package Gatekeeper / rollback approval is distinct and still required**.
