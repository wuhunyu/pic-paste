"""Synthetic fixture tests only. Passing these does not prove physical QA."""
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from scripts.release_signing_policy import (
    check_notary_result, check_release_approval, check_signer, read_json,
)
SHA = "1"*40
TEAM = "AB12CD34EF"
IDENTITY = f"Developer ID Application: Example Org ({TEAM})"

class ReleaseSigningPolicyTests(unittest.TestCase):
    def setUp(self):
        t=tempfile.TemporaryDirectory()
        self.addCleanup(t.cleanup)
        self.root=Path(t.name)
        self.gate_file=self.root/"release-gate.json"
        self.approval_file=self.root/"approval.json"
        self.gate={
            "schema":"picpaste.release-evidence/1", "source_sha":SHA,
            "metrics_passed":True, "manual_provenance_review_required":True,
            "macos":[{"macos":14,"physical_cases":10},{"macos":26,"physical_cases":10}],
            "ocr":{"acceptance":{"passed":True},"zh":{"samples":200},"mixed":{"samples":200}},
            "scroll":{"samples":50,"passed":48,"success_rate":0.96},
        }
        self.approval={
            "schema":"picpaste.release-approval/1","source_sha":SHA,
            "gate_sha256":"","reviewer":"synthetic-fixture-reviewer",
            "reviewed_at":"2026-10-29T03:00:00+00:00",
            "reviewed_original_device_video":True,
            "reviewed_original_ocr_and_scroll_samples":True,
            "verified_mac14":True,"verified_mac26":True,
            "verified_arm64_hardware":True,"verified_intel_hardware":True,
            "open_sev1":0,"open_sev2":0,
        }
        self.save()

    def save(self):
        self.gate_file.write_text(json.dumps(self.gate),encoding="utf-8")
        self.approval["gate_sha256"]=hashlib.sha256(self.gate_file.read_bytes()).hexdigest()
        self.approval_file.write_text(json.dumps(self.approval),encoding="utf-8")

    def check(self):
        return check_release_approval(self.gate_file,self.approval_file,SHA)

    def test_well_formed_structural_review(self):
        r=self.check()
        self.assertEqual(r["source_sha"],SHA)
        self.assertTrue(r["human_review_document_present"])
        self.assertTrue(r["real_provenance_cannot_be_verified_programmatically"])

    def test_missing_file_fails_closed(self):
        self.gate_file.unlink()
        with self.assertRaises((ValueError,OSError)):self.check()

    def test_gate_tampering_after_review(self):
        self.gate_file.write_bytes(self.gate_file.read_bytes()+b" ")
        with self.assertRaisesRegex(ValueError,"hash"):self.check()

    def test_gate_sha_mismatch(self):
        self.gate["source_sha"]="2"*40
        self.save()
        with self.assertRaisesRegex(ValueError,"source SHA"):self.check()

    def test_approval_sha_mismatch(self):
        self.approval["source_sha"]="2"*40
        self.save()
        with self.assertRaisesRegex(ValueError,"source SHA"):self.check()

    def test_failed_physical_case_and_ocr(self):
        self.gate["macos"][1]["physical_cases"]=9
        self.save()
        with self.assertRaisesRegex(ValueError,"physical"):self.check()
        self.gate["macos"][1]["physical_cases"]=10
        self.gate["ocr"]["acceptance"]["passed"]=False
        self.save()
        with self.assertRaisesRegex(ValueError,"OCR"):self.check()

    def test_duplicate_os(self):
        self.gate["macos"][1]["macos"]=14
        self.save()
        with self.assertRaisesRegex(ValueError,"14 and 26"):self.check()

    def test_low_scroll_rate(self):
        self.gate["scroll"].update(passed=47,success_rate=.94)
        self.save()
        with self.assertRaisesRegex(ValueError,"scroll"):self.check()

    def test_all_review_flags_are_mandatory(self):
        for flag in ("reviewed_original_device_video",
                     "reviewed_original_ocr_and_scroll_samples",
                     "verified_mac14","verified_mac26",
                     "verified_arm64_hardware","verified_intel_hardware"):
            with self.subTest(flag=flag):
                self.approval[flag]=False
                self.save()
                with self.assertRaisesRegex(ValueError,flag):self.check()
                self.approval[flag]=True

    def test_open_severity_blocker(self):
        self.approval["open_sev2"]=1
        self.save()
        with self.assertRaisesRegex(ValueError,"Sev"):self.check()

    def test_reviewer_and_timezone_required(self):
        self.approval["reviewer"]=" "
        self.save()
        with self.assertRaisesRegex(ValueError,"reviewer"):self.check()
        self.approval["reviewer"]="human"
        self.approval["reviewed_at"]="2026-10-29T03:00:00"
        self.save()
        with self.assertRaisesRegex(ValueError,"timezone"):self.check()

    def test_duplicate_json_rejected(self):
        self.approval_file.write_text('{"x":1,"x":2}',encoding="utf-8")
        with self.assertRaisesRegex(ValueError,"duplicate"):read_json(self.approval_file)

    def test_symlinked_evidence_rejected(self):
        alias=self.root/"alias.json"
        alias.symlink_to(self.approval_file)
        with self.assertRaisesRegex(ValueError,"symlink"):
            check_release_approval(self.gate_file,alias,SHA)

    def test_matching_developer_id_only(self):
        self.assertEqual(check_signer(IDENTITY,TEAM,"picpaste-release"),TEAM)
        for identity,team,profile in [
            ("-",TEAM,"good"),(IDENTITY,"UNKNOWN1XX","good"),
            ("Apple Distribution: Org (AB12CD34EF)",TEAM,"good"),
            (IDENTITY,TEAM,"--password"),(IDENTITY,TEAM,"")]:
            with self.subTest(identity=identity,team=team,profile=profile):
                with self.assertRaises(ValueError):check_signer(identity,team,profile)

    def test_notary_acceptance_and_submission_id_required(self):
        p=self.root/"notary.json"
        p.write_text(json.dumps({"id":"1234","status":"Accepted"}))
        self.assertEqual(check_notary_result(p)["status"],"Accepted")
        p.write_text(json.dumps({"id":"1234","status":"Invalid"}))
        with self.assertRaisesRegex(ValueError,"Accepted"):check_notary_result(p)
        p.write_text(json.dumps({"status":"Accepted"}))
        with self.assertRaisesRegex(ValueError,"submission"):check_notary_result(p)

if __name__=="__main__":
    unittest.main()
