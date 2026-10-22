#!/usr/bin/env python3
"""Check x64 ABI comment parity and run the platform image fixture.

The fixture checks image headers. It does not execute generated programs.
"""
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
X64 = ROOT / "rey-compiler" / "src" / "native" / "x64" / "main.rey"
PLATFORM = ROOT / "runtime-rey" / "platform" / "main.rey"
REQUIRED = (
    "linux-x64",
    "windows-x64",
    "darwin-x64",
)


def abi_lines(path):
    lines = []
    for line in path.read_text().splitlines():
        if line.startswith("// abi "):
            lines.append(line)
    return lines


def check_comments():
    emitter = abi_lines(X64)
    platform = abi_lines(PLATFORM)
    missing = []
    for name in REQUIRED:
        emitter_row = [line for line in emitter if " " + name + " " in line]
        platform_row = [line for line in platform if " " + name + " " in line]
        if emitter_row != platform_row or len(emitter_row) != 1:
            missing.append(name)
    if missing:
        print("error: x64 ABI comments differ for " + ", ".join(missing), file=sys.stderr)
        return 1
    return 0


def main():
    status = check_comments()
    if status != 0:
        return status
    engine = Path(os.environ.get("REY_ENGINE", ""))
    runtime = Path(os.environ.get("REY_RUNTIME", ""))
    if not engine.is_file() or not runtime.is_dir():
        print("ABI comments match; set REY_ENGINE and REY_RUNTIME to run the image fixture")
        return 0
    stage = Path(tempfile.mkdtemp(prefix="v04-platform-"))
    try:
        shutil.copytree(ROOT / "rey-compiler" / "src", stage / "src")
        shutil.copy(ROOT / "tests" / "v04" / "platforms" / "main.rey", stage / "src" / "main.rey")
        binary = stage / "platform-fixture"
        env = os.environ.copy()
        std = ROOT / "std" / "src"
        if std.is_dir():
            env["REY_STD_PATH"] = str(std)
        built = subprocess.run(
            [str(engine), "build", str(stage / "src" / "main.rey"), "-o", str(binary), "--runtime-dir", str(runtime)],
            cwd=ROOT,
            env=env,
        )
        if built.returncode != 0:
            return built.returncode
        ran = subprocess.run([str(binary)], cwd=ROOT)
        return ran.returncode
    finally:
        shutil.rmtree(stage, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
