#!/usr/bin/env python3
"""Measure allocator payload clearing and checked group lookup on Linux x64."""

import argparse
import hashlib
import json
import re
import shutil
import statistics
import struct
import subprocess
import sys
from pathlib import Path


HERE = Path(__file__).resolve().parent
BUILD_HELPER = HERE.parent / "checked-allocator" / "uncapped-build.sh"
WORKLOAD = HERE / "allocator-probe.rey"
DRIVER_SOURCE = HERE.parent / "checked-allocator" / "driver.rey"
CONTROL_REVISION = (HERE / "allocator-control-revision.txt").read_text().strip()
CASES = (
    ("zero-8x10000", 1, 10000, 8, 1),
    ("zero-1024x10000", 1, 10000, 1024, 1),
    ("groups-32x10000", 2, 10000, 8, 32),
    ("groups-128x10000", 2, 10000, 8, 128),
    ("groups-512x5000", 2, 5000, 8, 512),
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


def digest_bytes(data):
    return hashlib.sha256(data).hexdigest()


def tree_hashes(root):
    root = Path(root)
    return {str(path.relative_to(root)): sha256(path)
            for path in sorted(root.rglob("*")) if path.is_file()}


def run_limited(argv, timeout, *, stdout=None, stderr=None):
    command = ["nice", "-n", "19", "timeout", "--kill-after=30s", str(timeout), *map(str, argv)]
    return subprocess.run(command, stdout=stdout, stderr=stderr, check=False, pass_fds=(3,))


def git_source(source, revision, relative):
    result = subprocess.run(
        ["git", "-C", str(source), "show", revision + ":" + relative],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
    )
    require(result.returncode == 0,
            "could not load control source " + revision + ":" + relative + ": "
            + result.stderr.decode("utf-8", "replace"))
    return result.stdout


def prepare_runtime(source, destination, control):
    shutil.copytree(source / "runtime-rey", destination)
    if control:
        for module in ("layout", "regions"):
            relative = "runtime-rey/%s/main.rey" % module
            (destination / module / "main.rey").write_bytes(
                git_source(source, CONTROL_REVISION, relative))


def parse_time(path):
    text = Path(path).read_text(errors="replace")
    match = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", text)
    require(match is not None, "compiler time -v output has no maximum RSS: " + str(path))
    return int(match.group(1)) * 1024


def parse_metrics(stdout):
    require(len(stdout) == len(CASES) * 8,
            "probe emitted %d bytes, expected %d timing bytes" % (len(stdout), len(CASES) * 8))
    timings = struct.unpack("<%dq" % len(CASES), stdout)
    records = {}
    for (name, mode, count, payload, groups), cpu_us in zip(CASES, timings):
        require(cpu_us > 0, name + " reported invalid process CPU time")
        records[name] = {
            "mode": mode, "n": count, "payload": payload, "groups": groups,
            "cpu_us": cpu_us,
        }
    return records


class Probe:
    def __init__(self, seed, source, out, repeats, require_improvement):
        self.seed = Path(seed).resolve(strict=True)
        self.source = Path(source).resolve(strict=True)
        self.out = Path(out).resolve()
        self.repeats = repeats
        self.require_improvement = require_improvement
        self.builds = []
        self.samples = {}
        self.driver = None
        self.source_revision = self.git_revision()

    def git_revision(self):
        result = subprocess.run(["git", "-C", str(self.source), "rev-parse", "HEAD"],
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
        return result.stdout.decode().strip() if result.returncode == 0 else "unavailable"

    def prepare(self):
        require(self.seed.is_file() and self.seed.stat().st_mode & 0o111,
                "SEED must be an executable native compiler")
        require((self.source / "runtime-rey/regions/main.rey").is_file(),
                "SOURCE does not contain runtime-rey/regions/main.rey")
        require(self.out != self.source and self.source not in self.out.parents,
                "OUT must be outside SOURCE")
        require(not self.out.exists() or not any(self.out.iterdir()),
                "OUT must be fresh and empty: " + str(self.out))
        self.out.mkdir(parents=True, exist_ok=True)
        (self.out / "logs").mkdir()
        prepare_runtime(self.source, self.out / "control-runtime", control=True)
        prepare_runtime(self.source, self.out / "staged-runtime", control=False)
        self.results = {
            "schema": "rey-v04-allocator-probe-v1",
            "seed": str(self.seed),
            "seed_sha256": sha256(self.seed),
            "source": str(self.source),
            "source_revision": self.source_revision,
            "control_revision": CONTROL_REVISION,
            "control_runtime_sha256": tree_hashes(self.out / "control-runtime"),
            "staged_runtime_sha256": tree_hashes(self.out / "staged-runtime"),
            "workload_sha256": sha256(WORKLOAD),
            "repeats": self.repeats,
            "cases": {},
            "compiler_builds": self.builds,
        }

    def build_driver(self):
        compiler_source = self.out / "compiler-src"
        shutil.copytree(self.source / "rey-compiler/src", compiler_source)
        shutil.copyfile(DRIVER_SOURCE, compiler_source / "main.rey")
        image = self.out / "bin" / "checked-allocator-driver"
        image.parent.mkdir()
        timing = self.out / "logs" / "driver-build.time"
        stdout_path = self.out / "logs" / "driver-build.stdout"
        stderr_path = self.out / "logs" / "driver-build.stderr"
        cgroup_path = self.out / "logs" / "driver-build.cgroup"
        status_path = self.out / "logs" / "driver-build.status"
        time_command = [
            "/usr/bin/time", "-v", "-o", str(timing), str(self.seed), "build",
            str(compiler_source / "main.rey"), "-o", str(image), "--target", "linux-x64",
        ]
        command = [str(BUILD_HELPER), str(timing), str(cgroup_path), str(status_path), "2400",
                   "--", *time_command]
        with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
            result = subprocess.run(command, stdout=stdout, stderr=stderr, check=False,
                                    pass_fds=(3,))
        peak = parse_time(timing) if timing.is_file() and timing.stat().st_size else None
        cgroup = {}
        if cgroup_path.is_file():
            for line in cgroup_path.read_text().splitlines():
                key, separator, value = line.partition("=")
                if separator:
                    cgroup[key] = int(value) if key in ("memory.peak", "command_status") else value
        record = {
            "runtime": "compiler-driver",
            "command": command,
            "status": result.returncode,
            "image_sha256": sha256(image) if image.is_file() else None,
            "time_v": str(timing),
            "peak_rss_bytes": peak,
            "uncapped_cgroup": cgroup,
            "cgroup_record": str(cgroup_path),
            "status_record": str(status_path),
            "stdout": str(stdout_path),
            "stderr": str(stderr_path),
        }
        self.builds.append(record)
        self.results["compiler_driver"] = str(image)
        self.results["compiler_driver_source_sha256"] = sha256(DRIVER_SOURCE)
        (self.out / "results.json").write_text(json.dumps(self.results, indent=2, sort_keys=True) + "\n")
        require(result.returncode == 0 and image.is_file(),
                "trusted runtime compiler driver build failed; inspect " + str(stderr_path))
        require(peak is not None, "compiler driver peak RSS was not captured")
        require(cgroup.get("memory.max") == "max" and cgroup.get("memory.swap.max") == "max"
                and cgroup.get("memory.peak", 0) > 0,
                "compiler driver uncapped cgroup peak was not captured")
        image.chmod(image.stat().st_mode | 0o111)
        self.driver = image
        return image

    def build(self, runtime_name, runtime):
        case_dir = self.out / runtime_name
        case_dir.mkdir(parents=True)
        runtime_snapshot = case_dir / "runtime-rey"
        shutil.copytree(runtime, runtime_snapshot)
        image = case_dir / "allocator-probe"
        timing = self.out / "logs" / (runtime_name + "-build.time")
        stdout_path = self.out / "logs" / (runtime_name + "-build.stdout")
        stderr_path = self.out / "logs" / (runtime_name + "-build.stderr")
        time_command = [
            "/usr/bin/time", "-v", "-o", str(timing), str(self.driver),
            str(WORKLOAD), str(runtime_snapshot), str(image),
        ]
        cgroup_path = self.out / "logs" / (runtime_name + "-build.cgroup")
        status_path = self.out / "logs" / (runtime_name + "-build.status")
        command = [str(BUILD_HELPER), str(timing), str(cgroup_path), str(status_path), "2400",
                   "--", *time_command]
        with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
            result = subprocess.run(command, stdout=stdout, stderr=stderr, check=False,
                                    pass_fds=(3,))
        peak = parse_time(timing) if timing.is_file() and timing.stat().st_size else None
        cgroup = {}
        if cgroup_path.is_file():
            for line in cgroup_path.read_text().splitlines():
                key, separator, value = line.partition("=")
                if separator:
                    cgroup[key] = int(value) if key in ("memory.peak", "command_status") else value
        record = {
            "runtime": runtime_name,
            "command": command,
            "status": result.returncode,
            "image_sha256": sha256(image) if image.is_file() else None,
            "time_v": str(timing),
            "peak_rss_bytes": peak,
            "uncapped_cgroup": cgroup,
            "cgroup_record": str(cgroup_path),
            "status_record": str(status_path),
            "stdout": str(stdout_path),
            "stderr": str(stderr_path),
        }
        self.builds.append(record)
        if hasattr(self, "results"):
            (self.out / "results.json").write_text(json.dumps(self.results, indent=2, sort_keys=True) + "\n")
        require(result.returncode == 0 and image.is_file(),
                "compiler build failed for " + runtime_name
                + " with status " + str(result.returncode) + "; see " + str(stderr_path))
        image.chmod(image.stat().st_mode | 0o111)
        require(peak is not None, "compiler peak RSS was not captured for " + runtime_name)
        require(cgroup.get("memory.max") == "max" and cgroup.get("memory.swap.max") == "max"
                and cgroup.get("memory.peak", 0) > 0,
                "compiler uncapped cgroup peak was not captured for " + runtime_name)
        return image

    def run_image(self, image, runtime_name, iteration):
        output_log = self.out / "logs" / ("run-%s-%02d.stdout" % (runtime_name, iteration))
        stderr_log = self.out / "logs" / ("run-%s-%02d.stderr" % (runtime_name, iteration))
        command = [str(image)]
        with output_log.open("wb") as stdout, stderr_log.open("wb") as stderr:
            result = run_limited(command, 600, stdout=stdout, stderr=stderr)
        require(result.returncode == 0,
                "%s run failed with status %s: %s" % (runtime_name, result.returncode, stderr_log))
        return parse_metrics(output_log.read_bytes())

    def run(self):
        self.prepare()
        self.build_driver()
        images = {}
        for runtime_name, runtime in (
            ("control", self.out / "control-runtime"),
            ("staged", self.out / "staged-runtime"),
        ):
            images[runtime_name] = self.build(runtime_name, runtime)
        samples = {name: {"control": [], "staged": []} for name, *_ in CASES}
        for iteration in range(self.repeats):
            order = ("control", "staged") if iteration % 2 == 0 else ("staged", "control")
            for runtime_name in order:
                metrics = self.run_image(images[runtime_name], runtime_name, iteration)
                for name, fields in metrics.items():
                    samples[name][runtime_name].append(fields["cpu_us"])
            print("allocator-probe repeat", iteration + 1, "of", self.repeats, flush=True)
        self.samples = {}
        for name, mode, count, payload, groups in CASES:
            medians = {runtime: statistics.median(values)
                       for runtime, values in samples[name].items()}
            speedup = medians["control"] / medians["staged"] if medians["staged"] else 0
            self.samples[name] = {
                "parameters": {"mode": mode, "n": count, "payload": payload, "groups": groups},
                "cpu_us": samples[name],
                "median_cpu_us": medians,
                "control_over_staged": speedup,
            }
            print("allocator-probe", name, "control_us", medians["control"],
                  "staged_us", medians["staged"], "control_over_staged", round(speedup, 4), flush=True)
        self.results["cases"] = self.samples
        hot = self.samples["zero-1024x10000"]
        improvement = 1.0 - hot["median_cpu_us"]["staged"] / hot["median_cpu_us"]["control"]
        self.results["hot_fixture"] = "zero-1024x10000"
        self.results["hot_fixture_improvement"] = improvement
        self.results["peak_rss_bytes_by_build"] = [item["peak_rss_bytes"] for item in self.builds]
        self.results["summary"] = {
            "compiler_build_count": len(self.builds),
            "all_compiler_builds_measured": all(item["peak_rss_bytes"] > 0 for item in self.builds),
            "hot_fixture_improvement": improvement,
            "improvement_gate": ">= 0.10" if self.require_improvement else "measurement only",
        }
        (self.out / "results.json").write_text(json.dumps(self.results, indent=2, sort_keys=True) + "\n")
        if self.require_improvement:
            require(improvement >= 0.10,
                    "zero-1024x10000 median CPU improvement is %.2f%%, below 10%%" % (improvement * 100))
        print("allocator-probe hot-fixture-improvement %.2f%%" % (improvement * 100))
        print("allocator-probe builds %d; exact peak RSS bytes recorded in %s" %
              (len(self.builds), self.out / "results.json"))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("seed")
    parser.add_argument("source")
    parser.add_argument("out")
    parser.add_argument("--repeats", type=int, default=5)
    parser.add_argument("--require-improvement", action="store_true")
    args = parser.parse_args()
    try:
        require(args.repeats >= 3 and args.repeats <= 15, "repeats must be between 3 and 15")
        Probe(args.seed, args.source, args.out, args.repeats, args.require_improvement).run()
    except (Failure, OSError, subprocess.SubprocessError) as exc:
        print("FAIL allocator probe: " + str(exc), file=sys.stderr)
        raise SystemExit(1)


if __name__ == "__main__":
    main()
