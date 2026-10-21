#!/usr/bin/env python3
import platform
import struct
import subprocess
import sys
from pathlib import Path


EXPECTED = {
    "fail-zero": 1,
    "fail-17": 1,
    "fail-negative": 1,
    "fail-max": 1,
    "fail-before-99": 1,
    "conditional-skip": 0,
}


def need(ok, message):
    if not ok:
        raise SystemExit(message)


def check_macho(path):
    data = path.read_bytes()
    need(len(data) >= 32, path.name + " is too short for a Mach-O header")
    need(data[:4] == b"\xcf\xfa\xed\xfe", path.name + " is not little-endian Mach-O64")
    need(struct.unpack_from("<I", data, 4)[0] == 0x0100000C, path.name + " is not ARM64 Mach-O")


def check_elf(path):
    data = path.read_bytes()
    need(len(data) >= 64, path.name + " is too short for an ELF64 header")
    need(data[:4] == b"\x7fELF", path.name + " does not have an ELF header")
    need(data[4:7] == b"\x02\x01\x01", path.name + " is not little-endian ELF64")
    need(struct.unpack_from("<H", data, 16)[0] == 2, path.name + " is not ET_EXEC")
    need(struct.unpack_from("<H", data, 18)[0] == 183, path.name + " is not AArch64 ELF")


def run_images(output):
    for name, expected in EXPECTED.items():
        image = output / ("darwin-" + name)
        image.chmod(0o755)
        signature = subprocess.run(
            ["/usr/bin/codesign", "--verify", "--strict", str(image)],
            capture_output=True,
            text=True,
            timeout=10,
        )
        need(signature.returncode == 0, name + " codesign failed: " + signature.stderr.strip())
        result = subprocess.run([str(image)], capture_output=True, text=True, timeout=10)
        need(
            result.returncode == expected,
            name + " returned status " + str(result.returncode) + ", expected " + str(expected) + "; stderr=" + result.stderr.strip(),
        )
        print("PASS " + name + " Darwin status=" + str(result.returncode))


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: verify.py OUTPUT_DIRECTORY")
    output = Path(sys.argv[1])
    expected_files = {target + "-" + name for target in ("darwin", "linux") for name in EXPECTED}
    actual_files = {path.name for path in output.iterdir() if path.is_file()}
    need(
        actual_files == expected_files,
        "image set mismatch: missing=" + repr(sorted(expected_files - actual_files)) + " extra=" + repr(sorted(actual_files - expected_files)),
    )
    for name in EXPECTED:
        check_macho(output / ("darwin-" + name))
        check_elf(output / ("linux-" + name))
    need(sys.platform == "darwin" and platform.machine() == "arm64", "RawFail native verification requires macOS arm64")
    run_images(output)
    print("raw fail image manifest passed: " + str(len(EXPECTED)) + " cases per target")


if __name__ == "__main__":
    main()
