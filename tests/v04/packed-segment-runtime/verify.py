from pathlib import Path
import subprocess
import sys

out = Path(sys.argv[1])
sentinel = b"preserve\x00sentinel\xff"
paths = [
    out / "packed.bin",
    out / "bad-header.bin",
    out / "bad-lane.bin",
    out / "bad-tail.bin",
]
for path in paths[1:]:
    path.write_bytes(sentinel)

subprocess.run(
    [str(out / "packed-segment-fixture"), *(str(path) for path in paths)],
    check=True,
)

expected_body = bytes((index * 131 + 17) & 0xff for index in range(5003))
expected = b"\x00\x80\xffA" + expected_body
assert paths[0].read_bytes() == expected, "packed output mismatch"
for path in paths[1:]:
    assert path.read_bytes() == sentinel, f"invalid input changed {path.name}"
print("packed-segment-runtime exact-byte and invalid-input checks passed")
