#!/usr/bin/env python3
"""Run bounded, oracle-checked Rey v0.4 native benchmark comparisons."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import math
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
LANGUAGES = ("c", "cpp", "rust", "zig", "go", "rey", "python")
DEFAULT_LANGUAGES = ("c", "cpp", "rust", "zig", "go", "rey")
WORKLOADS = ("numeric", "churn", "parse", "stream")
DEFAULT_SIZES = {"numeric": 1_000_000, "churn": 256, "parse": 250_000, "stream": 1_000_000}
SIZE_LIMITS = {"numeric": 10_000_000, "churn": 4096, "parse": 5_000_000, "stream": 10_000_000}
VECTOR_WIDTH = 32
PARSE_TOKENS = ("11", "-23", "37", "-41", "53", "-67", "79", "-89")
OUTPUT_PATTERN = re.compile(r"checksum=(-?\d+) processed=(\d+)")
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
        raise argparse.ArgumentTypeError("must be non-negative") from None
    return parsed


def positive_float(value: str) -> float:
    try:
        parsed = float(value)
    except ValueError as exc:
        raise argparse.ArgumentTypeError("must be a number") from exc
    if parsed <= 0:
        raise argparse.ArgumentTypeError("must be greater than zero") from None
    return parsed


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--languages",
        default=",".join(DEFAULT_LANGUAGES),
        type=lambda value: parse_name_list(value, LANGUAGES, "--languages"),
        help="comma-separated language subset (default: c,cpp,rust,zig,go,rey)",
    )
    parser.add_argument(
        "--workloads",
        default=",".join(WORKLOADS),
        type=lambda value: parse_name_list(value, WORKLOADS, "--workloads"),
        help="comma-separated workload subset (default: numeric,churn,parse,stream)",
    )
    parser.add_argument("--size", type=positive_int, help="override each selected workload size")
    parser.add_argument("--repeats", type=positive_int, default=9, help="measured run samples")
    parser.add_argument("--warmups", type=positive_int, default=2, help="unmeasured workload warmups")
    parser.add_argument("--timeout", type=positive_float, default=120.0, help="per-process timeout in seconds")
    parser.add_argument("--rey", type=Path, help="native Rey compiler used for the Rey cell")
    parser.add_argument(
        "--rey-mode",
        choices=("native",),
        default="native",
        help="Rey backend mode; only direct native CLI builds are supported",
    )
    parser.add_argument(
        "--baseline-rey",
        type=Path,
        help="optional pinned native Rey compiler for paired build and run measurements",
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
    wrapper = ["nice", "-n", "19", "timeout", "--kill-after=5s", f"{timeout:g}s"]
    try:
        result = subprocess.run(wrapper + command, capture_output=True, text=True, timeout=timeout + 10, check=False)
    except (OSError, subprocess.TimeoutExpired):
        return None
    text = (result.stdout or result.stderr).strip()
    return text if result.returncode == 0 and text else None


def cpu_description() -> str | None:
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
        changed = subprocess.run(
            ["git", "-C", str(REPO_ROOT), "status", "--porcelain"],
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
        dirty = bool(changed.stdout.strip()) if changed.returncode == 0 else None
    except (OSError, subprocess.TimeoutExpired):
        dirty = None
    return {"revision": revision, "working_tree_dirty": dirty}


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
    if platform.system() == "Linux" and executable.is_file():
        return [str(executable), "-v", "--"], "KiB"
    return None, None


def decode_output(value: str | bytes | None) -> str:
    if value is None:
        return ""
    return value.decode(errors="replace") if isinstance(value, bytes) else value


def rss_value(stderr: str, unit: str | None) -> tuple[int | None, str]:
    if unit != "KiB":
        return None, "unavailable: GNU /usr/bin/time RSS format is unsupported"
    match = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", stderr)
    if not match:
        return None, "unavailable: time output did not contain peak RSS (KiB)"
    return int(match.group(1)), unit


def elapsed_time_value(stderr: str) -> float | None:
    match = re.search(r"Elapsed \(wall clock\) time \([^\n]+\):\s*([0-9:.]+)", stderr)
    if not match:
        return None
    value = 0.0
    for part in match.group(1).split(":"):
        value = value * 60.0 + float(part)
    return value


def add_budget_log_metrics(sample: dict[str, object], fallback_elapsed: float | None) -> None:
    combined = str(sample.get("stdout", "")) + "\n" + str(sample.get("stderr", ""))
    match = re.search(r"B01 budget log: ([^\r\n]+)", combined)
    log_path = Path(match.group(1)) if match else None
    log_text = ""
    if log_path is not None:
        try:
            log_text = log_path.read_text(encoding="utf-8", errors="replace")
            sample["budget_log"] = str(log_path)
        except OSError as exc:
            sample["budget_log_error"] = str(exc)
    elapsed = elapsed_time_value(log_text)
    peak_rss, peak_unit = rss_value(log_text, "KiB")
    sample["compiler_elapsed_seconds"] = elapsed if elapsed is not None else fallback_elapsed
    sample["compiler_elapsed_from_time"] = elapsed is not None
    sample["peak_rss"] = peak_rss
    sample["peak_rss_unit"] = peak_unit


def run_process(
    command: list[str], timeout: float, rss_prefix: list[str] | None, rss_unit: str | None
) -> dict[str, object]:
    bounded = ["nice", "-n", "19", "timeout", "--kill-after=30s", f"{timeout:g}s"] + command
    executed = (rss_prefix or []) + bounded
    started = time.perf_counter()
    try:
        result = subprocess.run(executed, capture_output=True, text=True, timeout=timeout + 35, check=False)
        elapsed = time.perf_counter() - started
        peak_rss, peak_rss_unit = rss_value(result.stderr, rss_unit)
        timed_out = result.returncode in (124, 137)
        return {
            "command": executed,
            "program_command": command,
            "elapsed_seconds": elapsed,
            "return_code": result.returncode,
            "status": "timeout" if timed_out else ("ok" if result.returncode == 0 else "nonzero_exit"),
            "stdout": result.stdout,
            "stderr": result.stderr,
            "peak_rss": peak_rss,
            "peak_rss_unit": peak_rss_unit,
        }
    except subprocess.TimeoutExpired as exc:
        return {
            "command": executed,
            "program_command": command,
            "elapsed_seconds": time.perf_counter() - started,
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


def summary(samples: list[dict[str, object]], field: str = "elapsed_seconds") -> dict[str, float | None]:
    values = [sample.get(field) for sample in samples]
    numeric = [float(value) for value in values if isinstance(value, (int, float))]
    if not numeric:
        return {"median": None, "mad": None, "min": None, "max": None}
    median = statistics.median(numeric)
    return {
        "median": median,
        "mad": statistics.median(abs(value - median) for value in numeric),
        "min": min(numeric),
        "max": max(numeric),
    }


def expected_result(workload: str, size: int) -> dict[str, int | str]:
    checksum = 0
    if workload == "numeric":
        state = 1
        for _ in range(size):
            state = (state * 48271) % 2147483647
            checksum += state % 1024
    elif workload == "churn":
        for round_index in range(size):
            checksum += sum((round_index * 17 + index * 13) % 1009 for index in range(VECTOR_WIDTH))
    elif workload == "parse":
        values = [int(token, 10) for token in PARSE_TOKENS]
        cycles, remainder = divmod(size, len(values))
        checksum = cycles * sum(values) + sum(values[:remainder])
    elif workload == "stream":
        state = 1
        for _ in range(size):
            state = (state * 48271) % 2147483647
            if state % 7 == 0:
                checksum = (checksum * 31 + state) % 2147483647
    else:
        raise ValueError(f"unknown workload: {workload}")
    processed = size * VECTOR_WIDTH if workload == "churn" else size
    return {
        "checksum": checksum,
        "processed": processed,
        "normalized": f"checksum={checksum} processed={processed}",
    }


def normalize_output(stdout: str) -> dict[str, int | str] | None:
    match = OUTPUT_PATTERN.fullmatch(stdout.strip())
    if not match:
        return None
    checksum, processed = int(match.group(1)), int(match.group(2))
    return {"checksum": checksum, "processed": processed, "normalized": f"checksum={checksum} processed={processed}"}


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


def resolve_rey(path: Path | None) -> tuple[Path | None, str]:
    if path is None:
        candidate = os.environ.get("REY_BIN")
        if candidate:
            path = Path(candidate)
            source = "REY_BIN"
        else:
            fallback = REPO_ROOT / "build" / "rey"
            path = fallback
            source = "build/rey fallback"
    else:
        source = "--rey" if path is not None else "unset"
    binary = path.expanduser().resolve()
    return (binary if binary.is_file() and os.access(binary, os.X_OK) else None), source


def tool_info(language: str, compilers: dict[str, list[str]], rey_path: Path | None, selection: str | None = None) -> dict[str, object]:
    if language == "rey":
        path = rey_path
        digest = sha256_file(path)
        queried = command_output([str(path), "--version"]) if path else None
        version = queried or (f"native compiler, sha256:{digest}" if digest else None)
        version_source = "--version" if queried else "content hash (binary does not expose a version flag)"
        return {
            "path": str(path) if path else None,
            "version": version,
            "version_source": version_source,
            "sha256": digest,
            "selection": selection,
        }
    command = compilers[language]
    resolved = shutil.which(command[0])
    path = Path(resolved).resolve() if resolved else None
    if language in ("zig", "go"):
        version_command = command + ["version"]
    else:
        version_command = command + ["--version"]
    queried = command_output(version_command) if path else None
    return {
        "path": str(path) if path else None,
        "version": queried.splitlines()[0] if queried else None,
        "version_output": queried,
        "version_source": "version command" if queried else "unavailable",
        "sha256": sha256_file(path),
        "selection": "environment or PATH",
    }


def fixture_path(language: str, workload: str) -> Path:
    return HERE / "fixtures" / language / f"{workload}{EXTENSIONS[language]}"


def build_command(language: str, source: Path, binary: Path, tool: list[str]) -> list[str]:
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
        return tool + ["build", str(source), "-o", str(binary), "--target", "linux-x64", "--regions", "--managed"]
    raise ValueError(f"{language} has no native build command")


def budget_build_command(command: list[str], timeout: float, label: str) -> list[str]:
    wrapper = REPO_ROOT / "scripts" / "v04-linux-x64-budget.sh"
    if not wrapper.is_file():
        raise FileNotFoundError(f"B01 build wrapper is required: {wrapper}")
    return [
        "sh",
        str(wrapper),
        "run",
        "--seconds",
        str(math.ceil(timeout)),
        "--label",
        label,
        "--",
        *command,
    ]


def run_command(language: str, binary: Path, workload: str, size: int) -> list[str]:
    arguments = [str(size)]
    if workload == "parse":
        arguments.extend(PARSE_TOKENS)
    if language == "python":
        return [sys.executable, str(binary), *arguments]
    return [str(binary), *arguments]


def validate_sample(sample: dict[str, object], expected: dict[str, int | str]) -> None:
    if sample["status"] != "ok":
        return
    normalized = normalize_output(str(sample.get("stdout", "")))
    sample["normalized_output"] = normalized
    if normalized != expected:
        sample["status"] = "output_mismatch"


def validate_rss_measurement(sample: dict[str, object]) -> None:
    if sample["status"] == "ok" and platform.system() == "Linux" and not isinstance(sample.get("peak_rss"), int):
        sample["status"] = "measurement_missing"
        sample["measurement_error"] = "GNU /usr/bin/time did not report peak RSS"


def benchmark_variant(
    language: str,
    workload: str,
    size: int,
    args: argparse.Namespace,
    tool: dict[str, object],
    tool_command: list[str],
    rss_prefix: list[str] | None,
    rss_unit: str | None,
    run_id: str,
    variant: str,
) -> dict[str, object]:
    source = fixture_path(language, workload)
    expected = expected_result(workload, size)
    record: dict[str, object] = {
        "language": language,
        "variant": variant,
        "workload": workload,
        "size": size,
        "source": str(source),
        "source_sha256": sha256_file(source),
        "toolchain": tool,
        "expected": expected,
        "build": {"status": "pending", "samples": [], "summary": None},
        "startup_probe": {"status": "pending", "samples": [], "summary": None},
        "warmups": [],
        "runs": [],
        "run_summary": None,
        "workload_summary": None,
        "rss_summary": None,
        "image_size_bytes": None,
        "binary_sha256": None,
        "status": "pending",
    }
    if not source.is_file():
        record.update(status="unavailable", reason=f"fixture missing: {source}")
        return record
    if tool.get("path") is None:
        record.update(status="unavailable", reason="toolchain executable is unavailable")
        return record

    output_component = "rey-baseline" if language == "rey" and variant == "baseline" else language
    binary_dir = args.output_dir / "binaries" / run_id / output_component / workload
    binary_dir.mkdir(parents=True, exist_ok=True)
    if language == "python":
        binary = source
        record["build"] = {
            "status": "not_applicable",
            "reason": "Python source is interpreted; no native build was timed",
            "samples": [],
            "summary": None,
        }
    else:
        build_samples: list[dict[str, object]] = []
        binary = None
        for index in range(args.repeats):
            candidate = binary_dir / f"build-{index + 1:02d}"
            compiler_command = build_command(language, source, candidate, tool_command)
            label = f"M01-{language}-{workload}-{variant}-{index + 1:02d}"
            measured_command = (rss_prefix or []) + compiler_command
            try:
                wrapped_command = budget_build_command(measured_command, args.timeout, label)
            except FileNotFoundError as exc:
                record["status"] = "build_wrapper_missing"
                record["reason"] = str(exc)
                return record
            contention_attempts = []
            for attempt in range(12):
                sample = run_process(wrapped_command, args.timeout, None, None)
                if sample.get("return_code") != 73:
                    break
                contention_attempts.append(
                    {"attempt": attempt + 1, "status": sample["status"], "stderr": sample.get("stderr", "")}
                )
                if attempt < 11:
                    time.sleep(5)
            sample["lock_contention_attempts"] = contention_attempts
            sample["sample"] = index + 1
            sample["compiler_command"] = compiler_command
            sample["compiler_measurement_command"] = measured_command
            sample["budget_wrapper_command"] = wrapped_command
            if sample.get("return_code") != 73:
                add_budget_log_metrics(sample, sample.get("elapsed_seconds"))
            if (
                sample["status"] == "ok"
                and platform.system() == "Linux"
                and (not sample.get("compiler_elapsed_from_time") or not isinstance(sample.get("peak_rss"), int))
            ):
                sample["status"] = "measurement_missing"
                sample["measurement_error"] = "B01 producer log did not contain GNU time wall time and peak RSS"
            sample["artifact"] = str(candidate)
            if sample["status"] == "ok" and (not candidate.is_file() or not os.access(candidate, os.X_OK)):
                sample["status"] = "missing_or_non_executable_image"
            sample["binary_sha256"] = sha256_file(candidate) if sample["status"] == "ok" else None
            sample["image_size_bytes"] = candidate.stat().st_size if sample["status"] == "ok" else None
            build_samples.append(sample)
            if sample["status"] == "ok":
                binary = candidate
        record["build"] = {
            "status": "ok" if len(build_samples) == args.repeats and all(sample["status"] == "ok" for sample in build_samples) else "failed",
            "cache_state": "toolchain default; caches were not cleared or classified as cold",
            "samples": build_samples,
            "summary": summary(build_samples, "compiler_elapsed_seconds"),
            "rss_summary": summary(
                [sample for sample in build_samples if isinstance(sample.get("peak_rss"), int)],
                "peak_rss",
            ),
            "reproducible_image_bytes": len({sample.get("binary_sha256") for sample in build_samples if sample.get("binary_sha256")}) <= 1,
        }
        if record["build"]["status"] != "ok" or binary is None:
            record["status"] = "build_failed"
            return record
        record["binary_sha256"] = sha256_file(binary)
        record["image_size_bytes"] = binary.stat().st_size

    startup_expected = expected_result(workload, 0)
    startup_samples: list[dict[str, object]] = []
    startup_repeats = max(3, args.warmups)
    startup_run_command = run_command(language, binary, workload, 0)
    for index in range(startup_repeats):
        sample = run_process(startup_run_command, args.timeout, rss_prefix, rss_unit)
        sample["sample"] = index + 1
        validate_sample(sample, startup_expected)
        validate_rss_measurement(sample)
        startup_samples.append(sample)
    startup_ok = all(sample["status"] == "ok" for sample in startup_samples)
    record["startup_probe"] = {
        "status": "ok" if startup_ok else "failed",
        "method": "median process wall time for the same executable and arguments with workload size zero",
        "samples": startup_samples,
        "summary": summary(startup_samples),
    }
    if not startup_ok:
        record["status"] = "startup_probe_failed"
        return record

    expected_startup = expected_result(workload, 0)
    workload_command = run_command(language, binary, workload, size)
    warmups: list[dict[str, object]] = []
    for index in range(args.warmups):
        sample = run_process(workload_command, args.timeout, rss_prefix, rss_unit)
        sample["sample"] = index + 1
        sample["kind"] = "warmup"
        validate_sample(sample, expected)
        validate_rss_measurement(sample)
        warmups.append(sample)
        if sample["status"] != "ok":
            record["warmups"] = warmups
            record["status"] = "warmup_failed"
            return record
    record["warmups"] = warmups

    measured: list[dict[str, object]] = []
    for index in range(args.repeats):
        sample = run_process(workload_command, args.timeout, rss_prefix, rss_unit)
        sample["sample"] = index + 1
        sample["kind"] = "measured"
        validate_sample(sample, expected)
        validate_rss_measurement(sample)
        measured.append(sample)
    record["runs"] = measured
    record["run_summary"] = summary(measured)
    startup_median = record["startup_probe"]["summary"]["median"]
    run_median = record["run_summary"]["median"]
    net_median = max(0.0, float(run_median) - float(startup_median)) if run_median is not None else None
    record["workload_summary"] = {
        "median_seconds": net_median,
        "method": "max(0, median full-run wall time minus median zero-size startup probe)",
    }
    rss_samples = [sample for sample in measured if isinstance(sample.get("peak_rss"), int)]
    record["rss_summary"] = summary(rss_samples, "peak_rss")
    record["status"] = "complete" if all(sample["status"] == "ok" for sample in measured) else "run_failed"
    return record


def compiler_options() -> dict[str, str]:
    return {
        "c": "selected CC -O2 -std=c11",
        "cpp": "selected CXX -O2 -std=c++17",
        "rust": "selected RUSTC --edition=2021 -O -C target-cpu=native",
        "zig": "selected ZIG build-exe -O ReleaseFast -mcpu=native -lc",
        "go": "selected GO build -trimpath (toolchain defaults)",
        "rey": "selected native Rey build SOURCE -o OUTPUT --target linux-x64 --regions --managed",
        "python": "selected Python interpreter; build not applicable",
    }


def baseline_comparison(candidate: dict[str, object], baseline: dict[str, object]) -> dict[str, object]:
    candidate_summary = candidate.get("workload_summary")
    baseline_summary = baseline.get("workload_summary")
    candidate_seconds = candidate_summary.get("median_seconds") if isinstance(candidate_summary, dict) else None
    baseline_seconds = baseline_summary.get("median_seconds") if isinstance(baseline_summary, dict) else None
    ratio = None
    change = None
    if isinstance(candidate_seconds, (int, float)) and isinstance(baseline_seconds, (int, float)) and baseline_seconds > 0:
        ratio = candidate_seconds / baseline_seconds
        change = (candidate_seconds - baseline_seconds) / baseline_seconds
    return {
        "candidate_workload_median_seconds": candidate_seconds,
        "baseline_workload_median_seconds": baseline_seconds,
        "candidate_over_baseline": ratio,
        "relative_change": change,
    }


def table_text(
    records: list[dict[str, object]],
    tools: dict[str, dict[str, object]],
    configuration: dict[str, object],
    host: dict[str, object],
) -> str:
    lines = [
        "# Rey v0.4 matched native benchmark results",
        "",
        f"Revision: `{configuration['source_revision']['revision']}`; run: `{configuration['run_id']}`.",
        f"Host: {host['system']} {host['release']} `{host['machine']}`; CPU: `{host['cpu']}`; logical CPUs: {host['logical_cpus']}.",
        f"Warmups: {configuration['warmups']}; recorded samples per cell: {configuration['repeats']}; process timeout: {configuration['timeout_seconds']} s.",
        "",
        "Startup is estimated as the median wall time for the same executable and arguments at workload size zero. Workload time is `max(0, full-run median - startup median)`; full-run wall time remains available in `results.json`.",
        "",
        "| Language | Workload | Size | Compile median (s) | Compile RSS median (KiB) | Startup median (s) | Workload median (s) | Run RSS median (KiB) | Image (bytes) | Baseline workload (s) | Candidate / baseline |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for record in records:
        build_summary = record.get("build", {}).get("summary") or {}
        build_rss = record.get("build", {}).get("rss_summary") or {}
        startup_summary = record.get("startup_probe", {}).get("summary") or {}
        run_rss = record.get("rss_summary") or {}
        comparison = record.get("baseline_comparison") or {}
        lines.append(
            "| {language} | {workload} | {size} | {compile} | {compile_rss} | {startup} | {workload_time} | {rss} | {image} | {base} | {ratio} |".format(
                language=record["language"],
                workload=record["workload"],
                size=record["size"],
                compile=fmt(build_summary.get("median")),
                compile_rss=fmt(build_rss.get("median")),
                startup=fmt(startup_summary.get("median")),
                workload_time=fmt((record.get("workload_summary") or {}).get("median_seconds")),
                rss=fmt(run_rss.get("median")),
                image=record.get("image_size_bytes") if record.get("image_size_bytes") is not None else "n/a",
                base=fmt(comparison.get("baseline_workload_median_seconds")),
                ratio=fmt(comparison.get("candidate_over_baseline")),
            )
        )
    lines.extend(["", "## Tool versions and options", ""])
    options = compiler_options()
    for language in configuration["languages"]:
        info = tools[language]
        lines.append(
            f"- `{language}`: {info.get('version') or 'unavailable'}; SHA-256 `{info.get('sha256') or 'unavailable'}`; options: `{options[language]}`."
        )
    if configuration.get("baseline_rey"):
        info = tools["rey-baseline"]
        lines.append(
            f"- `rey-baseline`: {info.get('version') or 'unavailable'}; SHA-256 `{info.get('sha256') or 'unavailable'}`; options: `{options['rey']}`."
        )
    lines.extend(
        [
            "",
            "## Measurement notes",
            "",
            "- Each native image is compiled `repeats` times and every successful image is directly executed; the final successful image supplies warmup and measured runs.",
            "- Every compiler command uses B01's build wrapper for the shared lock, available-memory check, command log, `nice -n 19`, and GNU `timeout`. `/usr/bin/time -v` records compiler wall time and peak RSS inside that wrapper. Program runs use `nice -n 19`, GNU `timeout`, and `/usr/bin/time -v` as well.",
            "- Numeric and stream sizes are iterations/records; churn sizes are vector construction rounds; parse size is the number of decimal tokens parsed and folded.",
            "- Parse inputs are the fixed eight signed decimal tokens listed in `results.json`; stream performs a constant-space generated-record filter and rolling fold without retaining the stream.",
            "- The Python checksum oracle is independent of every workload implementation. A cell is complete only if every build, startup probe, warmup, and measured output matches it.",
            "- Baseline comparisons use the same fixture bytes, sizes, compiler options, workload oracle, and host run. `candidate / baseline` below 1.0 means the candidate's estimated workload time is lower.",
            "",
        ]
    )
    return "\n".join(lines)


def fmt(value: object) -> str:
    return f"{float(value):.6f}" if isinstance(value, (float, int)) else "n/a"


def main() -> int:
    args = arguments()
    if args.repeats < 1:
        print("--repeats must be at least 1", file=sys.stderr)
        return 2
    if args.timeout > 2400:
        print("--timeout cannot exceed 2400 seconds", file=sys.stderr)
        return 2
    for workload in args.workloads:
        size = DEFAULT_SIZES[workload] if args.size is None else args.size
        if size > SIZE_LIMITS[workload]:
            print(f"size {size} exceeds {workload} limit {SIZE_LIMITS[workload]}", file=sys.stderr)
            return 2

    run_id = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    args.output_dir = args.output_dir.expanduser().resolve()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    compilers = compiler_choices()
    rey_path, rey_selection = resolve_rey(args.rey)
    baseline_rey_path = args.baseline_rey.expanduser().resolve() if args.baseline_rey else None
    if baseline_rey_path and (not baseline_rey_path.is_file() or not os.access(baseline_rey_path, os.X_OK)):
        baseline_rey_path = None

    tool_infos: dict[str, dict[str, object]] = {}
    for language in args.languages:
        tool_infos[language] = tool_info(
            language,
            compilers,
            rey_path if language == "rey" else None,
            rey_selection if language == "rey" else None,
        )
    if "rey" in args.languages and args.baseline_rey is not None:
        tool_infos["rey-baseline"] = tool_info("rey", compilers, baseline_rey_path, "--baseline-rey")

    rss_prefix, rss_unit = time_wrapper()
    records: list[dict[str, object]] = []
    for workload in args.workloads:
        size = DEFAULT_SIZES[workload] if args.size is None else args.size
        for language in args.languages:
            rey_compiler = rey_path if language == "rey" else None
            tool_command = [str(rey_compiler)] if language == "rey" and rey_compiler else compilers.get(language, [])
            candidate = benchmark_variant(
                language,
                workload,
                size,
                args,
                tool_infos[language],
                tool_command,
                rss_prefix,
                rss_unit,
                run_id,
                "candidate",
            )
            if language == "rey" and args.baseline_rey is not None:
                baseline_command = [str(baseline_rey_path)] if baseline_rey_path else []
                baseline = benchmark_variant(
                    "rey",
                    workload,
                    size,
                    args,
                    tool_infos["rey-baseline"],
                    baseline_command,
                    rss_prefix,
                    rss_unit,
                    run_id,
                    "baseline",
                )
                candidate["baseline"] = baseline
                candidate["baseline_comparison"] = baseline_comparison(candidate, baseline)
                if baseline["status"] != "complete" and candidate["status"] == "complete":
                    candidate["status"] = "baseline_failed"
            records.append(candidate)
            suffix = f" (baseline {candidate['baseline']['status']})" if candidate.get("baseline") else ""
            print(f"{language:>6} {workload:<8} {candidate['status']}{suffix}", flush=True)

    host = host_details()
    config: dict[str, object] = {
        "run_id": run_id,
        "languages": args.languages,
        "workloads": args.workloads,
        "sizes": {workload: DEFAULT_SIZES[workload] if args.size is None else args.size for workload in args.workloads},
        "size_override": args.size,
        "default_sizes": DEFAULT_SIZES,
        "size_limits": SIZE_LIMITS,
        "parse_tokens": PARSE_TOKENS,
        "warmups": args.warmups,
        "repeats": args.repeats,
        "timeout_seconds": args.timeout,
        "source_revision": git_state(),
        "candidate_rey": str(rey_path) if rey_path else None,
        "rey_mode": args.rey_mode,
        "baseline_rey": str(baseline_rey_path) if baseline_rey_path else None,
        "optimization_options": compiler_options(),
        "process_wrapper": ["nice", "-n", "19", "timeout", "--kill-after=30s"],
        "rss_measurement": {
            "wrapper": rss_prefix,
            "unit": rss_unit,
            "unavailable_reason": None if rss_prefix else "GNU /usr/bin/time not available",
        },
    }
    payload = {
        "schema_version": 2,
        "created_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "configuration": config,
        "host": host,
        "tools": tool_infos,
        "results": records,
        "interpretation": "Matched synthetic workloads with independent outputs; not a universal language ranking.",
    }
    result_path = args.output_dir / f"results-{run_id}.json"
    temporary = result_path.with_suffix(".json.tmp")
    temporary.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    temporary.replace(result_path)
    (args.output_dir / "table.md").write_text(table_text(records, tool_infos, config, host))

    valid = sum(record["status"] == "complete" for record in records)
    expected_cells = len(args.languages) * len(args.workloads)
    measured_total = sum(len(record.get("runs", [])) for record in records)
    skips = sum(record["status"] == "unavailable" for record in records)
    failures = expected_cells - valid - skips
    if args.warmups == 1 and args.repeats == 3 and expected_cells == 24 and valid == expected_cells:
        print(f"{valid}/{expected_cells} valid language/workload cells; 1 warmup and 3 recorded samples per cell ({measured_total} recorded total); zero skips")
    else:
        print(
            f"{valid}/{expected_cells} valid language/workload cells; {args.warmups} warmups and "
            f"{args.repeats} recorded samples per cell ({measured_total} recorded attempted); "
            f"{skips} unavailable, {failures} failed"
        )
    print(f"results: {result_path}")
    print(f"table: {args.output_dir / 'table.md'}")
    return 0 if valid == expected_cells else 1


if __name__ == "__main__":
    raise SystemExit(main())
