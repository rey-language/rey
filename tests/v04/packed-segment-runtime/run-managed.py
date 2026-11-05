#!/usr/bin/env python3
"""Compile a managed segment writer and check that the trusted hook is used."""
import argparse
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--compiler", type=Path, required=True)
args = parser.parse_args()
compiler = args.compiler.resolve()
work = Path(tempfile.mkdtemp(prefix="rey-v04-packed-segment-managed-"))
src = work / "src"
shutil.copytree(ROOT / "rey-compiler/src", src)
shutil.copyfile(Path(__file__).with_name("managed-main.rey"), src / "main.rey")
image = work / "fixture"
symbols = work / "fixture.map"
command = [str(compiler), "build", str(src / "main.rey"), "-o", str(image),
           "--target", "linux-x64", "--managed", "--regions", "--trust-packed-output",
           "--symbol-map", str(symbols)]
result = subprocess.run(command, cwd=ROOT, capture_output=True, timeout=600, check=False)
(work / "build.stdout").write_bytes(result.stdout)
(work / "build.stderr").write_bytes(result.stderr)
if result.returncode != 0:
    raise SystemExit(f"managed fixture build failed ({result.returncode}); see {work}")
if "managedWritePackedSegments" not in symbols.read_text():
    raise SystemExit(f"trusted segment hook was not linked; see {symbols}")
image.chmod(image.stat().st_mode | 0o100)
for mode in ("valid", "header", "lane", "tail"):
    path = work / (mode + ".bin")
    path.write_bytes(b"untouched sentinel")
    result = subprocess.run([str(image), str(path), mode], capture_output=True, timeout=120, check=False)
    (work / (mode + ".stderr")).write_bytes(result.stderr)
    if result.returncode != 0:
        raise SystemExit(f"{mode} returned {result.returncode}; see {work}")
    expected = bytes((0, 127, 128, 255)) + bytes((i * 131 + 7) % 256 for i in range(5003)) if mode == "valid" else b"untouched sentinel"
    if path.read_bytes() != expected:
        raise SystemExit(f"{mode} output mismatch; see {path}")
if Path("/dev/full").exists():
    result = subprocess.run([str(image), "/dev/full", "valid"], capture_output=True, timeout=120, check=False)
    if result.returncode == 0:
        raise SystemExit("full device unexpectedly accepted packed output")
print(f"managed packed segment writer passed; artifacts: {work}")
