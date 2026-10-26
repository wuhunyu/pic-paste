#!/bin/bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "PicPaste macOS packaging requires macOS" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${PICPASTE_DIST_DIR:-$ROOT/dist}"
BINARY="${PICPASTE_BINARY:-$ROOT/target/release/shot-macos}"
VARIANT="${PICPASTE_VARIANT:-Lite}"
case "$VARIANT" in Lite|Full) ;; *) echo "Unknown package variant: $VARIANT" >&2; exit 3;; esac
APP="$OUT/$VARIANT/PicPaste.app"
PLIST="$ROOT/packaging/macos/Info.plist"

if [[ ! -f "$BINARY" ]]; then
  echo "Missing binary: $BINARY. Run cargo build --release -p shot-macos first." >&2
  exit 2
fi

mkdir -p "$OUT/$VARIANT"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$BINARY" "$APP/Contents/MacOS/PicPaste"
chmod 755 "$APP/Contents/MacOS/PicPaste"
cp "$PLIST" "$APP/Contents/Info.plist"

if [[ "$VARIANT" == "Full" ]]; then
  OCR_BINARY="${PICPASTE_OCR_BINARY:-$ROOT/target/release/PicPasteOCR}"
  if [[ ! -f "$OCR_BINARY" ]]; then
    echo "Full variant requires Vision OCR helper at $OCR_BINARY" >&2
    exit 4
  fi
  mkdir -p "$APP/Contents/Helpers"
  cp "$OCR_BINARY" "$APP/Contents/Helpers/PicPasteOCR"
  chmod 755 "$APP/Contents/Helpers/PicPasteOCR"
  codesign --force --sign - --timestamp=none "$APP/Contents/Helpers/PicPasteOCR"
fi

# Non-production ad-hoc identity prevents unsigned executable surprises
# on the build runner. Developer ID signing and notarization MUST be done
# separately with approved CI secrets before public distribution.
codesign --force --sign - --timestamp=none "$APP/Contents/MacOS/PicPaste"
codesign --force --sign - --timestamp=none "$APP"
codesign --verify --deep --strict "$APP"

ARCH="$(uname -m)"
ZIP="$OUT/PicPaste-macos-$ARCH-$VARIANT-UNSIGNED-DEV.zip"
rm -f "$ZIP"
ditto -c -k --sequesterRsrc --keepParent "$APP" "$ZIP"
shasum -a 256 "$ZIP" > "$ZIP.sha256"

echo "Development-only bundle: $APP"
echo "Unsigned for Developer ID / not notarized: $ZIP"
echo "This package is NOT a production release."
