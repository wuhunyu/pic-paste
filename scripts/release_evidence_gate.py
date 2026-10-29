#!/usr/bin/env python3
"""PicPaste macOS release evidence gate, not a GUI test or proof of authenticity."""
from __future__ import annotations

import argparse
import csv
import json
import re
import subprocess
import sys
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

if __package__:
    from .bench_report import summarize
    from .native_acceptance import verify_report
    from .ocr_eval import evaluate
else:
    from bench_report import summarize
    from native_acceptance import verify_report
    from ocr_eval import evaluate

CATEGORIES = {
    "static_web": 20,
    "dynamic_web": 10,
    "document": 10,
    "horizontal": 5,
    "boundary": 5,
}
SCENARIOS = {
    "cold_launch": ("cold_launch_samples", "cold_launch_p95_ms"),
    "hotkey_to_selection": ("hotkey_samples", "hotkey_p95_ms"),
    "five_pin_frame": ("five_pin_frame_samples", "five_pin_frame_p95_ms"),
}


def reject_duplicates(pairs: list[tuple[str, object]]) -> dict:
    result: dict = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON field: {key}")
        result[key] = value
    return result


def reject_constant(value: str) -> None:
    raise ValueError(f"non-finite JSON constant: {value}")


def read_json(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as stream:
        result = json.load(
            stream,
            object_pairs_hook=reject_duplicates,
            parse_constant=reject_constant,
        )
    if type(result) is not dict:
        raise ValueError(f"JSON object expected: {path}")
    return result


def require_sha(value: str) -> str:
    if re.fullmatch(r"[0-9a-f]{40}", value) is None:
        raise ValueError("--source-sha must be a full lowercase 40-digit SHA")
    actual = subprocess.check_output(
        ["git", "rev-parse", "HEAD"],
        cwd=Path(__file__).resolve().parent.parent,
        text=True,
    ).strip()
    if actual != value:
        raise ValueError(f"evidence SHA {value} differs from checkout HEAD {actual}")
    return actual


def check_mac(path: Path, expected_major: int, source_sha: str) -> dict:
    report = read_json(path)
    verify_report(report)  # Existing strict ten-case and physical-metric gates.
    major = report["preflight"]["macos"]["major"]
    if major != expected_major:
        raise ValueError(f"{path}: expected macOS {expected_major}, got {major}")
    index = path.parent / "evidence-index.md"
    if not index.is_file():
        raise ValueError(f"missing human evidence index: {index}")
    contents = index.read_text(encoding="utf-8")
    metadata: dict[str, str] = {}
    for line in contents.splitlines():
        match = re.fullmatch(r"(source_sha|tester|observed_at):\s*(.*?)\s*", line)
        if match:
            key, value = match.groups()
            if key in metadata:
                raise ValueError(f"duplicate evidence metadata {key} in {index}")
            metadata[key] = value
    if metadata.get("source_sha") != source_sha:
        raise ValueError(f"missing exact matching source_sha in {index}")
    if not metadata.get("tester"):
        raise ValueError(f"missing nonempty tester in {index}")
    try:
        observed_at = datetime.fromisoformat(metadata["observed_at"].replace("Z", "+00:00"))
        if observed_at.tzinfo is None:
            raise ValueError("timezone is required")
    except (KeyError, ValueError) as error:
        raise ValueError(f"missing or invalid observed_at in {index}") from error
    raw = path.parent / "latency_samples.csv"
    metrics = summarize(raw)  # Existing deterministic percentile algorithm.
    recorded = report["performance"]
    for scenario, (count_key, p95_key) in SCENARIOS.items():
        if scenario not in metrics:
            raise ValueError(f"{raw}: missing measured scenario {scenario}")
        measured = metrics[scenario]
        if measured["samples"] != recorded[count_key]:
            raise ValueError(f"{scenario}: report sample count differs from raw CSV")
        if abs(measured["p95_ms"] - recorded[p95_key]) > 0.02:
            raise ValueError(f"{scenario}: report P95 differs from measured raw CSV")
    return {
        "macos": major,
        "physical_cases": len(report["physical_cases"]),
        "preflight_screens": report["preflight"]["screen_count"],
        "performance": recorded,
    }


def check_scroll(path: Path) -> dict:
    counts: Counter = Counter()
    total = passed = 0
    seen: set[str] = set()
    base = path.resolve().parent
    with path.open("r", encoding="utf-8-sig", newline="") as stream:
        reader = csv.DictReader(stream)
        if reader.fieldnames != ["scenario_id", "category", "axis", "passed", "evidence"]:
            raise ValueError("scroll CSV requires exactly scenario_id,category,axis,passed,evidence")
        for row in reader:
            if None in row or any(value is None for value in row.values()):
                raise ValueError(f"invalid scroll CSV row {reader.line_num}")
            scenario_id, category, axis, flag, evidence = (
                row[key].strip() for key in
                ("scenario_id", "category", "axis", "passed", "evidence")
            )
            if not scenario_id or scenario_id in seen:
                raise ValueError(f"missing/duplicate scroll scenario ID: {scenario_id}")
            if category not in CATEGORIES or axis not in ("vertical", "horizontal"):
                raise ValueError(f"{scenario_id}: unknown category/axis")
            if category == "horizontal" and axis != "horizontal":
                raise ValueError(f"{scenario_id}: horizontal suite must use horizontal axis")
            if flag not in ("true", "false"):
                raise ValueError(f"{scenario_id}: passed must be true/false")
            rel = Path(evidence)
            if not evidence or rel.is_absolute():
                raise ValueError(f"{scenario_id}: evidence must be relative")
            sample = (base / rel).resolve()
            if not sample.is_relative_to(base) or not sample.is_file():
                raise ValueError(f"{scenario_id}: missing or escaping evidence file")
            seen.add(scenario_id)
            counts[category] += 1
            total += 1
            passed += flag == "true"
    if total < 50:
        raise ValueError(f"requires at least 50 real scroll scenarios, got {total}")
    for category, lower in CATEGORIES.items():
        if counts[category] < lower:
            raise ValueError(f"{category}: requires >={lower}, got {counts[category]}")
    rate = passed / total
    if rate + 1e-12 < 0.95:
        raise ValueError(f"scroll trust rate {rate:.3%} <95%")
    return {"samples": total, "passed": passed, "success_rate": rate, "categories": dict(counts)}


def check_ocr(path: Path) -> dict:
    report = evaluate(path)  # Existing code-point weighted CER and hit-rate logic.
    if not report["acceptance"]["passed"]:
        raise ValueError("OCR quality gates failed: " + "; ".join(report["acceptance"]["reasons"]))
    if report["zh"]["samples"] < 200 or report["mixed"]["samples"] < 200:
        raise ValueError("sample balance requires >=200 zh and >=200 mixed")
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mac14", required=True, type=Path)
    parser.add_argument("--mac26", required=True, type=Path)
    parser.add_argument("--ocr", required=True, type=Path)
    parser.add_argument("--scroll", required=True, type=Path)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        sha = require_sha(args.source_sha)
        mac14 = check_mac(args.mac14, 14, sha)
        mac26 = check_mac(args.mac26, 26, sha)
        ocr = check_ocr(args.ocr)
        scroll = check_scroll(args.scroll)
        result = {
            "schema": "picpaste.release-evidence/1",
            "source_sha": sha,
            "checked_at_utc": datetime.now(timezone.utc).isoformat(),
            "macos": [mac14, mac26],
            "ocr": ocr,
            "scroll": scroll,
            "metrics_passed": True,
            "manual_provenance_review_required": True,
            "note": "This program cannot confirm that supplied video/data came from physical devices.",
        }
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"Structure and numeric thresholds passed; MANUAL provenance review still required: {args.output}")
        return 0
    except (OSError, ValueError, KeyError, TypeError, csv.Error, subprocess.CalledProcessError) as exc:
        print(f"Release evidence rejected: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
