import copy
import json
import unittest

from scripts.native_acceptance import (
    REQUIRED_CASES,
    build_report,
    parse_preflight,
    verify_report,
)


def preflight(screens=None):
    if screens is None:
        screens = [
            {
                "origin_x_pt": -1440.0,
                "origin_y_pt": 0.0,
                "width_pt": 1440.0,
                "height_pt": 900.0,
                "backing_scale": 2.0,
            },
            {
                "origin_x_pt": 0.0,
                "origin_y_pt": 0.0,
                "width_pt": 1920.0,
                "height_pt": 1080.0,
                "backing_scale": 1.0,
            },
        ]
    return {
        "schema": "picpaste.native-preflight/1",
        "macos": {"major": 26, "minor": 0, "patch": 1},
        "material": "native",
        "reduce_transparency": False,
        "screen_recording_preflight": False,
        "screen_count": len(screens),
        "has_negative_origin": any(
            s["origin_x_pt"] < 0 or s["origin_y_pt"] < 0 for s in screens
        ),
        "has_mixed_scales": len({s["backing_scale"] for s in screens}) > 1,
        "interactive_gui_observed": False,
        "screens": screens,
    }


def accepted_report():
    report = build_report(preflight())
    for case in report["physical_cases"]:
        case["status"] = "passed"
        case["evidence"] = f"Real Mac reproduction recorded for {case['id']}"
    report["performance"] = {
        "cold_launch_samples": 30,
        "cold_launch_p95_ms": 1200.0,
        "hotkey_samples": 100,
        "hotkey_p95_ms": 160.0,
        "idle_cpu_percent": 0.4,
        "idle_rss_mib": 72.0,
        "five_pin_frame_samples": 100,
        "five_pin_frame_p95_ms": 15.6,
    }
    return report


class NativeAcceptanceTests(unittest.TestCase):
    def test_parse_finite_strict_schema_and_preserve_actual_screen_points(self):
        source = preflight()
        parsed = parse_preflight(json.dumps(source))
        self.assertEqual(parsed["screens"][0]["origin_x_pt"], -1440.0)
        self.assertTrue(parsed["has_mixed_scales"])
        self.assertFalse(parsed["interactive_gui_observed"])

    def test_headless_runner_is_valid_input_but_never_physical_acceptance(self):
        headless = preflight([])
        report = build_report(parse_preflight(json.dumps(headless)))
        self.assertFalse(report["preflight"]["interactive_gui_observed"])
        self.assertEqual(report["preflight"]["screen_count"], 0)
        self.assertTrue(all(case["status"] == "not_run" for case in report["physical_cases"]))
        with self.assertRaises(ValueError):
            verify_report(report)

    def test_untrusted_json_rejects_nan_duplicate_keys_and_extra_records(self):
        raw = json.dumps(preflight())
        for payload in [
            raw + raw,
            raw.replace('"screen_count": 2', '"screen_count": NaN'),
            raw.replace('"schema":', '"schema":"invalid","schema":', 1),
        ]:
            with self.subTest(payload=payload[:70]), self.assertRaises(ValueError):
                parse_preflight(payload)

    def test_wrong_types_counts_material_and_forged_gui_observation_rejected(self):
        good = preflight()
        variants = []
        for key, value in [
            ("screen_recording_preflight", 1),
            ("screen_count", 1),
            ("interactive_gui_observed", True),
            ("has_negative_origin", False),
            ("has_mixed_scales", False),
            ("material", "unofficial-glass"),
        ]:
            bad = copy.deepcopy(good)
            bad[key] = value
            variants.append(bad)
        bad_scale = copy.deepcopy(good)
        bad_scale["screens"][0]["backing_scale"] = float("inf")
        variants.append(bad_scale)
        bad_version = copy.deepcopy(good)
        bad_version["macos"]["major"] = 0
        variants.append(bad_version)
        for bad in variants:
            with self.subTest(report=repr(bad)[:120]), self.assertRaises(ValueError):
                parse_preflight(json.dumps(bad))

    def test_prepare_never_marks_gui_or_performance_as_passed(self):
        report = build_report(preflight())
        self.assertEqual(report["schema"], "picpaste.native-acceptance/1")
        self.assertEqual(
            [case["id"] for case in report["physical_cases"]],
            [case[0] for case in REQUIRED_CASES],
        )
        self.assertTrue(all(case["status"] == "not_run" and case["evidence"] == ""
                            for case in report["physical_cases"]))
        self.assertEqual(report["performance"]["cold_launch_samples"], 0)
        self.assertIsNone(report["performance"]["hotkey_p95_ms"])
        with self.assertRaises(ValueError):
            verify_report(report)

    def test_evidence_and_all_required_cases_must_be_present_once(self):
        report = accepted_report()
        verify_report(report)
        for mutate in [
            lambda r: r["physical_cases"].pop(),
            lambda r: r["physical_cases"].append(copy.deepcopy(r["physical_cases"][0])),
            lambda r: r["physical_cases"][0].update(status="not_run"),
            lambda r: r["physical_cases"][0].update(evidence="  "),
            lambda r: r["physical_cases"][0].update(status="skipped"),
            lambda r: r["physical_cases"][0].update(id="unknown_case"),
        ]:
            bad = copy.deepcopy(report)
            mutate(bad)
            with self.subTest(mutation=mutate), self.assertRaises(ValueError):
                verify_report(bad)

    def test_performance_limits_enforced_with_real_sample_counts(self):
        report = accepted_report()
        failures = [
            ("cold_launch_samples", 29),
            ("cold_launch_p95_ms", 1500.1),
            ("hotkey_samples", 99),
            ("hotkey_p95_ms", 200.1),
            ("idle_cpu_percent", 0.51),
            ("idle_rss_mib", 80.1),
            ("five_pin_frame_samples", 0),
            ("five_pin_frame_p95_ms", 16.71),
            ("hotkey_p95_ms", float("nan")),
        ]
        for key, value in failures:
            bad = copy.deepcopy(report)
            bad["performance"][key] = value
            with self.subTest(metric=key, value=value), self.assertRaises(ValueError):
                verify_report(bad)


if __name__ == "__main__":
    unittest.main()
