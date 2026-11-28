#!/usr/bin/env python3
"""Acceptance runner for checked allocator semantics and measured CPU."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
BUILD_HELPER = HERE / "uncapped-build.sh"
PROBE = ROOT / "tests/v04/region-bench/allocator-probe.py"
REPEAT_GATES = HERE / "uncapped-repeat-gates.py"
FUNCTIONAL_FIXTURE = HERE / "main.rey"
CASES = (
    ("payload-zero-and-typed-null", b"0", 0),
    ("stale-region-handle", b"1", 1),
    ("foreign-context-handle", b"2", 1),
    ("corrupt-group-header", b"3", 1),
    ("corrupt-layout-descriptor", b"4", 1),
    ("corrupt-group-owner", b"5", 1),
    ("truncated-group-list", b"6", 1),
)


class Failure(RuntimeError):
    pass


def require(condition, message):
    if not condition:
        raise Failure(message)


def sha256(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def tree_hashes(root):
    root = Path(root)
    return {str(path.relative_to(root)): sha256(path)
            for path in sorted(root.rglob("*")) if path.is_file()}


def time_peak(path):
    text = Path(path).read_text(errors="replace")
    match = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", text)
    require(match is not None, "compiler /usr/bin/time -v did not report peak RSS")
    return int(match.group(1)) * 1024


def limited(argv, seconds, **kwargs):
    command = ["nice", "-n", "19", "timeout", "--kill-after=30s", str(seconds), *map(str, argv)]
    return subprocess.run(command, check=False, pass_fds=(3,), **kwargs)


class Runner:
    def __init__(self, workspace, seed, source, out, repeats):
        self.workspace = Path(workspace).resolve(strict=True)
        self.seed = Path(seed).resolve(strict=True)
        self.source = Path(source).resolve(strict=True)
        requested_out = Path(out).resolve()
        self.acceptance_output_root = requested_out
        self.acceptance_attempt = 1
        if requested_out.exists() and any(requested_out.iterdir()):
            self.acceptance_attempt = 2
            self.out = requested_out / ("attempt-%02d" % self.acceptance_attempt)
            while self.out.exists():
                self.acceptance_attempt += 1
                self.out = requested_out / ("attempt-%02d" % self.acceptance_attempt)
        else:
            self.out = requested_out
        self.repeats = repeats
        self.logs = self.out / "logs"
        self.runtime = self.out / "runtime-rey"
        self.commands = []
        self.driver = None

    def require_lock(self):
        if os.environ.get("REY_PARALLEL_BUILD") == "1":
            return
        try:
            held = Path(os.readlink("/proc/self/fd/3")).resolve()
        except OSError as exc:
            raise Failure("acceptance must inherit fd 3 for the global build lock: " + str(exc))
        require(held == Path("/home/ubuntu/rey/build.lock").resolve(),
                "acceptance did not inherit /home/ubuntu/rey/build.lock")

    def prepare(self):
        self.require_lock()
        require(self.workspace.is_dir(), "BASE workspace directory is missing")
        require(self.seed.is_file() and os.access(self.seed, os.X_OK), "SEED must be executable")
        require((self.source / "runtime-rey/regions/main.rey").is_file(),
                "SRC is missing the current region runtime")
        require(not self.out.exists() or not any(self.out.iterdir()),
                "OUT must be fresh and empty: " + str(self.out))
        require(self.source not in self.out.parents, "OUT must be outside SRC")
        self.out.mkdir(parents=True, exist_ok=True)
        self.logs.mkdir()
        shutil.copytree(self.source / "runtime-rey", self.runtime)
        self.fixture = FUNCTIONAL_FIXTURE
        self.results = {
            "schema": "rey-v04-checked-allocator-v1",
            "workspace": str(self.workspace),
            "seed": str(self.seed),
            "seed_sha256": sha256(self.seed),
            "source": str(self.source),
            "source_revision": self.git_revision(),
            "acceptance_output_root": str(self.acceptance_output_root),
            "acceptance_attempt": self.acceptance_attempt,
            "runtime_source_hashes": tree_hashes(self.source / "runtime-rey"),
            "staged_runtime_hashes": tree_hashes(self.runtime),
            "functional_fixture_sha256": sha256(FUNCTIONAL_FIXTURE),
            "trusted_driver_source_sha256": sha256(HERE / "driver.rey"),
            "repeats": self.repeats,
            "compiler_builds": self.commands,
            "functional_cases": {},
        }
        self.save()

    def git_revision(self):
        result = subprocess.run(["git", "-C", str(self.source), "rev-parse", "HEAD"],
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
        return result.stdout.decode().strip() if result.returncode == 0 else "unavailable"

    def save(self):
        (self.out / "results.json").write_text(json.dumps(self.results, indent=2, sort_keys=True) + "\n")

    def build_functional_fixture(self):
        image = self.out / "bin" / "checked-allocator"
        image.parent.mkdir()
        timing = self.logs / "functional-build.time"
        stdout_path = self.logs / "functional-build.stdout"
        stderr_path = self.logs / "functional-build.stderr"
        command = [
            "/usr/bin/time", "-v", "-o", str(timing), str(self.driver),
            str(self.fixture), str(self.runtime), str(image),
        ]
        cgroup_path = self.logs / "functional-build.cgroup"
        status_path = self.logs / "functional-build.status"
        command = [str(BUILD_HELPER), str(timing), str(cgroup_path), str(status_path), "2400",
                   "--", *command]
        with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
            result = subprocess.run(command, stdout=stdout, stderr=stderr, check=False,
                                    pass_fds=(3,))
        peak = time_peak(timing) if timing.is_file() and timing.stat().st_size else None
        cgroup = {}
        if cgroup_path.is_file():
            for line in cgroup_path.read_text().splitlines():
                key, separator, value = line.partition("=")
                if separator:
                    cgroup[key] = int(value) if key in ("memory.peak", "command_status") else value
        record = {
            "command": command,
            "status": result.returncode,
            "peak_rss_bytes": peak,
            "uncapped_cgroup": cgroup,
            "cgroup_record": str(cgroup_path),
            "status_record": str(status_path),
            "time_v": str(timing),
            "stdout": str(stdout_path),
            "stderr": str(stderr_path),
            "image_sha256": sha256(image) if image.is_file() else None,
        }
        self.commands.append(record)
        self.save()
        require(result.returncode == 0 and image.is_file(),
                "trusted runtime functional build failed; inspect " + str(stderr_path))
        require(peak is not None and peak > 0, "functional driver peak RSS was not recorded")
        require(cgroup.get("memory.max") == "max" and cgroup.get("memory.swap.max") == "max"
                and cgroup.get("memory.peak", 0) > 0,
                "functional driver uncapped cgroup peak was not recorded")
        image.chmod(image.stat().st_mode | 0o111)
        return image

    def run_functional_cases(self, image):
        for name, input_bytes, expected_status in CASES:
            stdout_path = self.logs / (name + ".stdout")
            stderr_path = self.logs / (name + ".stderr")
            command = [str(image)]
            with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
                result = limited(command, 120, input=input_bytes, stdout=stdout, stderr=stderr)
            status = result.returncode if result.returncode >= 0 else 128 + -result.returncode
            require(status == expected_status,
                    "%s returned %d, expected %d" % (name, status, expected_status))
            require(stdout_path.read_bytes() == b"", name + " wrote unexpected stdout")
            require(stderr_path.read_bytes() == b"", name + " wrote unexpected stderr")
            self.results["functional_cases"][name] = {
                "status": status,
                "expected_status": expected_status,
                "stdin": input_bytes.decode("ascii"),
                "stdout_sha256": sha256(stdout_path),
                "stderr_sha256": sha256(stderr_path),
            }
            self.save()

    def run_probe(self):
        probe_out = self.out / "probe"
        stdout_path = self.logs / "probe.stdout"
        stderr_path = self.logs / "probe.stderr"
        command = [
            "python3", "-B", str(PROBE), str(self.seed), str(self.source), str(probe_out),
            "--repeats", str(self.repeats), "--require-improvement",
        ]
        with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
            result = limited(command, 14400, stdout=stdout, stderr=stderr)
        self.results["probe"] = {
            "status": result.returncode,
            "stdout": str(stdout_path),
            "stderr": str(stderr_path),
            "results": str(probe_out / "results.json"),
        }
        self.save()
        require(result.returncode == 0,
                "allocator probe failed; inspect " + str(stderr_path) + " and " + str(stdout_path))
        probe_results = json.loads((probe_out / "results.json").read_text())
        self.results["compiler_builds"].extend(probe_results["compiler_builds"])
        self.results["probe_summary"] = probe_results["summary"]
        self.driver = Path(probe_results["compiler_driver"]).resolve(strict=True)
        self.results["compiler_driver"] = str(self.driver)
        self.save()

    def run_repeat_gates(self):
        gates_out = self.out / "repeat-gates"
        stdout_path = self.logs / "repeat-gates.stdout"
        stderr_path = self.logs / "repeat-gates.stderr"
        command = ["python3", "-B", str(REPEAT_GATES), str(self.seed), str(self.source), str(gates_out)]
        run_env = dict(os.environ)
        run_env["REY_CHECKED_ALLOCATOR_DRIVER"] = str(self.driver)
        with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
            result = limited(command, 18000, stdout=stdout, stderr=stderr, env=run_env)
        self.results["repeat_gates"] = {
            "status": result.returncode,
            "stdout": str(stdout_path),
            "stderr": str(stderr_path),
            "results": str(gates_out / "results.json"),
        }
        self.save()
        require(result.returncode == 0,
                "staged R01/R02 repeat failed; inspect " + str(stderr_path) + " and " + str(stdout_path))
        self.results["compiler_builds"].extend(
            json.loads((gates_out / "results.json").read_text()).get("r01", {}).get("compiler_builds", []))
        self.results["compiler_builds"].extend(
            json.loads((gates_out / "results.json").read_text()).get("r02", {}).get("compiler_builds", []))
        self.save()

    def run(self):
        self.prepare()
        self.run_probe()
        image = self.build_functional_fixture()
        self.run_functional_cases(image)
        self.run_repeat_gates()
        return self.results


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("workspace")
    parser.add_argument("seed")
    parser.add_argument("source")
    parser.add_argument("out")
    parser.add_argument("--repeats", type=int, required=True)
    args = parser.parse_args()
    try:
        require(args.repeats >= 5 and args.repeats <= 15,
                "acceptance requires at least five timing repetitions")
        result = Runner(args.workspace, args.seed, args.source, args.out, args.repeats).run()
    except (Failure, OSError, subprocess.SubprocessError, json.JSONDecodeError) as exc:
        print("FAIL checked allocator: " + str(exc), file=sys.stderr)
        raise SystemExit(1)
    print("checked allocator probe %.2f%% lower median CPU" %
          (result["probe_summary"]["hot_fixture_improvement"] * 100))
    print("checked allocator cases: payload zeros, typed nulls, stale, foreign, corrupt group and descriptor")
    print("PASS checked allocator")


if __name__ == "__main__":
    main()
