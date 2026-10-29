#!/usr/bin/env python3
"""Check release-candidate metadata before packaging; never signs or publishes."""
from __future__ import annotations

import argparse
import plistlib
import re
import sys
import tomllib
from pathlib import Path

EXPECTED_VERSION = "1.0.0"
BUNDLE_ID = "com.wuhunyu.picpaste"


def inspect_metadata(root: Path, expected_version: str = EXPECTED_VERSION) -> dict[str, str]:
    root = Path(root)
    with (root / "Cargo.toml").open("rb") as stream:
        cargo = tomllib.load(stream)
    with (root / "Cargo.lock").open("rb") as stream:
        lock = tomllib.load(stream)
    with (root / "packaging/macos/Info.plist").open("rb") as stream:
        info = plistlib.load(stream)
    version = cargo.get("workspace", {}).get("package", {}).get("version")
    if not isinstance(version, str) or not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ValueError("workspace Cargo version must be major.minor.patch")
    if version != expected_version or info.get("CFBundleShortVersionString") != version:
        raise ValueError("app version must match Cargo, Info.plist and release candidate")
    packages = [p for p in lock.get("package", []) if p.get("name", "").startswith("shot-")]
    if not packages:
        raise ValueError("Cargo.lock is missing PicPaste workspace packages")
    for package in packages:
        if package.get("version") != version:
            raise ValueError(f"Cargo.lock version mismatch: {package['name']}")
    if info.get("CFBundleIdentifier") != BUNDLE_ID:
        raise ValueError(f"app bundle identifier must be {BUNDLE_ID}, not an example placeholder")
    if info.get("CFBundleName") != "PicPaste" or info.get("CFBundleExecutable") != "PicPaste":
        raise ValueError("app bundle name/executable must be PicPaste")
    if info.get("CFBundlePackageType") != "APPL":
        raise ValueError("app package type must be APPL")
    if info.get("CFBundleIconFile") != "PicPaste.icns":
        raise ValueError("app icon name must be PicPaste.icns")
    if info.get("LSMinimumSystemVersion") != "14.0":
        raise ValueError("minimum macOS version must remain 14.0")
    build = info.get("CFBundleVersion")
    if not isinstance(build, str) or not re.fullmatch(r"[1-9]\d*", build):
        raise ValueError("app build number must be a positive decimal string")
    usage = info.get("NSScreenCaptureUsageDescription")
    if not isinstance(usage, str) or not usage.strip():
        raise ValueError("Screen Recording usage description must be nonempty")
    return {
        "version": version,
        "bundle_id": BUNDLE_ID,
        "build": build,
        "min_macos": "14.0",
        "icon": "PicPaste.icns",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--expected-version", default=EXPECTED_VERSION)
    args = parser.parse_args(argv)
    try:
        result = inspect_metadata(args.root, args.expected_version)
    except (OSError, ValueError, KeyError, TypeError, plistlib.InvalidFileError) as error:
        print(f"PicPaste release metadata rejected: {error}", file=sys.stderr)
        return 1
    print(
        f"Candidate metadata verified: {result['bundle_id']} {result['version']} "
        f"build={result['build']} macOS>={result['min_macos']}. "
        "This is NOT evidence of Developer ID signing or notarization."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
