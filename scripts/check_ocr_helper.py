#!/usr/bin/env python3
"""CI-only real-process IPC smoke tests for the offline macOS Vision helper."""
import hashlib
import json
import struct
import subprocess
import sys


def call(helper: str, data: bytes, *, expected_success: bool, args=()) -> dict:
    proc = subprocess.run(
        [helper, *args], input=data, capture_output=True, timeout=25, check=False
    )
    if expected_success != (proc.returncode == 0):
        raise AssertionError(
            f"Unexpected OCR helper status {proc.returncode}: "
            f"{proc.stderr.decode('utf-8', errors='replace')}"
        )
    if len(proc.stdout) < 4:
        raise AssertionError(
            f"No length-prefixed helper response for {args}: rc={proc.returncode}, "
            f"stdout_len={len(proc.stdout)}, "
            f"stderr={proc.stderr[:1000]!r}"
        )
    declared = struct.unpack("<I", proc.stdout[:4])[0]
    if declared == 0 or declared > 8 * 1024 * 1024 or len(proc.stdout) != declared + 4:
        raise AssertionError(
            f"Invalid Vision helper response frame for {args}: rc={proc.returncode}, "
            f"declared={declared}, stdout_len={len(proc.stdout)}, "
            f"head_hex={proc.stdout[:96].hex()}, "
            f"tail_hex={proc.stdout[-48:].hex()}, "
            f"stderr={proc.stderr[:1000]!r}"
        )
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
    for line in document["lines"]:
        assert isinstance(line["words"], list)

    characters = call(helper, request, expected_success=True, args=("--characters",))
    assert characters["document"]["image_hash"] == document["image_hash"]
    for line in characters["document"]["lines"]:
        assert isinstance(line["words"], list)

    qr_scan = call(helper, request, expected_success=True, args=("--qr",))
    assert qr_scan["document"] is None
    assert qr_scan["qr_document"]["provider"] == "apple-vision-qr"
    assert qr_scan["qr_document"]["image_hash"] == document["image_hash"]
    assert qr_scan["qr_document"]["codes"] == []

    bad_arg = call(helper, request, expected_success=False, args=("--unknown",))
    assert bad_arg["error"], bad_arg
    # Positive sample: CoreImage produces a local QR code (no network).
    generated = subprocess.run(
        ["swift", "scripts/generate_qr_fixture.swift"],
        capture_output=True,
        timeout=90,
        check=False,
    )
    if generated.returncode != 0:
        raise AssertionError(
            "Could not generate QR fixture: "
            + generated.stderr.decode("utf-8", errors="replace")
        )
    decoded = call(helper, generated.stdout, expected_success=True, args=("--qr",))
    payloads = [code["payload"] for code in decoded["qr_document"]["codes"]]
    assert payloads == ["PICPASTE-QR-OFFLINE-TEST"], payloads
    print("Vision Helper: text/character protocol and native offline QR decode passed")


if __name__ == "__main__":
    main(sys.argv[1])
