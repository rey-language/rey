#!/usr/bin/env python3
import platform
import struct
import subprocess
import sys
from pathlib import Path


CASES = {
    "valid": 0,
    "negative-create": 1,
    "negative-grow": 1,
    "negative-length": 1,
    "length-over-capacity": 1,
    "corrupt-magic": 1,
    "corrupt-length": 1,
    "corrupt-reserved": 1,
    "corrupt-null-data": 1,
}


def need(condition, message):
    if not condition:
        raise SystemExit(message)


def check_macho(path):
    data = path.read_bytes()
    need(len(data) >= 32, path.name + " is too short for a Mach-O header")
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
    need(signature.returncode == 0, name + " Darwin codesign failed: " + signature.stderr.strip())
    result = subprocess.run([str(image)], capture_output=True, text=True, timeout=10)
    need(result.stdout == "", name + " wrote unexpected native stdout: " + result.stdout.strip())
    need(
        result.returncode == expected,
        name + " returned status " + str(result.returncode) + ", expected " + str(expected) + "; stderr=" + result.stderr.strip(),
    )
    print("PASS " + name + " darwin status=" + str(result.returncode))


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: verify.py OUTPUT_DIRECTORY")
    output = Path(sys.argv[1])
    for name, expected in CASES.items():
        check_macho(output / ("darwin-" + name))
        check_elf(output / ("linux-" + name))
    if sys.platform == "darwin" and platform.machine() == "arm64":
        for name, expected in CASES.items():
            run_darwin(output, name, expected)
    else:
        print("Darwin ARM64 images are ready for host execution")
    print("Linux ARM64 images are ready for the root Docker gate: " + str(output))


if __name__ == "__main__":
    main()
