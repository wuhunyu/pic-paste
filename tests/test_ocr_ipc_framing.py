"""Parity tests for Python and Rust SHOC framed-response readers."""
import importlib.util
import struct
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "check_ocr_helper.py"
spec = importlib.util.spec_from_file_location("check_ocr_helper", SCRIPT)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class TestVisionIpcFraming(unittest.TestCase):
    @staticmethod
    def frame():
        payload = b'{"protocol_version":1,"error":"test"}'
        return struct.pack("<I", len(payload)) + payload

    def test_parses_one_complete_frame_with_apple_diagnostics_after_frame(self):
        data = self.frame() + b"Vision/CoreML: VM only supports CPU currently.."
        self.assertEqual(module.parse_framed_reply(data)["error"], "test")

    def test_exact_frame_parses_without_extra_bytes(self):
        self.assertEqual(module.parse_framed_reply(self.frame())["error"], "test")

    def test_truncated_oversized_and_zero_length_frames_are_rejected(self):
        for data in [
            self.frame()[:-1],
            struct.pack("<I", 8 * 1024 * 1024 + 1),
            struct.pack("<I", 0),
            b"\xff",
        ]:
            with self.subTest(data=data[:8]):
                with self.assertRaises(AssertionError):
                    module.parse_framed_reply(data)

    def test_qr_envelope_can_omit_nil_text_document(self):
        payload = b'{"protocol_version":1,"qr_document":{"codes":[]}}'
        framed = struct.pack("<I", len(payload)) + payload
        self.assertIsNone(module.parse_framed_reply(framed).get("document"))

    def test_unframed_preamble_is_not_accepted(self):
        with self.assertRaises(AssertionError):
            module.parse_framed_reply(b"log:" + self.frame())


if __name__ == "__main__":
    unittest.main()
