"""P6-T3 synthetic candidate inspection: not signed packages or real-device QA."""
from __future__ import annotations

import hashlib
import json
import os
import plistlib
import tempfile
import unittest
import zipfile
from pathlib import Path

from scripts.release_candidate_audit import audit_candidate

SHA = "a" * 40
TEAM = "AB12CD34EF"
VERSION = "1.0.0"


class CandidateAuditTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.candidate = self.root / "candidate"
        self.candidate.mkdir()
        self.gate = self.root / "gate.json"
        self.approval = self.root / "approval.json"
        gate = {
            "schema": "picpaste.release-evidence/1",
            "source_sha": SHA, "metrics_passed": True,
            "manual_provenance_review_required": True,
            "macos": [
                {"macos": 14, "physical_cases": 10},
                {"macos": 26, "physical_cases": 10},
            ],
            "ocr": {
                "acceptance": {"passed": True},
                "zh": {"samples": 200}, "mixed": {"samples": 200},
            },
            "scroll": {"samples": 50, "passed": 48, "success_rate": .96},
        }
        self.gate.write_text(json.dumps(gate), encoding="utf-8")
        approval = {
            "schema": "picpaste.release-approval/1",
            "source_sha": SHA,
            "gate_sha256": hashlib.sha256(self.gate.read_bytes()).hexdigest(),
            "reviewer": "synthetic-fixture",
            "reviewed_at": "2026-10-29T09:00:00+08:00",
            "reviewed_original_device_video": True,
            "reviewed_original_ocr_and_scroll_samples": True,
            "verified_mac14": True, "verified_mac26": True,
            "verified_arm64_hardware": True, "verified_intel_hardware": True,
            "open_sev1": 0, "open_sev2": 0,
        }
        self.approval.write_text(json.dumps(approval), encoding="utf-8")
        self.version = VERSION
        self.package_names = {
            "Lite": f"PicPaste-macos-universal2-Lite-{VERSION}-CANDIDATE.zip",
            "Full": f"PicPaste-macos-universal2-Full-{VERSION}-CANDIDATE.zip",
            "addon": f"PicPaste-OCR-AddOn-macos-universal2-{VERSION}-CANDIDATE.zip",
        }
        self.populate()

    def _app_info(self):
        return plistlib.dumps({
            "CFBundleIdentifier": "com.wuhunyu.picpaste",
            "CFBundleShortVersionString": VERSION,
            "CFBundleExecutable": "PicPaste",
            "CFBundleIconFile": "PicPaste.icns",
            "LSMinimumSystemVersion": "14.0",
        })

    def _zip(self, variant, override=None):
        if variant == "addon":
            content = {"PicPasteOCR": b"ADDON-DUMMY"}
        else:
            prefix = "PicPaste.app/Contents/"
            content = {
                prefix + "Info.plist": self._app_info(),
                prefix + "MacOS/PicPaste": b"FAT-MAIN-SYNTHETIC",
                prefix + "Resources/PicPaste.icns": b"icns-DUMMY",
            }
            if variant == "Full":
                content[prefix + "Helpers/PicPasteOCR"] = b"FAT-OCR-SYNTHETIC"
        if override:
            content.update(override)
        filename = self.candidate / self.package_names[variant]
        with zipfile.ZipFile(filename, "w", compression=zipfile.ZIP_DEFLATED) as zf:
            for name, body in content.items():
                zf.writestr(name, body)
        return filename

    def update_sums(self):
        entries = []
        for name in self.package_names.values():
            digest = hashlib.sha256((self.candidate / name).read_bytes()).hexdigest()
            entries.append(f"{digest}  ./{name}")
        (self.candidate / "SHA256SUMS.txt").write_text(
            "\n".join(entries) + "\n", encoding="utf-8"
        )

    def populate(self):
        for variant in ("Lite", "Full", "addon"):
            self._zip(variant)
        for variant in ("Lite", "Full"):
            app = self.candidate / variant / "PicPaste.app" / "Contents"
            (app / "MacOS").mkdir(parents=True)
            (app / "Resources").mkdir()
            (app / "MacOS" / "PicPaste").write_bytes(b"FAT-MAIN-SYNTHETIC")
            (app / "Info.plist").write_bytes(self._app_info())
            (app / "Resources" / "PicPaste.icns").write_bytes(b"icns-DUMMY")
            if variant == "Full":
                (app / "Helpers").mkdir()
                (app / "Helpers" / "PicPasteOCR").write_bytes(b"FAT-OCR-SYNTHETIC")
        (self.candidate / "OCR-AddOn").mkdir()
        (self.candidate / "OCR-AddOn" / "PicPasteOCR").write_bytes(b"ADDON-DUMMY")
        for variant in ("Lite", "Full", "addon"):
            (self.candidate / f"notary-{variant}.json").write_text(
                json.dumps({"id": "request-" + variant, "status": "Accepted"}),
                encoding="utf-8",
            )
        self.manifest = {
            "schema": "picpaste.signed-release-candidate/1",
            "source_sha": SHA,
            "apple_team_id": TEAM,
            "version": VERSION,
            "bundle_id": "com.wuhunyu.picpaste",
            "gate_sha256": hashlib.sha256(self.gate.read_bytes()).hexdigest(),
            "approval_sha256": hashlib.sha256(self.approval.read_bytes()).hexdigest(),
            "clean_download_gatekeeper_required": True,
            "manual_distribution_approval_required": True,
            "github_release_created": False,
        }
        self.save_manifest()
        self.update_sums()

    def save_manifest(self):
        (self.candidate / "release-manifest.json").write_text(
            json.dumps(self.manifest), encoding="utf-8"
        )

    def audit(self, verify_macos=False):
        return audit_candidate(
            self.candidate, SHA, TEAM, self.gate, self.approval,
            verify_macos=verify_macos,
        )

    def test_structurally_valid_candidate_remains_unreleased(self):
        result = self.audit()
        self.assertTrue(result["integrity_passed"])
        self.assertEqual(result["verified_archives"], 3)
        self.assertFalse(result["ready_for_public_release"])
        self.assertFalse(result["physical_download_verification_completed"])
        self.assertFalse(result["macos_signature_verified"])

    def test_rejects_modified_archive(self):
        name = self.package_names["Lite"]
        (self.candidate / name).write_bytes((self.candidate / name).read_bytes() + b"tampered")
        with self.assertRaisesRegex(ValueError, "checksum"):
            self.audit()

    def test_rejects_missing_or_duplicate_checksum(self):
        path = self.candidate / "SHA256SUMS.txt"
        first = path.read_text().splitlines()[0]
        path.write_text(first + "\n" + first + "\n")
        with self.assertRaisesRegex(ValueError, "duplicate|exactly"):
            self.audit()

    def test_rejects_missing_archive_member_even_if_checksum_updated(self):
        self._zip("Full", {"PicPaste.app/Contents/Helpers/PicPasteOCR": b""})
        self.update_sums()
        with self.assertRaisesRegex(ValueError, "OCR|empty"):
            self.audit()

    def test_rejects_zip_path_traversal(self):
        self._zip("Lite", {"../malicious": b"x"})
        self.update_sums()
        with self.assertRaisesRegex(ValueError, "path"):
            self.audit()

    def test_rejects_canary_companion_even_with_valid_checksums(self):
        self._zip("Lite", {"PicPaste.app/Contents/MacOS/ns-panel-probe": b"bad"})
        self.update_sums()
        with self.assertRaisesRegex(ValueError, "Canary"):
            self.audit()

    def test_rejects_mismatched_zip_and_staged_executable(self):
        target = self.candidate / "Full/PicPaste.app/Contents/Helpers/PicPasteOCR"
        target.write_bytes(b"different helper")
        with self.assertRaisesRegex(ValueError, "staged"):
            self.audit()

    def test_rejects_wrong_plist_bundle_identity(self):
        self._zip("Lite", {"PicPaste.app/Contents/Info.plist": plistlib.dumps(
            {"CFBundleIdentifier": "com.example.picpaste"}
        )})
        self.update_sums()
        with self.assertRaisesRegex(ValueError, "bundle|identity|plist"):
            self.audit()

    def test_rejects_unaccepted_or_duplicate_notary_result(self):
        (self.candidate / "notary-Full.json").write_text(
            json.dumps({"id": "bad", "status": "Invalid"})
        )
        with self.assertRaisesRegex(ValueError, "Accepted"):
            self.audit()
        (self.candidate / "notary-Full.json").write_text(
            json.dumps({"id": "request-Lite", "status": "Accepted"})
        )
        with self.assertRaisesRegex(ValueError, "duplicate"):
            self.audit()

    def test_rejects_review_or_gate_changes_after_signing(self):
        self.manifest["approval_sha256"] = "0" * 64
        self.save_manifest()
        with self.assertRaisesRegex(ValueError, "approval"):
            self.audit()
        self.populate()
        self.manifest["gate_sha256"] = "0" * 64
        self.save_manifest()
        with self.assertRaisesRegex(ValueError, "gate"):
            self.audit()

    def test_rejects_wrong_source_sha_and_apple_team(self):
        self.manifest["source_sha"] = "b" * 40
        self.save_manifest()
        with self.assertRaisesRegex(ValueError, "source SHA"):
            self.audit()
        self.populate()
        self.manifest["apple_team_id"] = "OTHERTEAM1"
        self.save_manifest()
        with self.assertRaisesRegex(ValueError, "Team"):
            self.audit()

    def test_rejects_manifest_marked_as_public_or_approved(self):
        self.manifest["github_release_created"] = True
        self.save_manifest()
        with self.assertRaisesRegex(ValueError, "release"):
            self.audit()
        self.populate()
        self.manifest["clean_download_gatekeeper_required"] = False
        self.save_manifest()
        with self.assertRaisesRegex(ValueError, "Gatekeeper"):
            self.audit()

    def test_rejects_symlinked_candidate_artifact(self):
        path = self.candidate / "notary-addon.json"
        alias = self.root / "notary-copy.json"
        alias.write_bytes(path.read_bytes())
        path.unlink()
        path.symlink_to(alias)
        with self.assertRaisesRegex(ValueError, "symlink"):
            self.audit()

    def test_native_signature_check_does_not_accept_synthetic_fixtures(self):
        with self.assertRaises((OSError, ValueError, RuntimeError)):
            self.audit(verify_macos=True)


if __name__ == "__main__":
    unittest.main()
