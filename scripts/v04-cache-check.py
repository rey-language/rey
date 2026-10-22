#!/usr/bin/env python3
"""Run the v0.4 incremental-cache fixture. Does not claim a timing result."""
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ENGINE = Path(os.environ.get("REY_ENGINE", ""))
RUNTIME = Path(os.environ.get("REY_RUNTIME", ""))


def fail(message):
    print("error: " + message, file=sys.stderr)
    return 2


def main():
    if not ENGINE.is_file() or not RUNTIME.is_dir():
        return fail("set REY_ENGINE and REY_RUNTIME; this script does not build the compiler")
    stage = Path(tempfile.mkdtemp(prefix="v04-cache-"))
    try:
        shutil.copytree(ROOT / "rey-compiler" / "src", stage / "src")
        shutil.copy(ROOT / "tests" / "v04" / "cache" / "main.rey", stage / "src" / "main.rey")
        binary = stage / "cache-fixture"
        store = stage / "store"
        store.mkdir()
        env = os.environ.copy()
        std = ROOT / "std" / "src"
        if std.is_dir():
            env["REY_STD_PATH"] = str(std)
        built = subprocess.run(
            [str(ENGINE), "build", str(stage / "src" / "main.rey"), "-o", str(binary), "--runtime-dir", str(RUNTIME)],
            cwd=ROOT,
            env=env,
        )
        if built.returncode != 0:
            return built.returncode
        ran = subprocess.run([str(binary), str(store)], cwd=ROOT)
        return ran.returncode
    finally:
        shutil.rmtree(stage, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
