#!/usr/bin/env python3
"""Fail-closed checks for ad-hoc macOS DMG test bundles. Not Developer ID approval.

Check metadata and payload presence. The macOS packager additionally verifies
real Mach-O slices, ad-hoc code signature and the finished .dmg image.
"""
from __future__ import annotations

import argparse
import os
import plistlib
import re
import sys
from pathlib import Path

VALID_VARIANTS = ('Lite', 'Full')
VALID_ARCHES = ('arm64', 'x86_64', 'universal2')


def dmg_name(arch: str, variant: str) -> str:
    if variant not in VALID_VARIANTS:
        raise ValueError('unknown test installer variant (Lite/Full only)')
    if arch not in VALID_ARCHES:
        raise ValueError('invalid test installer architecture')
    return f'PicPaste-macos-{arch}-{variant}-UNNOTARIZED-TEST.dmg'


def _regular(path: Path, label: str, *, executable: bool = False) -> None:
    if path.is_symlink():
        raise ValueError(f'{label} must not be a symlink: {path}')
    if not path.is_file() or path.stat().st_size == 0:
        raise ValueError(f'{label} file is missing or empty: {path}')
    if executable and not os.access(path, os.X_OK):
        raise ValueError(f'{label} must be executable: {path}')


def validate_app(app: Path, variant: str, arch: str) -> str:
    name = dmg_name(arch, variant)
    app = Path(app)
    if app.name != 'PicPaste.app' or app.is_symlink() or not app.is_dir():
        raise ValueError('expected an actual PicPaste.app bundle directory')
    contents = app / 'Contents'
    if contents.is_symlink():
        raise ValueError('app Contents cannot be a symlink')
    plist = contents / 'Info.plist'
    _regular(plist, 'Info.plist')
    with plist.open('rb') as fh:
        info = plistlib.load(fh)
    expected = {
        'CFBundleIdentifier': 'com.wuhunyu.picpaste',
        'CFBundleExecutable': 'PicPaste',
        'CFBundleName': 'PicPaste',
        'CFBundleShortVersionString': '1.0.0',
        'CFBundlePackageType': 'APPL',
        'CFBundleIconFile': 'PicPaste.icns',
        'LSMinimumSystemVersion': '14.0',
    }
    if any(info.get(key) != value for key, value in expected.items()):
        raise ValueError('PicPaste bundle identity, version, OS floor or icon mismatch')
    if not re.fullmatch(r'[1-9][0-9]*', str(info.get('CFBundleVersion', ''))):
        raise ValueError('invalid app build version')
    main = contents / 'MacOS/PicPaste'
    icon = contents / 'Resources/PicPaste.icns'
    helper = contents / 'Helpers/PicPasteOCR'
    _regular(main, 'main executable', executable=True)
    _regular(icon, 'icon')
    with icon.open('rb') as stream:
        if stream.read(4) != b'icns':
            raise ValueError('invalid ICNS icon header')
    if (contents / 'MacOS/ns-panel-probe').exists():
        raise ValueError('experimental native Canary companion must not be included in test installer')
    if variant == 'Full':
        try:
            _regular(helper, 'Full OCR helper', executable=True)
        except ValueError as exc:
            raise ValueError(f'Full OCR helper is not safe: {exc}') from exc
    elif helper.exists() or helper.is_symlink():
        raise ValueError('Lite must not include an OCR helper')
    return name


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app', type=Path, required=True)
    parser.add_argument('--variant', required=True)
    parser.add_argument('--arch', required=True)
    args = parser.parse_args(argv)
    try:
        print(validate_app(args.app, args.variant, args.arch))
    except (ValueError, OSError, plistlib.InvalidFileException, TypeError) as exc:
        print(f'Cannot create PicPaste test DMG: {exc}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
