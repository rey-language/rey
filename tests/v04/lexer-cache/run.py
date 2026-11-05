#!/usr/bin/env python3
"""Build and run the lexer cache regression fixture with a native compiler."""
import argparse
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
SOURCE = Path(__file__).with_name("main.rey")
EXPECTED = Path(__file__).with_name("expected.stdout")
TARGETS = ("linux-x64", "linux-arm64", "darwin-arm64")


def host_target() -> str:
    system = platform.system()
    machine = platform.machine().lower()
    if system == "Linux" and machine in ("x86_64", "amd64"):
        return "linux-x64"
    if system == "Linux" and machine in ("aarch64", "arm64"):
        return "linux-arm64"
    if system == "Darwin" and machine in ("aarch64", "arm64"):
        return "darwin-arm64"
    raise SystemExit(f"unsupported lexer-cache fixture host: {system} {machine}")


def run(command: list[str], cwd: Path, env: dict[str, str], stdout_path: Path, stderr_path: Path, timeout: int) -> None:
    try:
        result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, timeout=timeout, check=False)
    except (OSError, subprocess.TimeoutExpired) as exc:
        stdout_path.write_bytes(getattr(exc, "stdout", None) or b"")
        stderr_path.write_bytes(getattr(exc, "stderr", None) or b"")
        raise SystemExit(f"command failed to start or timed out: {command}; artifacts: {cwd}: {exc}") from exc
    stdout_path.write_bytes(result.stdout)
    stderr_path.write_bytes(result.stderr)
    if result.returncode != 0:
        raise SystemExit(f"command exited {result.returncode}: {command}; artifacts: {cwd}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", type=Path, required=True, help="native v0.4 compiler executable")
    parser.add_argument("--target", choices=TARGETS, help="default: supported native host target")
    args = parser.parse_args()
    compiler = args.compiler.expanduser().resolve()
    if not compiler.is_file() or not os.access(compiler, os.X_OK):
        parser.error(f"compiler is missing or not executable: {compiler}")
    target = args.target or host_target()

    artifacts = Path(tempfile.mkdtemp(prefix="rey-v04-lexer-cache-")).resolve()
    (artifacts / "lexer").mkdir()
    shutil.copyfile(ROOT / "rey-compiler/src/lexer/main.rey", artifacts / "lexer/main.rey")
    shutil.copyfile(SOURCE, artifacts / "main.rey")

    env = dict(os.environ)
    executable = artifacts / "lexer-cache-fixture"
    build = [
        str(compiler), "build", str(artifacts / "main.rey"), "-o", str(executable),
        "--target", target, "--managed", "--regions",
    ]
    run(build, artifacts, env, artifacts / "build.stdout", artifacts / "build.stderr", timeout=600)
    executable.chmod(executable.stat().st_mode | 0o111)
    run([str(executable)], artifacts, env, artifacts / "actual.stdout", artifacts / "run.stderr", timeout=60)

    expected = EXPECTED.read_bytes()
    actual = (artifacts / "actual.stdout").read_bytes()
    if actual != expected:
        offset = next((i for i, (want, got) in enumerate(zip(expected, actual)) if want != got), min(len(expected), len(actual)))
        raise SystemExit(
            f"lexer-cache stdout mismatch at byte {offset} "
            f"(expected {len(expected)} bytes, got {len(actual)}); artifacts: {artifacts}"
        )

    print(f"lexer-cache PASS (target {target}; artifacts: {artifacts})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
