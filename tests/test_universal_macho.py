"""Synthetic Mach-O architecture fixture tests, NOT actual macOS execution."""
from __future__ import annotations

import struct
import tempfile
import unittest
from pathlib import Path

from scripts.universal_macho import detect_thin_arch, validate_universal, main

ARM64 = 0x0100000C
X64 = 0x01000007


def thin(cpu: int) -> bytes:
    # A 64-bit little endian Mach-O header (32 bytes) suitable for static
    # structure testing only; no segments or executable instructions.
    return struct.pack('<IiiIIIII', 0xfeedfacf, cpu, 0, 2, 0, 0, 0, 0)


def fat(architectures: list[tuple[int, bytes]], *, is_64: bool = False,
        forced_size: int | None = None, overlapping: bool = False) -> bytes:
    entry_size = 32 if is_64 else 20
    current = 8 + entry_size * len(architectures)
    contents = bytearray(current)
    contents[:8] = struct.pack('>II', 0xcafebabf if is_64 else 0xcafebabe, len(architectures))
    for i, (cpu, body) in enumerate(architectures):
        # Fixed 2^2 alignment for synthetic fixtures.
        if not overlapping:
            current = (current + 3) // 4 * 4
        offset = current if not overlapping or i == 0 else 8 + entry_size * len(architectures)
        sz = forced_size if forced_size is not None else len(body)
        if is_64:
            entry = struct.pack('>IIQQII', cpu, 0, offset, sz, 2, 0)
        else:
            entry = struct.pack('>IIIII', cpu, 0, offset, sz, 2)
        contents[8 + i * entry_size:8 + (i + 1) * entry_size] = entry
        if len(contents) < offset + len(body):
            contents.extend(b'\0' * (offset + len(body) - len(contents)))
        contents[offset:offset + len(body)] = body
        if not overlapping:
            current = offset + len(body)
    return bytes(contents)


class UniversalMachOTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.path = Path(tmp.name) / 'test-binary'

    def write(self, data: bytes) -> Path:
        self.path.write_bytes(data)
        return self.path

    def test_thin_macho_accepts_correct_architecture(self):
        self.assertEqual(detect_thin_arch(self.write(thin(ARM64))), 'arm64')
        self.assertEqual(detect_thin_arch(self.write(thin(X64))), 'x86_64')

    def test_rejects_fat_or_corrupted_thin_inputs(self):
        with self.assertRaisesRegex(ValueError, '64-bit Mach-O'):
            detect_thin_arch(self.write(fat([(ARM64, thin(ARM64)), (X64, thin(X64))])))
        with self.assertRaisesRegex(ValueError, 'truncated'):
            detect_thin_arch(self.write(b'\xcf\xfa\xed\xfe'))

    def test_validates_both_fat32_architectures_in_any_order(self):
        p = self.write(fat([(X64, thin(X64)), (ARM64, thin(ARM64))]))
        self.assertEqual(set(validate_universal(p)), {'x86_64', 'arm64'})

    def test_validates_fat64_with_two_architectures(self):
        p = self.write(fat([(ARM64, thin(ARM64)), (X64, thin(X64))], is_64=True))
        self.assertEqual(set(validate_universal(p)), {'x86_64', 'arm64'})

    def test_rejects_single_architecture_and_duplicate_slices(self):
        with self.assertRaisesRegex(ValueError, 'exactly'):
            validate_universal(self.write(fat([(ARM64, thin(ARM64))])))
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            validate_universal(self.write(fat([(ARM64, thin(ARM64)), (ARM64, thin(ARM64))])))

    def test_rejects_thin_image_pretending_to_be_universal(self):
        with self.assertRaisesRegex(ValueError, 'fat Mach-O'):
            validate_universal(self.write(thin(ARM64)))

    def test_rejects_truncated_and_out_of_range_slices(self):
        with self.assertRaisesRegex(ValueError, 'header'):
            validate_universal(self.write(b'\xca\xfe\xba\xbe\0\0\0\2' + b'\0' * 10))
        with self.assertRaisesRegex(ValueError, 'bounds'):
            validate_universal(self.write(fat([(ARM64, thin(ARM64)), (X64, thin(X64))], forced_size=1000)))

    def test_rejects_nested_mismatched_macho_cpu_type(self):
        with self.assertRaisesRegex(ValueError, 'CPU'):
            validate_universal(self.write(fat([(ARM64, thin(X64)), (X64, thin(X64))])))

    def test_rejects_overlapping_architecture_regions(self):
        with self.assertRaisesRegex(ValueError, 'overlap'):
            validate_universal(self.write(fat([(ARM64, thin(ARM64)), (X64, thin(X64))], overlapping=True)))

    def test_rejects_unsupported_cpu_type(self):
        with self.assertRaisesRegex(ValueError, 'unsupported'):
            validate_universal(self.write(fat([(ARM64, thin(ARM64)), (0x01000012, thin(0x01000012))])))

    def test_cli_fail_closed_on_invalid_or_nonexistent_input(self):
        self.assertEqual(main(['--universal', str(self.write(thin(ARM64)))]), 1)
        self.assertEqual(main(['--thin', 'arm64', str(self.path.with_suffix('.missing'))]), 1)
        self.assertEqual(main(['--universal', str(self.write(fat([(ARM64, thin(ARM64)), (X64, thin(X64))])))]), 0)


if __name__ == '__main__':
    unittest.main()
