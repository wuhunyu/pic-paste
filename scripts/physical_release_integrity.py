#!/usr/bin/env python3
"""Read-only source ZIP to extracted signed-binary binding and data root IDs."""
from __future__ import annotations

import hashlib
import zipfile
from pathlib import Path


PACKAGE_MEMBERS = {
    "Lite": [
        "PicPaste.app/Contents/Info.plist",
        "PicPaste.app/Contents/MacOS/PicPaste",
        "PicPaste.app/Contents/Resources/PicPaste.icns",
    ],
    "Full": [
        "PicPaste.app/Contents/Info.plist",
        "PicPaste.app/Contents/MacOS/PicPaste",
        "PicPaste.app/Contents/Resources/PicPaste.icns",
        "PicPaste.app/Contents/Helpers/PicPasteOCR",
    ],
    "AddOn": ["PicPasteOCR"],
}


def directory_identity(root: Path) -> str:
    root = Path(root)
    if root.is_symlink() or not root.is_dir():
        raise ValueError("preservation directory must be an existing real directory")
    # Bind both snapshots to precisely the same local folder without putting
    # home directory/usernames/paths in a shareable report JSON.
    return hashlib.sha256(str(root.resolve(strict=True)).encode("utf-8")).hexdigest()


def verify_extraction(archive_path: Path, extracted: Path, variant: str) -> int:
    if variant not in PACKAGE_MEMBERS:
        raise ValueError("unknown candidate variant")
    archive_path = Path(archive_path)
    extracted = Path(extracted)
    if archive_path.is_symlink() or not archive_path.is_file():
        raise ValueError("downloaded candidate ZIP is missing or symlinked")
    if extracted.is_symlink():
        raise ValueError("symlinked extracted app/addon is not accepted")
    expected = PACKAGE_MEMBERS[variant]
    with zipfile.ZipFile(archive_path) as archive:
        names = archive.namelist()
        if len(names) != len(set(names)):
            raise ValueError("duplicate ZIP member in downloaded candidate")
        for member in expected:
            if member not in names:
                raise ValueError(f"missing signed ZIP member: {member}")
            target = extracted if variant == "AddOn" else extracted.parent / member
            if target.is_symlink() or not target.is_file():
                raise ValueError(f"missing or symlinked extracted signed member: {member}")
            left = hashlib.sha256()
            with archive.open(member) as stream:
                for block in iter(lambda: stream.read(1_048_576), b""):
                    left.update(block)
            right = hashlib.sha256()
            with target.open("rb") as stream:
                for block in iter(lambda: stream.read(1_048_576), b""):
                    right.update(block)
            if left.digest() != right.digest():
                raise ValueError(f"extracted member different from downloaded ZIP: {member}")
    return len(expected)
