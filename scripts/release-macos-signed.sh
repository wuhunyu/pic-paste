#!/usr/bin/env bash
# P6-T2: signed + notarized Universal 2 RELEASE CANDIDATES, not public releases.
# Run only on a trusted macOS release host with real physical QA and Apple ID.
set -euo pipefail
umask 077

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
[[ "$(uname -s)" == "Darwin" ]] || {
  echo "A trusted macOS release host with Xcode is required" >&2
  exit 1
}

: "${PICPASTE_SIGN_IDENTITY:?Developer ID Application identity is required}"
: "${PICPASTE_APPLE_TEAM_ID:?Actual Apple Team ID is required}"
: "${PICPASTE_NOTARY_PROFILE:?Keychain notarytool profile is required}"
: "${PICPASTE_RELEASE_SOURCE_SHA:?Expected full source SHA is required}"
: "${PICPASTE_RELEASE_GATE:?Real QA gate JSON path is required}"
: "${PICPASTE_RELEASE_APPROVAL:?Human review approval JSON path is required}"

# Never replace a functioning installation or enable an unaccepted Canary.
if [[ "${PICPASTE_NATIVE_CANARY:-0}" != "0" ]]; then
  echo "Native NSPanel Canary cannot be used in signed 1.0 candidates" >&2
  exit 2
fi
VERSION="${RELEASE_VERSION:-1.0.0}"
[[ "$VERSION" == "1.0.0" ]] || {
  echo "The signed candidate version must match the approved 1.0.0 plan" >&2
  exit 2
}
SHA="$(git rev-parse --verify HEAD)"
[[ "$SHA" == "$PICPASTE_RELEASE_SOURCE_SHA" ]] || {
  echo "Source SHA differs from the specifically approved release candidate" >&2
  exit 2
}
[[ -z "$(git status --porcelain --untracked-files=all)" ]] || {
  echo "Refusing to sign an unclean checkout" >&2
  exit 2
}

python3 "$ROOT/scripts/release_metadata.py" --root "$ROOT" --expected-version "$VERSION"
python3 "$ROOT/scripts/release_signing_policy.py" preflight \
  --source-sha "$SHA" --gate "$PICPASTE_RELEASE_GATE" \
  --approval "$PICPASTE_RELEASE_APPROVAL" --team-id "$PICPASTE_APPLE_TEAM_ID" \
  --identity "$PICPASTE_SIGN_IDENTITY" --notary-profile "$PICPASTE_NOTARY_PROFILE"

# Fail before costly cross-architecture builds if trust/credentials are absent.
if ! security find-identity -v -p codesigning |
    grep -F -- "$PICPASTE_SIGN_IDENTITY" > /dev/null; then
  echo "The approved Developer ID Application certificate is unavailable" >&2
  exit 3
fi
xcrun notarytool history --keychain-profile "$PICPASTE_NOTARY_PROFILE" --output-format json >/dev/null

OUT="$ROOT/dist/release-candidate-${SHA:0:12}"
[[ ! -e "$OUT" && ! -L "$OUT" ]] || {
  echo "Candidate already exists; refusing to overwrite: $OUT" >&2
  exit 4
}
mkdir -p "$ROOT/dist"
STAGING="$(mktemp -d "$ROOT/dist/.signed-staging.XXXXXXXX")"
cleanup() { rm -rf -- "$STAGING"; }
trap cleanup EXIT

# Rebuild all four architecture slices from THIS clean checkout. External
# prebuilt thin binaries/ad-hoc archives cannot be smuggled into signing.
(
  unset PICPASTE_UNIVERSAL_ARM64_BINARY PICPASTE_UNIVERSAL_ARM64_OCR_BINARY
  unset PICPASTE_ICON_BINARY PICPASTE_PACKAGE_ARCH PICPASTE_DIST_DIR
  PICPASTE_NATIVE_CANARY=0 PICPASTE_UNIVERSAL_DIST_DIR="$STAGING/build" \
    bash "$ROOT/scripts/build-macos-universal-dev.sh"
)
mkdir -p "$STAGING/output"

verify_signed() {
  local path="$1" description="$2" details
  codesign --verify --strict --verbose=2 "$path"
  details="$(codesign --display --verbose=4 "$path" 2>&1)" || {
    echo "Unable to inspect Developer ID signature: $description" >&2
    exit 5
  }
  [[ "$details" == *"Authority=Developer ID Application:"* ]] || {
    echo "Not Developer ID Application-signed: $description" >&2
    exit 5
  }
  [[ "$details" == *"TeamIdentifier=$PICPASTE_APPLE_TEAM_ID"* ]] || {
    echo "Mismatched Apple Team ID: $description" >&2
    exit 5
  }
  [[ "$details" == *"(runtime)"* ]] || {
    echo "Missing hardened runtime: $description" >&2
    exit 5
  }
}

sign_executable() {
  codesign --force --options runtime --timestamp --sign "$PICPASTE_SIGN_IDENTITY" "$1"
  verify_signed "$1" "$2"
}

notarize_zip() {
  local file="$1" report="$2"
  xcrun notarytool submit "$file" --keychain-profile "$PICPASTE_NOTARY_PROFILE" \
    --wait --output-format json > "$report"
  python3 "$ROOT/scripts/release_signing_policy.py" notary --report "$report"
}

for variant in Lite Full; do
  APP="$STAGING/output/$variant/PicPaste.app"
  mkdir -p "$STAGING/output/$variant"
  ditto "$STAGING/build/$variant/PicPaste.app" "$APP"
  python3 "$ROOT/scripts/universal_macho.py" --universal "$APP/Contents/MacOS/PicPaste"
  [[ ! -e "$APP/Contents/MacOS/ns-panel-probe" ]] || {
    echo "Unexpected Canary helper in candidate: $variant" >&2
    exit 5
  }
  if [[ "$variant" == "Full" ]]; then
    HELPER="$APP/Contents/Helpers/PicPasteOCR"
    python3 "$ROOT/scripts/universal_macho.py" --universal "$HELPER"
    sign_executable "$HELPER" "$variant embedded OCR helper"
  else
    [[ ! -e "$APP/Contents/Helpers/PicPasteOCR" ]] || {
      echo "Lite release candidate must not bundle an OCR helper" >&2
      exit 5
    }
  fi
  sign_executable "$APP/Contents/MacOS/PicPaste" "$variant main executable"
  codesign --force --options runtime --timestamp --sign "$PICPASTE_SIGN_IDENTITY" "$APP"
  codesign --verify --deep --strict --verbose=2 "$APP"
  verify_signed "$APP" "$variant app bundle"

  # Staple to the .app AFTER acceptance, THEN create final ZIP. Stapling an
  # already zipped app does not add the notarization ticket to that archive.
  SUBMIT="$STAGING/notary-submit-$variant.zip"
  ditto -c -k --sequesterRsrc --keepParent "$APP" "$SUBMIT"
  notarize_zip "$SUBMIT" "$STAGING/output/notary-$variant.json"
  xcrun stapler staple "$APP"
  xcrun stapler validate "$APP"
  spctl --assess --type execute --verbose "$APP"
  codesign --verify --deep --strict --verbose=2 "$APP"
  ZIP="$STAGING/output/PicPaste-macos-universal2-$variant-$VERSION-CANDIDATE.zip"
  ditto -c -k --sequesterRsrc --keepParent "$APP" "$ZIP"
done

# Standalone raw Mach-O cannot have a stapler ticket attached. Submit its
# signed ZIP for notarization; clean-download acceptance remains P6-T3 work.
ADDON="$STAGING/output/OCR-AddOn/PicPasteOCR"
mkdir -p "$(dirname "$ADDON")"
ditto "$STAGING/build/Full/PicPaste.app/Contents/Helpers/PicPasteOCR" "$ADDON"
chmod 755 "$ADDON"
python3 "$ROOT/scripts/universal_macho.py" --universal "$ADDON"
sign_executable "$ADDON" "standalone OCR Add-on"
[[ "$("$ADDON" --addon-info)" == "PICPASTE-OCR-ADDON/1 SHOC/1" ]] || {
  echo "Standalone OCR Add-on handshake incompatible" >&2
  exit 5
}
ADDON_ZIP="$STAGING/output/PicPaste-OCR-AddOn-macos-universal2-$VERSION-CANDIDATE.zip"
(cd "$(dirname "$ADDON")" && ditto -c -k --sequesterRsrc --keepParent PicPasteOCR "$ADDON_ZIP")
notarize_zip "$ADDON_ZIP" "$STAGING/output/notary-addon.json"

(
  cd "$STAGING/output"
  shasum -a 256 ./*-CANDIDATE.zip > SHA256SUMS.txt
  shasum -a 256 -c SHA256SUMS.txt
)
python3 - "$STAGING/output/release-manifest.json" "$SHA" "$PICPASTE_APPLE_TEAM_ID" <<'PY_MANIFEST'
import json
import sys
from pathlib import Path
Path(sys.argv[1]).write_text(json.dumps({
    "schema": "picpaste.signed-release-candidate/1",
    "source_sha": sys.argv[2],
    "apple_team_id": sys.argv[3],
    "clean_download_gatekeeper_required": True,
    "manual_distribution_approval_required": True,
    "github_release_created": False,
}, indent=2) + "\n", encoding="utf-8")
PY_MANIFEST
mv "$STAGING/output" "$OUT"
echo "Signed + notarized PRIVATE candidates staged: $OUT"
echo "STOP: test real downloaded macOS 14/26 Apple Silicon and Intel installations."
echo "No public GitHub Release or immutable v1.0.0 tag was created."
