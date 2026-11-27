#!/usr/bin/env python3
"""Repeat R01 and R02 with the current staged runtime and no memory cap."""

import argparse
import hashlib
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


FORBIDDEN = {
    "clang", "clang++", "cc", "gcc", "g++", "llc", "rustc", "cargo",
    "python", "python2", "python3", "zig", "go", "ld", "ld.lld", "as",
    "llvm-as", "llvm-objcopy",
}
BUILD_HELPER = Path(__file__).resolve().parent / "uncapped-build.sh"
# Managed self-hosted driver builds can spend significant time paging under load.
BUILD_TIMEOUT_SECONDS = 7200


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def peak_rss_bytes(time_path):
    if not Path(time_path).is_file():
        return None
    text = Path(time_path).read_text(errors="replace")
    match = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", text)
    if match is None:
        return None
    return int(match.group(1)) * 1024


def check_lock():
    if os.environ.get("REY_PARALLEL_BUILD") == "1":
        return
    try:
        path = Path(os.readlink("/proc/self/fd/3")).resolve()
    except OSError as exc:
        raise RuntimeError("repeat gates must inherit the global build lock: " + str(exc))
    if path != Path("/home/ubuntu/rey/build.lock").resolve():
        raise RuntimeError("repeat gates are not holding /home/ubuntu/rey/build.lock")


def check_exec_trace(path, producer, label):
    lines = Path(path).read_text(errors="replace").splitlines()
    exec_lines = [line for line in lines if "execve(" in line or "execveat(" in line]
    expected = str(Path(producer).resolve())
    allowed_wrappers = {"time", "systemd-run", "nice", "timeout", "sh", "sed", "cat"}
    successful = []
    for line in exec_lines:
        paths = re.findall(r'execve(?:at)?\("([^\"]+)"', line)
        for executable in paths:
            name = Path(executable).name.lower()
            if name in FORBIDDEN or name.startswith(("clang-", "python3.", "llvm-", "ld.lld-")):
                raise RuntimeError(label + " attempted a forbidden exec: " + line)
            if name in allowed_wrappers:
                continue
            if executable != expected:
                raise RuntimeError(label + " attempted an unexpected exec: " + line)
        if " = 0" in line and any(Path(executable).name.lower() not in allowed_wrappers
                                  for executable in paths):
            successful.append(line)
    if len(successful) != 1 or expected not in successful[0]:
        raise RuntimeError(label + " did not have exactly one successful producer exec: " + repr(exec_lines))


class UncappedRegionGate:
    def __init__(self, module, engine, source, out):
        self.module = module
        self.engine = Path(engine).resolve(strict=True)
        self.source = Path(source).resolve(strict=True)
        self.out = Path(out).resolve()
        self.logs = self.out / "logs"
        self.results = {
            "gate": "R01 current Linux x64 region gate (uncapped policy)",
            "engine": str(self.engine),
            "engine_sha256": module.digest_file(self.engine),
            "source": str(self.source),
            "platform": platform.platform(),
            "machine": platform.machine(),
            "resource_policy": "parallel buildbox; no shared lock or memory cap; nice -n 19, timeout, /usr/bin/time -v",
            "compiler_builds": [],
            "commands": [],
            "tests": {},
        }
        self.counter = 0
        self.cwd = None
        self.runtime = None
        self.compiler_copy = None
        self.region_driver = None
        self.ir_driver = None
        self.runtime_evaluator_driver = None
        self.fixture_paths = None

    def __getattr__(self, name):
        member = getattr(self.module.RegionGate, name, None)
        if member is None:
            raise AttributeError(name)
        return lambda *args, **kwargs: member(self, *args, **kwargs)

    def prepare(self):
        self.module.require(platform.system() == "Linux" and platform.machine().lower() in ("x86_64", "amd64"),
                            "R01 requires native Linux x64")
        check_lock()
        self.module.require(self.engine.is_file() and os.access(self.engine, os.X_OK),
                            "engine must be an executable native seed")
        self.module.require(self.source.is_dir() and (self.source / "rey-compiler/src").is_dir(),
                            "source root is missing rey-compiler/src")
        self.module.require(shutil.which("strace") is not None, "strace is required")
        try:
            self.out.relative_to(self.source)
            raise self.module.GateFailure("R01 output must be outside SOURCE")
        except ValueError:
            pass
        self.module.require(not self.out.exists() or not any(self.out.iterdir()),
                            "R01 output must be fresh and empty: " + str(self.out))
        self.out.mkdir(parents=True, exist_ok=True)
        self.logs.mkdir(parents=True, exist_ok=True)
        self.cwd = self.out / "run-cwd"
        (self.cwd / "build/v04").mkdir(parents=True, exist_ok=True)
        self.results["source_revision"] = self.module.RegionGate.git_revision(self)
        self.results["runtime_source_hashes"] = self.module.tree_hashes(self.source / "runtime-rey")
        self.runtime = self.out / "runtime-rey"
        shutil.copytree(self.source / "runtime-rey", self.runtime)
        self.results["staged_runtime_source_hashes"] = self.module.tree_hashes(self.runtime)
        self.results["compiler_source_hashes"] = self.module.tree_hashes(self.source / "rey-compiler")
        harness_files = (
            self.source / "scripts/v04-linux-x64-region-gate.sh",
            self.module.GATE_DIR / "run.py", self.module.GATE_DIR / "region-driver.rey",
            self.module.GATE_DIR / "ir-driver.rey", self.module.GATE_DIR / "runtime-evaluator-driver.rey",
            self.module.GATE_DIR / "layout-negative.rey", self.module.GATE_DIR / "raw-intrinsic.rey",
            self.module.GATE_DIR / "source-cases.json", self.module.GATE_DIR / "native-cases.json",
            self.module.GATE_DIR / "reclamation-cases.json", self.module.GATE_DIR / "static-cases.json",
            self.module.GATE_DIR / "function-cutoffs.json",
        )
        self.results["harness_hashes"] = {
            str(path.relative_to(self.source)): self.module.digest_file(path) for path in harness_files
        }
        self.results["manifests"] = self.module.RegionGate.validate_manifests(self)
        self.save()

    def save(self):
        self.module.write_json(self.out / "results.json", self.results)

    def command(self, label, argv, *, timeout=180, build=False, heavy=False, cwd=None,
                expected_status=0, expected_stdout=None, expected_stderr=None, env=None,
                resource_limit_mib=None):
        self.counter += 1
        slug = "%03d-%s" % (self.counter, self.module.safe_name(label))
        stdout_path = self.logs / (slug + ".stdout")
        stderr_path = self.logs / (slug + ".stderr")
        command = list(map(str, argv))
        producer = command[0]
        trace_path = self.logs / (slug + ".execve") if build else None
        timing_path = self.logs / (slug + ".time") if build else None
        cgroup_path = self.logs / (slug + ".cgroup") if build else None
        status_path = self.logs / (slug + ".build-status") if build else None
        if build:
            timeout = max(timeout, BUILD_TIMEOUT_SECONDS)
            wrapped_command = [
                "strace", "-f", "-e", "trace=execve,execveat", "-o", str(trace_path), "--",
                "/usr/bin/time", "-v", "-o", str(timing_path), *command,
            ]
        else:
            wrapped_command = command
            if resource_limit_mib is not None:
                shell = "ulimit -v " + str(resource_limit_mib * 1024) + " || exit 125; exec \"$@\""
                wrapped_command = ["sh", "-c", shell, "r03-repeat", *wrapped_command]
        if build:
            wrapped_command = [str(BUILD_HELPER), str(timing_path), str(cgroup_path), str(status_path),
                               str(timeout), "--", *wrapped_command]
        else:
            wrapped_command = ["nice", "-n", "19", "timeout", "--kill-after=30s", str(timeout),
                               *wrapped_command]
        run_env = dict(os.environ)
        if env:
            run_env.update(env)
        started = time.monotonic()
        with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
            try:
                result = subprocess.run(
                    wrapped_command, cwd=str(cwd or self.cwd), env=run_env,
                    stdout=stdout, stderr=stderr, check=False, timeout=timeout + 45,
                    start_new_session=True, pass_fds=(3,),
                )
            except subprocess.TimeoutExpired:
                raise self.module.GateFailure(label + " exceeded outer timeout")
        status = result.returncode if result.returncode >= 0 else 128 + -result.returncode
        stdout_bytes = stdout_path.read_bytes()
        stderr_bytes = stderr_path.read_bytes()
        item = {
            "label": label, "command": wrapped_command, "cwd": str(cwd or self.cwd),
            "status": status, "expected_status": expected_status,
            "elapsed_seconds": round(time.monotonic() - started, 6),
            "stdout_sha256": self.module.digest_bytes(stdout_bytes),
            "stderr_sha256": self.module.digest_bytes(stderr_bytes),
            "stdout_log": str(stdout_path), "stderr_log": str(stderr_path),
            "resource_policy": "uncapped; parallel buildbox; no shared lock; nice -n 19; timeout" if build else None,
        }
        if build:
            peak = peak_rss_bytes(timing_path)
            self.module.require(peak is not None and peak > 0,
                                "R01 compiler build did not record peak RSS")
            cgroup = {}
            for line in cgroup_path.read_text().splitlines():
                key, separator, value = line.partition("=")
                if separator:
                    cgroup[key] = int(value) if key in ("memory.peak", "command_status") else value
            if cgroup.get("memory.max") != "max" or cgroup.get("memory.swap.max") != "max" \
                    or cgroup.get("memory.peak", 0) <= 0:
                raise RuntimeError("R01 compiler build did not record an uncapped cgroup peak")
            item["execve_trace"] = str(trace_path)
            item["time_v"] = str(timing_path)
            item["peak_rss_bytes"] = peak
            item["uncapped_cgroup"] = cgroup
            item["cgroup_record"] = str(cgroup_path)
            item["status_record"] = str(status_path)
            self.results["compiler_builds"].append({
                "label": label, "producer": producer, "time_v": str(timing_path),
                "peak_rss_bytes": peak, "uncapped_cgroup": cgroup, "status": status,
            })
        self.results["commands"].append(item)
        self.save()
        self.module.require(status == expected_status,
                            label + ": expected status %d, got %d; see %s" %
                            (expected_status, status, stderr_path))
        if expected_stdout is not None:
            expected = expected_stdout if isinstance(expected_stdout, bytes) else expected_stdout.encode()
            self.module.checked_equal(stdout_bytes, expected, label + " stdout")
        if expected_stderr is not None:
            expected = expected_stderr if isinstance(expected_stderr, bytes) else expected_stderr.encode()
            self.module.checked_equal(stderr_bytes, expected, label + " stderr")
        if build:
            check_exec_trace(trace_path, producer, label)
        return item, stdout_bytes, stderr_bytes

    def run(self):
        self.prepare()
        self.module.RegionGate.build_drivers(self)
        self.module.RegionGate.build_and_run_static(self)
        self.module.RegionGate.run_evaluator_source_case(self)
        self.module.RegionGate.run_raw_intrinsic_rejection(self)
        self.module.RegionGate.run_function_cutoffs(self)
        self.module.RegionGate.run_region_sources(self)
        self.module.RegionGate.run_direct_runtime(self)
        self.module.RegionGate.run_reclamation(self)
        self.results["counts"] = {
            "normal": len(self.results["tests"]["region_source"]["normal"]),
            "stress": len(self.results["tests"]["region_source"]["stress"]),
            "direct_native": len(self.results["tests"]["direct_native"]),
            "reclamation_native": len(self.results["tests"]["reclamation_native"]),
        }
        self.module.require(self.results["counts"] == {
            "normal": 16, "stress": 16, "direct_native": 21, "reclamation_native": 5,
        }, "R01 acceptance counts are incomplete")
        self.save()
        print("R01 counts: 16/16 normal, 16/16 stress, 21/21 direct native, 5/5 reclamation")
        print("PASS region gate")


def run_r01(seed, source, out):
    path = source / "tests/v04/linux-x64-region/run.py"
    module = load_module("r03_r01", path)
    gate = UncappedRegionGate(module, seed, source, out)
    gate.run()
    return gate.results


def run_r02(seed, source, out):
    path = source / "tests/v04/linux-x64-teardown/run.py"
    module = load_module("r03_r02", path)

    class UncappedTeardown(module.Runner):
        def prepare(self):
            module.require(platform.system() == "Linux" and platform.machine().lower() in ("x86_64", "amd64"),
                           "R02 requires native Linux x64")
            check_lock()
            module.require(self.base.is_file() and os.access(self.base, os.X_OK), "BASE must be executable")
            module.require(sha256(self.base) == "64c8bbe4e559a2355b79bfa0d357bd6e81d6dee92bbac73fe520165004883ee6",
                           "BASE is not the hash-pinned native comparison producer")
            module.require(self.source.is_dir() and (self.source / "runtime-rey/regions/main.rey").is_file(),
                           "SRC does not contain the current region runtime")
            module.require(shutil.which("strace") is not None, "strace is required")
            module.require(shutil.which("/usr/bin/time") is not None, "/usr/bin/time is required")
            try:
                self.out.relative_to(self.source)
                raise module.Failure("OUT must be outside SRC")
            except ValueError:
                pass
            module.require(not self.out.exists() or not any(self.out.iterdir()),
                           "OUT must be fresh and empty: " + str(self.out))
            self.out.mkdir(parents=True, exist_ok=True)
            self.logs.mkdir(parents=True, exist_ok=True)
            self.evidence.mkdir(parents=True, exist_ok=True)
            self.results["resource_policy"] = "uncapped; parallel buildbox; no shared lock; nice -n 19; timeout; /usr/bin/time -v"
            self.results["source_revision"] = self.git_revision()
            self.results["runtime_source_hashes"] = module.tree_hashes(self.source / "runtime-rey")
            shutil.copytree(self.source / "runtime-rey", self.runtime)
            self.results["staged_runtime_source_hashes_before_test_hooks"] = module.tree_hashes(self.runtime)
            with (self.runtime / "regions/main.rey").open("a") as staged_regions:
                staged_regions.write((module.HERE / "test-hooks.rey").read_text())
            shutil.copyfile(module.HERE / "fixture.rey", self.fixture_source)
            self.results["staged_runtime_source_hashes"] = module.tree_hashes(self.runtime)
            self.results["test_hook_sha256"] = module.sha256(module.HERE / "test-hooks.rey")
            self.results["instrumented_regions_sha256"] = module.sha256(self.runtime / "regions/main.rey")
            self.results["harness_hashes"] = {
                str(path.relative_to(self.source)): module.sha256(path)
                for path in (module.HERE / "fixture.rey", module.HERE / "test-hooks.rey",
                             module.HERE / "run.py", module.HERE / "run.sh")
            }
            self.save()
            self.build_fixture()

        def invoke(self, label, argv, timeout, *, build=False, expected_status=0):
            self.counter += 1
            slug = "%03d-%s" % (self.counter, re.sub(r"[^A-Za-z0-9._-]", "_", label))
            stdout_path = self.logs / (slug + ".stdout")
            stderr_path = self.logs / (slug + ".stderr")
            command = list(map(str, argv))
            producer = command[0]
            trace_path = self.logs / (slug + ".execve") if build else None
            timing_path = self.logs / (slug + ".build.time") if build else None
            cgroup_path = self.logs / (slug + ".build.cgroup") if build else None
            status_path = self.logs / (slug + ".build-status") if build else None
            if build:
                timeout = max(timeout, BUILD_TIMEOUT_SECONDS)
                command = ["strace", "-f", "-e", "trace=execve,execveat", "-o", str(trace_path), "--",
                           "/usr/bin/time", "-v", "-o", str(timing_path), *command]
            if build:
                command = [str(BUILD_HELPER), str(timing_path), str(cgroup_path), str(status_path),
                           str(timeout), "--", *command]
            else:
                command = ["nice", "-n", "19", "timeout", "--kill-after=30s", str(timeout), *command]
            started = time.monotonic()
            with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
                result = subprocess.run(command, stdout=stdout, stderr=stderr, check=False,
                                        timeout=timeout + 45, start_new_session=True, pass_fds=(3,))
            status = result.returncode if result.returncode >= 0 else 128 + -result.returncode
            stdout_bytes = stdout_path.read_bytes()
            stderr_bytes = stderr_path.read_bytes()
            record = {
                "label": label, "argv": list(map(str, argv)), "status": status,
                "expected_status": expected_status,
                "elapsed_seconds": round(time.monotonic() - started, 6),
                "stdout_log": str(stdout_path), "stderr_log": str(stderr_path),
                "stdout_sha256": module.digest_bytes(stdout_bytes),
                "stderr_sha256": module.digest_bytes(stderr_bytes),
                "resource_policy": "uncapped; parallel buildbox; no shared lock; nice -n 19; timeout",
            }
            if build:
                peak = peak_rss_bytes(timing_path)
                module.require(peak is not None and peak > 0,
                               "R02 compiler build did not record peak RSS")
                cgroup = {}
                for line in cgroup_path.read_text().splitlines():
                    key, separator, value = line.partition("=")
                    if separator:
                        cgroup[key] = int(value) if key in ("memory.peak", "command_status") else value
                module.require(cgroup.get("memory.max") == "max"
                               and cgroup.get("memory.swap.max") == "max"
                               and cgroup.get("memory.peak", 0) > 0,
                               "R02 compiler build did not record an uncapped cgroup peak")
                record["execve_trace"] = str(trace_path)
                record["time_v"] = str(timing_path)
                record["peak_rss_bytes"] = peak
                record["uncapped_cgroup"] = cgroup
                record["cgroup_record"] = str(cgroup_path)
                record["status_record"] = str(status_path)
                self.results.setdefault("compiler_builds", []).append({
                    "label": label, "producer": producer, "time_v": str(timing_path),
                    "peak_rss_bytes": peak, "uncapped_cgroup": cgroup, "status": status,
                })
            self.results["commands"].append(record)
            self.save()
            module.require(status == expected_status,
                           label + ": expected status %d, got %d" % (expected_status, status))
            if build:
                check_exec_trace(trace_path, producer, label)
            return stdout_bytes, stderr_bytes, round(time.monotonic() - started, 6), slug

    runner = UncappedTeardown(seed, source, out)
    runner.run()
    return runner.results


def sha256(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("seed")
    parser.add_argument("source")
    parser.add_argument("out")
    args = parser.parse_args()
    check_lock()
    seed = Path(args.seed).resolve(strict=True)
    source = Path(args.source).resolve(strict=True)
    out = Path(args.out).resolve()
    if out.exists() and any(out.iterdir()):
        raise SystemExit("repeat-gates output must be fresh and empty")
    out.mkdir(parents=True, exist_ok=True)
    r01_out = out / "r01"
    r02_out = out / "r02"
    r01 = run_r01(seed, source, r01_out)
    r02 = run_r02(seed, source, r02_out)
    summary = {
        "resource_policy": "uncapped; parallel buildbox; no shared lock; nice -n 19; timeout; peak RSS via /usr/bin/time -v",
        "r01": {"results": str(r01_out / "results.json"), "compiler_builds": r01["compiler_builds"]},
        "r02": {"results": str(r02_out / "results.json"), "compiler_builds": r02.get("compiler_builds", [])},
    }
    (out / "results.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        print("FAIL uncapped R01/R02 repeat: " + str(exc), file=sys.stderr)
        raise SystemExit(1)
