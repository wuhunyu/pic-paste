#!/usr/bin/env python3
"""Native PicPaste on-device acceptance evidence, never simulated GUI success.

Generate a blank single-host report from an *actual* ns-panel-probe --preflight
JSON record. Strict verification checks explicit human-entered evidence and
measured performance limits; it cannot run macOS GUI/IME/Spaces/TCC tests.
"""
import argparse
import json
import math
import sys
from pathlib import Path

PREFLIGHT_SCHEMA = "picpaste.native-preflight/1"
REPORT_SCHEMA = "picpaste.native-acceptance/1"
REQUIRED_CASES = (
    ("window_lifecycle", "NSPanel open/close/resize/minimize/focus and full cleanup"),
    ("material_accessibility", "Native Liquid Glass or version fallback, light/dark and Reduce Transparency"),
    ("retina_multiscreen", "1x/2x, negative-origin screen and eight selection handles against source pixels"),
    ("native_markup_png", "Brush/arrow/rectangle/Chinese text IPC, PNG crop/alpha pixel check, undo/redo and cancel"),
    ("chinese_ime", "CJK marked text, candidate placement, Enter/Escape and focus recovery"),
    ("spaces_fullscreen", "Spaces/fullscreen, monitor unplug/restore and no screenshot window pollution"),
    ("tcc_recovery", "Screen Recording deny/grant/revoke/restart and safe error fallback"),
    ("clipboard_pins", "PNG clipboard paste and pinned window transparency/passthrough recovery"),
    ("hotkey_behavior", "Hotkey conflict/restore and repeatable screen selection"),
    ("performance_budgets", "Real 30 cold launches, 100 hotkeys, idle CPU/RSS and 5-pin frame samples"),
)
PERFORMANCE_MIN_SAMPLES = {
    "cold_launch_samples": 30,
    "hotkey_samples": 100,
    "five_pin_frame_samples": 100,
}
PERFORMANCE_MAX_VALUES = {
    "cold_launch_p95_ms": 1500.0,
    "hotkey_p95_ms": 200.0,
    "idle_cpu_percent": 0.5,
    "idle_rss_mib": 80.0,
    "five_pin_frame_p95_ms": 16.7,
}


def _no_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON field: {key}")
        result[key] = value
    return result


def _reject_constant(value):
    raise ValueError(f"non-finite JSON constant: {value}")


def _require_keys(obj, keys, name):
    if type(obj) is not dict or set(obj) != set(keys):
        raise ValueError(f"{name}: unexpected or missing keys")


def _number(value, field, *, positive=False):
    if type(value) not in (int, float) or not math.isfinite(value):
        raise ValueError(f"{field}: number must be finite")
    if positive and value <= 0:
        raise ValueError(f"{field}: number must be positive")
    if value < 0 and field in PERFORMANCE_MAX_VALUES:
        raise ValueError(f"{field}: value must be non-negative")
    return float(value)


def _natural(value, field):
    if type(value) is not int or value < 0:
        raise ValueError(f"{field}: integer must be non-negative")
    return value


def _boolean(value, field):
    if type(value) is not bool:
        raise ValueError(f"{field}: expected boolean")


def validate_preflight(preflight):
    _require_keys(
        preflight,
        {
            "schema", "macos", "material", "reduce_transparency",
            "screen_recording_preflight", "screen_count", "has_negative_origin",
            "has_mixed_scales", "interactive_gui_observed", "screens",
        },
        "preflight",
    )
    if preflight["schema"] != PREFLIGHT_SCHEMA:
        raise ValueError("unknown native preflight schema")
    _require_keys(preflight["macos"], {"major", "minor", "patch"}, "macos")
    major = _natural(preflight["macos"]["major"], "macos.major")
    _natural(preflight["macos"]["minor"], "macos.minor")
    _natural(preflight["macos"]["patch"], "macos.patch")
    if major < 14:
        raise ValueError("preflight requires macOS 14+")
    for key in (
        "reduce_transparency", "screen_recording_preflight",
        "has_negative_origin", "has_mixed_scales", "interactive_gui_observed",
    ):
        _boolean(preflight[key], key)
    if preflight["interactive_gui_observed"]:
        raise ValueError("preflight must not claim interactive GUI acceptance")
    material = preflight["material"]
    if material not in ("native", "vibrancy", "opaque"):
        raise ValueError("unexpected AppKit material")
    if preflight["reduce_transparency"] != (material == "opaque"):
        raise ValueError("opaque material must honor Reduce Transparency")
    if major < 26 and material != "vibrancy" and not preflight["reduce_transparency"]:
        raise ValueError("macOS <26 cannot claim native glass")
    screens = preflight["screens"]
    if type(screens) is not list:
        raise ValueError("screens must be an array")
    if _natural(preflight["screen_count"], "screen_count") != len(screens):
        raise ValueError("screen_count mismatch")
    for i, screen in enumerate(screens):
        _require_keys(
            screen,
            {"origin_x_pt", "origin_y_pt", "width_pt", "height_pt", "backing_scale"},
            f"screens[{i}]",
        )
        for key in ("origin_x_pt", "origin_y_pt", "width_pt", "height_pt", "backing_scale"):
            _number(screen[key], key, positive=key in ("width_pt", "height_pt", "backing_scale"))
    negative = any(s["origin_x_pt"] < 0 or s["origin_y_pt"] < 0 for s in screens)
    mixed = bool(screens) and any(
        abs(s["backing_scale"] - screens[0]["backing_scale"]) > 0.01
        for s in screens
    )
    if preflight["has_negative_origin"] != negative:
        raise ValueError("negative-origin flag disagrees with screens")
    if preflight["has_mixed_scales"] != mixed:
        raise ValueError("mixed-backing-scale flag disagrees with screens")


def parse_preflight(raw: str):
    preflight = json.loads(
        raw, object_pairs_hook=_no_duplicate_keys, parse_constant=_reject_constant
    )
    validate_preflight(preflight)
    return preflight


def build_report(preflight):
    # Re-parse to validate and deep-copy; no mutable caller data is retained.
    preflight = parse_preflight(json.dumps(preflight, allow_nan=False))
    return {
        "schema": REPORT_SCHEMA,
        "scope": "single_host_manual_evidence_only",
        "release_ready": False,
        "preflight": preflight,
        "physical_cases": [
            {"id": key, "description": desc, "status": "not_run", "evidence": ""}
            for key, desc in REQUIRED_CASES
        ],
        "performance": {
            **{key: 0 for key in PERFORMANCE_MIN_SAMPLES},
            **{key: None for key in PERFORMANCE_MAX_VALUES},
        },
    }


def verify_report(report):
    _require_keys(
        report,
        {"schema", "scope", "release_ready", "preflight", "physical_cases", "performance"},
        "acceptance report",
    )
    if report["schema"] != REPORT_SCHEMA or report["scope"] != "single_host_manual_evidence_only":
        raise ValueError("invalid acceptance report scope/schema")
    if report["release_ready"] is not False:
        raise ValueError("a single-device report cannot claim release readiness")
    validate_preflight(report["preflight"])
    if not report["preflight"]["screens"]:
        raise ValueError("headless preflight has no observed displays; physical QA is still required")

    cases = report["physical_cases"]
    if type(cases) is not list or [c.get("id") for c in cases if type(c) is dict] != [
        key for key, _description in REQUIRED_CASES
    ] or len(cases) != len(REQUIRED_CASES):
        raise ValueError("missing/duplicate/unknown physical acceptance case")
    for case, (expected_id, expected_description) in zip(cases, REQUIRED_CASES):
        _require_keys(case, {"id", "description", "status", "evidence"}, expected_id)
        if case["id"] != expected_id or case["description"] != expected_description:
            raise ValueError(f"{expected_id}: acceptance scenario changed")
        if case["status"] != "passed":
            raise ValueError(f"{expected_id}: actual device acceptance not passed")
        if type(case["evidence"]) is not str or not case["evidence"].strip():
            raise ValueError(f"{expected_id}: physical evidence note required")

    metrics = report["performance"]
    _require_keys(metrics, {*PERFORMANCE_MIN_SAMPLES, *PERFORMANCE_MAX_VALUES}, "performance")
    for key, lower in PERFORMANCE_MIN_SAMPLES.items():
        if _natural(metrics[key], key) < lower:
            raise ValueError(f"{key}: insufficient real samples (minimum {lower})")
    for key, ceiling in PERFORMANCE_MAX_VALUES.items():
        value = _number(metrics[key], key)
        if value > ceiling:
            raise ValueError(f"{key}: {value} exceeds acceptance threshold {ceiling}")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    prepare = sub.add_parser("prepare", help="blank physical-QA template from --preflight JSON")
    prepare.add_argument("--preflight", required=True, type=Path)
    prepare.add_argument("--output", required=True, type=Path)
    verify = sub.add_parser("verify", help="check explicit single-Mac QA evidence and P95 limits")
    verify.add_argument("--report", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        if args.command == "prepare":
            report = build_report(parse_preflight(args.preflight.read_text(encoding="utf-8")))
            args.output.write_text(
                json.dumps(report, indent=2, ensure_ascii=False, allow_nan=False) + "\n",
                encoding="utf-8",
            )
            print(f"Created UNVERIFIED single-host acceptance template: {args.output}")
        else:
            report = json.loads(
                args.report.read_text(encoding="utf-8"),
                object_pairs_hook=_no_duplicate_keys,
                parse_constant=_reject_constant,
            )
            verify_report(report)
            print("Single-host evidence report is complete; cross-host and release QA remain open.")
        return 0
    except (OSError, ValueError, TypeError, KeyError) as error:
        print(f"Native acceptance evidence rejected: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
