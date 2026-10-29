#!/usr/bin/env bash
# Create actual arm64 + x86_64 Universal 2 Lite/Full development archives.
# NO Developer ID signing, notarization, Gatekeeper bypass or public release.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Universal 2 builds require a macOS host with Xcode" >&2
  exit 1
fi
if [[ "${PICPASTE_NATIVE_CANARY:-0}" != "0" ]]; then
  echo "Universal 2 development packaging uses the stable backend; native canary is not included" >&2
  exit 2
fi
python3 "$ROOT/scripts/release_metadata.py" --root "$ROOT"
export MACOSX_DEPLOYMENT_TARGET=14.0

OUT="${PICPASTE_UNIVERSAL_DIST_DIR:-$ROOT/dist-universal2}"
mkdir -p "$OUT"
STAGING="$(mktemp -d "$OUT/.staging.XXXXXXXX")"
trap 'rm -rf "$STAGING"' EXIT

ARM_APP="${PICPASTE_UNIVERSAL_ARM64_BINARY:-}"
ARM_OCR="${PICPASTE_UNIVERSAL_ARM64_OCR_BINARY:-}"
if [[ -n "$ARM_APP" && -z "$ARM_OCR" ]] || [[ -z "$ARM_APP" && -n "$ARM_OCR" ]]; then
  echo "When reusing arm64 build outputs, supply BOTH ARM64 binary and OCR helper" >&2
  exit 3
fi

# An optional thin ARM64 pair allows CI to reuse its already-tested Release
# binaries. The caller must make sure the binaries were built from this HEAD.
if [[ -z "$ARM_APP" ]]; then
  rustup target add aarch64-apple-darwin
  cargo build --release -p shot-macos --target aarch64-apple-darwin --locked --manifest-path "$ROOT/Cargo.toml"
  ARM_APP="$ROOT/target/aarch64-apple-darwin/release/shot-macos"
  xcrun swiftc -O -target arm64-apple-macos14.0 \
    -framework Vision -framework CoreGraphics -framework Foundation -framework CryptoKit \
    "$ROOT/packaging/macos/PicPasteOCR.swift" -o "$STAGING/PicPasteOCR-arm64"
  ARM_OCR="$STAGING/PicPasteOCR-arm64"
fi

rustup target add x86_64-apple-darwin
cargo build --release -p shot-macos --target x86_64-apple-darwin --locked --manifest-path "$ROOT/Cargo.toml"
INTEL_APP="$ROOT/target/x86_64-apple-darwin/release/shot-macos"
xcrun swiftc -O -target x86_64-apple-macos14.0 \
  -framework Vision -framework CoreGraphics -framework Foundation -framework CryptoKit \
  "$ROOT/packaging/macos/PicPasteOCR.swift" -o "$STAGING/PicPasteOCR-x86_64"
INTEL_OCR="$STAGING/PicPasteOCR-x86_64"

for entry in "arm64:$ARM_APP" "arm64:$ARM_OCR" "x86_64:$INTEL_APP" "x86_64:$INTEL_OCR"; do
  arch="${entry%%:*}"
  file="${entry#*:}"
  if [[ ! -f "$file" || -L "$file" ]]; then
    echo "Required thin Mach-O is missing or a symlink: $file" >&2
    exit 4
  fi
  python3 "$ROOT/scripts/universal_macho.py" --thin "$arch" "$file"
done

# All four inputs are verified before lipo or before any prior package is
# overwritten. Sign the merged executables later via package-macos.sh.
lipo -create "$ARM_APP" "$INTEL_APP" -output "$STAGING/shot-macos"
lipo -create "$ARM_OCR" "$INTEL_OCR" -output "$STAGING/PicPasteOCR"
for name in shot-macos PicPasteOCR; do
  python3 "$ROOT/scripts/universal_macho.py" --universal "$STAGING/$name"
  lipo -verify_arch arm64 x86_64 "$STAGING/$name"
done

if [[ "$("$STAGING/PicPasteOCR" --addon-info)" != "PICPASTE-OCR-ADDON/1 SHOC/1" ]]; then
  echo "Merged OCR helper protocol is incompatible" >&2
  exit 5
fi

export PICPASTE_DIST_DIR="$OUT"
export PICPASTE_PACKAGE_ARCH="universal2"
export PICPASTE_NATIVE_CANARY="0"
export PICPASTE_BINARY="$STAGING/shot-macos"
export PICPASTE_OCR_BINARY="$STAGING/PicPasteOCR"

for variant in Lite Full; do
  PICPASTE_VARIANT="$variant" bash "$ROOT/scripts/package-macos.sh"
  app="$OUT/$variant/PicPaste.app"
  test -f "$app/Contents/Resources/PicPaste.icns"
  python3 "$ROOT/scripts/universal_macho.py" --universal "$app/Contents/MacOS/PicPaste"
  if [[ "$variant" == "Full" ]]; then
    python3 "$ROOT/scripts/universal_macho.py" --universal "$app/Contents/Helpers/PicPasteOCR"
  else
    test ! -e "$app/Contents/Helpers/PicPasteOCR"
  fi
  codesign --verify --deep --strict "$app"
  zip="$OUT/PicPaste-macos-universal2-$variant-UNSIGNED-DEV.zip"
  test -s "$zip"
  shasum -a 256 -c "$zip.sha256"
done

echo "Universal 2 DEV Lite/Full verified in: $OUT"
echo "These ad-hoc signed archives are NOT notarized or ready for public release."
