#!/bin/bash
# Standalone signed Vision OCR/QR Helper archive (not an installer pkg).
set -euo pipefail
[[ "$(uname -s)" == "Darwin" ]] || { echo "macOS required" >&2; exit 1; }

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${PICPASTE_DIST_DIR:-$ROOT/dist}"
OCR_BINARY="${PICPASTE_OCR_BINARY:-$ROOT/target/release/PicPasteOCR}"
SIGN_IDENTITY="${PICPASTE_SIGN_IDENTITY:--}"
[[ -f "$OCR_BINARY" ]] || { echo "Missing helper: $OCR_BINARY" >&2; exit 2; }

DIRECTORY="$OUT/OCR-AddOn"
mkdir -p "$DIRECTORY"
HELPER="$DIRECTORY/PicPasteOCR"
cp "$OCR_BINARY" "$HELPER"
chmod 755 "$HELPER"

if [[ "$SIGN_IDENTITY" == "-" ]]; then
  # No Team ID; the runtime and installer MUST reject this dev artifact.
  /usr/bin/codesign --force --sign - --timestamp=none "$HELPER"
  CHANNEL="UNSIGNED-DEV"
else
  if [[ "$SIGN_IDENTITY" != "Developer ID Application:"* ]]; then
    echo "PICPASTE_SIGN_IDENTITY must be a Developer ID Application identity" >&2
    exit 4
  fi
  /usr/bin/codesign --force --sign "$SIGN_IDENTITY" \
    --options runtime --timestamp "$HELPER"
  CHANNEL="DEVELOPER-ID-SIGNED"
fi
/usr/bin/codesign --verify --strict "$HELPER"
[[ "$("$HELPER" --addon-info)" == "PICPASTE-OCR-ADDON/1 SHOC/1" ]] ||
  { echo "Incompatible helper protocol" >&2; exit 3; }

ARCH="$(uname -m)"
ZIP="$OUT/PicPaste-OCR-AddOn-macos-$ARCH-$CHANNEL.zip"
rm -f "$ZIP"
(cd "$DIRECTORY" && /usr/bin/ditto -c -k --sequesterRsrc --keepParent PicPasteOCR "$ZIP")
/usr/bin/shasum -a 256 "$ZIP" > "$ZIP.sha256"
echo "OCR Add-on archive: $ZIP"
if [[ "$CHANNEL" == "UNSIGNED-DEV" ]]; then
  echo "WARNING: Ad-hoc development archive cannot be installed as a trusted Add-on."
else
  echo "Code-signed; notarization and real-device acceptance are still separate."
fi
