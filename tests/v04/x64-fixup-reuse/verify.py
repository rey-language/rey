#!/usr/bin/env python3
import json
import re
import struct
import sys
from pathlib import Path


def fail(message: str) -> None:
    raise SystemExit(f"FAIL x64 fixup reuse: {message}")


def check_fixture_log(path: Path, names: list[str]) -> None:
    lines = path.read_text().splitlines()
    for name in names:
        marker = f"PASS fixture {name}"
        if lines.count(marker) != 1:
            fail(f"{path.name}: expected exactly one {marker!r}")
    found = [line.removeprefix("PASS fixture ") for line in lines if line.startswith("PASS fixture ")]
    if found != names:
        fail(f"{path.name}: manifest order or fixture set differs: {found}")


def make_elf(code: bytes, offsets_text: str) -> bytes:
    entry_match = re.search(r"(?m)^entry=([0-9]+)$", offsets_text)
    if entry_match is None:
        fail("function-offset report has no entry offset")
    entry_offset = int(entry_match.group(1))
    if not code or entry_offset >= len(code):
        fail("entry offset is outside emitted code")
    code_offset = 4096
    base = 4194304
    file_size = code_offset + len(code)
    header = bytearray(code_offset)
    header[:7] = b"\x7fELF\x02\x01\x01"
    struct.pack_into(
        "<HHIQQQIHHHHHH", header, 16,
        2, 62, 1, base + code_offset + entry_offset, 64, 0, 0,
        64, 56, 2, 0, 0, 0,
    )
    struct.pack_into("<IIQQQQQQ", header, 64, 1, 5, 0, base, base, file_size, file_size, 4096)
    struct.pack_into("<IIQQQQQQ", header, 120, 1685382481, 6, 0, 0, 0, 0, 0, 16)
    return bytes(header) + code


def main() -> None:
    if len(sys.argv) != 4:
        fail("verifier expects manifest, output dir, and current run log")
    manifest_path, output_dir, current_log = map(Path, sys.argv[1:])
    manifest = json.loads(manifest_path.read_text())
    cases = manifest.get("cases")
    if not isinstance(cases, list) or not cases:
        fail("checked-in manifest has no cases")
    names = [case.get("name") for case in cases]
    if any(not isinstance(name, str) or not name for name in names) or len(names) != len(set(names)):
        fail("manifest case names are missing or duplicated")
    check_fixture_log(current_log, names)

    for case in cases:
        name = case["name"]
        control_code_path = output_dir / "control" / f"{name}.code"
        recycled_code_path = output_dir / "recycled" / f"{name}.code"
        control_offsets_path = output_dir / "control" / f"{name}.offsets"
        recycled_offsets_path = output_dir / "recycled" / f"{name}.offsets"
        paths = (control_code_path, recycled_code_path, control_offsets_path, recycled_offsets_path)
        if any(not path.is_file() for path in paths):
            fail(f"{name}: control/recycled code or function-offset file is missing")
        control_code = control_code_path.read_bytes()
        recycled_code = recycled_code_path.read_bytes()
        control_offsets = control_offsets_path.read_text()
        recycled_offsets = recycled_offsets_path.read_text()
        if control_code != recycled_code:
            fail(f"{name}: control and recycled machine code differ")
        if control_offsets != recycled_offsets:
            fail(f"{name}: control and recycled function offsets differ")
        control_elf = make_elf(control_code, control_offsets)
        recycled_elf = make_elf(recycled_code, recycled_offsets)
        if control_elf != recycled_elf:
            fail(f"{name}: control and recycled ELF bytes differ")
        if control_elf[:7] != b"\x7fELF\x02\x01\x01" or control_elf[16:18] != b"\x02\x00" or control_elf[18:20] != b"\x3e\x00":
            fail(f"{name}: generated image is not ELF64 x86-64 ET_EXEC")
        (output_dir / "control" / f"{name}.elf").write_bytes(control_elf)
        (output_dir / "recycled" / f"{name}.elf").write_bytes(recycled_elf)

    stats_line = next((line for line in current_log.read_text().splitlines() if line.startswith("FIXUP_SLOTS ")), None)
    if stats_line is None:
        fail("scaled run did not report fixup-slot metrics")
    stats = {key: int(value) for key, value in re.findall(r"([a-z_]+)=([0-9]+)", stats_line)}
    required = {"control_peak", "recycled_peak", "max_local", "retained_at_function_end", "publication", "trap_publication"}
    if set(stats) != required:
        fail(f"malformed fixup-slot report: {stats_line}")
    if stats["control_peak"] <= stats["recycled_peak"]:
        fail(f"scaled case did not lower peak slots: {stats_line}")
    if stats["max_local"] < 3:
        fail(f"scaled case did not exercise loop/branch local fixups: {stats_line}")
    if stats["retained_at_function_end"] != 0:
        fail(f"resolved local fixups remain after a function: {stats_line}")
    if stats["publication"] < 65:
        fail(f"call relocations were not held through publication: {stats_line}")
    if stats["trap_publication"] <= 1:
        fail(f"trap relocations were not held through publication: {stats_line}")

    print(f"X01 manifest: {len(cases)}/{len(cases)} control/recycled ELF bytes and function offsets match")


if __name__ == "__main__":
    main()
