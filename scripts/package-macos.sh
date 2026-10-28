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
NATIVE_CANARY="${PICPASTE_NATIVE_CANARY:-0}"
case "$NATIVE_CANARY" in 0|1) ;; *) echo "PICPASTE_NATIVE_CANARY must be 0 or 1" >&2; exit 5;; esac
NATIVE_PROBE_BINARY="${PICPASTE_NATIVE_PROBE_BINARY:-$ROOT/target/release/ns-panel-probe}"
APP="$OUT/$VARIANT/PicPaste.app"
PLIST="$ROOT/packaging/macos/Info.plist"

if [[ ! -f "$BINARY" ]]; then
  echo "Missing binary: $BINARY. Run cargo build --release -p shot-macos first." >&2
  exit 2
fi
if [[ "$NATIVE_CANARY" == "1" ]] && { [[ ! -f "$NATIVE_PROBE_BINARY" ]] || [[ -L "$NATIVE_PROBE_BINARY" ]] || [[ ! -x "$NATIVE_PROBE_BINARY" ]]; }; then
  echo "Missing or unsafe native canary companion: $NATIVE_PROBE_BINARY" >&2
  echo "Build with cargo build --release -p shot-macos --features native-panel-probe --bins --locked" >&2
  exit 5
fi

mkdir -p "$OUT/$VARIANT"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$BINARY" "$APP/Contents/MacOS/PicPaste"
chmod 755 "$APP/Contents/MacOS/PicPaste"
cp "$PLIST" "$APP/Contents/Info.plist"

if [[ "$NATIVE_CANARY" == "1" ]]; then
  cp "$NATIVE_PROBE_BINARY" "$APP/Contents/MacOS/ns-panel-probe"
  chmod 755 "$APP/Contents/MacOS/ns-panel-probe"
  codesign --force --sign - --timestamp=none "$APP/Contents/MacOS/ns-panel-probe"
fi

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
LABEL=""
if [[ "$NATIVE_CANARY" == "1" ]]; then
  LABEL="-NATIVE-CANARY"
fi
ZIP="$OUT/PicPaste-macos-$ARCH-$VARIANT$LABEL-UNSIGNED-DEV.zip"
rm -f "$ZIP"
ditto -c -k --sequesterRsrc --keepParent "$APP" "$ZIP"
shasum -a 256 "$ZIP" > "$ZIP.sha256"

echo "Development-only bundle: $APP"
echo "Unsigned for Developer ID / not notarized: $ZIP"
if [[ "$NATIVE_CANARY" == "1" ]]; then
  echo "Contains opt-in experimental NSPanel. Stable eframe capture is the default."
fi
echo "This package is NOT a production release."
