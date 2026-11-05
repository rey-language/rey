#!/usr/bin/env python3
import platform
import struct
import subprocess
import sys
from pathlib import Path


CASES = {
    "valid": 0,
    "capacity-negative": 1001,
    "capacity-over-id-range": 1001,
    "descriptor-size-overflow": 1001,
    "payload-negative": 1002,
    "reference-count-negative": 1002,
    "null-reference-offsets": 1002,
    "duplicate-offsets": 1002,
    "unsorted-offsets": 1002,
    "negative-offset": 1002,
    "unaligned-offset": 1002,
    "out-of-bounds-offset": 1002,
    "reference-count-exceeds-payload": 1002,
    "id-negative": 1003,
    "id-zero": 1003,
    "id-too-large": 1003,
    "index-negative": 1004,
    "index-too-large": 1004,
    "full-registry": 1005,
    "zero-capacity-full": 1005,
    "corrupt-magic": 1006,
    "corrupt-capacity": 1006,
    "corrupt-count": 1006,
    "corrupt-length": 1006,
    "corrupt-reserved": 1006,
    "corrupt-descriptor-payload": 1006,
    "corrupt-descriptor-count": 1006,
    "corrupt-descriptor-reserved": 1006,
    "corrupt-descriptor-offset": 1006,
    "corrupt-duplicate-slot": 1006,
}


def need(condition, message):
    if not condition:
        raise SystemExit(message)


def check_macho(path):
    data = path.read_bytes()
    need(len(data) >= 32, path.name + " is too short for Mach-O64")
    need(data[:4] == b"\xcf\xfa\xed\xfe", path.name + " is not little-endian Mach-O64")
    need(struct.unpack_from("<I", data, 4)[0] == 0x0100000C, path.name + " is not ARM64 Mach-O")


def check_elf(path):
    data = path.read_bytes()
    need(len(data) >= 64, path.name + " is too short for ELF64")
    need(data[:4] == b"\x7fELF", path.name + " does not have an ELF header")
    need(data[4:7] == b"\x02\x01\x01", path.name + " is not little-endian ELF64")
    need(struct.unpack_from("<H", data, 16)[0] == 2, path.name + " is not an executable ELF image")
    need(struct.unpack_from("<H", data, 18)[0] == 183, path.name + " is not an AArch64 ELF image")


def run_darwin(output, name, expected):
    image = output / ("darwin-" + name)
    image.chmod(0o755)
    signature = subprocess.run(
        ["/usr/bin/codesign", "--verify", "--strict", str(image)],
        capture_output=True,
        text=True,
        timeout=10,
    )
    need(signature.returncode == 0, name + " codesign failed: " + signature.stderr.strip())
    result = subprocess.run([str(image)], capture_output=True, text=True, timeout=20)
    need(result.stdout == "", name + " wrote unexpected native stdout: " + result.stdout.strip())
    need(
        result.returncode == (0 if expected == 0 else 1),
        name + " returned status " + str(result.returncode) + ", expected " + str(0 if expected == 0 else 1) + "; stderr=" + result.stderr.strip(),
    )
    print("PASS " + name + " darwin status=" + str(result.returncode))


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: verify.py OUTPUT_DIRECTORY")
    output = Path(sys.argv[1])
    expected_files = {target + "-" + name for target in ("darwin", "linux") for name in CASES}
    actual_files = {path.name for path in output.iterdir() if path.is_file()}
    need(
        actual_files == expected_files,
        "image set mismatch: missing=" + repr(sorted(expected_files - actual_files)) + " extra=" + repr(sorted(actual_files - expected_files)),
    )
    for name in CASES:
        check_macho(output / ("darwin-" + name))
        check_elf(output / ("linux-" + name))
    need(sys.platform == "darwin" and platform.machine() == "arm64", "Darwin ARM64 codesign and execution gate requires macOS arm64")
    for name, expected in CASES.items():
        run_darwin(output, name, expected)
    print("layout registry image manifest passed: " + str(len(CASES)) + " cases per target")
    print("Linux ARM64 images are ready for the root Docker gate: " + str(output))


if __name__ == "__main__":
    main()
