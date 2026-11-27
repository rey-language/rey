#!/usr/bin/env python3
"""Run R01 with the owner-approved uncapped build wrapper."""

import importlib.util
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import time


def load_gate(source):
    path = Path(source) / "tests/v04/linux-x64-region/run.py"
    spec = importlib.util.spec_from_file_location("rey_r01_region_gate", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def uncapped_prepare(self):
    require = self.gate_module.require
    require(platform.system() == "Linux" and platform.machine().lower() in ("x86_64", "amd64"),
            "R01 requires native Linux x64")
    require(self.engine.is_file() and os.access(self.engine, os.X_OK), "engine must be an executable native seed")
    require(self.source.is_dir() and (self.source / "rey-compiler/src").is_dir(),
            "source root is missing rey-compiler/src")
    try:
        self.out.relative_to(self.source)
        raise self.gate_module.GateFailure("evidence output must be outside the source tree")
    except ValueError:
        pass
    require(shutil.which("strace") is not None, "strace is required for producer exec auditing")
    require(not self.out.exists() or not any(self.out.iterdir()),
            "output directory must be fresh and empty: " + str(self.out))
    self.out.mkdir(parents=True, exist_ok=True)
    self.logs.mkdir(parents=True, exist_ok=True)
    self.cwd = self.out / "run-cwd"
    (self.cwd / "build/v04").mkdir(parents=True, exist_ok=True)
    self.results["source_revision"] = self.git_revision()
    self.results["runtime_source_hashes"] = self.gate_module.tree_hashes(self.source / "runtime-rey")
    self.runtime = self.out / "runtime-rey"
    shutil.copytree(self.source / "runtime-rey", self.runtime)
    self.results["staged_runtime_source_hashes"] = self.gate_module.tree_hashes(self.runtime)
    self.results["compiler_source_hashes"] = self.gate_module.tree_hashes(self.source / "rey-compiler")
    gate_dir = self.gate_module.GATE_DIR
    harness_files = (
        self.source / "scripts/v04-linux-x64-region-gate.sh",
        gate_dir / "run.py", gate_dir / "region-driver.rey", gate_dir / "ir-driver.rey",
        gate_dir / "runtime-evaluator-driver.rey", gate_dir / "layout-negative.rey",
        gate_dir / "raw-intrinsic.rey", gate_dir / "source-cases.json",
        gate_dir / "native-cases.json", gate_dir / "reclamation-cases.json",
        gate_dir / "static-cases.json", gate_dir / "function-cutoffs.json",
    )
    self.results["harness_hashes"] = {
        str(path.relative_to(self.source)): self.gate_module.digest_file(path) for path in harness_files
    }
    self.results["checkpoint_instrumentation"] = {
        "applies_only_to_native_fixture_copies": sorted(self.gate_module.CHECKPOINT_CASES),
        "evaluator_runs_use_original_sources": True,
    }
    self.results["manifests"] = self.validate_manifests()
    self.results["build_policy"] = "uncapped; timeout; per-command /usr/bin/time -v"
    self.results["resource_cap"] = None
    self.save()


def uncapped_command(self, label, argv, *, timeout=180, build=False, heavy=False, cwd=None,
                     expected_status=0, expected_stdout=None, expected_stderr=None, env=None,
                     resource_limit_mib=None):
    self.counter += 1
    slug = "%03d-%s" % (self.counter, self.gate_module.safe_name(label))
    stdout_path = self.logs / (slug + ".stdout")
    stderr_path = self.logs / (slug + ".stderr")
    status_path = self.logs / (slug + ".status")
    trace_path = self.logs / (slug + ".execve")
    time_path = self.logs / (slug + ".time")
    command = list(map(str, argv))
    producer = command[0]
    if build:
        banned = self.gate_module.FORBIDDEN_FLAGS.intersection(command)
        self.gate_module.require(not banned, "release-disqualifying compiler flags: " + ", ".join(sorted(banned)))
        command = ["strace", "-f", "-e", "trace=execve,execveat", "-o", str(trace_path), "--"] + command
    if resource_limit_mib is not None and not build:
        command = ["sh", "-c", 'ulimit -v "$1" || exit 125; shift; exec "$@"',
                   "uncapped-r01", str(resource_limit_mib), *command]
    effective_timeout = max(timeout, 7200) if build else timeout
    if build:
        command = ["/usr/bin/time", "-v", "-o", str(time_path),
                   "timeout", "--kill-after=30s", str(effective_timeout), *command]
    else:
        command = ["timeout", "--kill-after=30s", str(timeout), *command]
    run_env = dict(os.environ)
    if env:
        run_env.update(env)
    started = time.monotonic()
    try:
        completed = subprocess.run(command, cwd=str(cwd or self.cwd), env=run_env,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   timeout=effective_timeout + 40, check=False)
        stdout = completed.stdout
        stderr = completed.stderr
        status = completed.returncode
        error = None
    except (OSError, subprocess.TimeoutExpired) as exc:
        stdout = b""
        stderr = b""
        status = None
        error = str(exc)
    elapsed = time.monotonic() - started
    stdout_path.write_bytes(stdout)
    stderr_path.write_bytes(stderr)
    status_path.write_text(("" if status is None else str(status)) + "\n")
    item = {
        "label": label, "command": command, "cwd": str(cwd or self.cwd),
        "status": status, "expected_status": expected_status,
        "elapsed_seconds": round(elapsed, 6),
        "stdout_sha256": self.gate_module.digest_bytes(stdout),
        "stderr_sha256": self.gate_module.digest_bytes(stderr),
        "stdout_log": str(stdout_path), "stderr_log": str(stderr_path), "error": error,
    }
    if build:
        require_time = self.gate_module.require
        report = time_path.read_text(errors="replace") if time_path.is_file() else ""
        match = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", report)
        require_time(match is not None, label + ": /usr/bin/time -v peak RSS is missing")
        item["max_rss_kib"] = int(match.group(1))
        item["time_report"] = str(time_path)
        item["execve_trace"] = str(trace_path)
        self.results.setdefault("compiler_build_peaks", []).append({
            "label": label, "max_rss_kib": int(match.group(1)), "time_report": str(time_path),
        })
    self.results["commands"].append(item)
    self.save()
    self.gate_module.require(error is None, label + " failed to run: " + str(error))
    self.gate_module.require(status == expected_status,
                             label + ": expected status " + str(expected_status) + ", got " + str(status))
    if expected_stdout is not None:
        expected = expected_stdout if isinstance(expected_stdout, bytes) else expected_stdout.encode("utf-8")
        self.gate_module.checked_equal(stdout, expected, label + " stdout")
    if expected_stderr is not None:
        expected = expected_stderr if isinstance(expected_stderr, bytes) else expected_stderr.encode("utf-8")
        self.gate_module.checked_equal(stderr, expected, label + " stderr")
    if build:
        self.check_exec_trace(trace_path, label, producer)
    return item, stdout, stderr


def main():
    if len(sys.argv) != 4:
        raise SystemExit("usage: r01-uncapped.py ENGINE SOURCE OUT")
    engine, source, out = map(Path, sys.argv[1:])
    engine = engine.resolve(strict=True)
    source = source.resolve(strict=True)
    out = out.resolve()
    if out.exists():
        out = out / ("attempt-" + time.strftime("%Y%m%dT%H%M%SZ", time.gmtime()) + "-" + str(os.getpid()))
    gate_module = load_gate(source)
    gate_module.RegionGate.gate_module = gate_module
    gate_module.RegionGate.prepare = uncapped_prepare
    gate_module.RegionGate.command = uncapped_command
    gate = gate_module.RegionGate(engine, source, out)
    try:
        gate.run()
    except (gate_module.GateFailure, OSError, ValueError, KeyError, IndexError) as exc:
        print("FAIL region gate: " + str(exc), file=sys.stderr)
        raise SystemExit(1)
    counts = gate.results["counts"]
    print("%d/16 normal, %d/16 stress, %d/21 direct native, %d/5 reclamation native" % (
        counts["normal"], counts["stress"], counts["direct_native"], counts["reclamation_native"]))
    print("PASS region gate")
    for item in gate.results.get("compiler_build_peaks", []):
        print("compiler build peak RSS: %s %s KiB" % (item["label"], item["max_rss_kib"]))


if __name__ == "__main__":
    main()
