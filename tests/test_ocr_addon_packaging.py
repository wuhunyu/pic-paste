"""Functional boundaries of the user-invoked standalone OCR Add-on manager."""
import os
import platform
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANAGER = ROOT / "scripts" / "manage-ocr-addon.sh"
PACKAGER = ROOT / "scripts" / "package-ocr-addon.sh"


def run_manager(home: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["bash", str(MANAGER), *args],
        env={**os.environ, "HOME": str(home)},
        capture_output=True,
        text=True,
        timeout=10,
        check=False,
    )


class TestAddonPackaging(unittest.TestCase):
    def test_both_scripts_have_valid_bash_syntax(self):
        for script in (MANAGER, PACKAGER):
            with self.subTest(script=script.name):
                result = subprocess.run(
                    ["bash", "-n", str(script)],
                    capture_output=True,
                    text=True,
                    check=False,
                )
                self.assertEqual(result.returncode, 0, result.stderr)

    @unittest.skipUnless(platform.system() == "Darwin", "macOS security boundary")
    def test_clean_home_reports_absent_addon_without_mutation(self):
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory)
            result = run_manager(home, "status", "/nonexistent/PicPaste.app")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("not installed", result.stdout)
            self.assertEqual(list(home.iterdir()), [])

    @unittest.skipUnless(platform.system() == "Darwin", "macOS security boundary")
    def test_uninstall_only_removes_owned_helper_name(self):
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory)
            add_on = home / "Library/Application Support/PicPaste/AddOns/OCR"
            add_on.mkdir(parents=True)
            (add_on / "PicPasteOCR").write_bytes(b"sample executable bytes")
            (add_on / "important.txt").write_text("keep", encoding="utf-8")
            result = run_manager(home, "uninstall")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse((add_on / "PicPasteOCR").exists())
            self.assertEqual(
                (add_on / "important.txt").read_text(encoding="utf-8"), "keep"
            )

    @unittest.skipUnless(platform.system() == "Darwin", "macOS security boundary")
    def test_refuses_symlinked_addon_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "home"
            outside = Path(directory) / "outside"
            outside.mkdir()
            home.mkdir()
            parent = home / "Library/Application Support/PicPaste/AddOns"
            parent.mkdir(parents=True)
            (parent / "OCR").symlink_to(outside, target_is_directory=True)
            (outside / "PicPasteOCR").write_bytes(b"do not touch")
            result = run_manager(home, "uninstall")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("symlink", result.stderr)
            self.assertEqual((outside / "PicPasteOCR").read_bytes(), b"do not touch")

    @unittest.skipUnless(platform.system() == "Darwin", "macOS security boundary")
    def test_refuses_symlink_source_even_before_checking_app_signature(self):
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "home"
            home.mkdir()
            source = Path(directory) / "PicPasteOCR"
            source.write_bytes(b"not executable")
            linked = Path(directory) / "linked-helper"
            linked.symlink_to(source)
            result = run_manager(home, "install", "/no/PicPaste.app", str(linked))
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("symlink", result.stderr)
