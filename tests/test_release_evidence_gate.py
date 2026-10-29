"""Synthetic gate tests: these are not physical macOS acceptance evidence."""
import csv
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

try:
    from scripts import release_evidence_gate as gate
except ImportError:
    gate = None


class ReleaseEvidenceGateTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(gate, "release evidence gate must be implemented")
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)

    def make_scroll(self, *, failures=2):
        categories = (["static_web"] * 20 + ["dynamic_web"] * 10 +
                      ["document"] * 10 + ["horizontal"] * 5 +
                      ["boundary"] * 5)
        path = self.root / "scroll-scenarios.csv"
        with path.open("w", newline="", encoding="utf-8") as stream:
            writer = csv.writer(stream)
            writer.writerow(["scenario_id", "category", "axis", "passed", "evidence"])
            for n, category in enumerate(categories):
                evidence = f"evidence-{n:02d}.txt"
                (self.root / evidence).write_text("unit-test fixture only", encoding="utf-8")
                writer.writerow([
                    f"synthetic-{n:02d}", category,
                    "horizontal" if category == "horizontal" else "vertical",
                    "false" if n < failures else "true", evidence,
                ])
        return path

    def test_accepts_48_of_50_with_all_categories(self):
        report = gate.check_scroll(self.make_scroll())
        self.assertEqual(report["samples"], 50)
        self.assertEqual(report["passed"], 48)
        self.assertAlmostEqual(report["success_rate"], 0.96)

    def test_rejects_less_than_95_percent_pass_rate(self):
        with self.assertRaisesRegex(ValueError, "95%"):
            gate.check_scroll(self.make_scroll(failures=3))

    def test_rejects_missing_or_escaping_evidence_files(self):
        path = self.make_scroll()
        (self.root / "evidence-00.txt").unlink()
        with self.assertRaisesRegex(ValueError, "evidence"):
            gate.check_scroll(path)
        self.make_scroll()
        contents = path.read_text(encoding="utf-8")
        path.write_text(contents.replace("evidence-00.txt", "../outside.txt", 1), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "evidence"):
            gate.check_scroll(path)

    def test_rejects_duplicate_scenario_ids(self):
        path = self.make_scroll()
        content = path.read_text(encoding="utf-8")
        path.write_text(content.replace("synthetic-01", "synthetic-00", 1), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "duplicate"):
            gate.check_scroll(path)

    def test_rejects_horizontal_category_with_vertical_axis(self):
        path = self.make_scroll()
        with path.open(newline="", encoding="utf-8") as stream:
            rows = list(csv.DictReader(stream))
        rows[40]["axis"] = "vertical"
        with path.open("w", newline="", encoding="utf-8") as stream:
            writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
            writer.writeheader()
            writer.writerows(rows)
        with self.assertRaisesRegex(ValueError, "horizontal"):
            gate.check_scroll(path)

    def test_rejects_repeated_json_fields_and_nonfinite_values(self):
        path = self.root / "invalid.json"
        path.write_text('{"x":1,"x":2}', encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "duplicate"):
            gate.read_json(path)
        path.write_text('{"x":NaN}', encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "non-finite"):
            gate.read_json(path)

    def test_rejects_short_and_nonhex_commit_id(self):
        for sha in ["4083926", "g" * 40, "A" * 40]:
            with self.subTest(sha=sha), self.assertRaisesRegex(ValueError, "SHA"):
                gate.require_sha(sha)

    def test_rejects_macos_report_with_mismatched_raw_p95(self):
        report = {"preflight": {"macos": {"major": 14}, "screen_count": 1},
                  "physical_cases": [{}] * 10,
                  "performance": {
                      "cold_launch_samples": 30, "cold_launch_p95_ms": 1.0,
                      "hotkey_samples": 100, "hotkey_p95_ms": 1.0,
                      "five_pin_frame_samples": 100, "five_pin_frame_p95_ms": 1.0,
                  }}
        host = self.root / "macos14"
        host.mkdir()
        (host / "native-acceptance.json").write_text(json.dumps(report), encoding="utf-8")
        (host / "evidence-index.md").write_text(
            "source_sha: " + "a" * 40 + "\ntester: synthetic\nobserved_at: 2026-10-29T09:00:00+08:00\n", encoding="utf-8")
        with (host / "latency_samples.csv").open("w", newline="", encoding="utf-8") as stream:
            writer = csv.writer(stream)
            writer.writerow(["scenario", "latency_ms"])
            for name, count in [("cold_launch", 30), ("hotkey_to_selection", 100), ("five_pin_frame", 100)]:
                writer.writerows([(name, 10.0)] * count)
        with mock.patch.object(gate, "verify_report", return_value=None):
            with self.assertRaisesRegex(ValueError, "P95"):
                gate.check_mac(host / "native-acceptance.json", 14, "a" * 40)

    def test_rejects_duplicate_scroll_csv_header(self):
        path = self.make_scroll()
        content = path.read_text(encoding="utf-8")
        path.write_text(content.replace(
            "scenario_id,category,axis,passed,evidence",
            "scenario_id,category,axis,passed,evidence,evidence", 1
        ), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "scroll CSV"):
            gate.check_scroll(path)

    def test_rejects_invalid_evidence_metadata_even_when_strings_are_present(self):
        expected_sha = "a" * 40
        host = self.root / "macos14"
        host.mkdir()
        report = {"preflight": {"macos": {"major": 14}, "screen_count": 1},
                  "physical_cases": [{}] * 10,
                  "performance": {"cold_launch_samples": 30, "cold_launch_p95_ms": 10.0,
                                  "hotkey_samples": 100, "hotkey_p95_ms": 10.0,
                                  "five_pin_frame_samples": 100, "five_pin_frame_p95_ms": 10.0}}
        (host / "native-acceptance.json").write_text(json.dumps(report), encoding="utf-8")
        (host / "evidence-index.md").write_text(
            "not_source_sha: " + expected_sha + "\ntester:\nobserved_at:\n", encoding="utf-8")
        with (host / "latency_samples.csv").open("w", newline="", encoding="utf-8") as stream:
            writer = csv.writer(stream)
            writer.writerow(["scenario", "latency_ms"])
            for name, count in [("cold_launch", 30), ("hotkey_to_selection", 100), ("five_pin_frame", 100)]:
                writer.writerows([(name, 10.0)] * count)
        with mock.patch.object(gate, "verify_report", return_value=None):
            with self.assertRaisesRegex(ValueError, "source_sha"):
                gate.check_mac(host / "native-acceptance.json", 14, expected_sha)

    def test_refuses_imbalanced_ocr_even_with_global_pass(self):
        fake_score = {"acceptance": {"passed": True, "reasons": []},
                      "zh": {"samples": 399}, "mixed": {"samples": 1}}
        with mock.patch.object(gate, "evaluate", return_value=fake_score):
            with self.assertRaisesRegex(ValueError, "200"):
                gate.check_ocr(self.root / "unused.csv")

    def test_reports_missing_external_provenance_as_separate_requirement(self):
        self.assertTrue(callable(gate.main))
        self.assertFalse((self.root / "release-gate.json").exists())


if __name__ == "__main__":
    unittest.main()
