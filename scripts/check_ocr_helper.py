#!/usr/bin/env python3
"""CI-only real-process IPC smoke tests for the offline macOS Vision helper."""
import hashlib
import json
import struct
import subprocess
import sys


def parse_framed_reply(stdout: bytes) -> dict:
    """Decode precisely the SHOC length-prefixed JSON frame.

    Apple Vision/CoreML may emit diagnostics *after* its valid frame when
    executing under a hosted macOS VM. Rust read_response() also reads only
    the declared frame length, so the native smoke test must match that
    protocol rather than assuming the complete process stdout ends at EOF.
    Prefixes, oversized or truncated frames remain invalid.
    """
    if len(stdout) < 4:
        raise AssertionError("No length-prefixed helper response")
    declared = struct.unpack("<I", stdout[:4])[0]
    if declared == 0 or declared > 8 * 1024 * 1024 or len(stdout) < declared + 4:
        raise AssertionError(
            f"Invalid or truncated Vision helper response frame: "
            f"declared={declared}, stdout_len={len(stdout)}, "
            f"head_hex={stdout[:64].hex()}"
        )
    return json.loads(stdout[4 : 4 + declared].decode("utf-8"))


def call(helper: str, data: bytes, *, expected_success: bool, args=()) -> dict:
    proc = subprocess.run(
        [helper, *args], input=data, capture_output=True, timeout=25, check=False
    )
    if expected_success != (proc.returncode == 0):
        raise AssertionError(
            f"Unexpected OCR helper status {proc.returncode}: "
            f"{proc.stderr.decode('utf-8', errors='replace')}"
        )
    result = parse_framed_reply(proc.stdout)
    declared = struct.unpack("<I", proc.stdout[:4])[0]
    if len(proc.stdout) > declared + 4:
        # This occurs with Apple Vision's VM/CPU diagnostics *after* the valid
        # first frame. Never accept a prefix before the header or short data.
        print(
            f"Vision helper {args} emitted "
            f"{len(proc.stdout) - declared - 4} post-frame diagnostic bytes",
            file=sys.stderr,
        )
    if result["protocol_version"] != 1:
        raise AssertionError("Unexpected protocol version")
    return result


def main(helper: str) -> None:
    # Every standalone helper advertises its signed API/SHOC compatibility
    # without OCR, pixels, network, or any extra status text.
    capability = subprocess.run(
        [helper, "--addon-info"], input=b"", capture_output=True,
        timeout=5, check=False
    )
    assert capability.returncode == 0, capability.stderr
    assert capability.stdout == b"PICPASTE-OCR-ADDON/1 SHOC/1\\n", capability.stdout

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
    # Swift JSONEncoder omits optional fields set to nil; absence is valid.
    assert qr_scan.get("document") is None
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
