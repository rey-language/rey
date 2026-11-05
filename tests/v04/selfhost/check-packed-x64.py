#!/usr/bin/env python3
"""Compare complete Linux x64 images from ordinary and packed emission."""
import argparse
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--compiler", type=Path, required=True)
args = parser.parse_args()
compiler = args.compiler.resolve()
work = Path(tempfile.mkdtemp(prefix="rey-v04-packed-x64-cli-"))
source = work / "input.rey"
source.write_text('func main(): void {\n    var value = "\\x00\\x80\\xff";\n    if value.len() != 3 { var bad = 9223372036854775807 + 1; }\n}\n')
for mode in ("ordinary", "packed"):
    image = work / mode
    symbols = work / (mode + ".map")
    command = [str(compiler), "build", str(source), "-o", str(image), "--target", "linux-x64",
               "--managed", "--regions", "--trust-packed-output", "--selfhost-fast-exit",
               "--symbol-map", str(symbols)]
    if mode == "packed":
        command.append("--experimental-packed-x64")
    result = subprocess.run(command, capture_output=True, timeout=900, check=False)
    (work / (mode + ".stdout")).write_bytes(result.stdout)
    (work / (mode + ".stderr")).write_bytes(result.stderr)
    if result.returncode != 0:
        raise SystemExit(f"{mode} build returned {result.returncode}; see {work}")
    image.chmod(image.stat().st_mode | 0o100)
    if subprocess.run([str(image)], timeout=30, check=False).returncode != 0:
        raise SystemExit(f"{mode} image failed; see {work}")
if (work / "ordinary").read_bytes() != (work / "packed").read_bytes():
    raise SystemExit(f"generated images differ; see {work}")
if (work / "ordinary.map").read_bytes() != (work / "packed.map").read_bytes():
    raise SystemExit(f"symbol maps differ; see {work}")
print(f"packed x64 CLI passed; artifacts: {work}")
