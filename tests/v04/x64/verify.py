#!/usr/bin/env python3
"""Decode the emitter's intentionally small x86-64 instruction subset."""
import struct
import sys
from pathlib import Path


def read_i32(data, offset):
    if offset + 4 > len(data):
        raise AssertionError("truncated rel32/displacement at " + str(offset))
    return struct.unpack_from("<i", data, offset)[0]


def decode(data):
    offset = 0
    instructions = []
    while offset < len(data):
        start = offset
        first = data[offset]
        offset += 1
        if first in (0x55, 0xC9, 0xC3):
            name = {0x55: "push", 0xC9: "leave", 0xC3: "ret"}[first]
        elif first in (0xE8, 0xE9):
            displacement = read_i32(data, offset)
            offset += 4
            target = offset + displacement
            instructions.append((start, "call" if first == 0xE8 else "jmp", target))
            continue
        elif first == 0x31:
            if offset >= len(data) or data[offset] != 0xFF:
                raise AssertionError("unknown xor at " + str(start))
            offset += 1
            name = "xor"
        elif first == 0x89:
            if offset >= len(data) or data[offset] != 0xC7:
                raise AssertionError("unknown mov at " + str(start))
            offset += 1
            name = "mov"
        elif 0xB8 <= first <= 0xBF:
            if offset + 4 > len(data):
                raise AssertionError("truncated imm32 at " + str(start))
            offset += 4
            name = "mov32"
        elif first == 0x0F:
            if offset >= len(data):
                raise AssertionError("truncated 0f opcode")
            second = data[offset]
            offset += 1
            if second == 0x05:
                name = "syscall"
            elif 0x80 <= second <= 0x8F:
                displacement = read_i32(data, offset)
                offset += 4
                instructions.append((start, "jcc", offset + displacement, second))
                continue
            elif second in (0x94, 0x95, 0x9C, 0x9D, 0x9E, 0x9F):
                if offset >= len(data) or data[offset] != 0xC0:
                    raise AssertionError("unknown setcc ModRM")
                offset += 1
                name = "setcc"
            elif second == 0xB6:
                if offset >= len(data) or data[offset] != 0xC0:
                    raise AssertionError("unknown movzx ModRM")
                offset += 1
                name = "movzx"
            else:
                raise AssertionError("unknown 0f opcode %02x at %d" % (second, start))
        elif first in (0x48, 0x49, 0x4C):
            if offset >= len(data):
                raise AssertionError("truncated REX opcode")
            opcode = data[offset]
            offset += 1
            if 0xB8 <= opcode <= 0xBF:
                if offset + 8 > len(data):
                    raise AssertionError("truncated imm64 at " + str(start))
                offset += 8
                name = "mov64"
            elif opcode == 0x81:
                if offset >= len(data) or data[offset] not in (0xEC, 0xC4):
                    raise AssertionError("unknown stack adjustment")
                offset += 1
                if offset + 4 > len(data):
                    raise AssertionError("truncated stack size")
                offset += 4
                name = "stack"
            elif opcode in (0x89, 0x8B):
                if offset >= len(data):
                    raise AssertionError("truncated ModRM")
                modrm = data[offset]
                offset += 1
                mod, rm = modrm >> 6, modrm & 7
                if mod == 2:
                    if rm == 4:
                        if offset >= len(data):
                            raise AssertionError("truncated SIB")
                        offset += 1
                    offset += 4
                elif mod == 1:
                    offset += 1
                elif mod == 0 and rm == 4:
                    if offset >= len(data):
                        raise AssertionError("truncated SIB")
                    offset += 1
                name = "mov"
            elif opcode in (0x01, 0x29, 0x39, 0x85):
                if offset >= len(data):
                    raise AssertionError("truncated register ModRM")
                offset += 1
                name = {0x01: "add", 0x29: "sub", 0x39: "cmp", 0x85: "test"}[opcode]
            elif opcode == 0xC1:
                if offset + 2 > len(data):
                    raise AssertionError("truncated shift")
                offset += 2
                name = "shift"
            elif opcode == 0xF7:
                if offset >= len(data):
                    raise AssertionError("truncated F7 ModRM")
                modrm = data[offset]
                offset += 1
                if modrm not in (0xE9, 0xF9):
                    raise AssertionError("unsupported F7 ModRM %02x" % modrm)
                name = "imul" if modrm == 0xE9 else "idiv"
            elif opcode == 0x83:
                if offset + 2 > len(data) or data[offset] != 0xF9 or data[offset + 1] != 0xFF:
                    raise AssertionError("unknown cmp imm8")
                offset += 2
                name = "cmp"
            elif opcode == 0x99:
                name = "cqo"
            else:
                raise AssertionError("unknown REX opcode %02x at %d" % (opcode, start))
        else:
            raise AssertionError("unknown opcode %02x at %d" % (first, start))
        if offset > len(data):
            raise AssertionError("instruction at %d crosses end of code" % start)
        instructions.append((start, name, None))
    return instructions


def read_map(path):
    result = {"functions": []}
    for line in path.read_text().splitlines():
        key, value = line.split("=", 1)
        if key == "function":
            function_id, offset = value.split(",", 1)
            result["functions"].append((int(function_id), int(offset)))
        else:
            result[key] = int(value)
    return result


def verify(name, data, mapping):
    decoded = decode(data)
    starts = {item[0] for item in decoded}
    functions = dict(mapping["functions"])
    assert mapping["entry"] == 0
    assert len(functions) == len(mapping["functions"])
    assert all(offset in starts for offset in functions.values())
    assert all(data[offset:offset + 4] == b"\x55\x48\x89\xe5" for offset in functions.values())
    branches = [item for item in decoded if item[1] in ("call", "jmp", "jcc")]
    for item in branches:
        assert 0 <= item[2] < len(data), (name, item)
        assert item[2] in starts, (name, "branch into an instruction", item)
    calls = [item[2] for item in decoded if item[1] == "call"]
    assert calls and all(target in functions.values() for target in calls)
    assert data[mapping["overflow"]:] == b"\xbf\x01\x00\x00\x00\xb8\x3c\x00\x00\x00\x0f\x05"
    names = [item[1] for item in decoded]
    assert "syscall" in names and "leave" in names and "ret" in names
    if name == "scalar":
        assert "setcc" in names and "movzx" in names
        assert set(functions) == {0, 1, 2, 3, 4}
        assert all(name in names for name in ("add", "sub", "imul", "idiv", "cqo", "test", "shift"))
        jcc_ops = [item[3] for item in decoded if item[1] == "jcc"]
        assert 0x80 in jcc_ops and 0x84 in jcc_ops and 0x85 in jcc_ops
        assert data[functions[1]:functions[1] + 4] == b"\x55\x48\x89\xe5"
        assert calls.count(functions[2]) == 2  # entry call plus factorial self-recursion
        assert b"\x49\xba\x00\x00\x00\x00\x00\x00\x00\x80" in data
        # A dividend other than MIN skips only the MIN/-1 guard, then divides.
        # It must not jump to the shared arithmetic-error handler.
        minimum_check = bytes.fromhex("49ba00000000000000804c39d00f85")
        guards = []
        start = 0
        while (start := data.find(minimum_check, start)) >= 0:
            branch = start + 13
            target = branch + 6 + read_i32(data, branch + 2)
            assert target == start + 29 and data[target:target + 2] == b"\x48\x99"
            guards.append(start)
            start += len(minimum_check)
        assert len(guards) == 2
        # The sum8 function receives its seventh and eighth arguments from rbp+16/+24.
        assert b"\x48\x8b\x85\x10\x00\x00\x00" in data
        assert b"\x48\x8b\x85\x18\x00\x00\x00" in data
        # The entry call uses both stack argument slots under the SysV ABI.
        assert b"\x48\x89\x84\x24\x00\x00\x00\x00" in data
        assert b"\x48\x89\x84\x24\x08\x00\x00\x00" in data
    else:
        assert len(functions) == 1
        assert b"\x31\xff" in data[:functions[0]]
    print("PASS", name, "bytes=" + str(len(data)), "decoded=" + str(len(decoded)))


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: verify.py FIXTURE_DIRECTORY")
    directory = Path(sys.argv[1])
    expected = {"scalar", "void"}
    actual = {path.stem for path in directory.iterdir() if path.suffix == ".bin"}
    if actual != expected:
        raise SystemExit("fixture set mismatch: expected %r got %r" % (sorted(expected), sorted(actual)))
    for name in sorted(expected):
        data = (directory / (name + ".bin")).read_bytes()
        verify(name, data, read_map(directory / (name + ".map")))
    print("x64 structural fixtures passed: 2")


if __name__ == "__main__":
    main()
