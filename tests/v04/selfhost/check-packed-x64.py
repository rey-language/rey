#!/usr/bin/env python3
"""Compare complete Linux x64 images from ordinary and packed emission."""
import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--compiler", type=Path, required=True)
args = parser.parse_args()
compiler = args.compiler.resolve()
if os.environ.get("REY_PACKED_X64_BUDGETED") != "1":
    root = Path(__file__).resolve().parents[3]
    env = os.environ.copy()
    env["REY_PACKED_X64_BUDGETED"] = "1"
    label = "O01-packed-x64-" + str(os.getpid())
    result = subprocess.run(["sh", str(root / "scripts/v04-linux-x64-budget.sh"), "job",
                             "--seconds", "1200", "--label", label, "--",
                             sys.executable, "-B", str(Path(__file__).resolve()),
                             "--compiler", str(compiler)], env=env, check=False)
    raise SystemExit(result.returncode)
ROOT = Path(__file__).resolve().parents[3]
BUDGET = ROOT / "scripts/v04-linux-x64-budget.sh"
CAP = os.environ.get("REY_CGROUP_CAP")
if not CAP or not CAP.endswith("G"):
    raise SystemExit("packed x64 checks require the B01 cgroup memory cap")


def build(mode, command):
    label = "O01-packed-x64-" + mode + "-" + str(os.getpid())
    base = ["sh", str(BUDGET), "run-held", "--seconds", "900", "--label", label, "--"]
    capped = ["--scope", "--wait", "--quiet", "-p", "MemoryMax=" + CAP,
              "-p", "MemorySwapMax=512M", "--", "nice", "-n", "19", "timeout",
              "--kill-after=30s", "900", *command]
    result = subprocess.run(base + ["systemd-run", "--user", *capped], capture_output=True,
                            timeout=930, check=False, pass_fds=(3,))
    if result.returncode != 0 and b"Failed to connect to bus" in result.stderr:
        result = subprocess.run(base + ["sudo", "-n", "/usr/bin/systemd-run", "--system", *capped],
                                capture_output=True, timeout=930, check=False, pass_fds=(3,))
    return result


work = Path(tempfile.mkdtemp(prefix="rey-v04-packed-x64-cli-"))
source = work / "input.rey"
source.write_text('func main(): void {\n    var value = "\\x00\\x80\\xff";\n    if value.len() != 3 { var bad = 9223372036854775807 + 1; }\n}\n')
for mode in ("ordinary", "packed"):
    image = work / mode
    symbols = work / (mode + ".map")
    command = [str(compiler), "build", str(source), "-o", str(image), "--target", "linux-x64",
               "--managed", "--regions", "--symbol-map", str(symbols)]
    if mode == "packed":
        command.append("--packed-x64")
    result = build(mode, command)
    (work / (mode + ".stdout")).write_bytes(result.stdout)
    (work / (mode + ".stderr")).write_bytes(result.stderr)
    if result.returncode != 0:
        raise SystemExit(f"{mode} build returned {result.returncode}; see {work}")
    if image.stat().st_mode & 0o111 == 0o111:
        pass
    else:
        raise SystemExit(f"{mode} image lacks executable mode; see {work}")
    if subprocess.run([str(image)], timeout=30, check=False).returncode != 0:
        raise SystemExit(f"{mode} image failed; see {work}")
if (work / "ordinary").read_bytes() != (work / "packed").read_bytes():
    raise SystemExit(f"generated images differ; see {work}")
if (work / "ordinary.map").read_bytes() != (work / "packed.map").read_bytes():
    raise SystemExit(f"symbol maps differ; see {work}")
print(f"packed x64 CLI passed; artifacts: {work}")
