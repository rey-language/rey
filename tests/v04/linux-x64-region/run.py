#!/usr/bin/env python3
"""Current-source Linux x64 region evidence gate for v0.4."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import signal
import subprocess
import sys
import time


GATE_DIR = Path(__file__).resolve().parent
ROOT = GATE_DIR.parents[2]
EXPECTED_RUNTIME_FILES = (
    "layout/main.rey",
    "regions/main.rey",
    "managed/main.rey",
    "containers/main.rey",
    "task/main.rey",
    "host/main.rey",
)
RUNTIME_CASES = (
    "region-basic", "program-region", "group-separation",
    "promotion-address-identity", "promotion-older-source", "promotion-null",
    "promotion-dependency-chain", "promotion-cycle", "store-promotes-younger",
    "store-records-older", "destination-construction", "region-end-lifo",
    "region-end-program", "double-end", "context-handle-aba",
    "program-handle-aba", "container-current-region",
    "repeated-request-plateau", "long-lived-graph-retention", "task-lifecycle",
    "host-write",
)
RECLAMATION_CASES = (
    "reclaim-three-slabs", "direct-mapping-metrics", "opaque-handle-reuse",
    "active-context-destroy", "teardown-many-direct-chunks",
)
CHECKPOINT_CASES = {
    "region-end-lifo", "region-end-program", "double-end",
    "context-handle-aba", "program-handle-aba",
}
CHECKPOINT_TARGETS = {
    "region-end-lifo": ("managedRegionEnd(context, outer);", 1),
    "region-end-program": ("managedRegionEnd(context, managedProgramRegion(context));", 1),
    "double-end": ("managedRegionEnd(context, region);", 2),
    "context-handle-aba": ("if !regionHandleIsActive(second, stale) { hostExit(1); }", 1),
    "program-handle-aba": ("if !regionHandleIsActive(second, stale) { hostExit(1); }", 1),
}
FORBIDDEN_FLAGS = {
    "--selfhost-fast-exit", "--trust-packed-output", "--no-regions",
    "--experimental-escape-summary", "--experimental-packed-x64",
}


class GateFailure(RuntimeError):
    pass


def digest_bytes(data):
    return hashlib.sha256(data).hexdigest()


def digest_file(path):
    return digest_bytes(Path(path).read_bytes())


def tree_hashes(root):
    root = Path(root)
    return {
        str(path.relative_to(root)): digest_file(path)
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }


def write_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def read_json(path):
    try:
        return json.loads(Path(path).read_text())
    except (OSError, json.JSONDecodeError) as exc:
        raise GateFailure("cannot read manifest " + str(path) + ": " + str(exc))


def require(condition, message):
    if not condition:
        raise GateFailure(message)


def checked_equal(actual, expected, label):
    if actual != expected:
        raise GateFailure(label + ": expected " + repr(expected) + ", got " + repr(actual))


def safe_name(value):
    return "".join(ch if ch.isalnum() or ch in "-_." else "_" for ch in value)


class RegionGate:
    def __init__(self, engine, source, out):
        self.engine = Path(engine).resolve(strict=True)
        self.source = Path(source).resolve(strict=True)
        self.out = Path(out).resolve()
        self.logs = self.out / "logs"
        self.results = {
            "gate": "R01 current Linux x64 region gate",
            "engine": str(self.engine),
            "engine_sha256": digest_file(self.engine),
            "source": str(self.source),
            "platform": platform.platform(),
            "machine": platform.machine(),
            "permission_setup": "chmod is used only on generated test executables",
            "build_wrapper": str(self.source / "scripts/v04-linux-x64-budget.sh"),
            "commands": [],
            "tests": {},
        }
        self.counter = 0
        self.cwd = None
        self.runtime = None
        self.compiler_copy = None
        self.region_driver = None
        self.ir_driver = None
        self.fixture_paths = None

    def prepare(self):
        require(platform.system() == "Linux" and platform.machine().lower() in ("x86_64", "amd64"),
                "R01 requires native Linux x64")
        memory = subprocess.run(["free", "-g"], stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, timeout=10, check=False)
        require(memory.returncode == 0, "free -g failed before acceptance work")
        memory_lines = memory.stdout.decode("utf-8", "replace").splitlines()
        mem_row = next((line.split() for line in memory_lines if line.startswith("Mem:")), None)
        require(mem_row is not None and len(mem_row) >= 7,
                "free -g output has no available-memory field")
        available_gib = int(mem_row[6])
        require(available_gib >= 5,
                "available memory is below 5 GiB; no region gate builds may start")
        self.results["free_g_gib_available_before_gate"] = available_gib
        expected_lock = (Path.home() / "rey/build.lock").resolve()
        try:
            inherited_lock = Path(os.readlink("/proc/self/fd/3")).resolve()
        except OSError:
            inherited_lock = None
        require(inherited_lock == expected_lock,
                "R01 acceptance must hold the B01 global build lock for the whole gate")
        require(self.engine.is_file() and os.access(self.engine, os.X_OK),
                "engine must be an executable native seed")
        require(self.source.is_dir() and (self.source / "rey-compiler/src").is_dir(),
                "source root is missing rey-compiler/src")
        try:
            self.out.relative_to(self.source)
            raise GateFailure("evidence output must be outside the source tree")
        except ValueError:
            pass
        wrapper = self.source / "scripts/v04-linux-x64-budget.sh"
        require(wrapper.is_file(), "B01 budget wrapper is required: " + str(wrapper))
        require(shutil.which("strace") is not None, "strace is required for producer exec auditing")
        require(not self.out.exists() or not any(self.out.iterdir()),
                "output directory must be fresh and empty: " + str(self.out))
        self.out.mkdir(parents=True, exist_ok=True)
        self.logs.mkdir(parents=True, exist_ok=True)
        self.cwd = self.out / "run-cwd"
        (self.cwd / "build/v04").mkdir(parents=True, exist_ok=True)
        self.results["source_revision"] = self.git_revision()
        self.results["runtime_source_hashes"] = tree_hashes(self.source / "runtime-rey")
        self.runtime = self.out / "runtime-rey"
        shutil.copytree(self.source / "runtime-rey", self.runtime)
        self.results["staged_runtime_source_hashes"] = tree_hashes(self.runtime)
        self.results["compiler_source_hashes"] = tree_hashes(self.source / "rey-compiler")
        harness_files = (
            self.source / "scripts/v04-linux-x64-region-gate.sh",
            GATE_DIR / "run.py", GATE_DIR / "region-driver.rey",
            GATE_DIR / "layout-negative.rey", GATE_DIR / "raw-intrinsic.rey",
            GATE_DIR / "source-cases.json", GATE_DIR / "native-cases.json",
            GATE_DIR / "reclamation-cases.json", GATE_DIR / "static-cases.json",
            GATE_DIR / "function-cutoffs.json",
        )
        self.results["harness_hashes"] = {
            str(path.relative_to(self.source)): digest_file(path) for path in harness_files
        }
        self.results["checkpoint_instrumentation"] = {
            "applies_only_to_native_fixture_copies": sorted(CHECKPOINT_CASES),
            "evaluator_runs_use_original_sources": True,
        }
        self.results["manifests"] = self.validate_manifests()
        self.save()

    def git_revision(self):
        try:
            result = subprocess.run(["git", "-C", str(self.source), "rev-parse", "HEAD"],
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                    check=False, timeout=10)
            if result.returncode == 0:
                return result.stdout.decode("utf-8", "replace").strip()
        except (OSError, subprocess.TimeoutExpired):
            pass
        return "unavailable"

    def validate_manifests(self):
        source_cases = read_json(GATE_DIR / "source-cases.json")
        source_baseline = read_json(self.source / "tests/v04/region-source/cases.json")
        direct_cases = read_json(GATE_DIR / "native-cases.json")
        direct_baseline = read_json(self.source / "tests/v04/region-runtime/cases.json")
        reclamation_cases = read_json(GATE_DIR / "reclamation-cases.json")
        reclamation_baseline = read_json(self.source / "tests/v04/region-runtime-reclamation/cases.json")
        static_cases = read_json(GATE_DIR / "static-cases.json")
        cutoff_cases = read_json(GATE_DIR / "function-cutoffs.json")

        require(set(source_cases) == set(source_baseline),
                "source manifest does not cover every current region-source ID exactly once")
        for case, spec in source_baseline.items():
            if spec.get("native_only"):
                require(isinstance(spec.get("reason"), str) and bool(spec["reason"].strip()),
                        "native-only evaluator policy needs a reason: " + case)
            else:
                require(case == "container-cycle.rey",
                        "unexpected evaluator-enabled source case: " + case)
        require(set(direct_cases) == set(direct_baseline) == set(RUNTIME_CASES),
                "native region manifest must exactly match all 21 current direct case IDs")
        require(set(reclamation_cases) == set(reclamation_baseline) == set(RECLAMATION_CASES),
                "reclamation manifest must exactly match all five current case IDs")
        require(set(cutoff_cases) == {"511", "512", "513"},
                "function cutoff manifest must cover 511, 512, and 513")
        require(set(static_cases) == {"managed-regions", "managed-lifetime-safety", "layout-negative"},
                "static verifier manifest is incomplete")
        codes = {name: spec for name, spec in direct_baseline.items()}
        for name, spec in direct_cases.items():
            require(name in codes, "unknown direct case: " + name)
            expected = spec["evaluator"]
            if expected == "native-only":
                require(name in ("host-write", "context-handle-aba", "program-handle-aba"),
                        "unexpected native-only evaluator policy: " + name)
            else:
                require(expected == codes[name],
                        "evaluator code mismatch between direct and legacy manifests: " + name)
            require(type(spec.get("status")) is int and isinstance(spec.get("stdout"), str)
                    and isinstance(spec.get("stderr"), str), "invalid native expectations for " + name)
            if spec.get("checkpoint"):
                require(name in CHECKPOINT_CASES and spec["stdout"] == "checkpoint: " + name + "\n",
                        "negative native fixture lacks its exact pre-failure checkpoint: " + name)
        require(set(direct_baseline) == set(RUNTIME_CASES),
                "tests/v04/region-runtime/cases.json is missing current direct case IDs")
        self.results["manifest_hashes"] = {
            str(path.relative_to(self.source)): digest_file(path)
            for path in (
                GATE_DIR / "source-cases.json", GATE_DIR / "native-cases.json",
                GATE_DIR / "reclamation-cases.json", GATE_DIR / "static-cases.json",
                GATE_DIR / "function-cutoffs.json",
                self.source / "tests/v04/region-source/cases.json",
                self.source / "tests/v04/region-runtime/cases.json",
                self.source / "tests/v04/region-runtime-reclamation/cases.json",
            )
        }
        return {
            "source": len(source_cases), "direct_native": len(direct_cases),
            "reclamation": len(reclamation_cases), "cutoffs": sorted(cutoff_cases),
            "static": sorted(static_cases),
        }

    def save(self):
        write_json(self.out / "results.json", self.results)

    def command(self, label, argv, *, timeout=180, build=False, heavy=False, cwd=None,
                expected_status=0, expected_stdout=None, expected_stderr=None, env=None,
                resource_limit_mib=None):
        self.counter += 1
        slug = "%03d-%s" % (self.counter, safe_name(label))
        stdout_path = self.logs / (slug + ".stdout")
        stderr_path = self.logs / (slug + ".stderr")
        status_path = self.logs / (slug + ".status")
        cgroup_path = self.logs / (slug + ".cgroup")
        started_path = self.logs / (slug + ".started")
        wrapper_stdout_path = self.logs / (slug + ".budget.stdout")
        wrapper_stderr_path = self.logs / (slug + ".budget.stderr")
        wave1_command_log = Path.home() / "rey/wave1/logs" / (
            "R01-" + safe_name(self.out.name) + "-" + slug + "-command-output.log")
        trace_path = self.logs / (slug + ".execve")
        command = list(map(str, argv))
        producer = command[0]
        if build:
            banned = FORBIDDEN_FLAGS.intersection(command)
            require(not banned, "release-disqualifying compiler flags in command: " + ", ".join(sorted(banned)))
            command = ["strace", "-f", "-e", "trace=execve,execveat", "-o", str(trace_path), "--"] + command
            wrapper = self.source / "scripts/v04-linux-x64-budget.sh"
            capture = (
                'out=$1; err=$2; result=$3; evidence=$4; started=$5; transcript=$6; '
                'cap=$7; seconds=$8; mode=$9; shift 9; '
                'cap_gib=${cap%G}; '
                'case "$cap_gib" in ""|*[!0-9]*) exit 125 ;; esac; '
                'cg_rel=$(sed -n "s/^0:://p" /proc/self/cgroup); '
                '[ -n "$cg_rel" ] || exit 125; '
                'cg_dir=/sys/fs/cgroup$cg_rel; '
                '[ -r "$cg_dir/memory.max" ] && [ -r "$cg_dir/memory.swap.max" ] || exit 125; '
                'mem_max=$(cat "$cg_dir/memory.max"); swap_max=$(cat "$cg_dir/memory.swap.max"); '
                'cap_bytes=$((cap_gib * 1073741824)); swap_bytes=$((512 * 1048576)); '
                'case "$mem_max:$swap_max" in *[!0-9:]*|max:*|*:max) exit 125 ;; esac; '
                '[ "$mem_max" -gt 0 ] && [ "$mem_max" -le "$cap_bytes" ] || exit 125; '
                '[ "$swap_max" -ge 0 ] && [ "$swap_max" -le "$swap_bytes" ] || exit 125; '
                '{ printf "scope=%s\\ncgroup=%s\\nmemory.max=%s\\nmemory.swap.max=%s\\n" "$mode" "$cg_rel" "$mem_max" "$swap_max"; '
                'printf "memory.peak.before=%s\\nmemory.events.before:\\n" "$(cat "$cg_dir/memory.peak")"; '
                'cat "$cg_dir/memory.events"; } >"$evidence"; '
                ': >"$started"; '
                'nice -n 19 timeout --kill-after=30s "$seconds" "$@" >"$out" 2>"$err"; status=$?; '
                '{ printf "memory.peak.after=%s\\nmemory.events.after:\\n" "$(cat "$cg_dir/memory.peak")"; '
                'cat "$cg_dir/memory.events"; printf "command_status=%s\\n" "$status"; } >>"$evidence"; '
                '{ printf "stdout:\\n"; cat "$out"; printf "\\nstderr:\\n"; cat "$err"; } >"$transcript"; '
                'printf "%s\\n" "$status" >"$result"; exit "$status"'
            )
            launcher = (
                'capture=$1; out=$2; err=$3; result=$4; evidence=$5; started=$6; '
                'transcript=$7; seconds=$8; shift 8; cap=${REY_CGROUP_CAP:-}; '
                'case "$cap" in *G) ;; *) echo "budget wrapper supplied no cgroup cap" >&2; exit 125 ;; esac; '
                'run_scope() { mode=$1; shift; '
                'if [ "$mode" = user ]; then set -- systemd-run --user --scope --quiet --pipe '
                '-p "MemoryMax=$cap" -p MemorySwapMax=512M -- sh -c "$capture" region-gate '
                '"$out" "$err" "$result" "$evidence" "$started" "$transcript" "$cap" "$seconds" user "$@"; '
                'else set -- sudo -n /usr/bin/systemd-run --system --scope --quiet --pipe '
                '-p "MemoryMax=$cap" -p MemorySwapMax=512M -- sh -c "$capture" region-gate '
                '"$out" "$err" "$result" "$evidence" "$started" "$transcript" "$cap" "$seconds" system "$@"; fi; '
                '"$@"; }; '
                'set +e; run_scope user "$@"; scope_status=$?; set -e; '
                '[ ! -f "$started" ] || exit "$scope_status"; '
                'set +e; run_scope system "$@"; scope_status=$?; set -e; exit "$scope_status"'
            )
            inner_timeout = max(1, timeout - 30)
            wrapped = ["sh", str(wrapper), "run-held", "--seconds", str(timeout),
                       "--label", slug, "--", "sh", "-c", launcher, "region-gate",
                       capture, str(stdout_path), str(stderr_path), str(status_path),
                       str(cgroup_path), str(started_path), str(wave1_command_log),
                       str(inner_timeout)] + command
            command = wrapped
        else:
            if resource_limit_mib is not None:
                shell = "ulimit -v " + str(resource_limit_mib * 1024) + " || exit 125; exec \"$@\""
                command = ["sh", "-c", shell, "region-gate", *command]
            command = ["nice", "-n", "19", "timeout", "--kill-after=30s", str(timeout), *command]

        run_env = dict(os.environ)
        if env:
            run_env.update(env)
        started = time.monotonic()
        error = None
        status = None
        try:
            output_stream = wrapper_stdout_path.open("wb") if build else stdout_path.open("wb")
            error_stream = wrapper_stderr_path.open("wb") if build else stderr_path.open("wb")
            proc = subprocess.Popen(command, cwd=str(cwd or self.cwd), env=run_env,
                                    stdout=output_stream, stderr=error_stream,
                                    start_new_session=True, pass_fds=(3,) if build else ())
            deadline = started + timeout + 45
            next_memory_check = started
            while proc.poll() is None:
                now = time.monotonic()
                if now >= deadline:
                    error = "outer timeout"
                elif now >= next_memory_check:
                    try:
                        memory = subprocess.run(["free", "-g"], stdout=subprocess.PIPE,
                                                stderr=subprocess.PIPE, timeout=10, check=False)
                        lines = memory.stdout.decode("utf-8", "replace").splitlines()
                        mem_row = next((line.split() for line in lines if line.startswith("Mem:")), None)
                        available = int(mem_row[6]) if memory.returncode == 0 and mem_row and len(mem_row) >= 7 else None
                        if available is None:
                            error = "free -g failed while a command was running"
                        elif available < 3:
                            error = "available memory fell below 3 GiB while a command was running"
                    except (OSError, subprocess.TimeoutExpired, ValueError):
                        error = "could not read free -g while a command was running"
                    next_memory_check = now + 5
                if error is not None:
                    try:
                        os.killpg(proc.pid, signal.SIGTERM)
                    except ProcessLookupError:
                        pass
                    try:
                        proc.wait(timeout=30)
                    except subprocess.TimeoutExpired:
                        try:
                            os.killpg(proc.pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass
                    proc.wait()
                    break
                time.sleep(0.5)
            status = proc.returncode
            output_stream.close()
            error_stream.close()
        except OSError as exc:
            error = str(exc)
        stdout = b""
        stderr = b""
        elapsed = time.monotonic() - started
        wrapper_stdout = stdout
        wrapper_stderr = stderr
        if build:
            wrapper_stdout = wrapper_stdout_path.read_bytes() if wrapper_stdout_path.exists() else b""
            wrapper_stderr = wrapper_stderr_path.read_bytes() if wrapper_stderr_path.exists() else b""
            wrapper_stdout_path.write_bytes(wrapper_stdout)
            wrapper_stderr_path.write_bytes(wrapper_stderr)
            if status_path.is_file():
                status_text = status_path.read_text().strip()
                require(status_text.isdigit(), label + ": invalid child status record")
                child_status = int(status_text)
                if error is None:
                    require(status == child_status,
                            label + ": budget wrapper status disagrees with child status")
                status = child_status
                stdout = stdout_path.read_bytes() if stdout_path.is_file() else b""
                stderr = stderr_path.read_bytes() if stderr_path.is_file() else b""
            else:
                error = error or "budget wrapper exited without a child status record"
                stdout, stderr = b"", b""
            wrapper_text = (wrapper_stdout + wrapper_stderr).decode("utf-8", "replace")
            match = re.search(r"B01 budget log: ([^\r\n]+)", wrapper_text)
            if not match:
                match = re.search(r"log: ([^\r\n]+)\)", wrapper_text)
            budget_log = match.group(1) if match else None
        else:
            budget_log = None
        stdout_path.write_bytes(stdout)
        stderr_path.write_bytes(stderr)
        item = {
            "label": label, "command": command, "cwd": str(cwd or self.cwd),
            "status": status, "expected_status": expected_status,
            "elapsed_seconds": round(elapsed, 6), "stdout_sha256": digest_bytes(stdout),
            "stderr_sha256": digest_bytes(stderr), "stdout_log": str(stdout_path),
            "stderr_log": str(stderr_path), "error": error,
        }
        if build:
            item["execve_trace"] = str(trace_path)
            item["budget_wrapper_stdout_log"] = str(wrapper_stdout_path)
            item["budget_wrapper_stderr_log"] = str(wrapper_stderr_path)
            item["budget_log"] = budget_log
            item["cgroup_evidence"] = str(cgroup_path)
            item["wave1_command_log"] = str(wave1_command_log)
        self.results["commands"].append(item)
        self.save()
        require(error is None, label + " failed to run: " + str(error))
        require(status == expected_status,
                label + ": expected status " + str(expected_status) + ", got " + str(status)
                + (" (ABA status 77 is inconclusive and rejected)" if status == 77 else ""))
        if expected_stdout is not None:
            expected_bytes = expected_stdout if isinstance(expected_stdout, bytes) else expected_stdout.encode("utf-8")
            checked_equal(stdout, expected_bytes, label + " stdout")
        if expected_stderr is not None:
            expected_bytes = expected_stderr if isinstance(expected_stderr, bytes) else expected_stderr.encode("utf-8")
            checked_equal(stderr, expected_bytes, label + " stderr")
        if build:
            self.check_exec_trace(trace_path, label, producer)
        return item, stdout, stderr

    def check_exec_trace(self, trace_path, label, producer):
        lines = Path(trace_path).read_text(errors="replace").splitlines()
        exec_lines = [line for line in lines if "execve(" in line or "execveat(" in line]
        expected = str(Path(producer).resolve())
        paths_by_line = [(line, re.findall(r'execve(?:at)?\("([^\"]+)"', line))
                         for line in exec_lines]
        observed = [line for line in exec_lines if " = 0" in line]
        require(len(observed) == 1 and expected in observed[0],
                label + ": producer exec trace has an unexpected command: " + repr(exec_lines))
        forbidden = {"clang", "clang++", "cc", "gcc", "g++", "llc", "rustc", "cargo", "python", "python2", "python3", "zig", "go", "ld", "ld.lld", "as", "llvm-as", "llvm-objcopy"}
        for line, paths in paths_by_line:
            for path in paths:
                executable = Path(path).name.lower()
                if executable in forbidden or executable.startswith(("clang-", "clang++-", "python3.", "llvm-", "ld.lld-")):
                    raise GateFailure(label + ": forbidden tool exec attempt in trace: " + line)
                if path != expected:
                    raise GateFailure(label + ": unexpected exec attempt in trace: " + line)

    def compiler_stage(self, name, entry):
        stage = self.out / "drivers" / name / "compiler-src"
        stage.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(self.source / "rey-compiler/src", stage)
        shutil.copyfile(entry, stage / "main.rey")
        return stage

    def build_executable(self, label, source_file, output, *, runtime_root=None, heavy=True):
        command = [str(self.engine), "build", str(source_file), "-o", str(output),
                   "--target", "linux-x64", "--managed", "--regions"]
        if runtime_root is not None:
            command += ["--runtime-root", str(runtime_root)]
        self.command(label, command, timeout=600, build=True, heavy=heavy)
        require(Path(output).is_file(), label + ": compiler reported success without an image")
        # Permission changes here apply only to generated test drivers.
        Path(output).chmod(Path(output).stat().st_mode | 0o111)
        return Path(output)

    def build_drivers(self):
        out_bin = self.out / "bin"
        out_bin.mkdir(parents=True, exist_ok=True)
        driver_source = self.compiler_stage("region-driver", GATE_DIR / "region-driver.rey")
        self.region_driver = self.build_executable(
            "build-current-source-region-driver", driver_source / "main.rey",
            out_bin / "region-driver", runtime_root=self.runtime, heavy=True)

        self.ir_driver = self.build_executable(
            "build-current-source-ir-driver", self.source / "rey-compiler/ir-main.rey",
            out_bin / "ir-driver", runtime_root=self.runtime, heavy=True)
        self.results["drivers"] = {
            "region": {"path": str(self.region_driver), "sha256": digest_file(self.region_driver)},
            "ir": {"path": str(self.ir_driver), "sha256": digest_file(self.ir_driver)},
        }
        self.save()

    def build_and_run_static(self):
        expectations = read_json(GATE_DIR / "static-cases.json")
        source_entries = {
            "managed-regions": self.source / "tests/v04/managed-regions/main.rey",
            "managed-lifetime-safety": self.source / "tests/v04/managed-lifetime-safety/main.rey",
            "layout-negative": GATE_DIR / "layout-negative.rey",
        }
        records = {}
        for name, entry in source_entries.items():
            stage = self.compiler_stage("static-" + name, entry)
            binary = self.build_executable("build-static-" + name, stage / "main.rey",
                                           self.out / "bin" / ("static-" + name),
                                           runtime_root=self.runtime, heavy=True)
            item, stdout, stderr = self.command("run-static-" + name, [str(binary)], timeout=300)
            expected = expectations[name]
            checked_equal(stdout, expected["stdout"].encode(), name + " stdout")
            checked_equal(stderr, expected["stderr"].encode(), name + " stderr")
            records[name] = {"status": item["status"], "stdout": stdout.decode(),
                             "stderr": stderr.decode(), "sha256": digest_file(binary)}
        self.results["tests"]["static_verifiers"] = records
        self.save()

    def run_evaluator_source_case(self):
        case = "container-cycle.rey"
        source = self.source / "tests/v04/region-source/fixtures" / case
        for mode, flags in (("normal", []), ("stress", ["--promotion-stress"])):
            command = [str(self.ir_driver), "eval", str(source), "--managed", "--regions",
                       "--runtime-root", str(self.runtime)] + flags
            item, stdout, stderr = self.command(
                "evaluator-" + mode + "-" + case, command, timeout=600,
                build=True, heavy=False, expected_stdout=b"void\n", expected_stderr=b"")
            self.results["tests"].setdefault("evaluator", {})[mode] = {
                "case": case, "status": item["status"],
                "stdout_sha256": item["stdout_sha256"], "stderr_sha256": item["stderr_sha256"],
            }
            self.save()

    def run_raw_intrinsic_rejection(self):
        source = GATE_DIR / "raw-intrinsic.rey"
        command = [str(self.ir_driver), "check", str(source)]
        item, stdout, stderr = self.command(
            "ordinary-source-raw-intrinsic-rejection", command, timeout=300,
            build=True, heavy=False, expected_status=1, expected_stdout=b"")
        diagnostic = stderr.decode("utf-8", "replace")
        require("__reyRawMap" in diagnostic and "unknown function" in diagnostic,
                "ordinary source must reject raw intrinsic names with the expected diagnostic")
        self.results["tests"]["raw_intrinsic_rejection"] = {
            "status": item["status"], "stdout_sha256": item["stdout_sha256"],
            "stderr_sha256": item["stderr_sha256"], "source_sha256": digest_file(source),
            "diagnostic": diagnostic,
        }
        self.save()

    def generated_cutoff_source(self, count):
        directory = self.out / "cutoffs"
        directory.mkdir(parents=True, exist_ok=True)
        path = directory / ("functions-" + str(count) + ".rey")
        functions = ["func cutoff_%03d(): void { cutoff_%03d(); }" % (i, i + 1)
                     for i in range(count - 2)]
        functions.append("func cutoff_%03d(): void {}" % (count - 2))
        functions.append("func main(): void { cutoff_000(); }")
        require(len(functions) == count, "cutoff source generator produced the wrong function count")
        path.write_text("\n".join(functions) + "\n")
        return path

    def run_function_cutoffs(self):
        expected = read_json(GATE_DIR / "function-cutoffs.json")
        observations = {}
        for count_text, output in expected.items():
            count = int(count_text)
            source = self.generated_cutoff_source(count)
            actual_count = sum(1 for line in source.read_text().splitlines()
                               if line.startswith("func "))
            require(actual_count == count, "generated cutoff fixture function count mismatch")
            trace_before = len(self.results["commands"])
            command = [str(self.ir_driver), "dump", str(source), "--managed", "--regions",
                       "--runtime-root", str(self.runtime)]
            item, stdout, stderr = self.command(
                "function-count-" + count_text, command, timeout=600, build=True,
                heavy=True, expected_status=output["status"],
                expected_stderr=output["stderr"].encode())
            dump_text = stdout.decode("utf-8", "replace")
            dumped_names = re.findall(r"(?m)^func %f\d+ [^\n]*::(cutoff_\d{3}) \(",
                                      dump_text)
            expected_names = ["cutoff_%03d" % index for index in range(count - 1)]
            require(len(dumped_names) == count - 1 and sorted(dumped_names) == expected_names,
                    "managed dump omitted or duplicated a cutoff function at " + count_text)
            require(re.search(r"(?m)^func %f\d+ [^\n]*::main \(", dump_text) is not None,
                    "managed dump omitted the user entry function at " + count_text)
            observations[count_text] = {
                "input_function_count": actual_count,
                "user_functions_dumped": len(dumped_names) + 1,
                "compiler_command_index": trace_before,
                "status": item["status"],
                "source_sha256": digest_file(source),
                "dump_sha256": digest_bytes(stdout),
            }
        self.results["tests"]["function_cutoffs"] = observations
        self.save()

    def run_region_sources(self):
        specs = read_json(GATE_DIR / "source-cases.json")
        baseline = read_json(self.source / "tests/v04/region-source/cases.json")
        directory = self.out / "images/region-source"
        directory.mkdir(parents=True, exist_ok=True)
        records = {"normal": {}, "stress": {}}
        for case, expected in specs.items():
            source = self.source / "tests/v04/region-source/fixtures" / case
            require(source.is_file(), "missing region-source fixture: " + case)
            base = baseline[case]
            for mode, flags in (("normal", []), ("stress", ["--promotion-stress"])):
                image = directory / (mode + "-" + case[:-4])
                command = [str(self.engine), "build", str(source), "-o", str(image),
                           "--target", "linux-x64", "--managed", "--regions",
                           "--runtime-root", str(self.runtime)] + flags
                self.command("build-" + mode + "-" + case, command, timeout=600,
                             build=True, heavy=False)
                require(image.is_file(), "compiler reported success without native fixture image: " + case)
                # This is test-fixture setup only; it does not repair a release artifact.
                image.chmod(image.stat().st_mode | 0o111)
                item, stdout, stderr = self.command(
                    "run-" + mode + "-" + case, [str(image)], timeout=300,
                    expected_status=expected["status"], expected_stdout=expected["stdout"].encode(),
                    expected_stderr=expected["stderr"].encode())
                records[mode][case] = {
                    "status": item["status"], "stdout_sha256": item["stdout_sha256"],
                    "stderr_sha256": item["stderr_sha256"], "image_sha256": digest_file(image),
                    "source_sha256": digest_file(source),
                    "evaluator_policy": ("native-only: " + base["reason"] if base.get("native_only") else "evaluated"),
                }
        require(len(records["normal"]) == 16 and len(records["stress"]) == 16,
                "region-source matrix did not execute exactly 16 cases in both modes")
        self.results["tests"]["region_source"] = records
        self.save()

    def prepare_runtime_fixtures(self):
        source = self.out / "fixtures/region-runtime"
        staged = self.out / "fixtures/region-runtime-staged"
        reclamation = self.out / "fixtures/reclamation"
        for path in (source, staged, reclamation):
            path.mkdir(parents=True, exist_ok=True)
        base = self.source / "tests/v04/region-runtime/fixtures"
        for name in RUNTIME_CASES:
            src = base / (name + ".rey")
            require(src.is_file(), "missing direct region fixture: " + str(src))
            shutil.copyfile(src, source / src.name)
            staged_text = src.read_text()
            if name in CHECKPOINT_CASES:
                marker, occurrence = CHECKPOINT_TARGETS[name]
                require(staged_text.count(marker) == occurrence,
                        "expected pre-failure site changed in " + name)
                position = staged_text.rfind(marker)
                checkpoint = 'println("checkpoint: ' + name + '");\n    '
                staged_text = staged_text[:position] + checkpoint + staged_text[position:]
            (staged / src.name).write_text(staged_text)
        for name in RECLAMATION_CASES:
            src = self.source / "tests/v04/region-runtime-reclamation/fixtures" / (name + ".rey")
            require(src.is_file(), "missing reclamation fixture: " + str(src))
            shutil.copyfile(src, reclamation / src.name)
        return source, staged, reclamation

    def run_direct_runtime(self):
        direct = read_json(GATE_DIR / "native-cases.json")
        source_fixtures, staged_fixtures, _ = self.prepare_runtime_fixtures()
        self.fixture_paths = (source_fixtures, staged_fixtures, _)
        direct_codes = read_json(self.source / "tests/v04/region-runtime/cases.json")
        source_dir = self.out / "drivers/runtime-source"
        for relative in EXPECTED_RUNTIME_FILES:
            target = source_dir / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(self.source / "runtime-rey" / relative, target)
        (source_dir / "main.rey").write_text("")
        generated = self.out / "images/direct-region"
        generated.mkdir(parents=True, exist_ok=True)
        records = {}
        for name, expected in direct.items():
            command = [str(self.region_driver), "region", str(source_fixtures),
                       str(staged_fixtures), str(source_dir), str(generated), name]
            item, stdout, stderr = self.command("emit-direct-" + name, command, timeout=300,
                                                build=True, heavy=False)
            checked_equal(stdout, ("region-runtime-case " + name + " "
                                   + str(direct_codes[name]) + "\n").encode(), name + " driver output")
            image = generated / ("linux-x64-" + name)
            require(image.is_file(), name + ": fixture driver did not emit a Linux x64 image")
            # The generated image is test-only; setup permission is not a compiler release-path check.
            image.chmod(image.stat().st_mode | 0o111)
            item, native_stdout, native_stderr = self.command(
                "run-direct-" + name, [str(image)], timeout=300,
                expected_status=expected["status"], expected_stdout=expected["stdout"].encode(),
                expected_stderr=expected["stderr"].encode())
            steps_text = (generated / (name + ".steps")).read_text().strip()
            evaluator_policy = expected["evaluator"]
            if evaluator_policy == "native-only":
                require(steps_text == "native-only",
                        name + ": native-only evaluator policy was not recorded exactly")
                evaluator_evidence = {"policy": "native-only", "steps": None}
            else:
                require(steps_text.isdigit(), name + ": evaluator step count is absent or invalid")
                evaluator_stderr = (generated / (name + ".evaluator.stderr")).read_text()
                evaluator_ir = generated / (name + ".evaluator.ir")
                require(evaluator_ir.is_file(), name + ": evaluator IR evidence is missing")
                if evaluator_policy == 0:
                    checked_equal(evaluator_stderr, "", name + " evaluator stderr")
                else:
                    require("trusted runtime failure code " + str(evaluator_policy) in evaluator_stderr,
                            name + ": evaluator evidence has the wrong trap diagnostic")
                evaluator_evidence = {
                    "policy": "evaluated", "steps": int(steps_text),
                    "expected_trap_code": evaluator_policy if evaluator_policy else None,
                    "stderr": evaluator_stderr,
                    "stderr_sha256": digest_file(generated / (name + ".evaluator.stderr")),
                    "ir_sha256": digest_file(evaluator_ir),
                }
            ordinary_diagnostic = (generated / (name + ".ordinary-source.stderr")).read_text()
            require("unknown function" in ordinary_diagnostic
                    and "__reyRaw" in ordinary_diagnostic,
                    name + ": ordinary source did not reject the raw runtime intrinsic")
            native_ir = generated / (name + ".native.ir")
            require(native_ir.is_file(), name + ": native IR evidence is missing")
            records[name] = {
                "evaluator_evidence": evaluator_evidence,
                "native_evidence": {
                    "status": item["status"], "stdout_sha256": item["stdout_sha256"],
                    "stderr_sha256": item["stderr_sha256"],
                    "image_sha256": digest_file(image), "ir_sha256": digest_file(native_ir),
                },
                "original_fixture_sha256": digest_file(source_fixtures / (name + ".rey")),
                "native_fixture_sha256": digest_file(staged_fixtures / (name + ".rey")),
                "ordinary_source_raw_intrinsic_rejection": True,
                "ordinary_source_diagnostic_sha256": digest_file(generated / (name + ".ordinary-source.stderr")),
                "pre_failure_checkpoint": bool(expected.get("checkpoint")),
            }
        require(len(records) == 21, "direct native suite did not execute exactly 21 IDs")
        self.results["tests"]["direct_native"] = records
        self.save()

    def run_reclamation(self):
        manifest = read_json(GATE_DIR / "reclamation-cases.json")
        require(self.fixture_paths is not None, "region fixture setup did not run")
        _, _, fixtures = self.fixture_paths
        source_dir = self.out / "drivers/reclamation-source"
        for relative in EXPECTED_RUNTIME_FILES:
            target = source_dir / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(self.source / "runtime-rey" / relative, target)
        (source_dir / "main.rey").write_text("")
        generated = self.out / "images/reclamation"
        generated.mkdir(parents=True, exist_ok=True)
        records = {}
        for name, expected in manifest.items():
            command = [str(self.region_driver), "reclamation", str(fixtures), str(fixtures),
                       str(source_dir), str(generated), name]
            item, stdout, stderr = self.command("emit-reclamation-" + name, command, timeout=300,
                                                build=True, heavy=False)
            expected_driver_line = re.compile(
                r"^region-runtime-reclamation-case " + re.escape(name)
                + r" emitted steps=(?:native-only|[0-9]+)\n$"
            )
            require(expected_driver_line.fullmatch(stdout.decode("utf-8", "replace")) is not None,
                    name + ": reclamation driver omitted its case record")
            steps_text = (generated / (name + ".steps")).read_text().strip()
            evaluator_runs = name not in (
                "reclaim-three-slabs", "active-context-destroy", "teardown-many-direct-chunks")
            if evaluator_runs:
                require(steps_text.isdigit(), name + ": reclamation evaluator step count is invalid")
                checked_equal(stdout, ("region-runtime-reclamation-case " + name
                                       + " emitted steps=" + steps_text + "\n").encode(),
                              name + " reclamation driver output")
                evaluator_stderr = generated / (name + ".evaluator.stderr")
                evaluator_ir = generated / (name + ".evaluator.ir")
                require(evaluator_stderr.is_file() and evaluator_ir.is_file(),
                        name + ": reclamation evaluator evidence is incomplete")
                checked_equal(evaluator_stderr.read_bytes(), b"", name + " evaluator stderr")
            else:
                checked_equal(steps_text, "native-only", name + " evaluator policy")
            ordinary_diagnostic = (generated / (name + ".ordinary-source.stderr")).read_text()
            require("unknown function" in ordinary_diagnostic
                    and "__reyRaw" in ordinary_diagnostic,
                    name + ": ordinary source did not reject the raw runtime intrinsic")
            image = generated / ("linux-x64-" + name)
            require(image.is_file(), name + ": fixture driver did not emit a Linux x64 image")
            image.chmod(image.stat().st_mode | 0o111)
            item, native_stdout, native_stderr = self.command(
                "run-reclamation-" + name,
                [str(image)], timeout=300, expected_status=expected["status"],
                expected_stdout=expected["stdout"].encode(), expected_stderr=expected["stderr"].encode(),
                resource_limit_mib=expected.get("address_space_mib"))
            records[name] = {
                "evaluator_evidence": {
                    "policy": "evaluated" if evaluator_runs else "native-only",
                    "steps": int(steps_text) if evaluator_runs else None,
                    "stderr_sha256": digest_file(generated / (name + ".evaluator.stderr")) if evaluator_runs else None,
                    "ir_sha256": digest_file(generated / (name + ".evaluator.ir")) if evaluator_runs else None,
                },
                "native_evidence": {
                    "status": item["status"], "stdout_sha256": item["stdout_sha256"],
                    "stderr_sha256": item["stderr_sha256"], "image_sha256": digest_file(image),
                    "ir_sha256": digest_file(generated / (name + ".native.ir")),
                },
                "source_sha256": digest_file(fixtures / (name + ".rey")),
                "ordinary_source_diagnostic_sha256": digest_file(generated / (name + ".ordinary-source.stderr")),
            }
        require(len(records) == 5, "reclamation suite did not execute exactly five IDs")
        self.results["tests"]["reclamation_native"] = records
        self.save()

    def run(self):
        self.prepare()
        self.build_drivers()
        self.build_and_run_static()
        self.run_evaluator_source_case()
        self.run_raw_intrinsic_rejection()
        self.run_function_cutoffs()
        self.run_region_sources()
        self.run_direct_runtime()
        self.run_reclamation()
        self.results["counts"] = {
            "normal": len(self.results["tests"]["region_source"]["normal"]),
            "stress": len(self.results["tests"]["region_source"]["stress"]),
            "direct_native": len(self.results["tests"]["direct_native"]),
            "reclamation_native": len(self.results["tests"]["reclamation_native"]),
        }
        require(self.results["counts"] == {
            "normal": 16, "stress": 16, "direct_native": 21, "reclamation_native": 5,
        }, "final R01 acceptance counts are incomplete")
        self.save()


def parse_args():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", required=True, help="native Linux x64 seed executable")
    parser.add_argument("--source", required=True, help="current immutable Rey source root")
    parser.add_argument("--out", required=True, help="fresh external evidence directory")
    parser.add_argument("--suite", choices=("regions",), required=True)
    return parser.parse_args()


def main():
    args = parse_args()
    try:
        gate = RegionGate(args.engine, args.source, args.out)
        gate.run()
    except (GateFailure, OSError, ValueError, KeyError, IndexError) as exc:
        print("FAIL region gate: " + str(exc), file=sys.stderr)
        raise SystemExit(1)
    counts = gate.results["counts"]
    print("%d/16 normal, %d/16 stress, %d/21 direct native, %d/5 reclamation native" % (
        counts["normal"], counts["stress"], counts["direct_native"], counts["reclamation_native"]))
    print("PASS region gate")


if __name__ == "__main__":
    main()
