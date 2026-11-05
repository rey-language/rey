#!/usr/bin/env python3
"""Independent structural and SHA-256 oracle for the v0.4 image fixtures."""
from __future__ import annotations

import hashlib
import platform
from pathlib import Path
import struct
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "build/v04/native-image"
PAGE = 16384


def need(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit("FAIL " + message)


def u32le(data: bytes, offset: int) -> int:
    return struct.unpack_from("<I", data, offset)[0]


def macho(path: Path) -> None:
    data = path.read_bytes()
    need(data[:4] == bytes.fromhex("cffaedfe"), "Mach-O magic")
    magic, cpu, subtype, filetype, ncmds, sizeofcmds, flags, reserved = struct.unpack_from("<IiiIIIII", data)
    need((magic, cpu, subtype, filetype, ncmds, sizeofcmds, flags, reserved) ==
         (0xFEEDFACF, 0x0100000C, 0, 2, 9, 472, 2097157, 0), "Mach-O header")
    commands: list[tuple[int, int, int]] = []
    segments: dict[str, tuple[int, int, int, int, int, int]] = {}
    offset = 32
    for _ in range(ncmds):
        command, size = struct.unpack_from("<II", data, offset)
        need(size >= 8 and offset + size <= 32 + sizeofcmds, "load-command bounds")
        commands.append((command, size, offset))
        if command == 0x19:
            name = data[offset + 8:offset + 24].split(b"\0", 1)[0].decode()
            vmaddr, vmsize, fileoff, filesize, maxprot, initprot, nsects, segflags = struct.unpack_from("<QQQQiiII", data, offset + 24)
            need(segflags == 0, "segment flags")
            if name == "__TEXT":
                need(size == 152 and nsects == 1, "text segment command shape")
                section = offset + 72
                sectname = data[section:section + 16].split(b"\0", 1)[0]
                segname = data[section + 16:section + 32].split(b"\0", 1)[0]
                addr, section_size, section_offset, alignment, reloff, nreloc, section_flags, reserved1, reserved2, reserved3 = struct.unpack_from("<QQIIIIIIII", data, section + 32)
                need((sectname, segname, addr, section_size, section_offset, alignment,
                      reloff, nreloc, section_flags, reserved1, reserved2, reserved3) ==
                     (b"__text", b"__TEXT", 0x100001000, 12, 4096, 2, 0, 0, 0x80000400, 0, 0, 0),
                     "__TEXT,__text section")
            else:
                need(size == 72 and nsects == 0, "segment command shape")
            segments[name] = (vmaddr, vmsize, fileoff, filesize, maxprot, initprot)
        offset += size
    need(offset == 32 + sizeofcmds, "load-command range")
    need([c for c, _, _ in commands] == [0x19, 0x19, 0x19, 27, 14, 12, 0x80000028, 0x32, 0x1D], "command sequence")
    need(segments["__PAGEZERO"] == (0, 0x100000000, 0, 0, 0, 0), "pagezero segment")
    text = segments["__TEXT"]
    linkedit = segments["__LINKEDIT"]
    need(text[0] == 0x100000000 and text[1] == text[3] and text[2] == 0 and text[3] % PAGE == 0,
         "text segment layout")
    need(text[4:] == (5, 5) and linkedit[0] == text[0] + text[1] and
         linkedit[2] == text[3] and linkedit[3] == len(data) - linkedit[2] and
         linkedit[4:] == (1, 1) and linkedit[1] % PAGE == 0, "linkedit layout")
    uuid = next(item for item in commands if item[0] == 27)
    _, uuid_size, uuid_offset = uuid
    need(uuid_size == 24 and any(data[uuid_offset + 8:uuid_offset + 24]), "LC_UUID deterministic id")
    dylinker = next(item for item in commands if item[0] == 14)
    _, dylinker_size, dylinker_offset = dylinker
    linker_name_offset = u32le(data, dylinker_offset + 8)
    need(dylinker_size == 32 and linker_name_offset == 12 and
         data[dylinker_offset + linker_name_offset:dylinker_offset + dylinker_size].split(b"\0", 1)[0] == b"/usr/lib/dyld",
         "LC_LOAD_DYLINKER path")
    dylib = next(item for item in commands if item[0] == 12)
    _, dylib_size, dylib_offset = dylib
    dylib_name_offset = u32le(data, dylib_offset + 8)
    need(dylib_size == 56 and dylib_name_offset == 24 and
         data[dylib_offset + dylib_name_offset:dylib_offset + dylib_size].split(b"\0", 1)[0] == b"/usr/lib/libSystem.B.dylib",
         "libSystem startup dependency")
    main_command = next(item for item in commands if item[0] == 0x80000028)
    _, main_size, main_offset = main_command
    entryoff, stacksize = struct.unpack_from("<QQ", data, main_offset + 8)
    need((main_size, entryoff, stacksize) == (24, 4096, 0), "LC_MAIN file offset")
    need(data[4096:4108] == bytes.fromhex("400580d2300080d2011000d4"), "Darwin return-42 instructions")

    build_command = next(item for item in commands if item[0] == 0x32)
    _, build_size, build_offset = build_command
    need(build_size == 24 and struct.unpack_from("<IIII", data, build_offset + 8) ==
         (1, 1769472, 1769472, 0), "macOS LC_BUILD_VERSION")
    sig_command = next(item for item in commands if item[0] == 0x1D)
    _, _, sig_cmd_offset = sig_command
    sig_offset, sig_size = struct.unpack_from("<II", data, sig_cmd_offset + 8)
    need((sig_offset, sig_size) == (linkedit[2], linkedit[3]), "LC_CODE_SIGNATURE range")
    signature = data[sig_offset:sig_offset + sig_size]
    super_magic, super_size, count, slot, cd_offset = struct.unpack_from(">IIIII", signature)
    need((super_magic, super_size, count, slot, cd_offset) == (0xFADE0CC0, sig_size, 1, 0, 20), "SuperBlob header")
    cd_magic, cd_size, version, cd_flags, hash_offset, ident_offset, special, slots, code_limit = struct.unpack_from(">9I", signature, cd_offset)
    hash_size, hash_type, _, page_log = struct.unpack_from(">4B", signature, cd_offset + 36)
    spare2, scatter, team, spare3, code_limit64, exec_base, exec_limit, exec_flags = struct.unpack_from(">IIIIQQQQ", signature, cd_offset + 40)
    need((cd_magic, cd_size, version, cd_flags, special, code_limit, hash_size, hash_type, page_log) ==
         (0xFADE0C02, sig_size - 20, 0x20400, 0x20002, 0, text[3], 32, 2, 12), "CodeDirectory fields")
    need((spare2, scatter, team, spare3, code_limit64, exec_base, exec_limit, exec_flags) ==
         (0, 0, 0, 0, 0, 0, 4108, 1), "CodeDirectory executable segment")
    need(slots == text[3] // 4096 and ident_offset == 88 and hash_offset == 99,
         "CodeDirectory slot offsets")
    ident = signature[cd_offset + ident_offset:cd_offset + hash_offset]
    need(ident == b"rey-native\0", "CodeDirectory identifier")
    for page in range(slots):
        hash_page = 4096
        expected = hashlib.sha256(data[page * hash_page:(page + 1) * hash_page]).digest()
        actual = signature[cd_offset + hash_offset + page * hash_size:
                            cd_offset + hash_offset + (page + 1) * hash_size]
        need(actual == expected, f"CodeDirectory SHA-256 page {page}")

    if platform.system() == "Darwin" and platform.machine() == "arm64":
        path.chmod(0o755)
        result = subprocess.run(["/usr/bin/codesign", "--verify", "--strict", str(path)], capture_output=True)
        need(result.returncode == 0, "codesign --verify --strict: " + result.stderr.decode(errors="replace"))
        result = subprocess.run(["/usr/bin/otool", "-l", str(path)], capture_output=True)
        need(result.returncode == 0 and all(x in result.stdout for x in
             (b"__PAGEZERO", b"__TEXT", b"__LINKEDIT", b"LC_MAIN", b"LC_LOAD_DYLINKER", b"LC_CODE_SIGNATURE")), "otool load-command inspection")
        result = subprocess.run([str(path)], capture_output=True)
        need(result.returncode == 42, f"Mach-O exit status was {result.returncode}, expected 42")
        print("Darwin Mach-O execution and signature: PASS")
    else:
        print("Darwin Mach-O host execution/signature gate: SKIP (requires Darwin arm64)")


def elf(path: Path) -> None:
    data = path.read_bytes()
    need(data[:6] == b"\x7fELF\x02\x01", "ELF64 little-endian magic")
    fields = struct.unpack_from("<16sHHIQQQIHHHHHH", data)
    ident, kind, machine, version, entry, phoff, shoff, flags, ehsize, phentsize, phnum, shentsize, shnum, shstrndx = fields
    need((kind, machine, version, entry, phoff, shoff, flags, ehsize, phentsize, phnum, shentsize, shnum, shstrndx) ==
         (2, 183, 1, 0x401000, 64, 0, 0, 64, 56, 1, 0, 0, 0), "ELF header")
    p_type, p_flags, p_offset, p_vaddr, p_paddr, p_filesz, p_memsz, p_align = struct.unpack_from("<IIQQQQQQ", data, phoff)
    need((p_type, p_flags, p_offset, p_vaddr, p_paddr, p_filesz, p_memsz, p_align) ==
         (1, 5, 0, 0x400000, 0x400000, len(data), len(data), 4096), "ELF RX PT_LOAD")
    need(entry == p_vaddr + 4096, "ELF entry points at codeFileOffset within PT_LOAD")
    need((p_offset - p_vaddr) % p_align == 0 and data[4096:4112] == bytes.fromhex("400580d2a80b80d2010000d4"),
         "ELF load alignment and Linux exit-42 instructions")
    print("ELF64 AArch64 structure: PASS (execution not run without an AArch64 Linux runner)")


def main() -> None:
    macho(OUT / "return42-uuid.macho")
    elf(OUT / "return42-uuid.elf")
    for path in sorted(OUT.glob("return42-uuid.*")):
        print(f"{path.name} sha256={hashlib.sha256(path.read_bytes()).hexdigest()}")


if __name__ == "__main__":
    main()
