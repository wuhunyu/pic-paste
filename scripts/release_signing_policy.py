#!/usr/bin/env python3
"""Fail-closed signing eligibility checks, NOT proof of genuine physical QA.

The gate JSON is a machine-generated summary and the approval JSON is a
human declaration. This program checks internal consistency only; it cannot
authenticate underlying videos, devices, Apple certificates or signatures.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from datetime import datetime
from pathlib import Path
from typing import Any

TEAM = re.compile(r"[A-Z0-9]{10}\Z")
SHA = re.compile(r"[0-9a-f]{40}\Z")
PROFILE = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.-]{0,63}\Z")
IDENTITY = re.compile(r"Developer ID Application: .+ \(([A-Z0-9]{10})\)\Z")
REVIEW_FLAGS = (
    "reviewed_original_device_video",
    "reviewed_original_ocr_and_scroll_samples",
    "verified_mac14",
    "verified_mac26",
    "verified_arm64_hardware",
    "verified_intel_hardware",
)


def _no_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON field: {key}")
        result[key] = value
    return result


def _no_constant(value: str) -> None:
    raise ValueError(f"invalid nonfinite JSON value: {value}")


def read_json(path: Path) -> dict[str, Any]:
    path = Path(path)
    if path.is_symlink():
        raise ValueError(f"refusing symlinked evidence file: {path}")
    if not path.is_file():
        raise ValueError(f"required evidence file missing: {path}")
    if path.stat().st_size > 4_194_304:
        raise ValueError("evidence JSON exceeds safe 4MiB limit")
    with path.open(encoding="utf-8") as stream:
        data = json.load(stream, object_pairs_hook=_no_duplicate_keys,
                         parse_constant=_no_constant)
    if type(data) is not dict:
        raise ValueError("evidence must be a JSON object")
    return data


def _is_int(value: Any, minimum: int = 0) -> bool:
    return type(value) is int and value >= minimum


def check_signer(identity: str, team_id: str, notary_profile: str) -> str:
    if not TEAM.fullmatch(team_id):
        raise ValueError("Apple Team ID must be exactly ten uppercase letters/digits")
    match = IDENTITY.fullmatch(identity)
    if match is None or match.group(1) != team_id:
        raise ValueError("only an exact matching Developer ID Application identity is allowed")
    if not PROFILE.fullmatch(notary_profile):
        raise ValueError("invalid local notary Keychain profile name")
    return team_id


def check_release_approval(gate_path: Path, approval_path: Path, source_sha: str) -> dict[str, Any]:
    if not SHA.fullmatch(source_sha):
        raise ValueError("source SHA must be a full lowercase 40-character commit ID")
    gate = read_json(gate_path)
    approval = read_json(approval_path)
    if gate.get("schema") != "picpaste.release-evidence/1":
        raise ValueError("unknown release evidence schema")
    if gate.get("source_sha") != source_sha or approval.get("source_sha") != source_sha:
        raise ValueError("source SHA differs between checkout, evidence and reviewer approval")
    if gate.get("metrics_passed") is not True:
        raise ValueError("release measurements are not marked as passed")
    if gate.get("manual_provenance_review_required") is not True:
        raise ValueError("release evidence is missing the manual provenance warning")

    mac = gate.get("macos")
    if (type(mac) is not list or len(mac) != 2 or
            {item.get("macos") for item in mac if type(item) is dict} != {14, 26}):
        raise ValueError("release requires real macOS 14 and 26 evidence summaries")
    if any(not _is_int(item.get("physical_cases"), 10) for item in mac):
        raise ValueError("each macOS version requires at least ten physical cases")

    ocr = gate.get("ocr")
    if (type(ocr) is not dict or type(ocr.get("acceptance")) is not dict or
            ocr["acceptance"].get("passed") is not True):
        raise ValueError("OCR acceptance must be passed")
    if not all(type(ocr.get(name)) is dict and _is_int(ocr[name].get("samples"), 200)
               for name in ("zh", "mixed")):
        raise ValueError("OCR requires at least 200 genuine zh and mixed samples each")

    scroll = gate.get("scroll")
    if type(scroll) is not dict:
        raise ValueError("scroll acceptance is missing")
    total, passed, rate = (scroll.get("samples"), scroll.get("passed"),
                           scroll.get("success_rate"))
    if (not _is_int(total, 50) or not _is_int(passed) or passed > total or
            type(rate) not in (float, int) or abs(rate - passed / total) > 1e-9 or
            passed / total + 1e-12 < .95):
        raise ValueError("scroll sample count, success rate or category gate failed")

    if approval.get("schema") != "picpaste.release-approval/1":
        raise ValueError("unknown manual approval schema")
    gate_hash = hashlib.sha256(Path(gate_path).read_bytes()).hexdigest()
    if approval.get("gate_sha256") != gate_hash:
        raise ValueError("approval evidence hash mismatch: release gate changed after review")
    reviewer = approval.get("reviewer")
    if not isinstance(reviewer, str) or not reviewer.strip():
        raise ValueError("nonempty human reviewer is required")
    timestamp = approval.get("reviewed_at")
    try:
        if not isinstance(timestamp, str):
            raise ValueError("invalid timestamp")
        when = datetime.fromisoformat(timestamp.replace("Z", "+00:00"))
        if when.tzinfo is None or when.utcoffset() is None:
            raise ValueError("missing timezone")
    except ValueError as exc:
        raise ValueError("reviewed_at must be ISO8601 with timezone") from exc
    for flag in REVIEW_FLAGS:
        if approval.get(flag) is not True:
            raise ValueError(f"manual provenance requirement not approved: {flag}")
    if (approval.get("open_sev1") != 0 or approval.get("open_sev2") != 0 or
            any(type(approval.get(key)) is not int for key in ("open_sev1", "open_sev2"))):
        raise ValueError("Sev1/Sev2 release blockers must both be zero")

    return {
        "source_sha": source_sha,
        "human_review_document_present": True,
        "real_provenance_cannot_be_verified_programmatically": True,
        "reviewer": reviewer.strip(),
        "gate_sha256": gate_hash,
    }


def check_notary_result(path: Path) -> dict[str, Any]:
    report = read_json(path)
    if report.get("status") != "Accepted":
        raise ValueError("Apple notarization status is not Accepted")
    if not isinstance(report.get("id"), str) or not report["id"].strip():
        raise ValueError("Apple notarization submission ID is missing")
    return {"id": report["id"], "status": "Accepted"}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    preflight = sub.add_parser("preflight")
    preflight.add_argument("--gate", type=Path, required=True)
    preflight.add_argument("--approval", type=Path, required=True)
    preflight.add_argument("--source-sha", required=True)
    preflight.add_argument("--team-id", required=True)
    preflight.add_argument("--identity", required=True)
    preflight.add_argument("--notary-profile", required=True)
    notary = sub.add_parser("notary")
    notary.add_argument("--report", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        if args.action == "preflight":
            check_signer(args.identity, args.team_id, args.notary_profile)
            result = check_release_approval(args.gate, args.approval, args.source_sha)
            print("Structural gate and human declaration matched for source SHA: "
                  + result["source_sha"])
            print("IMPORTANT: Review of original QA evidence remains a human responsibility.")
        else:
            result = check_notary_result(args.report)
            print("Apple notary status Accepted; submission ID: " + result["id"])
        return 0
    except (ValueError, OSError, KeyError, TypeError) as exc:
        print(f"Signing eligibility rejected: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
