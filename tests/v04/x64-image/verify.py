#!/usr/bin/env python3
"""Independent ELF layout checks; execution is a separate Linux-host gate."""
import struct
import sys
from pathlib import Path


def verify(path):
    data = Path(path).read_bytes()
    assert data[:16] == b"\x7fELF\x02\x01\x01" + bytes(9)
    header = struct.unpack_from("<HHIQQQIHHHHHH", data, 16)
    assert header == (2, 62, 1, 0x401001, 64, 0, 0, 64, 56, 2, 0, 0, 0), header
    assert struct.unpack_from("<IIQQQQQQ", data, 64) == (1, 5, 0, 0x400000, 0x400000, len(data), len(data), 4096)
    assert struct.unpack_from("<IIQQQQQQ", data, 120) == (0x6474E551, 6, 0, 0, 0, 0, 0, 16)
    assert data[176:4096] == bytes(4096 - 176)
    assert data[4096:] == bytes.fromhex("90bf2a000000b83c0000000f05")
    print("x64 ELF structure passed")


if __name__ == "__main__":
    verify(sys.argv[1])
