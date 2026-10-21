#!/usr/bin/env python3
"""Run selected, provenance-recorded v0.4 seed baseline gates."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import pathlib
import platform
import shlex
import signal
import shutil
import subprocess
import sys
import time
from typing import Any


REPO = pathlib.Path(__file__).resolve().parents[1]
SEED_PROBE = REPO / "scripts/v04-seed-probe.py"
TYPECHECK_FIXTURES = REPO / "tests/native-typecheck"
DEFAULT_GATES = ("frontend", "conformance", "runtime", "seed")
GATE_DESCRIPTIONS = {
    "frontend": "check positive and negative tests/native-typecheck source fixtures",
    "conformance": "run scripts/conformance.sh against the selected engine and runtime",
    "runtime": "run the existing native compiler/runtime E2E fixtures",
    "seed": "run the v0.4 seed binary and typed-source capability probes",
}


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="milliseconds")


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def tree_hashes(root: pathlib.Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for path in sorted(root.rglob("*")):
        if path.is_file() and not path.is_symlink():
            result[path.relative_to(root).as_posix()] = sha256_file(path)
    return result


def resolve_engine(value: str) -> pathlib.Path:
    candidate = pathlib.Path(value).expanduser()
    try:
        engine = candidate.resolve(strict=True)
    except FileNotFoundError as error:
        raise ValueError(f"engine executable does not exist: {candidate}") from error
    if not engine.is_file() or not os.access(engine, os.X_OK):
        raise ValueError(f"engine is not an executable file: {engine}")
    return engine


def resolve_runtime(value: str) -> pathlib.Path:
    candidate = pathlib.Path(value).expanduser()
    try:
        runtime = candidate.resolve(strict=True)
    except FileNotFoundError as error:
        raise ValueError(f"runtime directory does not exist: {candidate}") from error
    if not runtime.is_dir() or not (runtime / "librey_rt.a").is_file():
        raise ValueError(f"runtime directory must contain librey_rt.a: {runtime}")
    return runtime


def prepare_output(value: str | None) -> pathlib.Path:
    root = (REPO / "build/v04/seed").resolve()
    root.mkdir(parents=True, exist_ok=True)
    if value:
        candidate = pathlib.Path(value).expanduser()
        if not candidate.is_absolute():
            candidate = REPO / candidate
        output = candidate.resolve()
        try:
            output.relative_to(root)
        except ValueError as error:
            raise ValueError(f"output directory must be under {root}") from error
        if output.exists() and any(output.iterdir()):
            raise ValueError(f"output directory is not empty: {output}")
        output.mkdir(parents=True, exist_ok=True)
    else:
        stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        output = root / f"baseline-{stamp}-{os.getpid()}"
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
        "stdout_sha256": hashlib.sha256(stdout_data).hexdigest(),
        "stdout_text": stdout_data.decode("utf-8", errors="replace"),
        "stderr_path": str(stderr_path),
        "stderr_bytes": len(stderr_data),
        "stderr_sha256": hashlib.sha256(stderr_data).hexdigest(),
        "stderr_text": stderr_data.decode("utf-8", errors="replace"),
    }


def exact_version(run: dict[str, Any]) -> str | None:
    if run["status"] != 0 or run["timed_out"] or run["error"]:
        return None
    raw = pathlib.Path(run["stdout_path"]).read_bytes()
    try:
        text = raw.decode("ascii")
    except UnicodeDecodeError:
        return None
    prefix = "rey "
    version = text[:-1] if text.endswith("\n") else text
    if not version.startswith(prefix):
        return None
    value = version[len(prefix):]
    parts = value.split(".")
    if len(parts) != 3 or any(not part.isdigit() for part in parts):
        return None
    return value


def atomic_json(path: pathlib.Path, value: Any) -> None:
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(path)


def gate_frontend(
    engine: pathlib.Path,
    runtime: pathlib.Path,
    output: pathlib.Path,
    timeout: float,
    env: dict[str, str],
) -> dict[str, Any]:
    gate_dir = output / "gates/frontend"
    fixture_copy = gate_dir / "fixtures"
    shutil.copytree(TYPECHECK_FIXTURES, fixture_copy)
    # Metadata fixtures import the compiler frontend. Preserve that module
    # context in the isolated tree rather than accidentally testing missing files.
    for module in ("lexer", "parser", "typecheck", "diagnostics"):
        shutil.copytree(REPO / "rey-compiler/src" / module, fixture_copy / "positive" / module)
    entries = sorted((fixture_copy / "positive").glob("*.rey"))
    entries += sorted((fixture_copy / "negative").glob("*.rey"))
    entries += sorted((fixture_copy / "graphs").glob("*/main.rey"))
    cases: list[dict[str, Any]] = []
    for source in entries:
        relative = source.relative_to(fixture_copy)
        expectation = source.with_suffix(".expect_check_fail")
        if relative.parts[0] == "graphs":
            expectation = source.parent / "expect_check_fail"
        expect_fail = relative.parts[0] == "negative" or expectation.exists()
        expected_text = expectation.read_text().strip() if expectation.exists() else ""
        case_dir = gate_dir / "cases" / relative.parent / source.stem
        case_dir.mkdir(parents=True, exist_ok=True)
        case_env = env.copy()
        case_env["TMPDIR"] = str(case_dir / "tmp")
        pathlib.Path(case_env["TMPDIR"]).mkdir(parents=True, exist_ok=True)
        command = [str(engine), "check", str(source), "--runtime-dir", str(runtime)]
        run = run_bounded(
            command,
            source.parent,
            timeout,
            case_dir / "stdout",
            case_dir / "stderr",
            case_env,
        )
        expected_status = "nonzero" if expect_fail else "zero"
        status_matches = (
            run["status"] is not None and run["status"] > 0 if expect_fail else run["status"] == 0
        )
        diagnostics_match = not expected_text or expected_text in (run["stdout_text"] + run["stderr_text"])
        case_status = "PASS" if status_matches and diagnostics_match and not run["timed_out"] and not run["error"] else "FAIL"
        cases.append({
            "fixture": str(source.relative_to(REPO)),
            "expected": expected_status,
            "expected_diagnostic": expected_text,
            "diagnostic_matches": diagnostics_match,
            "status": case_status,
            "run": run,
        })
    passed = sum(case["status"] == "PASS" for case in cases)
    return {
        "name": "frontend",
        "status": "PASS" if cases and passed == len(cases) else "FAIL",
        "fixture_count": len(cases),
        "passed": passed,
        "failed": len(cases) - passed,
        "cases": cases,
    }


def gate_conformance(
    engine: pathlib.Path,
    runtime: pathlib.Path,
    output: pathlib.Path,
    timeout: float,
    case_timeout: int,
    env: dict[str, str],
) -> dict[str, Any]:
    gate_dir = output / "gates/conformance"
    scratch = gate_dir / "scratch"
    scratch.mkdir(parents=True, exist_ok=True)
    adapter = gate_dir / "engine-with-runtime.sh"
    adapter.write_text(
        "#!/bin/sh\nexec " + shlex.quote(str(engine)) + " \"$@\" --runtime-dir "
        + shlex.quote(str(runtime)) + "\n",
        encoding="utf-8",
    )
    adapter.chmod(0o755)
    command = [
        "/bin/bash",
        str(REPO / "scripts/conformance.sh"),
        "--engine",
        shlex.quote(str(adapter)),
    ]
    gate_env = env.copy()
    gate_env["REY_CONFORMANCE_SCRATCH"] = str(scratch)
    gate_env["CONFORMANCE_TIMEOUT_SECONDS"] = str(case_timeout)
    run = run_bounded(
        command,
        REPO,
        timeout,
        gate_dir / "stdout",
        gate_dir / "stderr",
        gate_env,
    )
    return {
        "name": "conformance",
        "status": "PASS" if run["status"] == 0 and not run["timed_out"] and not run["error"] else "FAIL",
        "run": run,
    }


def gate_runtime(
    engine: pathlib.Path,
    runtime: pathlib.Path,
    output: pathlib.Path,
    timeout: float,
    case_timeout: int,
    env: dict[str, str],
) -> dict[str, Any]:
    gate_dir = output / "gates/runtime"
    staged_repo = gate_dir / "staged-repo"
    staged_e2e = staged_repo / "rey-compiler/tests/e2e"
    shutil.copytree(REPO / "rey-compiler/tests/e2e", staged_e2e)
    temp_root = gate_dir / "tmp"
    temp_root.mkdir(parents=True, exist_ok=True)
    gate_env = env.copy()
    gate_env["TMPDIR"] = str(temp_root)
    gate_env["REY_E2E_TIMEOUT_SECONDS"] = str(case_timeout)
    gate_env["REY_KEEP_E2E_SCRATCH"] = "1"
    command = [
        "/bin/bash",
        str(staged_e2e / "run.sh"),
        "--engine", str(engine),
        "--runtime-dir", str(runtime),
    ]
    run = run_bounded(
        command,
        staged_repo,
        timeout,
        gate_dir / "stdout",
        gate_dir / "stderr",
        gate_env,
    )
    return {
        "name": "runtime",
        "status": "PASS" if run["status"] == 0 and not run["timed_out"] and not run["error"] else "FAIL",
        "run": run,
        "staged_fixture_root": str(staged_e2e),
        "temporary_output_root": str(temp_root),
    }


def gate_seed(
    engine: pathlib.Path,
    runtime: pathlib.Path,
    output: pathlib.Path,
    timeout: float,
    expected_version: str,
    env: dict[str, str],
) -> dict[str, Any]:
    gate_dir = output / "gates/seed"
    probe_output = gate_dir / "probe"
    probe_output.mkdir(parents=True, exist_ok=True)
    command = [
        sys.executable,
        str(SEED_PROBE),
        "--engine", str(engine),
        "--runtime-dir", str(runtime),
        "--expected-version", expected_version,
        "--timeout-seconds", str(timeout),
        "--output-dir", str(probe_output),
    ]
    run = run_bounded(
        command,
        REPO,
        timeout * 4 + 10,
        gate_dir / "stdout",
        gate_dir / "stderr",
        env,
    )
    result_path = probe_output / "result.json"
    nested = None
    if result_path.is_file():
        try:
            nested = json.loads(result_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            nested = {"status": "FAIL", "error": "probe result.json is invalid"}
    return {
        "name": "seed",
        "status": "PASS" if run["status"] == 0 and not run["timed_out"] and nested and nested.get("status") == "PASS" else "FAIL",
        "run": run,
        "probe_result": str(result_path),
        "probe": nested,
    }


def parse_gates(value: str) -> tuple[str, ...]:
    names = tuple(item.strip() for item in value.split(",") if item.strip())
    unknown = sorted(set(names) - set(GATE_DESCRIPTIONS))
    if not names:
        raise ValueError("select at least one gate")
    if unknown:
        raise ValueError("unknown gate(s): " + ", ".join(unknown))
    if len(names) != len(set(names)):
        raise ValueError("each gate may be selected only once")
    return names


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", help="explicit compiler/seed executable")
    parser.add_argument("--runtime-dir", help="explicit directory containing librey_rt.a")
    parser.add_argument("--expected-version", default="0.3.1")
    parser.add_argument("--gates", default=",".join(DEFAULT_GATES), help="comma-separated gate names")
    parser.add_argument("--timeout-seconds", type=float, default=1800.0, help="outer timeout per gate")
    parser.add_argument("--case-timeout-seconds", type=int, default=60)
    parser.add_argument("--output-dir", help="empty output directory under build/v04/seed")
    parser.add_argument("--dry-run", action="store_true", help="list exact gate commands without running them")
    parser.add_argument("--list-gates", action="store_true", help="list supported gate names")
    args = parser.parse_args()
    if args.list_gates:
        for name, description in GATE_DESCRIPTIONS.items():
            print(f"{name}\t{description}")
        return 0
    if not args.engine or not args.runtime_dir:
        parser.error("--engine and --runtime-dir are required")
    if args.timeout_seconds <= 0 or args.case_timeout_seconds <= 0:
        parser.error("timeouts must be positive")
    if not args.expected_version.replace(".", "").isdigit() or len(args.expected_version.split(".")) != 3:
        parser.error("--expected-version must be an exact semantic version, for example 0.3.1")
    try:
        gates = parse_gates(args.gates)
        engine = resolve_engine(args.engine)
        runtime = resolve_runtime(args.runtime_dir)
        if args.dry_run:
            root = (REPO / "build/v04/seed").resolve()
            output = pathlib.Path(args.output_dir).expanduser() if args.output_dir else root / "baseline-<timestamp>"
            if not output.is_absolute():
                output = REPO / output
            output = output.resolve()
        else:
            output = prepare_output(args.output_dir)
    except ValueError as error:
        parser.error(str(error))

    if args.dry_run:
        plan = {
            "engine": str(engine),
            "runtime_dir": str(runtime),
            "output_dir": str(output),
            "gates": [],
        }
        for gate in gates:
            if gate == "frontend":
                command = [str(engine), "check", "<fixture.rey>", "--runtime-dir", str(runtime)]
            elif gate == "conformance":
                command = ["/bin/bash", str(REPO / "scripts/conformance.sh"), "--engine", "<isolated adapter with explicit runtime>"]
            elif gate == "runtime":
                command = ["/bin/bash", "<staged rey-compiler/tests/e2e/run.sh>", "--engine", str(engine), "--runtime-dir", str(runtime)]
            else:
                command = [sys.executable, str(SEED_PROBE), "--engine", str(engine), "--runtime-dir", str(runtime)]
            plan["gates"].append({"name": gate, "description": GATE_DESCRIPTIONS[gate], "command": command})
        print(json.dumps(plan, indent=2))
        return 0

    env = os.environ.copy()
    env["LC_ALL"] = "C"
    started_at = utc_now()
    version_run = run_bounded(
        [str(engine), "--version"],
        REPO,
        args.case_timeout_seconds,
        output / "version.stdout",
        output / "version.stderr",
        env,
    )
    version = exact_version(version_run)
    version_matches = version == args.expected_version
    result: dict[str, Any] = {
        "schema": "rey-v04-baseline-v1",
        "started_at": started_at,
        "expected_version": args.expected_version,
        "provenance": {
            "engine": str(engine),
            "engine_sha256": sha256_file(engine),
            "engine_version": version,
            "runtime_dir": str(runtime),
            "runtime_files_sha256": tree_hashes(runtime),
            "host": {
                "system": platform.system(),
                "release": platform.release(),
                "version": platform.version(),
                "machine": platform.machine(),
                "processor": platform.processor(),
                "python": platform.python_version(),
            },
        },
        "version_check": {
            **version_run,
            "expected": args.expected_version,
            "actual": version,
            "matches": version_matches,
            "status": "PASS" if version_matches else "FAIL",
        },
        "selected_gates": list(gates),
        "output_dir": str(output),
        "gates": [],
        "status": "PASS" if version_matches else "FAIL",
    }
    gate_functions = {
        "frontend": lambda: gate_frontend(engine, runtime, output, args.case_timeout_seconds, env),
        "conformance": lambda: gate_conformance(
            engine, runtime, output, args.timeout_seconds, args.case_timeout_seconds, env
        ),
        "runtime": lambda: gate_runtime(
            engine, runtime, output, args.timeout_seconds, args.case_timeout_seconds, env
        ),
        "seed": lambda: gate_seed(
            engine, runtime, output, args.case_timeout_seconds, args.expected_version, env
        ),
    }
    for name in gates:
        try:
            gate_result = gate_functions[name]()
        except (OSError, shutil.Error, ValueError) as error:
            gate_result = {"name": name, "status": "FAIL", "error": str(error)}
        result["gates"].append(gate_result)
        if gate_result.get("status") != "PASS":
            result["status"] = "FAIL"
        print(f"{name}: {gate_result.get('status', 'FAIL')}")

    result["finished_at"] = utc_now()
    result_path = output / "result.json"
    atomic_json(result_path, result)
    print(f"baseline: {result['status']}")
    print(f"engine version: {version!r} expected={args.expected_version!r}")
    print(f"result: {result_path}")
    return 0 if result["status"] == "PASS" else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ValueError as error:
        print(f"v04-baseline: {error}", file=sys.stderr)
        raise SystemExit(2)
