#!/usr/bin/env python3
"""Probe the pinned v0.3 seed's binary and typed-source capabilities."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import pathlib
import platform
import re
import shutil
import signal
import subprocess
import sys
import time
from typing import Any


REPO = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_ENGINE = REPO / "build/rey"
DEFAULT_RUNTIME = REPO / "build/runtime"
FIXTURES = REPO / "tests/v04/seed/fixtures"
EXPECTED = REPO / "tests/v04/seed/expected"
EXPECTED_MANIFEST = REPO / "tests/v04/seed/expected.json"
EXPECTED_VERSION = "0.3.1"


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="milliseconds")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def file_hash(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def tree_hashes(root: pathlib.Path) -> dict[str, str]:
    hashes: dict[str, str] = {}
    for path in sorted(root.rglob("*")):
        if path.is_file() and not path.is_symlink():
            hashes[path.relative_to(root).as_posix()] = file_hash(path)
    return hashes


def resolve_executable(value: str | None) -> pathlib.Path:
    candidate = pathlib.Path(value).expanduser() if value else DEFAULT_ENGINE
    try:
        resolved = candidate.resolve(strict=True)
    except FileNotFoundError as error:
        raise ValueError(f"engine executable does not exist: {candidate}") from error
    if not resolved.is_file() or not os.access(resolved, os.X_OK):
        raise ValueError(f"engine is not an executable file: {resolved}")
    return resolved


def resolve_runtime(value: str | None) -> pathlib.Path:
    candidate = pathlib.Path(value).expanduser() if value else DEFAULT_RUNTIME
    try:
        resolved = candidate.resolve(strict=True)
    except FileNotFoundError as error:
        raise ValueError(f"runtime directory does not exist: {candidate}") from error
    if not resolved.is_dir() or not (resolved / "librey_rt.a").is_file():
        raise ValueError(f"runtime directory must contain librey_rt.a: {resolved}")
    return resolved


def prepare_output(value: str | None, stem: str) -> pathlib.Path:
    base = REPO / "build/v04/seed"
    base.mkdir(parents=True, exist_ok=True)
    if value:
        output = pathlib.Path(value).expanduser()
        if not output.is_absolute():
            output = REPO / output
        output = output.resolve()
        try:
            output.relative_to(base.resolve())
        except ValueError as error:
            raise ValueError(f"output directory must be under {base}") from error
        if output.exists() and any(output.iterdir()):
            raise ValueError(f"output directory is not empty: {output}")
        output.mkdir(parents=True, exist_ok=True)
    else:
        stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        output = base / f"{stem}-{stamp}-{os.getpid()}"
        output.mkdir(parents=False, exist_ok=False)
    return output


def run_bounded(
    argv: list[str],
    cwd: pathlib.Path,
    timeout_seconds: float,
    stdout_path: pathlib.Path,
    stderr_path: pathlib.Path,
    env: dict[str, str],
) -> dict[str, Any]:
    started_at = utc_now()
    started = time.monotonic()
    timed_out = False
    status: int | None = None
    error: str | None = None
    with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
        try:
            process = subprocess.Popen(
                argv,
                cwd=cwd,
                env=env,
                stdout=stdout,
                stderr=stderr,
                start_new_session=True,
            )
        except OSError as exc:
            error = f"could not start command: {exc}"
        else:
            try:
                status = process.wait(timeout=timeout_seconds)
            except subprocess.TimeoutExpired:
                timed_out = True
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                try:
                    process.wait(timeout=1)
                except subprocess.TimeoutExpired:
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    status = process.wait()
            else:
                # Catch forgotten descendants while the new process group is owned.
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
    elapsed = time.monotonic() - started
    stdout_data = stdout_path.read_bytes()
    stderr_data = stderr_path.read_bytes()
    return {
        "command": argv,
        "cwd": str(cwd),
        "started_at": started_at,
        "elapsed_seconds": round(elapsed, 6),
        "status": status,
        "timed_out": timed_out,
        "error": error,
        "stdout_path": str(stdout_path),
        "stdout_bytes": len(stdout_data),
        "stdout_sha256": sha256_bytes(stdout_data),
        "stdout_text": stdout_data.decode("utf-8", errors="replace"),
        "stderr_path": str(stderr_path),
        "stderr_bytes": len(stderr_data),
        "stderr_sha256": sha256_bytes(stderr_data),
        "stderr_text": stderr_data.decode("utf-8", errors="replace"),
    }


def actual_version(run: dict[str, Any]) -> tuple[str | None, str]:
    if run["status"] != 0 or run["timed_out"] or run["error"]:
        return None, "version-command-failed"
    raw = pathlib.Path(run["stdout_path"]).read_bytes()
    try:
        text = raw.decode("ascii")
    except UnicodeDecodeError:
        return None, "unparseable-version-output"
    match = re.fullmatch(r"rey ([0-9]+\.[0-9]+\.[0-9]+)\n?", text)
    if not match:
        return None, "unparseable-version-output"
    return match.group(1), "ok"


def atomic_json(path: pathlib.Path, value: Any) -> None:
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(path)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", help=f"engine executable (default: {DEFAULT_ENGINE})")
    parser.add_argument("--runtime-dir", help=f"directory containing librey_rt.a (default: {DEFAULT_RUNTIME})")
    parser.add_argument("--expected-version", default=EXPECTED_VERSION)
    parser.add_argument("--timeout-seconds", type=float, default=60.0)
    parser.add_argument("--output-dir", help="empty output directory under build/v04/seed")
    args = parser.parse_args()
    if args.timeout_seconds <= 0:
        parser.error("--timeout-seconds must be positive")
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", args.expected_version):
        parser.error("--expected-version must be an exact semantic version, for example 0.3.1")
    try:
        engine = resolve_executable(args.engine)
        runtime = resolve_runtime(args.runtime_dir)
        output = prepare_output(args.output_dir, "probe")
    except ValueError as error:
        parser.error(str(error))

    result: dict[str, Any] = {
        "schema": "rey-v04-seed-probe-v1",
        "started_at": utc_now(),
        "expected_version": args.expected_version,
        "provenance": {
            "engine_path": str(engine),
            "engine_sha256": file_hash(engine),
            "runtime_path": str(runtime),
            "runtime_files_sha256": tree_hashes(runtime),
            "runtime_tree_sha256": None,
            "host": {
                "system": platform.system(),
                "release": platform.release(),
                "version": platform.version(),
                "machine": platform.machine(),
                "processor": platform.processor(),
                "python": platform.python_version(),
            },
        },
        "version_check": {},
        "fixtures": [],
        "status": "PASS",
        "output_dir": str(output),
    }
    runtime_map = result["provenance"]["runtime_files_sha256"]
    result["provenance"]["runtime_tree_sha256"] = sha256_bytes(
        "".join(f"{name}\0{digest}\n" for name, digest in runtime_map.items()).encode()
    )
    env = os.environ.copy()
    home = output / "home"
    std_link = home / ".reyc/std/src"
    std_link.parent.mkdir(parents=True, exist_ok=True)
    std_link.symlink_to(REPO / "std/src", target_is_directory=True)
    env["HOME"] = str(home)
    env["TMPDIR"] = str(output / "tmp")
    env["LC_ALL"] = "C"
    pathlib.Path(env["TMPDIR"]).mkdir(parents=True, exist_ok=True)

    version_run = run_bounded(
        [str(engine), "--version"],
        REPO,
        args.timeout_seconds,
        output / "version.stdout",
        output / "version.stderr",
        env,
    )
    version, version_parse_status = actual_version(version_run)
    version_matches = version == args.expected_version
    result["provenance"]["engine_version"] = version
    result["version_check"] = {
        **version_run,
        "parse_status": version_parse_status,
        "expected": args.expected_version,
        "actual": version,
        "matches": version_matches,
        "status": "PASS" if version_matches else "FAIL",
    }
    if not version_matches:
        result["status"] = "FAIL"

    try:
        expected_manifest = json.loads(EXPECTED_MANIFEST.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        result["status"] = "FAIL"
        result["manifest_error"] = str(error)
        expected_manifest = {}

    cases = [
        ("all-bytes", "all-bytes.rey", "all-bytes.bin"),
        ("typed-ir", "typed-ir.rey", "typed-ir.bin"),
    ]
    for case_name, source_name, output_name in cases:
        case_dir = output / "cases" / case_name
        case_dir.mkdir(parents=True, exist_ok=False)
        shutil.copytree(FIXTURES, case_dir / "fixtures")
        source = case_dir / "fixtures" / source_name
        expected_path = EXPECTED / output_name
        expected_data = expected_path.read_bytes()
        manifest = expected_manifest.get(output_name, {})
        expected_hash = sha256_bytes(expected_data)
        expected_length = len(expected_data)
        metadata_ok = (
            manifest.get("length") == expected_length
            and manifest.get("hex") == expected_data.hex()
            and manifest.get("sha256") == expected_hash
        )
        case_env = env.copy()
        case_env["TMPDIR"] = str(case_dir / "tmp")
        pathlib.Path(case_env["TMPDIR"]).mkdir(parents=True, exist_ok=True)
        command = [str(engine), "run", source.name, "--runtime-dir", str(runtime)]
        command_run = run_bounded(
            command,
            source.parent,
            args.timeout_seconds,
            case_dir / "stdout",
            case_dir / "stderr",
            case_env,
        )
        actual_path = source.parent / output_name
        actual_data = actual_path.read_bytes() if actual_path.is_file() else b""
        output_matches = actual_path.is_file() and actual_data == expected_data
        case_ok = (
            metadata_ok
            and command_run["status"] == 0
            and not command_run["timed_out"]
            and command_run["error"] is None
            and command_run["stdout_bytes"] == 0
            and command_run["stderr_bytes"] == 0
            and output_matches
        )
        case_result = {
            "name": case_name,
            "fixture": str(source.relative_to(REPO)),
            "status": "PASS" if case_ok else "FAIL",
            "golden": {
                "path": str(expected_path),
                "length": expected_length,
                "hex": expected_data.hex(),
                "sha256": expected_hash,
                "manifest_matches": metadata_ok,
            },
            "output": {
                "path": str(actual_path),
                "exists": actual_path.is_file(),
                "length": len(actual_data),
                "hex": actual_data.hex(),
                "sha256": sha256_bytes(actual_data),
                "matches": output_matches,
            },
            "run": command_run,
        }
        result["fixtures"].append(case_result)
        if not case_ok:
            result["status"] = "FAIL"

    result["finished_at"] = utc_now()
    result_path = output / "result.json"
    atomic_json(result_path, result)
    print(f"seed probe: {result['status']}")
    print(f"engine: {engine} version={version!r} sha256={result['provenance']['engine_sha256']}")
    print(f"runtime: {runtime} tree_sha256={result['provenance']['runtime_tree_sha256']}")
    for case in result["fixtures"]:
        print(
            f"{case['name']}: {case['status']} expected={case['golden']['length']} bytes "
            f"sha256={case['golden']['sha256']} actual={case['output']['length']} bytes "
            f"sha256={case['output']['sha256']}"
        )
    print(f"result: {result_path}")
    return 0 if result["status"] == "PASS" else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ValueError as error:
        print(f"v04-seed-probe: {error}", file=sys.stderr)
        raise SystemExit(2)
