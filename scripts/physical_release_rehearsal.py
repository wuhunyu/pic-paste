#!/usr/bin/env python3
"""Physical Mac first-download and rollback rehearsal receipts (never auto-release).

Prepare must run on a trusted physical macOS test account.
Synthetic unit tests only exercise the schema; reports and device provenance
are independently reviewed by a person before any distribution decision.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import platform
import plistlib
import re
import subprocess
import sys
import tempfile
import zipfile
from datetime import datetime, timezone
from pathlib import Path

if __package__:
    from .physical_release_integrity import directory_identity, verify_extraction
else:
    from physical_release_integrity import directory_identity, verify_extraction

SCHEMA = "picpaste.physical-release-rehearsal/1"
SOURCE_RE = re.compile(r"[0-9a-f]{40}\Z")
HASH_RE = re.compile(r"[0-9a-f]{64}\Z")
TEAM_RE = re.compile(r"[A-Z0-9]{10}\Z")
LABEL_RE = re.compile(r"[a-z0-9][a-z0-9-]{2,39}\Z")
CASE_IDS = (
    "quarantined_first_launch",
    "tcc_screen_recording",
    "capture_png_alpha",
    "pin_clipboard_and_ime",
    "lite_full_switch",
    "standalone_signed_ocr",
    "data_preserving_rollback",
)
REQUIRED_ARCHIVES = {
    "Lite": "PicPaste-macos-universal2-Lite-1.0.0-CANDIDATE.zip",
    "Full": "PicPaste-macos-universal2-Full-1.0.0-CANDIDATE.zip",
    "AddOn": "PicPaste-OCR-AddOn-macos-universal2-1.0.0-CANDIDATE.zip",
}


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _duplicates(pairs):
    data = {}
    for k, v in pairs:
        if k in data:
            raise ValueError(f"duplicate JSON field: {k}")
        data[k] = v
    return data


def _bad_constant(s):
    raise ValueError(f"non-finite JSON constant: {s}")


def read_report(path: Path) -> dict:
    path = Path(path)
    if path.is_symlink() or not path.is_file() or path.stat().st_size > 2_000_000:
        raise ValueError(f"missing, symlink or oversized report file: {path}")
    with path.open(encoding="utf-8") as f:
        data = json.load(f, object_pairs_hook=_duplicates, parse_constant=_bad_constant)
    if type(data) is not dict:
        raise ValueError("report must be a JSON object")
    return data


def _hash_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for block in iter(lambda: f.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def fingerprint_tree(root: Path) -> dict:
    """Read-only PNG/config directory fingerprint; never log filenames or bytes."""
    root = Path(root)
    if root.is_symlink():
        raise ValueError("symlinked preservation fixture root is unsafe")
    if not root.is_dir():
        raise ValueError("PNG preservation fixture directory missing")
    files = sorted(root.rglob("*"))
    if len(files) > 10_000:
        raise ValueError("preservation fixture is unexpectedly large")
    h = hashlib.sha256()
    count = pngs = 0
    for p in files:
        if p.is_symlink():
            raise ValueError("symlink inside preservation fixture is unsafe")
        if p.is_dir():
            continue
        if not p.is_file():
            raise ValueError("special file in preservation fixture is unsafe")
        rel = p.relative_to(root).as_posix()
        encoded = rel.encode("utf-8")
        h.update(len(encoded).to_bytes(4, "big"))
        h.update(encoded)
        h.update(bytes.fromhex(_hash_file(p)))
        count += 1
        if p.suffix.lower() == ".png":
            pngs += 1
    if pngs == 0:
        raise ValueError("PNG preservation fixture must contain at least one PNG")
    return {"sha256": h.hexdigest(), "files": count, "png_files": pngs}


def _require_sha(sha: str, label: str, rx=HASH_RE) -> str:
    if type(sha) is not str or rx.fullmatch(sha) is None:
        raise ValueError(f"{label} must be a correctly formatted SHA256/source SHA")
    return sha


def _validate_preflight(preflight: dict, *, expected_sha: str | None = None,
                        expected_team: str | None = None) -> None:
    if type(preflight) is not dict:
        raise ValueError("preflight is missing")
    sha = _require_sha(preflight.get("source_sha"), "source SHA", SOURCE_RE)
    if expected_sha is not None and sha != expected_sha:
        raise ValueError("source SHA mismatches frozen candidate")
    team = preflight.get("team_id")
    if type(team) is not str or TEAM_RE.fullmatch(team) is None:
        raise ValueError("Apple Team ID format invalid")
    if expected_team is not None and team != expected_team:
        raise ValueError("Apple Team ID differs across hosts")
    label = preflight.get("host_label")
    if type(label) is not str or LABEL_RE.fullmatch(label) is None:
        raise ValueError("host label must be a nonidentifying lowercase slug")
    version = preflight.get("macos_version")
    if type(version) is not str or re.fullmatch(r"(14|26)\.[0-9]+(\.[0-9]+)?", version) is None:
        raise ValueError("macOS version must be a tested 14.x or 26.x device")
    if preflight.get("architecture") not in ("arm64", "x86_64"):
        raise ValueError("physical architecture must be arm64 or x86_64")
    hashes = preflight.get("archive_sha256")
    if type(hashes) is not dict or set(hashes) != set(REQUIRED_ARCHIVES):
        raise ValueError("expected three verified archive SHA256 hashes")
    for k in REQUIRED_ARCHIVES:
        _require_sha(hashes[k], f"{k} archive SHA256")
    _require_sha(preflight.get("preservation_root_sha256"), "preservation directory SHA256")
    if preflight.get("quarantine_zip") != dict.fromkeys(REQUIRED_ARCHIVES, True):
        raise ValueError("all three browser-downloaded ZIPs must retain quarantine")
    if preflight.get("quarantine_apps") != {"Lite": True, "Full": True}:
        raise ValueError("both Finder-extracted .app bundles must retain quarantine")
    for flag in ("signatures_verified", "stapler_verified", "gatekeeper_assessed"):
        if preflight.get(flag) is not True:
            raise ValueError(f"physical preflight missing trusted {flag}")


def build_report(preflight: dict, data_before: dict) -> dict:
    _validate_preflight(preflight)
    _validate_snapshot(data_before)
    return {
        "schema": SCHEMA,
        "release_ready": False,
        "physical_provenance_requires_human_review": True,
        "preflight": copy.deepcopy(preflight),
        "data_before": dict(data_before),
        "data_after": None,
        "prepared_at_utc": _now(),
        "completed_at_utc": None,
        "cases": [{"id": cid, "status": "not_run", "evidence": "", "notes": ""}
                  for cid in CASE_IDS],
    }


def _timestamp(value, label):
    if type(value) is not str or not value.strip():
        raise ValueError(f"{label}: a timezone-aware ISO8601 timestamp is required")
    try:
        t = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ValueError(f"{label}: invalid ISO8601 timestamp") from exc
    if t.tzinfo is None or t.utcoffset() is None:
        raise ValueError(f"{label}: missing timezone")
    return t


def _validate_snapshot(s: dict) -> None:
    if (type(s) is not dict or set(s) != {"sha256", "files", "png_files"} or
            not isinstance(s["sha256"], str) or not HASH_RE.fullmatch(s["sha256"]) or
            type(s["files"]) is not int or type(s["png_files"]) is not int or
            s["files"] < s["png_files"] or s["png_files"] < 1):
        raise ValueError("invalid PNG preservation snapshot")


def _safe_evidence(root: Path, raw: str) -> Path:
    if type(raw) is not str or not raw or "\\" in raw:
        raise ValueError("evidence path must be a nonempty relative path")
    rel = Path(raw)
    if rel.is_absolute() or any(p in (".", "..") for p in rel.parts):
        raise ValueError("evidence path must be relative and must not traverse parents")
    root = Path(root).resolve()
    full = root / rel
    if not full.resolve().is_relative_to(root):
        raise ValueError("evidence path escapes evidence directory")
    if any(part.is_symlink() for part in [full, *full.parents] if part != root and part.is_relative_to(root)):
        raise ValueError("symlinked evidence files or directories are not allowed")
    if not full.is_file() or full.stat().st_size == 0:
        raise ValueError(f"evidence file missing or empty: {raw}")
    return full


def add_case_evidence(report: dict, case_id: str, status: str, evidence: str, notes: str) -> None:
    if case_id not in CASE_IDS:
        raise ValueError("unknown physical test case")
    if status not in ("passed", "failed"):
        raise ValueError("status must be passed or failed; never automatic success")
    if type(evidence) is not str or not evidence or Path(evidence).is_absolute() or any(
            part in (".", "..") for part in Path(evidence).parts):
        raise ValueError("evidence must be a relative recorded file path")
    if type(notes) is not str or len(notes.strip()) < 10:
        raise ValueError("operator notes must describe observed outcome")
    matches = [c for c in report.get("cases", []) if c.get("id") == case_id]
    if len(matches) != 1:
        raise ValueError("missing or duplicate case")
    matches[0].update(status=status, evidence=evidence, notes=notes)


def verify_report(report: dict, evidence_dir: Path, *, expected_sha: str | None = None,
                  expected_team: str | None = None) -> dict:
    if (report.get("schema") != SCHEMA or report.get("release_ready") is not False or
            report.get("physical_provenance_requires_human_review") is not True):
        raise ValueError("release_ready must remain false; invalid device report schema")
    p = report.get("preflight")
    _validate_preflight(p, expected_sha=expected_sha, expected_team=expected_team)
    cases = report.get("cases")
    if (type(cases) is not list or len(cases) != len(CASE_IDS) or
            [c.get("id") for c in cases if type(c) is dict] != list(CASE_IDS)):
        raise ValueError("missing or duplicate physical test cases")
    for c in cases:
        if c.get("status") != "passed":
            raise ValueError(f"physical test case not passed: {c['id']}")
        if type(c.get("notes")) is not str or len(c["notes"].strip()) < 10:
            raise ValueError(f"physical test case requires operator notes: {c['id']}")
        _safe_evidence(evidence_dir, c.get("evidence"))
    before, after = report.get("data_before"), report.get("data_after")
    _validate_snapshot(before)
    _validate_snapshot(after)
    if before != after:
        raise ValueError("PNG/config data preservation failed after rollback")
    start = _timestamp(report.get("prepared_at_utc"), "prepared_at_utc")
    finish = _timestamp(report.get("completed_at_utc"), "completed_at_utc")
    if finish < start:
        raise ValueError("post-rollback timestamp precedes physical preflight")
    return {
        "schema": "picpaste.physical-rehearsal-check/1",
        "source_sha": p["source_sha"],
        "host_label": p["host_label"],
        "os_major": int(p["macos_version"].split(".")[0]),
        "architecture": p["architecture"],
        "evidence_complete": True,
        "physical_provenance_requires_human_review": True,
        "ready_for_public_release": False,
    }


def verify_matrix(reports: list[dict], evidence_dirs: list[Path], *,
                  expected_sha: str | None = None, expected_team: str | None = None) -> dict:
    if len(reports) != 4 or len(evidence_dirs) != 4:
        raise ValueError("physical release matrix requires four Mac14/26 x arm64/Intel reports")
    observed = set()
    hosts = set()
    digest_reference = None
    sha_reference = expected_sha or reports[0]["preflight"]["source_sha"]
    team_reference = expected_team or reports[0]["preflight"]["team_id"]
    for r, directory in zip(reports, evidence_dirs):
        verified = verify_report(r, directory, expected_sha=sha_reference,
                                 expected_team=team_reference)
        key = (verified["os_major"], verified["architecture"])
        if key in observed or verified["host_label"] in hosts:
            raise ValueError("duplicate physical release matrix combination or host label")
        observed.add(key)
        hosts.add(verified["host_label"])
        hashes = r["preflight"]["archive_sha256"]
        if digest_reference is None:
            digest_reference = hashes
        elif digest_reference != hashes:
            raise ValueError("archive SHA256 differs between physical Mac reports")
    expected = {(major, arch) for major in (14, 26) for arch in ("arm64", "x86_64")}
    if observed != expected:
        raise ValueError("physical release matrix must cover Mac14/26 on arm64 and x86_64")
    return {
        "schema": "picpaste.physical-release-matrix/1",
        "source_sha": sha_reference,
        "team_id": team_reference,
        "observed_combinations": 4,
        "archive_sha256": digest_reference,
        "eligible_for_final_human_review": True,
        "physical_provenance_requires_human_review": True,
        "ready_for_public_release": False,
        "note": "This checks self-reported physical evidence and data hashes; not source authenticity, Gatekeeper provenance, or permission to publish.",
    }


def _run(*args: str) -> str:
    done = subprocess.run(args, check=True, capture_output=True, text=True, timeout=60)
    return done.stdout + done.stderr


def _signature(path: Path, team: str, *, app=False) -> None:
    verify = ["codesign", "--verify", "--strict"]
    if app:
        verify.append("--deep")
    _run(*verify, str(path))
    identity = _run("codesign", "--display", "--verbose=4", str(path))
    if ("Authority=Developer ID Application:" not in identity or
            f"TeamIdentifier={team}" not in identity or "(runtime)" not in identity):
        raise ValueError("Developer ID Application signature, Team ID or hardened runtime mismatch")


def _app(path: Path, team: str, *, full: bool) -> None:
    if path.is_symlink() or not (path / "Contents/Info.plist").is_file():
        raise ValueError("extracted .app path is not a regular app bundle")
    with (path / "Contents/Info.plist").open("rb") as stream:
        info = plistlib.load(stream)
    if (info.get("CFBundleIdentifier") != "com.wuhunyu.picpaste" or
            info.get("CFBundleShortVersionString") != "1.0.0"):
        raise ValueError("bundle identity/version mismatch")
    helper = path / "Contents/Helpers/PicPasteOCR"
    if full and not helper.is_file():
        raise ValueError("Full app missing offline OCR helper")
    if not full and helper.exists():
        raise ValueError("Lite app unexpectedly embeds offline OCR")
    _signature(path, team, app=True)
    _signature(path / "Contents/MacOS/PicPaste", team)
    _run("lipo", str(path / "Contents/MacOS/PicPaste"), "-verify_arch", "arm64", "x86_64")
    if full:
        _signature(helper, team)
        _run("lipo", str(helper), "-verify_arch", "arm64", "x86_64")
    _run("xcrun", "stapler", "validate", str(path))
    _run("spctl", "--assess", "--type", "execute", "--verbose", str(path))


def _native_prepare(args) -> dict:
    if sys.platform != "darwin" or os.environ.get("CI") or os.environ.get("GITHUB_ACTIONS"):
        raise ValueError("physical preflight cannot run on Linux or CI: use an interactive real Mac")
    os_major = _run("sw_vers", "-productVersion").strip()
    arch = platform.machine()
    if not re.fullmatch(r"(14|26)\.[0-9]+(\.[0-9]+)?", os_major) or arch not in ("arm64", "x86_64"):
        raise ValueError("real macOS 14/26 physical host and arm64/x86_64 are required")
    paths = {"Lite": args.lite_zip, "Full": args.full_zip, "AddOn": args.addon_zip}
    expected = {"Lite": args.lite_sha256, "Full": args.full_sha256, "AddOn": args.addon_sha256}
    hashes = {}
    quarantine = {}
    for k, path in paths.items():
        if path.name != REQUIRED_ARCHIVES[k] or path.is_symlink() or not path.is_file():
            raise ValueError(f"{k} downloaded archive missing or renamed")
        _require_sha(expected[k], f"expected {k} SHA256")
        hashes[k] = _hash_file(path)
        if hashes[k] != expected[k]:
            raise ValueError(f"{k} downloaded archive SHA256 disagrees with signed candidate manifest")
        quarantine[k] = bool(_run("xattr", "-p", "com.apple.quarantine", str(path)).strip())
        if not quarantine[k]:
            raise ValueError(f"{k} downloaded archive lost macOS quarantine")
    _require_sha(args.source_sha, "source SHA", SOURCE_RE)
    if not TEAM_RE.fullmatch(args.team_id):
        raise ValueError("invalid Apple Team ID")
    _app(args.lite_app, args.team_id, full=False)
    _app(args.full_app, args.team_id, full=True)
    _signature(args.addon_exe, args.team_id)
    _run("lipo", str(args.addon_exe), "-verify_arch", "arm64", "x86_64")
    verify_extraction(args.lite_zip, args.lite_app, "Lite")
    verify_extraction(args.full_zip, args.full_app, "Full")
    verify_extraction(args.addon_zip, args.addon_exe, "AddOn")
    app_quarantine = {}
    for name, app in (("Lite", args.lite_app), ("Full", args.full_app)):
        app_quarantine[name] = bool(_run("xattr", "-p", "com.apple.quarantine", str(app)).strip())
        if not app_quarantine[name]:
            raise ValueError(f"{name} extracted app has no quarantine; use Finder extraction")
    return {
        "macos_version": os_major,
        "architecture": arch,
        "host_label": args.host_label,
        "preservation_root_sha256": directory_identity(args.data_root),
        "source_sha": args.source_sha,
        "team_id": args.team_id,
        "archive_sha256": hashes,
        "quarantine_zip": quarantine,
        "quarantine_apps": app_quarantine,
        "signatures_verified": True,
        "stapler_verified": True,
        "gatekeeper_assessed": True,
    }


def _save(path: Path, data: dict, *, overwrite: bool) -> None:
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.is_symlink():
        raise ValueError("refusing symlinked report output")
    blob = json.dumps(data, indent=2, ensure_ascii=False, allow_nan=False) + "\n"
    if not overwrite:
        with path.open("x", encoding="utf-8") as f:
            f.write(blob)
    else:
        fd, tmp = tempfile.mkstemp(prefix=".rehearsal-", dir=path.parent)
        try:
            with os.fdopen(fd, "w", encoding="utf-8") as f:
                f.write(blob)
            os.replace(tmp, path)
        finally:
            if os.path.exists(tmp):
                os.unlink(tmp)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    prepare = sub.add_parser("prepare", help="read-only preflight on real Mac, blank physical QA")
    for name in ("lite-zip", "full-zip", "addon-zip", "lite-app", "full-app", "addon-exe", "data-root", "output"):
        prepare.add_argument("--" + name, type=Path, required=True)
    for name in ("lite-sha256", "full-sha256", "addon-sha256", "source-sha", "team-id", "host-label"):
        prepare.add_argument("--" + name, required=True)
    mark = sub.add_parser("mark", help="operator attaches real evidence to one manual case")
    mark.add_argument("--report", required=True, type=Path)
    mark.add_argument("--case", required=True, choices=CASE_IDS)
    mark.add_argument("--status", choices=("passed", "failed"), required=True)
    mark.add_argument("--evidence", required=True)
    mark.add_argument("--notes", required=True)
    post = sub.add_parser("postcheck", help="compare PNG/config data after rollback")
    post.add_argument("--report", required=True, type=Path)
    post.add_argument("--data-root", required=True, type=Path)
    verify = sub.add_parser("verify", help="verify physical case receipts, not authenticity")
    verify.add_argument("--report", required=True, type=Path)
    matrix = sub.add_parser("matrix", help="verify 14/26 x arm64/x86_64 with consistent ZIP SHA256")
    matrix.add_argument("--reports", nargs=4, required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        if args.action == "prepare":
            report = build_report(_native_prepare(args), fingerprint_tree(args.data_root))
            _save(args.output, report, overwrite=False)
            print(f"Created NOT-RUN physical rehearsal at {args.output}; no app launched or installed.")
        elif args.action == "mark":
            data = read_report(args.report)
            add_case_evidence(data, args.case, args.status, args.evidence, args.notes)
            _safe_evidence(args.report.parent, args.evidence)
            _save(args.report, data, overwrite=True)
            print(f"Recorded operator-observed {args.case}={args.status}; not release authorization.")
        elif args.action == "postcheck":
            data = read_report(args.report)
            if data.get("preflight", {}).get("preservation_root_sha256") != directory_identity(args.data_root):
                raise ValueError("PNG/config preservation root directory differs from preflight")
            data["data_after"] = fingerprint_tree(args.data_root)
            data["completed_at_utc"] = _now()
            _save(args.report, data, overwrite=True)
            if data["data_before"] != data["data_after"]:
                raise ValueError("PNG/config preservation FAILED after rollback; report saved for diagnosis")
            print("Rollback data fingerprint matches original fixture; still requires manual review.")
        elif args.action == "verify":
            result = verify_report(read_report(args.report), args.report.parent)
            print(json.dumps(result, indent=2))
        else:
            files = args.reports
            result = verify_matrix([read_report(x) for x in files], [x.parent for x in files])
            print(json.dumps(result, indent=2))
        return 0
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError, plistlib.InvalidFileException, zipfile.BadZipFile) as exc:
        print(f"Physical release rehearsal rejected: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
