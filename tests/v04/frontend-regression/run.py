#!/usr/bin/env python3
"""Compare frontend-only checking over the archived and workspace snapshots."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tarfile
import time
from typing import Any


REPO = Path(__file__).resolve().parents[3]
SEED = REPO / "build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME = SEED.parent / "runtime"
BASELINE_REV = "6b36981"
FIXTURE_PATH = Path("tests/native-typecheck")
FRONTEND_MODULES = ("lexer", "parser", "typecheck", "diagnostics")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def tree_hashes(root: Path) -> dict[str, str]:
    return {
        path.relative_to(root).as_posix(): sha256_file(path)
        for path in sorted(root.rglob("*"))
        if path.is_file() and not path.is_symlink()
    }


def tree_digest(files: dict[str, str]) -> str:
    digest = hashlib.sha256()
    for relative, file_hash in sorted(files.items()):
        digest.update(relative.encode("utf-8"))
        digest.update(b"\0")
        digest.update(file_hash.encode("ascii"))
        digest.update(b"\n")
    return digest.hexdigest()


def atomic_json(path: Path, value: Any) -> None:
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(path)


def run_bounded(argv: list[str], cwd: Path, timeout: float, env: dict[str, str] | None = None) -> dict[str, Any]:
    started = time.monotonic()
    try:
        process = subprocess.Popen(
            argv,
            cwd=cwd,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=True,
        )
    except OSError as error:
        return {
            "command": argv,
            "status": None,
            "timed_out": False,
            "error": f"could not start command: {error}",
            "elapsed_seconds": round(time.monotonic() - started, 6),
            "stdout": "",
            "stderr": "",
        }
    timed_out = False
    try:
        stdout, stderr = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out = True
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            stdout, stderr = process.communicate(timeout=2)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            stdout, stderr = process.communicate()
    return {
        "command": argv,
        "status": process.returncode,
        "timed_out": timed_out,
        "error": None,
        "elapsed_seconds": round(time.monotonic() - started, 6),
        "stdout": stdout.decode("utf-8", errors="replace"),
        "stderr": stderr.decode("utf-8", errors="replace"),
    }


def safe_extract(archive: Path, destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=False)
    with tarfile.open(archive, "r:") as bundle:
        members = bundle.getmembers()
        for member in members:
            target = (destination / member.name).resolve()
            if destination.resolve() not in target.parents and target != destination.resolve():
                raise ValueError(f"archive member escapes extraction root: {member.name}")
        bundle.extractall(destination, members=members, filter="data")


def archive_baseline(output: Path) -> tuple[Path, str]:
    archive = output / "baseline-6b36981.tar"
    with archive.open("wb") as sink:
        completed = subprocess.run(
            ["git", "archive", "--format=tar", BASELINE_REV, "rey-compiler/src", str(FIXTURE_PATH)],
            cwd=REPO,
            stdout=sink,
            stderr=subprocess.PIPE,
            timeout=60,
            check=False,
        )
    if completed.returncode != 0:
        raise RuntimeError("git archive failed: " + completed.stderr.decode("utf-8", errors="replace"))
    return archive, sha256_file(archive)


def prepare_tree(source_root: Path, fixture_root: Path, driver_source: Path, destination: Path) -> dict[str, Any]:
    src_root = destination / "src"
    shutil.copytree(source_root, src_root)
    staged_fixture = src_root / FIXTURE_PATH
    staged_fixture.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(fixture_root, staged_fixture)
    shutil.copy2(driver_source, src_root / "frontend-main.rey")
    positive = staged_fixture / "positive"
    for module in FRONTEND_MODULES:
        shutil.copytree(src_root / module, positive / module)
    (staged_fixture / "stdroot").mkdir()
    files = tree_hashes(src_root)
    return {
        "src": str(src_root),
        "fixture_root": str(staged_fixture),
        "file_count": len(files),
        "tree_sha256": tree_digest(files),
        "files": files,
    }


def prepare_run(output: Path) -> dict[str, Any]:
    output.mkdir(parents=True, exist_ok=False)
    archive, archive_hash = archive_baseline(output)
    extracted = output / "baseline-archive"
    safe_extract(archive, extracted)
    baseline_src = extracted / "rey-compiler/src"
    baseline_tests = extracted / FIXTURE_PATH
    candidate_src = REPO / "rey-compiler/src"
    candidate_tests = REPO / FIXTURE_PATH
    if not baseline_src.is_dir() or not baseline_tests.is_dir():
        raise RuntimeError("baseline archive is missing compiler source or native-typecheck fixtures")
    if not candidate_src.is_dir() or not candidate_tests.is_dir():
        raise RuntimeError("workspace candidate is missing compiler source or native-typecheck fixtures")
    driver = REPO / "rey-compiler/frontend-main.rey"
    if not driver.is_file():
        raise RuntimeError(f"frontend CLI driver is missing: {driver}")
    snapshots = output / "snapshots"
    snapshots.mkdir()
    baseline = prepare_tree(baseline_src, baseline_tests, driver, snapshots / "baseline")
    candidate = prepare_tree(candidate_src, candidate_tests, driver, snapshots / "candidate")
    return {
        "schema": 1,
        "state": "prepared",
        "baseline_revision": BASELINE_REV,
        "baseline_archive": str(archive),
        "baseline_archive_sha256": archive_hash,
        "seed": {"path": str(SEED), "sha256": sha256_file(SEED)},
        "runtime": {"path": str(RUNTIME), "sha256": sha256_file(RUNTIME / "librey_rt.a")},
        "driver_source": str(driver),
        "driver_sha256": sha256_file(driver),
        "baseline": baseline,
        "candidate": candidate,
        "builds": {},
        "smoke": {},
        "fixtures": {},
    }


def prepare_from_snapshots(source_run: Path, output: Path, driver_source: Path) -> dict[str, Any]:
    """Reuse an earlier immutable compiler/test snapshot with the current driver."""
    previous_path = source_run / "results.json"
    previous = json.loads(previous_path.read_text(encoding="utf-8"))
    if previous.get("state") not in ("prepared", "build-failed", "complete"):
        raise RuntimeError(f"source snapshot run has unusable state: {previous.get('state')}")
    output.mkdir(parents=True, exist_ok=False)
    (output / "snapshots").mkdir()
    manifest = dict(previous)
    manifest.update({
        "state": "prepared",
        "source_snapshot_run": str(source_run),
        "driver_source": str(driver_source),
        "driver_sha256": sha256_file(driver_source),
        "builds": {},
        "smoke": {},
        "fixtures": {},
    })
    manifest.pop("summary", None)
    manifest.pop("comparisons", None)
    manifest.pop("partial_cases", None)
    for name in ("baseline", "candidate"):
        old_snapshot = previous[name]
        old_src = Path(old_snapshot["src"])
        if tree_digest(tree_hashes(old_src)) != old_snapshot["tree_sha256"]:
            raise RuntimeError(f"{name} source snapshot changed after its original preparation")
        new_root = output / "snapshots" / name
        new_src = new_root / "src"
        shutil.copytree(old_src, new_src)
        shutil.copy2(driver_source, new_src / "frontend-main.rey")
        files = tree_hashes(new_src)
        snapshot = dict(old_snapshot)
        snapshot.update({
            "src": str(new_src),
            "fixture_root": str(new_src / FIXTURE_PATH),
            "file_count": len(files),
            "tree_sha256": tree_digest(files),
            "files": files,
        })
        manifest[name] = snapshot
    return manifest


def normalize_text(text: str, src_root: Path) -> str:
    prefix = str(src_root)
    return text.replace(prefix, "<snapshot>/src")


def build_snapshot(name: str, snapshot: dict[str, Any], output: Path, timeout: float) -> dict[str, Any]:
    src_root = Path(snapshot["src"])
    binary = output / "snapshots" / name / "rey-frontend"
    argv = [str(SEED), "build", str(src_root / "frontend-main.rey"), "-o", str(binary), "--runtime-dir", str(RUNTIME)]
    result = run_bounded(argv, src_root, timeout)
    result["binary"] = str(binary)
    result["binary_sha256"] = sha256_file(binary) if result["status"] == 0 and binary.is_file() else None
    return result


def fixture_entries(fixture_root: Path) -> list[Path]:
    entries = sorted((fixture_root / "positive").glob("*.rey"))
    entries += sorted((fixture_root / "negative").glob("*.rey"))
    entries += sorted((fixture_root / "graphs").glob("*/main.rey"))
    return entries


def expectation_for(source: Path) -> tuple[bool, str]:
    if source.parent.parent.name == "graphs":
        expectation = source.parent / "expect_check_fail"
        category = "graphs"
    else:
        expectation = source.with_suffix(".expect_check_fail")
        category = source.parent.name
    expect_fail = category == "negative" or expectation.exists()
    expected_text = expectation.read_text(encoding="utf-8").strip() if expectation.exists() else ""
    return expect_fail, expected_text


def run_case(driver: Path, source: Path, fixture_root: Path, src_root: Path, timeout: float, case_dir: Path) -> dict[str, Any]:
    expect_fail, expected_text = expectation_for(source)
    case_dir.mkdir(parents=True, exist_ok=False)
    temp_dir = case_dir / "tmp"
    temp_dir.mkdir()
    env = os.environ.copy()
    env["TMPDIR"] = str(temp_dir)
    std_root = fixture_root / "stdroot"
    argv = [str(driver), "check", str(source), "--std-root", str(std_root)]
    run = run_bounded(argv, source.parent, timeout, env)
    status = run["status"]
    status_matches = (status is not None and status != 0) if expect_fail else status == 0
    combined = run["stdout"] + run["stderr"]
    diagnostics_match = not expected_text or expected_text in combined
    accepted = status_matches and diagnostics_match and not run["timed_out"] and not run["error"]
    relative = source.relative_to(fixture_root).as_posix()
    return {
        "fixture": relative,
        "expected": "check-fail" if expect_fail else "check-pass",
        "expected_diagnostic": expected_text,
        "exit_status": status,
        "status_semantics": "zero" if status == 0 else "nonzero" if status is not None else "no-status",
        "timed_out": run["timed_out"],
        "error": run["error"],
        "diagnostic_matches": diagnostics_match,
        "matches_expectation": accepted,
        "stdout": normalize_text(run["stdout"], src_root),
        "stderr": normalize_text(run["stderr"], src_root),
        "elapsed_seconds": run["elapsed_seconds"],
        "command": [normalize_text(item, src_root) for item in argv],
    }


def run_smoke(name: str, snapshot: dict[str, Any], binary: Path, output: Path, timeout: float) -> dict[str, Any]:
    src_root = Path(snapshot["src"])
    fixture_root = Path(snapshot["fixture_root"])
    smoke_dir = output / "smoke" / name
    smoke_dir.mkdir(parents=True, exist_ok=False)
    tests = [
        ("help", [str(binary), "--help"], 0, "Usage:"),
        ("missing-command-args", [str(binary), "check"], 2, "check requires a source file"),
        ("missing-file", [str(binary), "check", str(fixture_root / "missing.rey")], 1, "not found"),
        ("std-root-option", [str(binary), "check", str(fixture_root / "positive/containers.rey"), "--std-root", str(fixture_root / "stdroot")], 0, ""),
    ]
    results: dict[str, Any] = {}
    for label, argv, expected_status, expected_text in tests:
        case_dir = smoke_dir / label
        case_dir.mkdir()
        env = os.environ.copy()
        env["TMPDIR"] = str(case_dir)
        run = run_bounded(argv, fixture_root, timeout, env)
        combined = run["stdout"] + run["stderr"]
        ok = run["status"] == expected_status and not run["timed_out"] and not run["error"] and expected_text in combined
        results[label] = {
            "expected_status": expected_status,
            "exit_status": run["status"],
            "timed_out": run["timed_out"],
            "error": run["error"],
            "expected_text": expected_text,
            "output": normalize_text(combined, src_root),
            "passed": ok,
        }
    return results


def summarize_cases(cases: list[dict[str, Any]]) -> dict[str, int]:
    by_category: dict[str, int] = {}
    for case in cases:
        category = case["fixture"].split("/", 1)[0]
        by_category.setdefault(category + "_total", 0)
        by_category.setdefault(category + "_passed", 0)
        by_category[category + "_total"] += 1
        if case["matches_expectation"]:
            by_category[category + "_passed"] += 1
    by_category["total"] = len(cases)
    by_category["passed"] = sum(case["matches_expectation"] for case in cases)
    by_category["failed"] = len(cases) - by_category["passed"]
    return by_category


def run_all(run_dir: Path, build_timeout: float, case_timeout: float) -> int:
    manifest_path = run_dir / "results.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("state") != "prepared":
        raise RuntimeError("run directory is not in prepared state")
    for name in ("baseline", "candidate"):
        snapshot = manifest[name]
        src_root = Path(snapshot["src"])
        if tree_digest(tree_hashes(src_root)) != snapshot["tree_sha256"]:
            raise RuntimeError(f"{name} source snapshot changed after preparation")
    if sha256_file(REPO / "rey-compiler/frontend-main.rey") != manifest["driver_sha256"]:
        raise RuntimeError("frontend CLI driver changed after source snapshots were prepared")

    builds: dict[str, Any] = {}
    for name in ("baseline", "candidate"):
        builds[name] = build_snapshot(name, manifest[name], run_dir, build_timeout)
        print(f"build {name}: status={builds[name]['status']} timeout={builds[name]['timed_out']}", flush=True)
        manifest["builds"] = builds
        atomic_json(manifest_path, manifest)
        if builds[name]["status"] != 0 or builds[name]["timed_out"] or builds[name]["error"]:
            manifest["state"] = "build-failed"
            atomic_json(manifest_path, manifest)
            return 2

    smoke: dict[str, Any] = {}
    for name in ("baseline", "candidate"):
        smoke[name] = run_smoke(
            name,
            manifest[name],
            Path(builds[name]["binary"]),
            run_dir,
            case_timeout,
        )
    manifest["smoke"] = smoke

    case_maps: dict[str, dict[str, Any]] = {"baseline": {}, "candidate": {}}
    for name in ("baseline", "candidate"):
        snapshot = manifest[name]
        fixture_root = Path(snapshot["fixture_root"])
        src_root = Path(snapshot["src"])
        driver = Path(builds[name]["binary"])
        entries = fixture_entries(fixture_root)
        if len(entries) != 70:
            raise RuntimeError(f"expected 70 frontend fixtures in {name}, found {len(entries)}")
        for index, source in enumerate(entries, start=1):
            relative = source.relative_to(fixture_root).as_posix()
            case_dir = run_dir / "cases" / name / Path(relative).parent / source.stem
            result = run_case(driver, source, fixture_root, src_root, case_timeout, case_dir)
            case_maps[name][relative] = result
            print(f"{name} {index:02d}/70 {result['matches_expectation']} {relative} status={result['exit_status']}", flush=True)
            manifest["partial_cases"] = case_maps
            atomic_json(manifest_path, manifest)

    comparisons = []
    unexplained_status_changes = []
    for fixture in sorted(case_maps["baseline"]):
        before = case_maps["baseline"][fixture]
        after = case_maps["candidate"][fixture]
        baseline_ok = before["matches_expectation"]
        candidate_ok = after["matches_expectation"]
        change = "unchanged"
        if baseline_ok and not candidate_ok:
            change = "regression"
        elif not baseline_ok and candidate_ok:
            change = "improvement"
        same_process_status = (before["exit_status"], before["timed_out"], before["error"]) == (after["exit_status"], after["timed_out"], after["error"])
        if not same_process_status and change == "unchanged":
            unexplained_status_changes.append(fixture)
        comparisons.append({
            "fixture": fixture,
            "expected": before["expected"],
            "expected_diagnostic": before["expected_diagnostic"],
            "baseline": before,
            "candidate": after,
            "change": change,
            "same_process_status": same_process_status,
        })
    regressions = [case["fixture"] for case in comparisons if case["change"] == "regression"]
    improvements = [case["fixture"] for case in comparisons if case["change"] == "improvement"]
    manifest["state"] = "complete"
    manifest["case_timeout_seconds"] = case_timeout
    manifest["build_timeout_seconds"] = build_timeout
    manifest["summary"] = {
        "baseline": summarize_cases(list(case_maps["baseline"].values())),
        "candidate": summarize_cases(list(case_maps["candidate"].values())),
        "regressions": regressions,
        "improvements": improvements,
        "unexplained_status_changes": unexplained_status_changes,
        "same_expected_semantics": len(regressions) == 0 and len(improvements) == 0,
        "smoke_passed": {
            name: sum(item["passed"] for item in smoke[name].values()) for name in smoke
        },
    }
    manifest["comparisons"] = comparisons
    smoke_ok = all(item["passed"] for group in smoke.values() for item in group.values())
    manifest["passed"] = not regressions and not unexplained_status_changes and smoke_ok
    manifest.pop("partial_cases", None)
    atomic_json(manifest_path, manifest)
    summary = manifest["summary"]
    print("baseline:", summary["baseline"], flush=True)
    print("candidate:", summary["candidate"], flush=True)
    print("regressions:", regressions, flush=True)
    print("improvements:", improvements, flush=True)
    return 0 if manifest["passed"] else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, help="new output directory under build/v04/frontend-regression")
    parser.add_argument("--run-dir", type=Path, help="run an already prepared snapshot directory")
    parser.add_argument("--reuse-source-snapshots", type=Path, help="prepare a fresh run using compiler/test snapshots from an earlier run")
    parser.add_argument("--prepare-only", action="store_true", help="prepare immutable source snapshots without building or running tests")
    parser.add_argument("--build-timeout", type=float, default=300.0)
    parser.add_argument("--case-timeout", type=float, default=15.0)
    args = parser.parse_args()
    if args.build_timeout <= 0 or args.case_timeout <= 0:
        parser.error("timeouts must be positive")
    root = REPO / "build/v04/frontend-regression"
    root.mkdir(parents=True, exist_ok=True)
    if args.run_dir and args.reuse_source_snapshots:
        parser.error("--run-dir and --reuse-source-snapshots cannot be combined")
    if args.run_dir:
        run_dir = args.run_dir.expanduser()
        if not run_dir.is_absolute():
            run_dir = REPO / run_dir
        run_dir = run_dir.resolve(strict=True)
        run_dir.relative_to(root.resolve())
    else:
        if args.prepare_only and args.output_dir is None:
            parser.error("--prepare-only requires --output-dir")
        if args.output_dir:
            run_dir = args.output_dir.expanduser()
            if not run_dir.is_absolute():
                run_dir = REPO / run_dir
            run_dir = run_dir.resolve()
            run_dir.relative_to(root.resolve())
        else:
            stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
            run_dir = root / f"run-{stamp}-{os.getpid()}"
        if args.reuse_source_snapshots:
            source_run = args.reuse_source_snapshots.expanduser()
            if not source_run.is_absolute():
                source_run = REPO / source_run
            source_run = source_run.resolve(strict=True)
            source_run.relative_to(root.resolve())
            manifest = prepare_from_snapshots(source_run, run_dir, REPO / "rey-compiler/frontend-main.rey")
        else:
            manifest = prepare_run(run_dir)
        atomic_json(run_dir / "results.json", manifest)
        print(f"prepared snapshots: {run_dir}", flush=True)
        if args.prepare_only:
            return 0
    return run_all(run_dir, args.build_timeout, args.case_timeout)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        print(f"frontend regression runner error: {error}", file=sys.stderr)
        raise SystemExit(2)
