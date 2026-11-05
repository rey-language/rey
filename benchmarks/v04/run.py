#!/usr/bin/env python3
"""Run matched one-program-at-a-time Rey v0.4 baseline workloads."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import platform
import re
import shutil
import statistics
import subprocess
import sys
import time
from pathlib import Path


HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[1]
OUTPUT_DEFAULT = REPO_ROOT / "build" / "v04" / "benchmarks"
LANGUAGES = ("c", "cpp", "rust", "zig", "go", "python", "rey")
WORKLOADS = ("numeric", "churn")
DEFAULT_SIZES = {"numeric": 1_000_000, "churn": 256}
SIZE_LIMITS = {"numeric": 10_000_000, "churn": 4096}
VECTOR_WIDTH = 32
OUTPUT_PATTERN = re.compile(r"checksum=(-?\d+) processed=(\d+)")
PUBLISHED_REY = (
    REPO_ROOT
    / "build/release/v0.3.1/macos-arm64-winruntime/package/"
    "rey-v0.3.1-macos-arm64/bin/rey"
)

EXTENSIONS = {
    "c": ".c",
    "cpp": ".cpp",
    "rust": ".rs",
    "zig": ".zig",
    "go": ".go",
    "python": ".py",
    "rey": ".rey",
}


def parse_name_list(value: str, choices: tuple[str, ...], label: str) -> list[str]:
    names = [part.strip().lower() for part in value.split(",") if part.strip()]
    invalid = [name for name in names if name not in choices]
    if not names or invalid:
        raise argparse.ArgumentTypeError(
            f"{label} must be a comma-separated subset of {', '.join(choices)}"
        )
    if len(set(names)) != len(names):
        raise argparse.ArgumentTypeError(f"{label} contains a duplicate")
    return names


def positive_int(value: str) -> int:
    try:
        parsed = int(value, 10)
    except ValueError as exc:
        raise argparse.ArgumentTypeError("must be a base-10 integer") from exc
    if parsed < 0:
        raise argparse.ArgumentTypeError("must be non-negative")
    return parsed


def positive_float(value: str) -> float:
    try:
        parsed = float(value)
    except ValueError as exc:
        raise argparse.ArgumentTypeError("must be a number") from exc
    if parsed <= 0:
        raise argparse.ArgumentTypeError("must be greater than zero")
    return parsed


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--languages",
        default=",".join(LANGUAGES),
        type=lambda value: parse_name_list(value, LANGUAGES, "--languages"),
        help="comma-separated language subset (default: all seven)",
    )
    parser.add_argument(
        "--workloads",
        default=",".join(WORKLOADS),
        type=lambda value: parse_name_list(value, WORKLOADS, "--workloads"),
        help="comma-separated workload subset (default: numeric,churn)",
    )
    parser.add_argument(
        "--size",
        type=positive_int,
        help="runtime input size for every selected workload; defaults vary by workload",
    )
    parser.add_argument("--repeats", type=positive_int, default=9, help="measured run samples")
    parser.add_argument("--warmups", type=positive_int, default=2, help="unmeasured run warmups")
    parser.add_argument("--timeout", type=positive_float, default=120.0, help="per-process timeout in seconds")
    parser.add_argument(
        "--rey",
        type=Path,
        help="Rey seed executable (also REY_BIN); defaults to the packaged v0.3.1 seed, then build/rey",
    )
    parser.add_argument(
        "--runtime-dir",
        type=Path,
        help="directory containing librey_rt.a (also REY_RUNTIME_DIR)",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=OUTPUT_DEFAULT,
        help=f"artifact directory (default: {OUTPUT_DEFAULT})",
    )
    return parser.parse_args()


def sha256_file(path: Path | None) -> str | None:
    if path is None or not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def command_output(command: list[str], timeout: float = 10.0) -> str | None:
    try:
        result = subprocess.run(command, capture_output=True, text=True, timeout=timeout, check=False)
    except (OSError, subprocess.TimeoutExpired):
        return None
    text = (result.stdout or result.stderr).strip()
    return text if result.returncode == 0 and text else None


def cpu_description() -> str | None:
    if platform.system() == "Darwin":
        model = command_output(["/usr/sbin/sysctl", "-n", "machdep.cpu.brand_string"])
        if model:
            return model
    if platform.processor():
        return platform.processor()
    if platform.system() == "Linux":
        try:
            for line in Path("/proc/cpuinfo").read_text(errors="replace").splitlines():
                if line.lower().startswith("model name") or line.lower().startswith("hardware"):
                    return line.split(":", 1)[-1].strip()
        except OSError:
            pass
    return None


def memory_description() -> dict[str, int | str] | None:
    if platform.system() == "Darwin":
        value = command_output(["/usr/sbin/sysctl", "-n", "hw.memsize"])
        if value and value.isdigit():
            return {"value": int(value), "unit": "bytes"}
    if platform.system() == "Linux":
        try:
            for line in Path("/proc/meminfo").read_text().splitlines():
                if line.startswith("MemTotal:"):
                    return {"value": int(line.split()[1]), "unit": "KiB"}
        except (OSError, ValueError, IndexError):
            pass
    return None


def git_state() -> dict[str, str | bool | None]:
    revision = command_output(["git", "-C", str(REPO_ROOT), "rev-parse", "HEAD"])
    try:
        changes_result = subprocess.run(
            ["git", "-C", str(REPO_ROOT), "status", "--porcelain"],
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
        dirty = bool(changes_result.stdout.strip()) if changes_result.returncode == 0 else None
    except (OSError, subprocess.TimeoutExpired):
        dirty = None
    return {
        "revision": revision,
        "working_tree_dirty": dirty,
    }


def host_details() -> dict[str, object]:
    return {
        "system": platform.system(),
        "release": platform.release(),
        "version": platform.version(),
        "machine": platform.machine(),
        "cpu": cpu_description(),
        "logical_cpus": os.cpu_count(),
        "memory": memory_description(),
        "python": sys.version.splitlines()[0],
    }


def time_wrapper() -> tuple[list[str] | None, str | None]:
    executable = Path("/usr/bin/time")
    if not executable.is_file():
        return None, None
    if platform.system() == "Darwin":
        return [str(executable), "-l"], "bytes"
    if platform.system() == "Linux":
        return [str(executable), "-v", "--"], "KiB"
    return None, None


def rss_value(stderr: str, unit: str | None) -> tuple[int | None, str]:
    if unit is None:
        return None, "unavailable: no supported /usr/bin/time format on this platform"
    if unit == "bytes":
        match = re.search(r"^\s*(\d+)\s+maximum resident set size\s*$", stderr, re.IGNORECASE | re.MULTILINE)
    else:
        match = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", stderr)
    if not match:
        return None, f"unavailable: time output did not contain peak RSS ({unit})"
    return int(match.group(1)), unit


def run_process(command: list[str], timeout: float, rss_prefix: list[str] | None, rss_unit: str | None) -> dict[str, object]:
    executed = (rss_prefix or []) + command
    started = time.perf_counter()
    try:
        result = subprocess.run(executed, capture_output=True, text=True, timeout=timeout, check=False)
        elapsed = time.perf_counter() - started
        peak_rss, peak_rss_unit = rss_value(result.stderr, rss_unit)
        return {
            "command": executed,
            "program_command": command,
            "elapsed_seconds": elapsed,
            "return_code": result.returncode,
            "status": "ok" if result.returncode == 0 else "nonzero_exit",
            "stdout": result.stdout,
            "stderr": result.stderr,
            "peak_rss": peak_rss,
            "peak_rss_unit": peak_rss_unit,
        }
    except subprocess.TimeoutExpired as exc:
        elapsed = time.perf_counter() - started
        return {
            "command": executed,
            "program_command": command,
            "elapsed_seconds": elapsed,
            "return_code": None,
            "status": "timeout",
            "stdout": decode_output(exc.stdout),
            "stderr": decode_output(exc.stderr),
            "peak_rss": None,
            "peak_rss_unit": "unavailable: process timed out",
        }
    except OSError as exc:
        return {
            "command": executed,
            "program_command": command,
            "elapsed_seconds": None,
            "return_code": None,
            "status": "launch_error",
            "stdout": "",
            "stderr": str(exc),
            "peak_rss": None,
            "peak_rss_unit": "unavailable: process did not launch",
        }


def decode_output(value: str | bytes | None) -> str:
    if value is None:
        return ""
    return value.decode(errors="replace") if isinstance(value, bytes) else value


def summary(samples: list[dict[str, object]]) -> dict[str, object]:
    values = [sample["elapsed_seconds"] for sample in samples if isinstance(sample.get("elapsed_seconds"), (int, float))]
    if not values:
        return {"median_seconds": None, "mad_seconds": None, "min_seconds": None, "max_seconds": None}
    median = statistics.median(values)
    deviations = [abs(value - median) for value in values]
    return {
        "median_seconds": median,
        "mad_seconds": statistics.median(deviations),
        "min_seconds": min(values),
        "max_seconds": max(values),
    }


def expected_result(workload: str, size: int) -> dict[str, int | str]:
    checksum = 0
    if workload == "numeric":
        state = 1
        for _ in range(size):
            state = (state * 48271) % 2147483647
            checksum += state % 1024
        processed = size
    else:
        for round_index in range(size):
            checksum += sum((round_index * 17 + index * 13) % 1009 for index in range(VECTOR_WIDTH))
        processed = size * VECTOR_WIDTH
    return {
        "checksum": checksum,
        "processed": processed,
        "normalized": f"checksum={checksum} processed={processed}",
    }


def normalize_output(stdout: str) -> dict[str, int | str] | None:
    match = OUTPUT_PATTERN.fullmatch(stdout.strip())
    if not match:
        return None
    checksum = int(match.group(1))
    processed = int(match.group(2))
    return {
        "checksum": checksum,
        "processed": processed,
        "normalized": f"checksum={checksum} processed={processed}",
    }


def compiler_choices() -> dict[str, list[str]]:
    env = os.environ
    return {
        "c": [env.get("CC", "clang")],
        "cpp": [env.get("CXX", "clang++")],
        "rust": [env.get("RUSTC", "rustc")],
        "zig": [env.get("ZIG", "zig")],
        "go": [env.get("GO", "go")],
        "python": [sys.executable],
    }


def rey_selection(args: argparse.Namespace) -> tuple[Path | None, str, Path | None]:
    if args.rey is not None:
        binary = args.rey.expanduser().resolve()
        source = "--rey"
    elif os.environ.get("REY_BIN"):
        binary = Path(os.environ["REY_BIN"]).expanduser().resolve()
        source = "REY_BIN"
    elif PUBLISHED_REY.is_file():
        binary = PUBLISHED_REY.resolve()
        source = "packaged v0.3.1 seed default"
    else:
        binary = REPO_ROOT / "build" / "rey"
        source = "build/rey fallback"
    if not binary.is_file() or not os.access(binary, os.X_OK):
        return binary, source, None
    if args.runtime_dir is not None:
        runtime = args.runtime_dir.expanduser().resolve()
        runtime_source = "--runtime-dir"
    elif os.environ.get("REY_RUNTIME_DIR"):
        runtime = Path(os.environ["REY_RUNTIME_DIR"]).expanduser().resolve()
        runtime_source = "REY_RUNTIME_DIR"
    else:
        near = binary.parent / "runtime"
        fallback = REPO_ROOT / "build" / "runtime"
        if (near / "librey_rt.a").is_file():
            runtime, runtime_source = near.resolve(), "adjacent runtime default"
        else:
            runtime, runtime_source = fallback.resolve(), "build/runtime default"
    if not (runtime / "librey_rt.a").is_file():
        return binary, f"{source}; runtime selected from {runtime_source} is missing librey_rt.a", None
    return binary, source, runtime


def tool_info(language: str, compilers: dict[str, list[str]], rey_path: Path | None) -> dict[str, object]:
    if language == "rey":
        command = [str(rey_path)] if rey_path is not None else []
        path = rey_path
        version_command = command + ["--version"] if command else []
    else:
        command = compilers[language]
        resolved = shutil.which(command[0])
        path = Path(resolved).resolve() if resolved else None
        if language in ("zig", "go"):
            version_command = command + ["version"]
        else:
            version_command = command + ["--version"]
    if path is None or not path.is_file():
        return {"path": None, "version": None, "sha256": None}
    return {
        "path": str(path),
        "version": command_output(version_command),
        "sha256": sha256_file(path),
    }


def build_command(language: str, source: Path, binary: Path, tool: list[str], runtime: Path | None) -> list[str]:
    if language == "c":
        return tool + ["-O2", "-std=c11", str(source), "-o", str(binary)]
    if language == "cpp":
        return tool + ["-O2", "-std=c++17", str(source), "-o", str(binary)]
    if language == "rust":
        return tool + ["--edition=2021", "-O", "-C", "target-cpu=native", str(source), "-o", str(binary)]
    if language == "zig":
        return tool + ["build-exe", "-O", "ReleaseFast", "-mcpu=native", "-lc", str(source), f"-femit-bin={binary}"]
    if language == "go":
        return tool + ["build", "-trimpath", "-o", str(binary), str(source)]
    if language == "rey":
        if runtime is None:
            raise ValueError("runtime directory is unavailable")
        return tool + ["build", str(source), "-o", str(binary), "--runtime-dir", str(runtime)]
    raise ValueError(f"{language} has no separate build command")


def run_command(language: str, binary: Path, python: Path, size: int) -> list[str]:
    if language == "python":
        return [str(python), str(binary), str(size)]
    return [str(binary), str(size)]


def fixture_path(language: str, workload: str) -> Path:
    return HERE / "fixtures" / language / f"{workload}{EXTENSIONS[language]}"


def benchmark_one(
    language: str,
    workload: str,
    size: int,
    args: argparse.Namespace,
    tool: dict[str, object],
    tool_command: list[str],
    runtime: Path | None,
    rss_prefix: list[str] | None,
    rss_unit: str | None,
    run_id: str,
) -> dict[str, object]:
    source = fixture_path(language, workload)
    record: dict[str, object] = {
        "language": language,
        "workload": workload,
        "size": size,
        "source": str(source),
        "source_sha256": sha256_file(source),
        "toolchain": tool,
        "expected": expected_result(workload, size),
        "build": {"status": "not_applicable", "reason": None, "samples": [], "summary": None},
        "warmups": [],
        "runs": [],
        "run_summary": None,
        "status": "pending",
    }
    if not source.is_file():
        record["status"] = "unavailable"
        record["reason"] = f"fixture missing: {source}"
        return record
    if tool.get("path") is None:
        record["status"] = "unavailable"
        record["reason"] = "toolchain executable is unavailable"
        return record
    if language == "rey" and runtime is None:
        record["status"] = "unavailable"
        record["reason"] = "no valid Rey runtime directory"
        return record

    binary_dir = args.output_dir / "binaries" / run_id / language / workload
    binary_dir.mkdir(parents=True, exist_ok=True)
    binary: Path | None = None
    if language != "python":
        build_samples: list[dict[str, object]] = []
        for index in range(args.repeats):
            candidate = binary_dir / f"build-{index + 1:02d}"
            command = build_command(language, source, candidate, tool_command, runtime)
            sample = run_process(command, args.timeout, rss_prefix, rss_unit)
            sample["sample"] = index + 1
            sample["artifact"] = str(candidate)
            sample["binary_sha256"] = sha256_file(candidate) if sample["status"] == "ok" else None
            build_samples.append(sample)
            if sample["status"] == "ok":
                binary = candidate
        record["build"] = {
            "status": "ok" if all(sample["status"] == "ok" for sample in build_samples) else "failed",
            "cache_state": "toolchain default; caches were not cleared or classified as cold",
            "samples": build_samples,
            "summary": summary(build_samples),
        }
        if any(sample["status"] != "ok" for sample in build_samples):
            record["status"] = "build_failed"
            return record
    else:
        record["build"] = {
            "status": "not_applicable",
            "reason": "Python source is interpreted; no native build was timed",
            "samples": [],
            "summary": None,
        }
        binary = source

    assert binary is not None
    python = Path(str(tool["path"])) if language == "python" else Path(sys.executable)
    command = run_command(language, binary, python, size)
    expected = record["expected"]
    run_failed = False
    for index in range(args.warmups):
        sample = run_process(command, args.timeout, rss_prefix, rss_unit)
        sample["sample"] = index + 1
        sample["kind"] = "warmup"
        if sample["status"] != "ok":
            run_failed = True
        else:
            normalized = normalize_output(str(sample["stdout"]))
            sample["normalized_output"] = normalized
            if normalized != expected:
                sample["status"] = "output_mismatch"
                run_failed = True
        record["warmups"].append(sample)
        if run_failed:
            record["status"] = "warmup_failed"
            return record

    measured: list[dict[str, object]] = []
    for index in range(args.repeats):
        sample = run_process(command, args.timeout, rss_prefix, rss_unit)
        sample["sample"] = index + 1
        sample["kind"] = "measured"
        if sample["status"] == "ok":
            normalized = normalize_output(str(sample["stdout"]))
            sample["normalized_output"] = normalized
            if normalized != expected:
                sample["status"] = "output_mismatch"
        measured.append(sample)
        if sample["status"] != "ok":
            run_failed = True
    record["runs"] = measured
    record["run_summary"] = summary(measured)
    record["status"] = "run_failed" if run_failed else "complete"
    return record


def main() -> int:
    args = arguments()
    if args.repeats < 1:
        print("--repeats must be at least 1", file=sys.stderr)
        return 2
    for workload in args.workloads:
        size = DEFAULT_SIZES[workload] if args.size is None else args.size
        if size > SIZE_LIMITS[workload]:
            print(
                f"size {size} exceeds {workload} limit {SIZE_LIMITS[workload]} "
                "(churn is bounded for the current non-collecting Rey runtime)",
                file=sys.stderr,
            )
            return 2

    run_id = dt.datetime.now().astimezone().strftime("%Y%m%dT%H%M%S%z")
    args.output_dir = args.output_dir.expanduser().resolve()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    compilers = compiler_choices()
    rey_path, rey_selection_source, runtime = rey_selection(args)
    rss_prefix, rss_unit = time_wrapper()
    details: dict[str, dict[str, object]] = {}
    for language in args.languages:
        if language == "rey":
            info = tool_info(language, compilers, rey_path)
            info["selection"] = rey_selection_source
            info["runtime_dir"] = str(runtime) if runtime else None
            runtime_archive = runtime / "librey_rt.a" if runtime else None
            info["runtime_sha256"] = sha256_file(runtime_archive)
            if runtime_archive and not runtime_archive.is_file():
                info["path"] = None
                info["unavailable_reason"] = "selected runtime directory does not contain librey_rt.a"
            if rey_path is None or not rey_path.is_file():
                info["path"] = None
                info["unavailable_reason"] = "selected Rey executable is missing or not executable"
            details[language] = info
        elif language == "python":
            details[language] = tool_info(language, compilers, None)
        else:
            details[language] = tool_info(language, compilers, None)

    result_records = []
    for workload in args.workloads:
        size = DEFAULT_SIZES[workload] if args.size is None else args.size
        for language in args.languages:
            record = benchmark_one(
                language,
                workload,
                size,
                args,
                details[language],
                compilers.get(language, [str(rey_path)] if rey_path else []),
                runtime,
                rss_prefix,
                rss_unit,
                run_id,
            )
            result_records.append(record)
            print(f"{language:>6} {workload:<8} {record['status']}")

    payload = {
        "schema_version": 1,
        "created_at": dt.datetime.now().astimezone().isoformat(),
        "run_id": run_id,
        "source_revision": git_state(),
        "host": host_details(),
        "configuration": {
            "languages": args.languages,
            "workloads": args.workloads,
            "size_override": args.size,
            "default_sizes": DEFAULT_SIZES,
            "warmups": args.warmups,
            "repeats": args.repeats,
            "timeout_seconds": args.timeout,
            "optimization_policy": {
                "c": "clang/selected CC -O2 -std=c11",
                "cpp": "clang++/selected CXX -O2 -std=c++17",
                "rust": "rustc -O -C target-cpu=native",
                "zig": "zig ReleaseFast -mcpu=native",
                "go": "go build defaults",
                "python": "standard interpreter; build not applicable",
                "rey": "selected seed defaults",
            },
            "run_timing_includes_process_startup": True,
            "build_cache_policy": "current toolchain cache state; no cache clearing, cold-build, or no-op-build claim",
            "rss_measurement": {
                "wrapper": rss_prefix,
                "unit": rss_unit,
                "unavailable_reason": None if rss_prefix else "platform /usr/bin/time format not supported",
            },
        },
        "results": result_records,
        "interpretation": "Synthetic matched workloads only; results do not establish a universal language ranking.",
    }
    output_path = args.output_dir / f"results-{run_id}.json"
    temporary = output_path.with_suffix(".json.tmp")
    temporary.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    temporary.replace(output_path)
    print(f"results: {output_path}")
    return 0 if all(record["status"] in ("complete", "unavailable") for record in result_records) else 1


if __name__ == "__main__":
    raise SystemExit(main())
