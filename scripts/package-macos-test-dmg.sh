#!/usr/bin/env bash
# Ad-hoc-signed, NOT NOTARIZED, NOT Developer ID: manual macOS UI approval only.
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "PicPaste DMG packaging requires a macOS host" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${PICPASTE_DIST_DIR:-$ROOT/dist}"
VARIANT="${PICPASTE_VARIANT:-Full}"
ARCH="${PICPASTE_PACKAGE_ARCH:-$(uname -m)}"
APP="$OUT/$VARIANT/PicPaste.app"

if [[ "${PICPASTE_NATIVE_CANARY:-0}" != "0" ]]; then
  echo "Stable test DMGs must not contain experimental NSPanel Canary" >&2
  exit 2
fi

# Structural identity check BEFORE creating a disk image. Filename returned
# by the validator explicitly warns that the result is an unnotarized test.
DMG_NAME="$(python3 "$ROOT/scripts/macos_test_dmg.py" --app "$APP" --variant "$VARIANT" --arch "$ARCH")"
if [[ "$ARCH" == "universal2" ]]; then
  python3 "$ROOT/scripts/universal_macho.py" --universal "$APP/Contents/MacOS/PicPaste"
  if [[ "$VARIANT" == "Full" ]]; then
    python3 "$ROOT/scripts/universal_macho.py" --universal "$APP/Contents/Helpers/PicPasteOCR"
  fi
else
  python3 "$ROOT/scripts/universal_macho.py" --thin "$ARCH" "$APP/Contents/MacOS/PicPaste"
  if [[ "$VARIANT" == "Full" ]]; then
    python3 "$ROOT/scripts/universal_macho.py" --thin "$ARCH" "$APP/Contents/Helpers/PicPasteOCR"
  fi
fi
codesign --verify --deep --strict "$APP"
DETAILS="$(codesign --display --verbose=4 "$APP" 2>&1)"
if [[ "$DETAILS" != *"Signature=adhoc"* ]]; then
  echo "Refusing to label a Developer ID or unknown signed app as ad-hoc TEST" >&2
  exit 3
fi

mkdir -p "$OUT"
STAGING="$(mktemp -d "$OUT/.test-dmg.XXXXXXXX")"
MOUNTED=0
MOUNT="$STAGING/mounted"
cleanup() {
  if [[ "$MOUNTED" == "1" ]]; then
    hdiutil detach "$MOUNT" -quiet || true
  fi
  rm -rf "$STAGING"
}
trap cleanup EXIT

PAYLOAD="$STAGING/payload"
mkdir -p "$PAYLOAD" "$MOUNT"
ditto "$APP" "$PAYLOAD/PicPaste.app"
ln -s /Applications "$PAYLOAD/Applications"
cp "$ROOT/docs/unsigned-macos-test-install.md" "$PAYLOAD/READ-ME-TEST-APP.md"

TEMP_DMG="$STAGING/$DMG_NAME"
hdiutil create -quiet -format UDZO -fs HFS+ \
  -volname "PicPaste $VARIANT TEST" -srcfolder "$PAYLOAD" "$TEMP_DMG"
hdiutil verify -quiet "$TEMP_DMG"
hdiutil attach -quiet -readonly -nobrowse -mountpoint "$MOUNT" "$TEMP_DMG"
MOUNTED=1
test -d "$MOUNT/PicPaste.app"
test -L "$MOUNT/Applications"
test "$(readlink "$MOUNT/Applications")" = "/Applications"
python3 "$ROOT/scripts/macos_test_dmg.py" \
  --app "$MOUNT/PicPaste.app" --variant "$VARIANT" --arch "$ARCH" >/dev/null
codesign --verify --deep --strict "$MOUNT/PicPaste.app"
hdiutil detach "$MOUNT" -quiet
MOUNTED=0

# Keep an earlier good DMG until the *new* image is verified and mountable.
FINAL_DMG="$OUT/$DMG_NAME"
mv -f "$TEMP_DMG" "$FINAL_DMG"
(cd "$OUT" && shasum -a 256 "$DMG_NAME" > "$DMG_NAME.sha256" && \
  shasum -a 256 -c "$DMG_NAME.sha256")
echo "Test-only DMG (ad-hoc signed, NOT notarized): $FINAL_DMG"
echo "Use Finder and System Settings to approve opening. Do not disable Gatekeeper."
