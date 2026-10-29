#!/usr/bin/env python3
"""Fail-closed structural check for macOS 64-bit thin / Universal 2 binaries.

This does not execute code, verify a code signature, validate a source SHA,
or substitute for physical Intel/Apple Silicon device tests.
"""
from __future__ import annotations

import argparse
import struct
import sys
from pathlib import Path

ARM64_CPU = 0x0100000C
X86_64_CPU = 0x01000007
ARCHES = {ARM64_CPU: 'arm64', X86_64_CPU: 'x86_64'}
MACHO_64_LE = b'\xcf\xfa\xed\xfe'
FAT_MAGIC = b'\xca\xfe\xba\xbe'
FAT_MAGIC_64 = b'\xca\xfe\xba\xbf'


def _thin_cpu(handle, offset: int, length: int) -> int:
    if length < 32:
        raise ValueError('truncated 64-bit Mach-O slice')
    handle.seek(offset)
    header = handle.read(32)
    if len(header) != 32 or header[:4] != MACHO_64_LE:
        raise ValueError('expected little-endian 64-bit Mach-O slice')
    return struct.unpack('<I', header[4:8])[0]


def detect_thin_arch(path: Path) -> str:
    """Return architecture of a genuine 64-bit thin Mach-O header."""
    path = Path(path)
    with path.open('rb') as handle:
        cpu = _thin_cpu(handle, 0, path.stat().st_size)
    if cpu not in ARCHES:
        raise ValueError(f'unsupported 64-bit Mach-O CPU type 0x{cpu:08x}')
    return ARCHES[cpu]


def validate_universal(path: Path) -> dict[str, tuple[int, int]]:
    """Ensure exactly two non-overlapping arm64/x86_64 slices of correct type."""
    path = Path(path)
    size = path.stat().st_size
    with path.open('rb') as handle:
        header = handle.read(8)
        if len(header) != 8 or header[:4] not in (FAT_MAGIC, FAT_MAGIC_64):
            raise ValueError('expected fat Mach-O Universal 2 header')
        is_64 = header[:4] == FAT_MAGIC_64
        count = struct.unpack('>I', header[4:8])[0]
        entry_size = 32 if is_64 else 20
        if count == 0 or count > 16:
            raise ValueError('invalid fat Mach-O architecture count')
        header_end = 8 + count * entry_size
        if size < header_end:
            raise ValueError('truncated fat Mach-O architecture header')
        table = handle.read(count * entry_size)
        if len(table) != count * entry_size:
            raise ValueError('truncated fat Mach-O architecture header')
        slices: dict[str, tuple[int, int]] = {}
        for index in range(count):
            pos = index * entry_size
            if is_64:
                cpu, _subtype, offset, length, align, _reserved = struct.unpack_from('>IIQQII', table, pos)
            else:
                cpu, _subtype, offset, length, align = struct.unpack_from('>IIIII', table, pos)
            if cpu not in ARCHES:
                raise ValueError(f'unsupported Universal 2 CPU type 0x{cpu:08x}')
            arch = ARCHES[cpu]
            if arch in slices:
                raise ValueError(f'duplicate Universal 2 architecture: {arch}')
            if (align > 28 or offset < header_end or length < 32 or
                    offset > size or length > size - offset or offset % (1 << align) != 0):
                raise ValueError(f'{arch} slice out of bounds, invalid alignment or too small')
            slices[arch] = (offset, length)
        ordered = sorted(slices.items(), key=lambda kv: kv[1][0])
        for (_, (start, length)), (_, (next_start, _)) in zip(ordered, ordered[1:]):
            if start + length > next_start:
                raise ValueError('overlapping Universal 2 slices')
        if set(slices) != {'arm64', 'x86_64'}:
            raise ValueError('Universal 2 must contain exactly arm64 and x86_64')
        for arch, (offset, length) in slices.items():
            cpu = _thin_cpu(handle, offset, length)
            if ARCHES.get(cpu) != arch:
                raise ValueError(f'{arch} inner Mach-O CPU does not match fat architecture table')
    return slices


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    exclusive = parser.add_mutually_exclusive_group(required=True)
    exclusive.add_argument('--thin', nargs=2, metavar=('ARCH', 'FILE'))
    exclusive.add_argument('--universal', metavar='FILE', type=Path)
    args = parser.parse_args(argv)
    try:
        if args.thin is not None:
            expected, file_path = args.thin
            if expected not in ARCHES.values():
                raise ValueError('thin architecture must be arm64 or x86_64')
            actual = detect_thin_arch(Path(file_path))
            if expected != actual:
                raise ValueError(f'expected thin {expected}, got {actual}: {file_path}')
            print(f'Thin Mach-O {actual} header verified: {file_path}')
        else:
            slices = validate_universal(args.universal)
            print(f'Universal 2 Mach-O headers verified: {args.universal} ({", ".join(sorted(slices))})')
        return 0
    except (OSError, ValueError) as exc:
        print(f'Mach-O architecture rejected: {exc}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
