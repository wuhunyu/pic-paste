"""Release-candidate metadata and deterministic icon tests; NOT real notarization."""
from __future__ import annotations
import plistlib
import struct
import tempfile
import unittest
import zlib
from pathlib import Path

from scripts.release_metadata import inspect_metadata
from scripts.build_macos_icon import build_icns, make_png


class ReleaseIdentityTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / "packaging/macos").mkdir(parents=True)
        (self.root / "Cargo.toml").write_text(
            '[workspace.package]\nversion = "1.0.0"\n', encoding="utf8"
        )
        (self.root / "Cargo.lock").write_text(
            'version = 4\n[[package]]\nname = "shot-core"\nversion = "1.0.0"\n'
            '[[package]]\nname = "shot-macos"\nversion = "1.0.0"\n', encoding="utf8"
        )
        self.info = {
            "CFBundleIdentifier": "com.wuhunyu.picpaste",
            "CFBundleShortVersionString": "1.0.0",
            "CFBundleVersion": "10000",
            "CFBundleName": "PicPaste",
            "CFBundleExecutable": "PicPaste",
            "CFBundlePackageType": "APPL",
            "CFBundleIconFile": "PicPaste.icns",
            "LSMinimumSystemVersion": "14.0",
            "NSScreenCaptureUsageDescription": "用户主动截图，不自动上传。",
        }
        self.save()

    def save(self):
        (self.root / "packaging/macos/Info.plist").write_bytes(plistlib.dumps(self.info))

    def test_metadata_is_consistent(self):
        data = inspect_metadata(self.root)
        self.assertEqual((data["version"], data["bundle_id"], data["min_macos"]),
                         ("1.0.0", "com.wuhunyu.picpaste", "14.0"))

    def test_rejects_placeholder_bundle_id(self):
        self.info["CFBundleIdentifier"] = "com.example.picpaste"
        self.save()
        with self.assertRaisesRegex(ValueError, "bundle identifier"):
            inspect_metadata(self.root)

    def test_rejects_mismatched_plist_version(self):
        self.info["CFBundleShortVersionString"] = "0.1.0"
        self.save()
        with self.assertRaisesRegex(ValueError, "version"):
            inspect_metadata(self.root)

    def test_rejects_mismatched_workspace_lock_version(self):
        lock = self.root / "Cargo.lock"
        lock.write_text(lock.read_text().replace(
            'name = "shot-core"\nversion = "1.0.0"',
            'name = "shot-core"\nversion = "0.1.0"'
        ))
        with self.assertRaisesRegex(ValueError, "shot-core"):
            inspect_metadata(self.root)

    def test_rejects_missing_icon_key(self):
        self.info.pop("CFBundleIconFile")
        self.save()
        with self.assertRaisesRegex(ValueError, "icon"):
            inspect_metadata(self.root)

    def test_rejects_wrong_macos_floor_and_build_number(self):
        self.info["LSMinimumSystemVersion"] = "13.0"
        self.save()
        with self.assertRaisesRegex(ValueError, "minimum"):
            inspect_metadata(self.root)
        self.info["LSMinimumSystemVersion"] = "14.0"
        self.info["CFBundleVersion"] = "no"
        self.save()
        with self.assertRaisesRegex(ValueError, "build number"):
            inspect_metadata(self.root)

    def test_rejects_empty_usage_description(self):
        self.info["NSScreenCaptureUsageDescription"] = ""
        self.save()
        with self.assertRaisesRegex(ValueError, "Screen Recording"):
            inspect_metadata(self.root)


class IconTests(unittest.TestCase):
    def test_icns_has_all_png_resolutions(self):
        data = build_icns()
        self.assertEqual(data[:4], b"icns")
        self.assertEqual(struct.unpack(">I", data[4:8])[0], len(data))
        expected = {b"icp4":16, b"icp5":32, b"icp6":64,
                    b"ic07":128, b"ic08":256, b"ic09":512, b"ic10":1024}
        position = 8
        observed = {}
        while position < len(data):
            tag = data[position:position + 4]
            n = struct.unpack(">I", data[position + 4:position + 8])[0]
            self.assertGreater(n, 8)
            png = data[position + 8:position + n]
            self.assertTrue(png.startswith(b"\x89PNG\r\n\x1a\n"))
            observed[tag] = struct.unpack(">II", png[16:24])
            position += n
        self.assertEqual(position, len(data))
        self.assertEqual(observed, {tag:(size,size) for tag,size in expected.items()})

    def test_icon_png_has_transparent_corner_and_opaque_center(self):
        png = make_png(32)
        self.assertEqual(png[25], 6)  # RGBA
        position = 8
        compressed = []
        while position < len(png):
            n = struct.unpack(">I", png[position:position + 4])[0]
            if png[position + 4:position + 8] == b"IDAT":
                compressed.append(png[position + 8:position + 8 + n])
            position += n + 12
        raw = zlib.decompress(b"".join(compressed))
        self.assertEqual(raw[4], 0)
        self.assertGreater(raw[16 * (32 * 4 + 1) + 1 + 16 * 4 + 3], 0)

    def test_icon_generation_is_deterministic(self):
        self.assertEqual(make_png(32), make_png(32))


if __name__ == "__main__":
    unittest.main()
