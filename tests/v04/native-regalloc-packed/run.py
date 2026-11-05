#!/usr/bin/env python3
"""Build and execute focused x64/ARM64 register and packed-output fixtures."""
import argparse
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
SOURCE = Path(__file__).with_name("main.rey")
EXPECTED_REGISTER_STDOUT = b"v0.4 native register allocation fixtures passed\n"
EXPECTED_PACKED_STDOUT = b"v0.4 packed output fixture passed\n"
EXPECTED_NATIVE_STDOUT = b"30\n130\n158\n"
EXPECTED_BYTES = bytes((0, 1, 127, 128, 255, 0))


def host_target() -> str:
    system = platform.system()
    machine = platform.machine().lower()
    if system == "Linux" and machine in ("x86_64", "amd64"):
        return "linux-x64"
    if system == "Linux" and machine in ("aarch64", "arm64"):
        return "linux-arm64"
    if system == "Darwin" and machine in ("aarch64", "arm64"):
        return "darwin-arm64"
    raise SystemExit(f"unsupported native fixture host: {system} {machine}")


def run(command: list[str], cwd: Path, env: dict[str, str], stdout_path: Path, stderr_path: Path) -> None:
    try:
        result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, timeout=600, check=False)
    except (OSError, subprocess.TimeoutExpired) as exc:
        stdout_path.write_bytes(getattr(exc, "stdout", None) or b"")
        stderr_path.write_bytes(getattr(exc, "stderr", None) or b"")
        raise SystemExit(f"command failed to start or timed out: {command}; see {stderr_path}: {exc}") from exc
    stdout_path.write_bytes(result.stdout)
    stderr_path.write_bytes(result.stderr)
    if result.returncode != 0:
        raise SystemExit(f"command exited {result.returncode}: {command}; see {stderr_path}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", type=Path, required=True, help="verified native v0.4 compiler executable")
    parser.add_argument("--target", choices=("linux-x64", "linux-arm64", "darwin-arm64"), help="default: native host target")
    args = parser.parse_args()
    compiler = args.compiler.expanduser().resolve()
    if not compiler.is_file() or not os.access(compiler, os.X_OK):
        parser.error(f"compiler is missing or not executable: {compiler}")

    output = Path(tempfile.mkdtemp(prefix="rey-v04-regalloc-packed-", dir="/tmp")).resolve()
    packed_path = output / "packed.bin"
    stages = {}
    for name, fixture_path in (("registers", SOURCE), ("packed", SOURCE.with_name("packed-main.rey"))):
        stage = output / ("compiler-" + name)
        stage.mkdir()
        shutil.copytree(ROOT / "rey-compiler/src", stage, dirs_exist_ok=True)
        fixture = fixture_path.read_text()
        if name == "packed":
            if fixture.count("@@OUTPUT_PATH@@") != 1:
                raise SystemExit("packed fixture must contain exactly one output path marker")
            fixture = fixture.replace("@@OUTPUT_PATH@@", str(packed_path))
        (stage / "main.rey").write_text(fixture)
        stages[name] = stage

    env = dict(os.environ)
    target = args.target or host_target()
    register_executable = output / "register-fixture"
    register_build = [str(compiler), "build", str(stages["registers"] / "main.rey"), "-o", str(register_executable), "--target", target]
    run(register_build, ROOT, env, output / "register-build.stdout", output / "register-build.stderr")
    register_executable.chmod(register_executable.stat().st_mode | 0o111)
    run([str(register_executable)], ROOT, env, output / "register-run.stdout", output / "register-run.stderr")
    if (output / "register-run.stdout").read_bytes() != EXPECTED_REGISTER_STDOUT:
        raise SystemExit(f"register fixture output mismatch; see {output / 'register-run.stdout'}")

    packed_executable = output / "packed-fixture"
    packed_build = [
        str(compiler), "build", str(stages["packed"] / "main.rey"), "-o", str(packed_executable),
        "--target", target, "--regions", "--managed", "--trust-packed-output",
    ]
    run(packed_build, ROOT, env, output / "packed-build.stdout", output / "packed-build.stderr")
    packed_executable.chmod(packed_executable.stat().st_mode | 0o111)
    run([str(packed_executable)], ROOT, env, output / "packed-run.stdout", output / "packed-run.stderr")
    if (output / "packed-run.stdout").read_bytes() != EXPECTED_PACKED_STDOUT:
        raise SystemExit(f"packed fixture output mismatch; see {output / 'packed-run.stdout'}")

    native_executable = output / "native-exec-fixture"
    native_build = [
        str(compiler), "build", str(SOURCE.with_name("native-exec.rey")), "-o", str(native_executable),
        "--target", target, "--managed", "--regions", "--reuse-slots",
    ]
    run(native_build, ROOT, env, output / "native-exec-build.stdout", output / "native-exec-build.stderr")
    native_executable.chmod(native_executable.stat().st_mode | 0o111)
    run([str(native_executable)], ROOT, env, output / "native-exec-run.stdout", output / "native-exec-run.stderr")
    if (output / "native-exec-run.stdout").read_bytes() != EXPECTED_NATIVE_STDOUT:
        raise SystemExit(f"native reuse execution output mismatch; see {output / 'native-exec-run.stdout'}")

    if not packed_path.is_file() or packed_path.read_bytes() != EXPECTED_BYTES:
        raise SystemExit(f"packed binary output mismatch; see {packed_path}")
    print(f"native-regalloc-packed PASS (4 output checks; artifacts: {output})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
