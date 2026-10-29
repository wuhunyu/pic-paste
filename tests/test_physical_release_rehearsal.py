"""Synthetic cases only: do not represent physical Gatekeeper evidence."""
import json
import tempfile
import unittest
from pathlib import Path

from scripts.physical_release_integrity import directory_identity
from scripts.physical_release_rehearsal import (
    CASE_IDS,
    add_case_evidence,
    build_report,
    fingerprint_tree,
    read_report,
    verify_matrix,
    verify_report,
)

SOURCE_SHA = "a" * 40
TEAM_ID = "AB12CD34EF"
HASHES = {"Lite": "1" * 64, "Full": "2" * 64, "AddOn": "3" * 64}


class RehearsalTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.base = Path(tmp.name)
        self.sentinel = self.base / "sentinel"
        self.sentinel.mkdir()
        (self.sentinel / "saved.png").write_bytes(b"\x89PNG\r\n\x1a\n-SYNTHETIC")
        (self.sentinel / "settings.json").write_text('{"test":true}')
        self.before = fingerprint_tree(self.sentinel)

    def report(self, os_major=14, arch="arm64", host="test-host-1"):
        observed = {
            "macos_version": f"{os_major}.2.1",
            "architecture": arch,
            "host_label": host,
            "preservation_root_sha256": directory_identity(self.sentinel),
            "source_sha": SOURCE_SHA,
            "team_id": TEAM_ID,
            "archive_sha256": HASHES,
            "quarantine_zip": {key: True for key in HASHES},
            "quarantine_apps": {"Lite": True, "Full": True},
            "signatures_verified": True,
            "stapler_verified": True,
            "gatekeeper_assessed": True,
        }
        record = build_report(observed, self.before)
        self.assertEqual(record["release_ready"], False)
        return record

    def evidence(self, name):
        path = self.base / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(b"synthetic-video-evidence-not-real")
        return name

    def completed(self, os_major=14, arch="arm64", host="test-host-1"):
        r = self.report(os_major, arch, host)
        for cid in CASE_IDS:
            add_case_evidence(r, cid, "passed", self.evidence(host + "-" + cid + ".txt"),
                              "Operator observed this scenario in the synthetic fixture.")
        r["data_after"] = self.before.copy()
        r["completed_at_utc"] = r["prepared_at_utc"]
        return r

    def test_fingerprint_is_repeatable_and_detects_png_change(self):
        a = fingerprint_tree(self.sentinel)
        b = fingerprint_tree(self.sentinel)
        self.assertEqual(a, b)
        self.assertEqual(a["png_files"], 1)
        (self.sentinel / "saved.png").write_bytes(b"different-png")
        self.assertNotEqual(a["sha256"], fingerprint_tree(self.sentinel)["sha256"])

    def test_fingerprint_refuses_symlink_and_empty_directory(self):
        (self.sentinel / "shortcut").symlink_to(self.sentinel / "saved.png")
        with self.assertRaisesRegex(ValueError, "symlink"):
            fingerprint_tree(self.sentinel)
        with self.assertRaisesRegex(ValueError, "PNG"):
            fingerprint_tree(self.base / "nothing")

    def test_blank_report_never_claims_physical_pass(self):
        r = self.report()
        self.assertTrue(all(case["status"] == "not_run" for case in r["cases"]))
        with self.assertRaisesRegex(ValueError, "not passed"):
            verify_report(r, self.base)

    def test_complete_evidence_is_eligible_for_manual_review_only(self):
        result = verify_report(self.completed(), self.base)
        self.assertTrue(result["evidence_complete"])
        self.assertFalse(result["ready_for_public_release"])
        self.assertTrue(result["physical_provenance_requires_human_review"])

    def test_case_requires_real_relative_evidence_and_notes(self):
        record = self.report()
        with self.assertRaisesRegex(ValueError, "relative"):
            add_case_evidence(record, CASE_IDS[0], "passed", "/tmp/video.mp4", "test")
        with self.assertRaisesRegex(ValueError, "notes"):
            add_case_evidence(record, CASE_IDS[0], "passed", "x.mp4", "")
        with self.assertRaisesRegex(ValueError, "status"):
            add_case_evidence(record, CASE_IDS[0], "success", "x.mp4", "details")

    def test_rejects_traversal_and_missing_evidence(self):
        r = self.completed()
        r["cases"][0]["evidence"] = "../secret.png"
        with self.assertRaisesRegex(ValueError, "path|relative"):
            verify_report(r, self.base)
        r = self.completed()
        r["cases"][0]["evidence"] = "missing.mp4"
        with self.assertRaisesRegex(ValueError, "missing"):
            verify_report(r, self.base)

    def test_detects_modified_rollback_data(self):
        r = self.completed()
        r["data_after"]["sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "preservation"):
            verify_report(r, self.base)

    def test_rejects_untrusted_preflight_or_ci_flag(self):
        for key in ("gatekeeper_assessed", "signatures_verified", "stapler_verified"):
            r = self.completed()
            r["preflight"][key] = False
            with self.assertRaisesRegex(ValueError, "preflight"):
                verify_report(r, self.base)
        r = self.completed()
        r["preflight"]["quarantine_zip"]["Full"] = False
        with self.assertRaisesRegex(ValueError, "quarantine"):
            verify_report(r, self.base)

    def test_rejects_invalid_manual_result_and_future_claim(self):
        r = self.completed()
        r["cases"][0]["status"] = "failed"
        with self.assertRaisesRegex(ValueError, "not passed"):
            verify_report(r, self.base)
        r = self.completed()
        r["release_ready"] = True
        with self.assertRaisesRegex(ValueError, "release_ready"):
            verify_report(r, self.base)

    def test_rejects_duplicate_fields_and_nonfinite_json(self):
        p = self.base / "bad.json"
        p.write_text('{"test":1,"test":2}')
        with self.assertRaisesRegex(ValueError, "duplicate"):
            read_report(p)
        p.write_text('{"nan":NaN}')
        with self.assertRaisesRegex(ValueError, "non-finite"):
            read_report(p)

    def test_matrix_requires_4_distinct_os_arch_combinations(self):
        reports = [self.completed(14, "arm64", "a14"),
                   self.completed(14, "x86_64", "i14"),
                   self.completed(26, "arm64", "a26")]
        with self.assertRaisesRegex(ValueError, "matrix"):
            verify_matrix(reports, [self.base] * 3)
        reports.append(self.completed(26, "x86_64", "i26"))
        res = verify_matrix(reports, [self.base] * 4)
        self.assertEqual(res["observed_combinations"], 4)
        self.assertFalse(res["ready_for_public_release"])

    def test_matrix_rejects_duplicate_arch_or_inconsistent_archive(self):
        r = [self.completed(14, "arm64", "a14"),
             self.completed(14, "x86_64", "i14"),
             self.completed(26, "arm64", "a26"),
             self.completed(26, "x86_64", "i26")]
        r[3]["preflight"]["architecture"] = "arm64"
        with self.assertRaisesRegex(ValueError, "matrix|duplicate"):
            verify_matrix(r, [self.base] * 4)
        r[3]["preflight"]["architecture"] = "x86_64"
        r[3]["preflight"]["archive_sha256"]["Lite"] = "f" * 64
        with self.assertRaisesRegex(ValueError, "archive"):
            verify_matrix(r, [self.base] * 4)

    def test_postcheck_does_not_accept_identical_copies_from_another_directory(self):
        from scripts.physical_release_rehearsal import main
        copied = self.base / "copied"
        copied.mkdir()
        (copied / "saved.png").write_bytes((self.sentinel / "saved.png").read_bytes())
        (copied / "settings.json").write_bytes((self.sentinel / "settings.json").read_bytes())
        report = self.base / "report.json"
        report.write_text(json.dumps(self.report()))
        self.assertEqual(main(["postcheck", "--report", str(report), "--data-root", str(copied)]), 1)

    def test_rejects_time_without_timezone_or_out_of_order(self):
        r = self.completed()
        r["completed_at_utc"] = "2026-10-29T09:00:00"
        with self.assertRaisesRegex(ValueError, "timezone"):
            verify_report(r, self.base)
        r["completed_at_utc"] = "2000-01-01T00:00:00+00:00"
        with self.assertRaisesRegex(ValueError, "precedes"):
            verify_report(r, self.base)

    def test_rejects_evidence_symlink(self):
        r = self.completed()
        target = self.base / "real-video.txt"
        target.write_bytes(b"synthetic")
        fake = self.base / "alias-video.txt"
        fake.symlink_to(target)
        r["cases"][0]["evidence"] = fake.name
        with self.assertRaisesRegex(ValueError, "symlink"):
            verify_report(r, self.base)

    def test_rejects_archive_hash_and_source_sha_drift(self):
        r = self.completed()
        r["preflight"]["archive_sha256"]["AddOn"] = "bad"
        with self.assertRaisesRegex(ValueError, "SHA256"):
            verify_report(r, self.base)
        r = self.completed()
        r["preflight"]["source_sha"] = "b" * 40
        with self.assertRaisesRegex(ValueError, "source SHA"):
            verify_report(r, self.base, expected_sha=SOURCE_SHA)


if __name__ == "__main__":
    unittest.main()
