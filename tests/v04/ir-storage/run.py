#!/usr/bin/env python3
"""Build and run the independent conservative stack-slot planner fixtures."""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile
import time
from typing import Any


ROOT = Path(__file__).resolve().parents[3]
SEED = ROOT / "build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME = SEED.parent / "runtime"
BASELINE = ROOT / "build/v04/seed/baseline-seed-winruntime/result.json"
OUTPUT_BASE = ROOT / "build/v04/ir-storage"
EXPECTED_SEED = "89f41b7cac5f38018c32b74d7a134d4d32e1314d58ddf292efcea174d182a921"
EXPECTED_RUNTIME = "f3d625762a0ea9dfc403e81b5c3cc8cce49aaec64bd276e5b2dec95fad666667"
EXPECTED_RUNTIME_TREE = "4b7e15928dcc6a5df4f776393074907d577b1ac64c248f65f8276dd18715f40a"
EXPECTED_FIXTURE_LINE = "v0.4 conservative storage planner fixtures passed"


class GateError(RuntimeError):
    pass


def digest_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def file_hash(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def tree_hashes(root: Path) -> dict[str, str]:
    return {
        path.relative_to(root).as_posix(): file_hash(path)
        for path in sorted(root.rglob("*"))
        if path.is_file() and not path.is_symlink()
    }


def runtime_tree_hash(hashes: dict[str, str]) -> str:
    payload = "".join(f"{name}\0{value}\n" for name, value in hashes.items()).encode()
    return digest_bytes(payload)


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="milliseconds")


def create_output(value: str | None) -> Path:
    OUTPUT_BASE.mkdir(parents=True, exist_ok=True)
    base = OUTPUT_BASE.resolve()
    if value:
        output = Path(value).expanduser()
        if not output.is_absolute():
            output = ROOT / output
        output = output.resolve()
        try:
            output.relative_to(base)
        except ValueError as exc:
            raise ValueError(f"output directory must be under {base}") from exc
        if output.exists():
            raise ValueError(f"output directory must be new: {output}")
        output.mkdir(parents=True)
        return output
    return Path(tempfile.mkdtemp(prefix="run-", dir=base))


def controlled_environment(output: Path) -> dict[str, str]:
    home = output / "home"
    temporary = output / "tmp"
    home.mkdir()
    temporary.mkdir()
    std_link = home / ".reyc/std/src"
    std_link.parent.mkdir(parents=True)
    std_link.symlink_to(ROOT / "std/src", target_is_directory=True)
    return {
        "PATH": "/usr/bin:/bin:/usr/sbin:/sbin",
        "HOME": str(home),
        "TMPDIR": str(temporary),
        "LC_ALL": "C",
    }


def run_process(
    argv: list[str], cwd: Path, env: dict[str, str], output: Path,
    name: str, timeout: float = 300,
) -> dict[str, Any]:
    started_at = utc_now()
    started = time.monotonic()
    stdout_path = output / f"{name}.stdout"
    stderr_path = output / f"{name}.stderr"
    status: int | None = None
    error: str | None = None
    try:
        completed = subprocess.run(
            argv, cwd=cwd, env=env, capture_output=True, timeout=timeout, check=False
        )
        status = completed.returncode
        stdout_path.write_bytes(completed.stdout)
        stderr_path.write_bytes(completed.stderr)
    except (OSError, subprocess.TimeoutExpired) as exc:
        error = str(exc)
        stdout_path.write_bytes(getattr(exc, "stdout", None) or b"")
        stderr_path.write_bytes(getattr(exc, "stderr", None) or b"")
    elapsed = round(time.monotonic() - started, 6)
    stdout = stdout_path.read_bytes()
    stderr = stderr_path.read_bytes()
    return {
        "command": argv,
        "cwd": str(cwd),
        "environment": dict(env),
        "started_at": started_at,
        "elapsed_seconds": elapsed,
        "status": status,
        "error": error,
        "stdout": {
            "path": str(stdout_path.relative_to(output)),
            "bytes": len(stdout),
            "sha256": digest_bytes(stdout),
        },
        "stderr": {
            "path": str(stderr_path.relative_to(output)),
            "bytes": len(stderr),
            "sha256": digest_bytes(stderr),
        },
    }


def require_process_ok(record: dict[str, Any], name: str) -> None:
    if record["error"] or record["status"] != 0:
        raise GateError(f"{name} failed: {record}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", help="new output path under build/v04/ir-storage")
    args = parser.parse_args()
    output = create_output(args.output_dir)
    report: dict[str, Any] = {
        "schema": "rey-v04-ir-storage-v1",
        "started_at": utc_now(),
        "status": "FAIL",
        "output_dir": str(output),
        "host": {
            "system": platform.system(),
            "release": platform.release(),
            "machine": platform.machine(),
            "python": platform.python_version(),
        },
        "processes": {},
        "evidence": {},
    }
    try:
        if not SEED.is_file() or not os.access(SEED, os.X_OK):
            raise GateError(f"pinned v0.3.1 seed is missing or not executable: {SEED}")
        if not RUNTIME.is_dir() or not (RUNTIME / "librey_rt.a").is_file():
            raise GateError(f"matching seed runtime is missing: {RUNTIME}")
        if not BASELINE.is_file():
            raise GateError(f"verified seed baseline is missing: {BASELINE}")

        seed_hash = file_hash(SEED)
        runtime_files = tree_hashes(RUNTIME)
        runtime_hash = runtime_files.get("librey_rt.a")
        runtime_tree = runtime_tree_hash(runtime_files)
        baseline_bytes = BASELINE.read_bytes()
        baseline = json.loads(baseline_bytes)
        baseline_provenance = baseline.get("provenance", {})
        baseline_runtime_files = baseline_provenance.get("runtime_files_sha256", {})
        baseline_runtime_tree = runtime_tree_hash(baseline_runtime_files) if isinstance(baseline_runtime_files, dict) else None
        report["evidence"].update({
            "seed": str(SEED.relative_to(ROOT)),
            "seed_sha256": seed_hash,
            "runtime": str(RUNTIME.relative_to(ROOT)),
            "runtime_files_sha256": runtime_files,
            "runtime_archive_sha256": runtime_hash,
            "runtime_tree_sha256": runtime_tree,
            "baseline": str(BASELINE.relative_to(ROOT)),
            "baseline_sha256": digest_bytes(baseline_bytes),
            "baseline_status": baseline.get("status"),
            "baseline_runtime_tree_sha256": baseline_runtime_tree,
        })
        if seed_hash != EXPECTED_SEED or runtime_hash != EXPECTED_RUNTIME or runtime_tree != EXPECTED_RUNTIME_TREE:
            raise GateError("pinned seed or runtime hash differs from the v0.4 baseline")
        if (
            baseline.get("status") != "PASS"
            or baseline_provenance.get("engine_sha256") != EXPECTED_SEED
            or baseline_runtime_files != runtime_files
            or baseline_runtime_tree != EXPECTED_RUNTIME_TREE
        ):
            raise GateError("seed/runtime hashes do not match the recorded passing baseline")

        env = controlled_environment(output)
        version = run_process([str(SEED), "--version"], ROOT, env, output, "version")
        report["processes"]["version"] = version
        require_process_ok(version, "seed version check")
        version_text = (output / version["stdout"]["path"]).read_text(encoding="ascii")
        if version_text != "rey 0.3.1\n":
            raise GateError(f"pinned seed reported unexpected version: {version_text!r}")
        report["evidence"]["seed_version"] = version_text.strip()

        stage = output / "staged-compiler"
        shutil.copytree(ROOT / "rey-compiler/src", stage / "src")
        harness = Path(__file__).with_name("main.rey")
        shutil.copyfile(harness, stage / "src/main.rey")
        staged_hashes = {
            path.relative_to(stage).as_posix(): file_hash(path)
            for path in sorted((stage / "src").rglob("*.rey"))
        }
        report["evidence"]["harness_sha256"] = file_hash(harness)
        safe_source = Path(__file__).with_name("safe.rey").resolve()
        report["evidence"]["safe_source_fixture"] = str(safe_source.relative_to(ROOT))
        report["evidence"]["safe_source_fixture_sha256"] = file_hash(safe_source)
        report["evidence"]["staged_source_sha256"] = staged_hashes
        executable = stage / "ir-storage-fixtures"
        build = run_process(
            [str(SEED), "build", str(stage / "src/main.rey"), "-o", str(executable),
             "--runtime-dir", str(RUNTIME)],
            ROOT, env, output, "build", timeout=600,
        )
        report["processes"]["build"] = build
        require_process_ok(build, "staged compiler build")
        if not executable.is_file():
            raise GateError("staged build returned success but its executable is missing")
        report["evidence"]["fixture_executable_sha256"] = file_hash(executable)

        execution = run_process([str(executable), str(safe_source)], ROOT, env, output, "fixtures", timeout=300)
        report["processes"]["fixtures"] = execution
        require_process_ok(execution, "fixture execution")
        stdout = (output / execution["stdout"]["path"]).read_text(encoding="utf-8")
        stderr = (output / execution["stderr"]["path"]).read_bytes()
        if stdout != EXPECTED_FIXTURE_LINE + "\n" or stderr:
            raise GateError(f"fixture process evidence differs: stdout={stdout!r}, stderr={stderr!r}")
        report["status"] = "PASS"
        report["finished_at"] = utc_now()
        (output / "result.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
        print(f"ir-storage: PASS (independent oracle, conflict fixtures, bounded-work checks)")
        print(f"report: {output / 'result.json'}")
        return 0
    except (OSError, ValueError, KeyError, json.JSONDecodeError, GateError) as exc:
        report["error"] = str(exc)
        report["finished_at"] = utc_now()
        (output / "result.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
        raise SystemExit(f"ir-storage: FAIL: {exc}")


if __name__ == "__main__":
    raise SystemExit(main())
