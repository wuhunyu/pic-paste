"""Synthetic ZIP/file relations; these cannot prove real Apple signatures."""
import tempfile
import unittest
import zipfile
from pathlib import Path

from scripts.physical_release_integrity import directory_identity, verify_extraction


class PhysicalArchiveLinkTests(unittest.TestCase):
    def setUp(self):
        t = tempfile.TemporaryDirectory()
        self.addCleanup(t.cleanup)
        self.root = Path(t.name)
        self.app = self.root / "Full" / "PicPaste.app"
        contents = self.app / "Contents"
        (contents / "MacOS").mkdir(parents=True)
        (contents / "Resources").mkdir()
        (contents / "Helpers").mkdir()
        self.members = {
            "PicPaste.app/Contents/Info.plist": b"plist-synthetic",
            "PicPaste.app/Contents/MacOS/PicPaste": b"universal-main-synthetic",
            "PicPaste.app/Contents/Resources/PicPaste.icns": b"icon-synthetic",
            "PicPaste.app/Contents/Helpers/PicPasteOCR": b"ocr-synthetic",
        }
        for name, body in self.members.items():
            target = self.app.parent / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(body)
        self.zip_path = self.root / "download.zip"
        self.build_zip()

    def build_zip(self):
        with zipfile.ZipFile(self.zip_path, "w") as zf:
            for name, body in self.members.items():
                zf.writestr(name, body)

    def test_full_extracted_bytes_match_zip(self):
        self.assertEqual(verify_extraction(self.zip_path, self.app, "Full"), 4)

    def test_rejects_different_extracted_main(self):
        (self.app / "Contents/MacOS/PicPaste").write_bytes(b"different-signed-main")
        with self.assertRaisesRegex(ValueError, "different|mismatch"):
            verify_extraction(self.zip_path, self.app, "Full")

    def test_rejects_replaced_helper_even_if_main_matches(self):
        (self.app / "Contents/Helpers/PicPasteOCR").write_bytes(b"different-ocr")
        with self.assertRaisesRegex(ValueError, "different|mismatch"):
            verify_extraction(self.zip_path, self.app, "Full")

    def test_lite_requires_only_main_info_and_icon(self):
        self.assertEqual(verify_extraction(self.zip_path, self.app, "Lite"), 3)

    def test_standalone_addon_needs_exact_downloaded_bytes(self):
        addon = self.root / "PicPasteOCR"
        addon.write_bytes(b"addon-content")
        with zipfile.ZipFile(self.zip_path, "w") as zf:
            zf.writestr("PicPasteOCR", b"addon-content")
        self.assertEqual(verify_extraction(self.zip_path, addon, "AddOn"), 1)
        addon.write_bytes(b"different-addon")
        with self.assertRaisesRegex(ValueError, "different|mismatch"):
            verify_extraction(self.zip_path, addon, "AddOn")

    def test_rejects_symlinked_extracted_file(self):
        target = self.app / "Contents/Helpers/PicPasteOCR"
        target.unlink()
        target.symlink_to(self.root / "external")
        with self.assertRaisesRegex(ValueError, "symlink"):
            verify_extraction(self.zip_path, self.app, "Full")

    def test_rejects_missing_zip_member(self):
        self.members.pop("PicPaste.app/Contents/Resources/PicPaste.icns")
        self.build_zip()
        with self.assertRaisesRegex(ValueError, "missing"):
            verify_extraction(self.zip_path, self.app, "Full")

    def test_preservation_folder_identity_binds_same_root(self):
        a = self.root / "a"
        b = self.root / "b"
        a.mkdir()
        b.mkdir()
        self.assertEqual(directory_identity(a), directory_identity(a))
        self.assertNotEqual(directory_identity(a), directory_identity(b))
        with self.assertRaisesRegex(ValueError, "directory"):
            directory_identity(self.root / "missing")


if __name__ == "__main__":
    unittest.main()
