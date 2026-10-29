#!/usr/bin/env python3
"""Audit *private* PicPaste signed-candidate contents and provenance links.

Structural JSON, ZIP and checksum checks are portable. --verify-macos also
checks signatures and stapled ticket on a trusted Mac, but neither mode proves
physical downloaded-package Gatekeeper acceptance or authorizes publishing.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import plistlib
import re
import subprocess
import sys
import zipfile
from pathlib import Path
from typing import Any

if __package__:
    from .release_signing_policy import (
        TEAM, SHA, check_notary_result, check_release_approval, read_json,
    )
else:
    from release_signing_policy import (
        TEAM, SHA, check_notary_result, check_release_approval, read_json,
    )

BUNDLE_ID = "com.wuhunyu.picpaste"
VERSION = "1.0.0"
CHECKSUM_LINE = re.compile(r"([a-f0-9]{64})  \./([A-Za-z0-9_.-]+)\Z")


def _safe_file(path: Path) -> Path:
    if path.is_symlink():
        raise ValueError(f"symlinked candidate file is not allowed: {path}")
    if not path.is_file():
        raise ValueError(f"required candidate file is missing: {path}")
    return path


def _hash_file(path: Path) -> str:
    digest = hashlib.sha256()
    with _safe_file(path).open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _hash_member(archive: zipfile.ZipFile, name: str) -> str:
    digest = hashlib.sha256()
    info = archive.getinfo(name)
    if info.file_size <= 0:
        raise ValueError(f"missing or empty required archive member: {name}")
    with archive.open(name) as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _verify_archive(archive_path: Path, variant: str, root: Path) -> None:
    with zipfile.ZipFile(_safe_file(archive_path)) as archive:
        members = archive.infolist()
        names: set[str] = set()
        for info in members:
            name = info.filename
            parts = name.split("/")
            if (not name or name.startswith("/") or "\\" in name or
                    any(part in (".", "..") for part in parts) or
                    (info.create_system == 3 and (info.external_attr >> 16) & 0o170000 == 0o120000)):
                raise ValueError(f"unsafe ZIP path or symlink: {name}")
            if name in names:
                raise ValueError(f"duplicate ZIP path: {name}")
            names.add(name)
            if name.endswith("ns-panel-probe"):
                raise ValueError("experimental NSPanel Canary cannot be included in a release candidate")
        if archive.testzip() is not None:
            raise ValueError("ZIP CRC integrity verification failed")
        if variant == "addon":
            required = {"PicPasteOCR": root / "OCR-AddOn/PicPasteOCR"}
            if "PicPasteOCR" not in names:
                raise ValueError("missing standalone OCR Add-on executable")
        else:
            base = "PicPaste.app/Contents/"
            actual = root / variant / "PicPaste.app/Contents"
            required = {
                base + "Info.plist": actual / "Info.plist",
                base + "MacOS/PicPaste": actual / "MacOS/PicPaste",
                base + "Resources/PicPaste.icns": actual / "Resources/PicPaste.icns",
            }
            helper = base + "Helpers/PicPasteOCR"
            if variant == "Full":
                required[helper] = actual / "Helpers/PicPasteOCR"
            elif helper in names:
                raise ValueError("Lite release candidate must not include embedded OCR")
            if base + "Info.plist" not in names:
                raise ValueError(f"{variant} archive missing app Info.plist")
            try:
                info = plistlib.loads(archive.read(base + "Info.plist"))
            except (plistlib.InvalidFileException, ValueError, TypeError) as exc:
                raise ValueError(f"{variant} archive Info.plist is invalid") from exc
            if (info.get("CFBundleIdentifier") != BUNDLE_ID or
                    info.get("CFBundleShortVersionString") != VERSION or
                    info.get("CFBundleExecutable") != "PicPaste" or
                    info.get("CFBundleIconFile") != "PicPaste.icns" or
                    info.get("LSMinimumSystemVersion") != "14.0"):
                raise ValueError(f"{variant} bundle identity, version, icon or OS floor mismatch in plist")
        for name, on_disk in required.items():
            if name not in names:
                raise ValueError(f"missing archive member: {name}")
            if _hash_member(archive, name) != _hash_file(on_disk):
                raise ValueError(f"ZIP member differs from staged signed file: {name}")


def _verify_macos(root: Path, expected_team: str) -> None:
    if sys.platform != "darwin":
        raise ValueError("native signature verification requires a real macOS host")
    paths: list[tuple[Path, bool]] = []
    for variant in ("Lite", "Full"):
        app = root / variant / "PicPaste.app"
        paths.extend([(app, True), (app / "Contents/MacOS/PicPaste", False)])
        if variant == "Full":
            paths.append((app / "Contents/Helpers/PicPasteOCR", False))
        subprocess.run(["xcrun", "stapler", "validate", str(app)], check=True,
                       capture_output=True, text=True)
        subprocess.run(["spctl", "--assess", "--type", "execute", "--verbose",
                        str(app)], check=True, capture_output=True, text=True)
    paths.append((root / "OCR-AddOn/PicPasteOCR", False))
    for path, deep in paths:
        _safe_file(path) if not path.name.endswith(".app") else None
        cmd = ["codesign", "--verify", "--strict"]
        if deep:
            cmd.append("--deep")
        subprocess.run(cmd + [str(path)], check=True, capture_output=True, text=True)
        result = subprocess.run(["codesign", "--display", "--verbose=4", str(path)],
                                capture_output=True, text=True, check=True)
        details = result.stdout + result.stderr
        if ("Authority=Developer ID Application:" not in details or
                f"TeamIdentifier={expected_team}" not in details or
                "(runtime)" not in details):
            raise ValueError(f"missing same-Team Developer ID/hardened runtime signature: {path}")


def _check_sums(root: Path, expected_names: set[str]) -> None:
    checksum_path = _safe_file(root / "SHA256SUMS.txt")
    if checksum_path.stat().st_size > 4096:
        raise ValueError("candidate checksum file is unexpectedly large")
    found: dict[str, str] = {}
    for raw in checksum_path.read_text(encoding="utf-8").splitlines():
        match = CHECKSUM_LINE.fullmatch(raw)
        if match is None:
            raise ValueError("malformed candidate checksum line")
        sha, name = match.groups()
        if name in found:
            raise ValueError(f"duplicate candidate checksum: {name}")
        found[name] = sha
    if set(found) != expected_names:
        raise ValueError("candidate checksums must list exactly three expected archives")
    for name, expected_hash in found.items():
        if _hash_file(root / name) != expected_hash:
            raise ValueError(f"candidate checksum mismatch for {name}")


def audit_candidate(root: Path, source_sha: str, team: str, gate_path: Path,
                    approval_path: Path, *, verify_macos: bool = False) -> dict[str, Any]:
    root = Path(root)
    if root.is_symlink() or not root.is_dir():
        raise ValueError("candidate directory must be an actual local directory, not a symlink")
    if not SHA.fullmatch(source_sha):
        raise ValueError("source SHA must be a full 40-digit lowercase hex commit ID")
    if not TEAM.fullmatch(team):
        raise ValueError("Apple Team ID must be ten uppercase letters/digits")

    # Re-run all the previous human declaration/schema checks against the QA
    # JSONs. Neither file can prove its own physical-origin authenticity.
    check_release_approval(gate_path, approval_path, source_sha)
    manifest = read_json(root / "release-manifest.json")
    if manifest.get("schema") != "picpaste.signed-release-candidate/1":
        raise ValueError("unknown signed candidate manifest schema")
    if manifest.get("source_sha") != source_sha:
        raise ValueError("candidate source SHA does not match frozen checkout")
    if manifest.get("apple_team_id") != team:
        raise ValueError("candidate Apple Team ID mismatch")
    if manifest.get("version") != VERSION or manifest.get("bundle_id") != BUNDLE_ID:
        raise ValueError("candidate release version or bundle identity mismatch")
    if manifest.get("gate_sha256") != _hash_file(gate_path):
        raise ValueError("candidate gate hash differs from approved QA evidence")
    if manifest.get("approval_sha256") != _hash_file(approval_path):
        raise ValueError("candidate approval hash differs from human declaration")
    if manifest.get("github_release_created") is not False:
        raise ValueError("candidate manifest must not claim that a GitHub release exists")
    if manifest.get("clean_download_gatekeeper_required") is not True:
        raise ValueError("candidate must still require real clean-download Gatekeeper checks")
    if manifest.get("manual_distribution_approval_required") is not True:
        raise ValueError("candidate must require final manual distribution approval")

    archive_names = {
        "Lite": f"PicPaste-macos-universal2-Lite-{VERSION}-CANDIDATE.zip",
        "Full": f"PicPaste-macos-universal2-Full-{VERSION}-CANDIDATE.zip",
        "addon": f"PicPaste-OCR-AddOn-macos-universal2-{VERSION}-CANDIDATE.zip",
    }
    _check_sums(root, set(archive_names.values()))
    notary_ids: set[str] = set()
    for variant in ("Lite", "Full", "addon"):
        result = check_notary_result(root / f"notary-{variant}.json")
        if result["id"] in notary_ids:
            raise ValueError("duplicate Apple notarization submission ID")
        notary_ids.add(result["id"])
        _verify_archive(root / archive_names[variant], variant, root)
    if verify_macos:
        _verify_macos(root, team)
    return {
        "schema": "picpaste.candidate-audit/1",
        "source_sha": source_sha,
        "apple_team_id": team,
        "integrity_passed": True,
        "verified_archives": 3,
        "macos_signature_verified": verify_macos,
        "notary_records_structurally_accepted": 3,
        "physical_download_verification_completed": False,
        "ready_for_public_release": False,
        "note": "Authenticity of Apple submissions and QA provenance, first-download Gatekeeper, upgrade and rollback still require independent human and physical-device approval.",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--team-id", required=True)
    parser.add_argument("--gate", type=Path, required=True)
    parser.add_argument("--approval", type=Path, required=True)
    parser.add_argument("--verify-macos", action="store_true")
    args = parser.parse_args(argv)
    try:
        report = audit_candidate(args.candidate, args.source_sha, args.team_id,
                                 args.gate, args.approval, verify_macos=args.verify_macos)
        print(json.dumps(report, indent=2, ensure_ascii=False))
        return 0
    except (OSError, ValueError, KeyError, TypeError, zipfile.BadZipFile,
            subprocess.CalledProcessError) as exc:
        print(f"Signed candidate audit rejected: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
