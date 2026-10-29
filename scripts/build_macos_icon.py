#!/usr/bin/env python3
"""Generate an original layered-crop PicPaste icon with stdlib-only PNG-in-ICNS.

No third-party fonts, vector renderers, GUI session, network or dependencies.
The PNG resolutions cover 16/32/64/128/256/512/1024 physical pixels.
"""
from __future__ import annotations

import argparse
import math
import struct
import zlib
from pathlib import Path

ICON_SLOTS = {
    b"icp4": 16,
    b"icp5": 32,
    b"icp6": 64,
    b"ic07": 128,
    b"ic08": 256,
    b"ic09": 512,
    b"ic10": 1024,
}


def _round_rect(x: float, y: float, bounds: tuple[int, int, int, int],
                radius: float, pixel: float) -> float:
    left, top, right, bottom = bounds
    cx, cy = (left + right) / 2, (top + bottom) / 2
    bx, by = (right - left) / 2 - radius, (bottom - top) / 2 - radius
    qx, qy = abs(x - cx) - bx, abs(y - cy) - by
    distance = math.hypot(max(qx, 0), max(qy, 0)) + min(max(qx, qy), 0) - radius
    return max(0.0, min(1.0, 0.5 - distance / pixel))


def _merge(base: tuple[float, float, float], overlay: tuple[int, int, int],
           alpha: float) -> tuple[float, float, float]:
    return tuple(a * (1.0 - alpha) + b * alpha for a, b in zip(base, overlay))


def _pixel(x: float, y: float, pixel: float) -> bytes:
    clip = _round_rect(x, y, (24, 24, 1000, 1000), 220, pixel)
    if clip <= 0.0:
        return b"\x00\x00\x00\x00"
    gradient = max(0.0, min(1.0, (x + 0.6 * y) / 1640.0))
    rgb: tuple[float, float, float] = (
        54 - 13 * gradient, 109 + 12 * gradient, 137 - 6 * gradient
    )
    # The rear translucent excerpt, the foreground window and crop brackets.
    rgb = _merge(rgb, (29, 71, 92),
                 _round_rect(x + 14, y + 14, (278, 275, 861, 843), 88, pixel) * .24)
    rgb = _merge(rgb, (174, 208, 201),
                 _round_rect(x, y, (280, 266, 848, 834), 91, pixel))
    rgb = _merge(rgb, (242, 232, 215),
                 _round_rect(x, y, (161, 164, 720, 725), 96, pixel))
    rgb = _merge(rgb, (221, 168, 135),
                 _round_rect(x, y, (248, 246, 636, 639), 36, pixel))
    rgb = _merge(rgb, (249, 240, 221),
                 _round_rect(x, y, (277, 274, 607, 611), 19, pixel))
    corners = (
        (300, 299, 365, 319), (300, 299, 320, 369),
        (539, 299, 584, 319), (564, 299, 584, 369),
        (300, 516, 320, 586), (300, 566, 365, 586),
        (539, 566, 584, 586), (564, 516, 584, 586),
    )
    crop = max(_round_rect(x, y, bounds, 6, pixel) for bounds in corners)
    rgb = _merge(rgb, (52, 107, 127), crop)
    # A warm muted pin makes the second card visually distinct.
    pin = max(0.0, min(1.0, .5 - (math.hypot(x - 738, y - 470) - 50) / pixel))
    stem = _round_rect(x, y, (731, 514, 746, 660), 6, pixel)
    rgb = _merge(rgb, (211, 135, 102), max(pin, stem))
    return bytes((*(max(0, min(255, round(c))) for c in rgb), round(clip * 255)))


def _chunk(kind: bytes, payload: bytes) -> bytes:
    crc = zlib.crc32(kind + payload) & 0xffffffff
    return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", crc)


def make_png(size: int) -> bytes:
    if size not in ICON_SLOTS.values():
        raise ValueError("unsupported ICNS image size")
    step = 1024.0 / size
    raw = bytearray()
    for y in range(size):
        raw.append(0)  # PNG filter None
        for x in range(size):
            raw.extend(_pixel((x + .5) * step, (y + .5) * step, step))
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n" + _chunk(b"IHDR", ihdr) +
            _chunk(b"IDAT", zlib.compress(raw, level=9)) + _chunk(b"IEND", b""))


def build_icns() -> bytes:
    payload = b"".join(
        tag + struct.pack(">I", len(png) + 8) + png
        for tag, size in ICON_SLOTS.items()
        for png in (make_png(size),)
    )
    return b"icns" + struct.pack(">I", len(payload) + 8) + payload


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(build_icns())
    print(f"Generated deterministic multi-resolution PicPaste icon: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
