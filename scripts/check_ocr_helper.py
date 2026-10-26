#!/usr/bin/env python3
"""CI-only real-process IPC smoke tests for the offline macOS Vision helper."""
import hashlib
import json
import struct
import subprocess
import sys


def call(helper: str, data: bytes, *, expected_success: bool) -> dict:
    proc = subprocess.run(
        [helper], input=data, capture_output=True, timeout=25, check=False
    )
    if expected_success != (proc.returncode == 0):
        raise AssertionError(
            f"Unexpected OCR helper status {proc.returncode}: "
            f"{proc.stderr.decode('utf-8', errors='replace')}"
        )
    if len(proc.stdout) < 4:
        raise AssertionError("No length-prefixed helper response")
    declared = struct.unpack("<I", proc.stdout[:4])[0]
    if declared == 0 or declared > 8 * 1024 * 1024 or len(proc.stdout) != declared + 4:
        raise AssertionError("Invalid response length prefix")
    result = json.loads(proc.stdout[4:].decode("utf-8"))
    if result["protocol_version"] != 1:
        raise AssertionError("Unexpected protocol version")
    return result


def main(helper: str) -> None:
    invalid = b"NOPE" + struct.pack("<III", 1, 2, 2) + bytes(16)
    answer = call(helper, invalid, expected_success=False)
    assert answer["error"], answer
    oversized = b"SHOC" + struct.pack("<III", 1, 65535, 65535)
    answer = call(helper, oversized, expected_success=False)
    assert answer["error"], answer

    width, height = 256, 64
    rgba = bytes([255, 255, 255, 255]) * (width * height)
    request = b"SHOC" + struct.pack("<III", 1, width, height) + rgba
    answer = call(helper, request, expected_success=True)
    document = answer["document"]
    assert document["provider"] == "apple-vision", document
    assert (document["pixel_width"], document["pixel_height"]) == (width, height)
    assert document["image_hash"] == hashlib.sha256(rgba).hexdigest()
    assert isinstance(document["lines"], list)
    print("Vision helper SHOC round trip: success")


if __name__ == "__main__":
    main(sys.argv[1])
