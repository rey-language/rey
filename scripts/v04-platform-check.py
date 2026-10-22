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


def check_native_targets(engine, runtime, env):
    """Build the native driver and verify cross-target images structurally."""
    stage = Path(tempfile.mkdtemp(prefix="v04-native-targets-"))
    try:
        shutil.copytree(ROOT / "rey-compiler" / "src", stage / "src")
        shutil.copy(ROOT / "rey-compiler" / "native-main.rey", stage / "src" / "main.rey")
        driver = stage / "rey-native"
        subprocess.run(
            [str(engine), "build", str(stage / "src" / "main.rey"), "-o", str(driver), "--runtime-dir", str(runtime)],
            cwd=ROOT,
            env=env,
            check=True,
        )
        source = stage / "scalar.rey"
        source.write_text("func add(a: int, b: int): int { return a + b; }\nfunc main(): void { var value: int = add(3, 4); return; }\n")
        outputs = {}
        for target in ("linux-x64", "windows-x64", "darwin-x64", "macos-x64"):
            output = stage / ("scalar-" + target)
            subprocess.run(
                [str(driver), "build", str(source), "-o", str(output), "--target", target],
                cwd=ROOT,
                env=env,
                check=True,
            )
            outputs[target] = output.read_bytes()
        if outputs["linux-x64"][:4] != b"\x7fELF" or int.from_bytes(outputs["linux-x64"][18:20], "little") != 62:
            raise SystemExit("linux-x64 driver output is not an ELF64 x86-64 image")
        windows = outputs["windows-x64"]
        if windows[:2] != b"MZ" or windows[0x80:0x84] != b"PE\0\0" or int.from_bytes(windows[0x84:0x86], "little") != 0x8664:
            raise SystemExit("windows-x64 driver output is not a PE32+ x86-64 image")
        for name in ("darwin-x64", "macos-x64"):
            image = outputs[name]
            if image[:4] != b"\xcf\xfa\xed\xfe" or int.from_bytes(image[4:8], "little") != 0x01000007:
                raise SystemExit(name + " driver output is not a Mach-O x86-64 image")
        if outputs["darwin-x64"] != outputs["macos-x64"]:
            raise SystemExit("macos-x64 is not an exact alias for darwin-x64")
        print("native driver cross-target images passed")
    finally:
        shutil.rmtree(stage, ignore_errors=True)


def main():
    status = check_comments()
    if status != 0:
        return status
    engine = Path(os.environ.get("REY_ENGINE", ""))
    runtime = Path(os.environ.get("REY_RUNTIME", ""))
    if not engine.is_file() or not runtime.is_dir():
        print("ABI comments match; set REY_ENGINE and REY_RUNTIME to run the image fixture")
        return 0
    env = os.environ.copy()
    std = ROOT / "std" / "src"
    if std.is_dir():
        env["REY_STD_PATH"] = str(std)
    stage = Path(tempfile.mkdtemp(prefix="v04-platform-"))
    try:
        shutil.copytree(ROOT / "rey-compiler" / "src", stage / "src")
        shutil.copy(ROOT / "tests" / "v04" / "platforms" / "main.rey", stage / "src" / "main.rey")
        binary = stage / "platform-fixture"
        subprocess.run(
            [str(engine), "build", str(stage / "src" / "main.rey"), "-o", str(binary), "--runtime-dir", str(runtime)],
            cwd=ROOT,
            env=env,
            check=True,
        )
        ran = subprocess.run([str(binary)], cwd=ROOT)
        if ran.returncode != 0:
            return ran.returncode
    finally:
        shutil.rmtree(stage, ignore_errors=True)
    return check_native_targets(engine, runtime, env)


if __name__ == "__main__":
    sys.exit(main())
