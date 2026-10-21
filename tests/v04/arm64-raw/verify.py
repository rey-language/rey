#!/usr/bin/env python3
import platform
import struct
import subprocess
import sys
from pathlib import Path


MIN_I64 = -(1 << 63)
MAX_I64 = (1 << 63) - 1


def raw_i64_round_trip(value):
    return struct.unpack("<q", struct.pack("<q", value))[0]


def raw_byte_round_trip(value):
    memory = bytearray(1)
    memory[0] = value
    return memory[0]


def separate_mappings_result():
    first = bytearray(8)
    second = bytearray(8)
    struct.pack_into("<q", first, 0, 11)
    struct.pack_into("<q", second, 0, 22)
    return struct.unpack_from("<q", first, 0)[0] + struct.unpack_from("<q", second, 0)[0]


def expected_cases():
    return {
        "zero-initialized": 0,
        "i64-min": raw_i64_round_trip(MIN_I64),
        "i64-max": raw_i64_round_trip(MAX_I64),
        "byte-zero": raw_byte_round_trip(0),
        "byte-127": raw_byte_round_trip(127),
        "byte-128": raw_byte_round_trip(128),
        "byte-255": raw_byte_round_trip(255),
        "offset-access": raw_i64_round_trip(4660),
        "null-equal": 2,
        "null-not-mapping-equal": 0,
        "same-mapping-equal": 2,
        "distinct-mappings-not-equal": 0,
        "separate-mappings": separate_mappings_result(),
        "raw-call-block-param": raw_i64_round_trip(MAX_I64),
        "address-cell-null": 2,
        "address-cell-roundtrip": 2,
        "map-size-zero": None,
        "map-size-negative": None,
        "map-size-too-large": None,
        "offset-null": None,
        "offset-negative": None,
        "offset-overflow": None,
        "load-i64-unaligned": None,
        "store-i64-unaligned": None,
        "store-byte-negative": None,
        "store-byte-overflow": None,
        "load-address-unaligned": None,
        "store-address-unaligned": None,
        "unmap-size-zero": None,
        "unmap-syscall-failure": None,
    }


def need(ok, message):
    if not ok:
        raise SystemExit(message)


def check_macho(path):
    data = path.read_bytes()
    need(len(data) >= 32, path.name + " is too short for a Mach-O header")
    need(data[:4] == b"\xcf\xfa\xed\xfe", path.name + " does not have a 64-bit little-endian Mach-O header")
    cpu_type = struct.unpack_from("<I", data, 4)[0]
    need(cpu_type == 0x0100000C, path.name + " is not an ARM64 Mach-O image")


def check_elf(path):
    data = path.read_bytes()
    need(len(data) >= 64, path.name + " is too short for an ELF64 header")
    need(data[:4] == b"\x7fELF", path.name + " does not have an ELF header")
    need(data[4] == 2 and data[5] == 1 and data[6] == 1, path.name + " is not little-endian ELF64")
    need(struct.unpack_from("<H", data, 16)[0] == 2, path.name + " is not an executable ELF image")
    need(struct.unpack_from("<H", data, 18)[0] == 183, path.name + " is not an AArch64 ELF image")


def run_images(output, cases, target):
    prefix = target + "-"
    for name, expected in cases.items():
        image = output / (prefix + name)
        image.chmod(0o755)
        if target == "darwin":
            signature = subprocess.run(
                ["/usr/bin/codesign", "--verify", "--strict", str(image)],
                capture_output=True,
                text=True,
                timeout=10,
            )
            need(signature.returncode == 0, name + " Darwin codesign failed: " + signature.stderr.strip())
        result = subprocess.run([str(image)], capture_output=True, text=True, timeout=10)
        expected_status = 1 if expected is None else 2
        need(
            result.returncode == expected_status,
            name
            + " "
            + target
            + " returned status "
            + str(result.returncode)
            + ", expected "
            + str(expected_status)
            + (" (checked raw-memory trap)" if expected is None else " (full-width expected value " + str(expected) + ")")
            + "; stderr="
            + result.stderr.strip(),
        )
        print("PASS " + name + " " + target + " status=" + str(result.returncode))


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: verify.py OUTPUT_DIRECTORY")
    output = Path(sys.argv[1])
    cases = expected_cases()
    expected_files = {target + "-" + name for target in ("darwin", "linux") for name in cases}
    actual_files = {path.name for path in output.iterdir() if path.is_file()}
    need(
        actual_files == expected_files,
        "fixture image set mismatch: missing="
        + repr(sorted(expected_files - actual_files))
        + " extra="
        + repr(sorted(actual_files - expected_files)),
    )

    for name in cases:
        check_macho(output / ("darwin-" + name))
        check_elf(output / ("linux-" + name))

    if sys.platform == "darwin" and platform.machine() == "arm64":
        run_images(output, cases, "darwin")
        print("Linux ARM64 ELF images ready: " + str(output))
    elif sys.platform == "linux" and platform.machine() in ("aarch64", "arm64"):
        run_images(output, cases, "linux")
        print("Darwin ARM64 Mach-O images verified: " + str(output))
    else:
        raise SystemExit("raw-memory native execution requires macOS arm64 or Linux arm64")
    print("raw-memory image manifest passed: " + str(len(cases)) + " cases per target")


if __name__ == "__main__":
    main()
