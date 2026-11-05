#!/usr/bin/env python3
"""Check symbol-map path collisions without changing input or output files."""
import argparse
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--compiler", type=Path, required=True)
args = parser.parse_args()
compiler = args.compiler.resolve()
work = Path(tempfile.mkdtemp(prefix="rey-v04-symbol-paths-"))
source = work / "source.rey"
output = work / "image"
symbols = work / "symbols"
source_bytes = b"func main(): void {}\n"
source.write_bytes(source_bytes)
output.write_bytes(b"output sentinel")

def run(target: str, symbol_path: str | Path, output_path: str | Path = output) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run([str(compiler), "build", str(source), "-o", str(output_path),
                           "--target", target, "--symbol-map", str(symbol_path)],
                          capture_output=True, timeout=60, check=False)

for path in (source, output, f"{work}/./source.rey", f"{work}/./image"):
    result = run("linux-x64", path)
    if result.returncode != 2 or b"symbol map must differ" not in result.stderr:
        raise SystemExit(f"collision was not rejected: {path}; {result.stderr!r}")
    if source.read_bytes() != source_bytes or output.read_bytes() != b"output sentinel":
        raise SystemExit("collision modified the source or output")
result = run("linux-x64", symbols, f"{work}/./source.rey")
if result.returncode != 2 or b"output must differ" not in result.stderr or source.read_bytes() != source_bytes:
    raise SystemExit("source/output lexical alias was not rejected")
result = run("darwin-arm64", symbols)
if result.returncode != 2 or b"only supported for linux-x64" not in result.stderr:
    raise SystemExit(f"unsupported target was not rejected: {result.stderr!r}")
if source.read_bytes() != source_bytes or output.read_bytes() != b"output sentinel" or symbols.exists():
    raise SystemExit("unsupported target modified files")
result = run("linux-x64", symbols)
if result.returncode != 0 or b"<entry>" not in symbols.read_bytes():
    raise SystemExit(f"separate symbol map failed: {result.stderr!r}")
output.chmod(output.stat().st_mode | 0o100)
if subprocess.run([str(output)], timeout=10, check=False).returncode != 0:
    raise SystemExit("generated executable failed")
print(f"symbol-map paths passed; artifacts: {work}")
