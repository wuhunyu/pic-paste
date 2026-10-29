import os
import plistlib
import tempfile
import unittest
from pathlib import Path

from scripts.macos_test_dmg import validate_app, dmg_name


class DmgTests(unittest.TestCase):
    def setUp(self):
        t = tempfile.TemporaryDirectory()
        self.addCleanup(t.cleanup)
        self.root = Path(t.name)
        self.app = self.root / 'PicPaste.app'
        contents = self.app / 'Contents'
        (contents / 'MacOS').mkdir(parents=True)
        (contents / 'Resources').mkdir()
        (contents / 'Helpers').mkdir()
        info = {
            'CFBundleIdentifier': 'com.wuhunyu.picpaste',
            'CFBundleExecutable': 'PicPaste',
            'CFBundleName': 'PicPaste',
            'CFBundleShortVersionString': '1.0.0',
            'CFBundleVersion': '10000',
            'CFBundlePackageType': 'APPL',
            'CFBundleIconFile': 'PicPaste.icns',
            'LSMinimumSystemVersion': '14.0',
        }
        (contents / 'Info.plist').write_bytes(plistlib.dumps(info))
        exe = contents / 'MacOS/PicPaste'
        exe.write_bytes(b'mock-main')
        exe.chmod(0o755)
        (contents / 'Resources/PicPaste.icns').write_bytes(b'icns' + b'fake')
        helper = contents / 'Helpers/PicPasteOCR'
        helper.write_bytes(b'mock-helper')
        helper.chmod(0o755)

    def test_valid_full_app_and_explicit_test_name(self):
        validate_app(self.app, 'Full', 'arm64')
        self.assertEqual(dmg_name('arm64', 'Full'), 'PicPaste-macos-arm64-Full-UNNOTARIZED-TEST.dmg')

    def test_valid_lite_requires_no_ocr(self):
        (self.app / 'Contents/Helpers/PicPasteOCR').unlink()
        validate_app(self.app, 'Lite', 'arm64')

    def test_full_requires_ocr_and_lite_rejects_ocr(self):
        with self.assertRaisesRegex(ValueError, 'Lite.*OCR'):
            validate_app(self.app, 'Lite', 'arm64')
        (self.app / 'Contents/Helpers/PicPasteOCR').unlink()
        with self.assertRaisesRegex(ValueError, 'Full.*OCR'):
            validate_app(self.app, 'Full', 'arm64')

    def test_rejects_canary_or_probe(self):
        probe = self.app / 'Contents/MacOS/ns-panel-probe'
        probe.write_bytes(b'canary')
        with self.assertRaisesRegex(ValueError, 'Canary'):
            validate_app(self.app, 'Full', 'arm64')

    def test_rejects_bad_bundle_identity(self):
        path = self.app / 'Contents/Info.plist'
        info = plistlib.loads(path.read_bytes())
        info['CFBundleIdentifier'] = 'com.example.picpaste'
        path.write_bytes(plistlib.dumps(info))
        with self.assertRaisesRegex(ValueError, 'identity|bundle'):
            validate_app(self.app, 'Full', 'arm64')

    def test_rejects_inaccurate_label_or_variant(self):
        for arch in ('bad', 'ARM', 'aarch64'):
            with self.subTest(arch=arch):
                with self.assertRaisesRegex(ValueError, 'architecture'):
                    validate_app(self.app, 'Full', arch)
        for variant in ('Canary', 'Public', ''):
            with self.subTest(variant=variant):
                with self.assertRaisesRegex(ValueError, 'variant'):
                    validate_app(self.app, variant, 'arm64')

    def test_rejects_symlinked_executable_and_icon(self):
        p = self.app / 'Contents/MacOS/PicPaste'
        p.unlink()
        p.symlink_to(self.app / 'Contents/Info.plist')
        with self.assertRaisesRegex(ValueError, 'symlink'):
            validate_app(self.app, 'Full', 'arm64')
        p.unlink()
        p.write_bytes(b'mock')
        p.chmod(0o755)
        icon = self.app / 'Contents/Resources/PicPaste.icns'
        icon.unlink()
        icon.symlink_to(self.app / 'Contents/Info.plist')
        with self.assertRaisesRegex(ValueError, 'symlink'):
            validate_app(self.app, 'Full', 'arm64')

    def test_rejects_missing_or_wrong_icon_header(self):
        p = self.app / 'Contents/Resources/PicPaste.icns'
        p.write_bytes(b'not-an-icon')
        with self.assertRaisesRegex(ValueError, 'icon'):
            validate_app(self.app, 'Full', 'arm64')

    def test_rejects_nonexecutables(self):
        p = self.app / 'Contents/MacOS/PicPaste'
        p.chmod(0o644)
        with self.assertRaisesRegex(ValueError, 'executable'):
            validate_app(self.app, 'Full', 'arm64')


if __name__ == '__main__':
    unittest.main()
