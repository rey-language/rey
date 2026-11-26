#!/usr/bin/env python3
"""Bounded current-source teardown and mapping-ownership checks for Linux x64."""

import argparse
from collections import Counter
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


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
POSITIVE = (
    "reclaim-three-slabs",
    "direct-mapping-metrics",
    "opaque-handle-reuse",
    "active-context-destroy",
    "teardown-many-direct-chunks",
)
NEGATIVE = (
    "excessive-used",
    "wrong-owner",
    "bad-size",
    "wrong-base",
    "duplicate-mapping",
    "corrupt-slab-links",
    "wrong-accounting",
)
FORBIDDEN = {
    "clang", "clang++", "cc", "gcc", "g++", "llc", "rustc", "cargo",
    "python", "python2", "python3", "zig", "go", "ld", "ld.lld", "as",
    "llvm-as", "llvm-objcopy",
}


class Failure(RuntimeError):
    pass


def require(condition, message):
    if not condition:
        raise Failure(message)


def sha256(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def digest_bytes(data):
    return hashlib.sha256(data).hexdigest()


def memory_available_gib():
    try:
        rows = [line.split() for line in Path("/proc/meminfo").read_text().splitlines()
                if line.startswith("MemAvailable:")]
    except OSError as exc:
        raise Failure("could not read MemAvailable from /proc/meminfo: " + str(exc))
    require(len(rows) == 1 and len(rows[0]) == 3 and rows[0][2] == "kB",
            "/proc/meminfo has no valid MemAvailable field")
    available_kib = int(rows[0][1])
    require(available_kib >= 0, "/proc/meminfo has a negative MemAvailable value")
    return available_kib // (1024 * 1024)


def tree_hashes(root):
    root = Path(root)
    return {str(path.relative_to(root)): sha256(path)
            for path in sorted(root.rglob("*")) if path.is_file()}


class Runner:
    def __init__(self, base, source, out):
        self.base = Path(base).resolve(strict=True)
        self.source = Path(source).resolve(strict=True)
        self.out = Path(out).resolve()
        self.budget = self.source / "scripts/v04-linux-x64-budget.sh"
        self.results = {
            "gate": "R02 current-source teardown",
            "base": str(self.base),
            "base_sha256": sha256(self.base),
            "source": str(self.source),
            "platform": platform.platform(),
            "machine": platform.machine(),
            "permission_setup": "chmod is applied only to the generated test fixture executable",
            "commands": [],
            "walks": {},
            "faults": {},
            "checks": {},
        }
        self.logs = self.out / "logs"
        self.evidence = self.out / "evidence"
        self.runtime = self.out / "runtime-rey"
        self.fixture_source = self.runtime / "teardown-fixture.rey"
        self.binary = self.out / "bin/teardown-fixture"
        self.counter = 0

    def prepare(self):
        require(platform.system() == "Linux" and platform.machine().lower() in ("x86_64", "amd64"),
                "R02 requires native Linux x64")
        available = memory_available_gib()
        require(available >= 5, "available memory is below 5 GiB; no build may start")
        self.results["free_g_available_gib_before_build"] = available
        try:
            lock = Path(os.readlink("/proc/self/fd/3")).resolve()
        except OSError:
            lock = None
        require(lock == (Path.home() / "rey/build.lock").resolve(),
                "R02 acceptance must hold the B01 build lock for the complete suite")
        require(self.base.is_file() and os.access(self.base, os.X_OK), "BASE must be executable")
        require(sha256(self.base) == "64c8bbe4e559a2355b79bfa0d357bd6e81d6dee92bbac73fe520165004883ee6",
                "BASE is not the hash-pinned native comparison producer")
        require(self.source.is_dir() and (self.source / "runtime-rey/regions/main.rey").is_file(),
                "SRC does not contain runtime-rey/regions/main.rey")
        require(self.budget.is_file(), "B01 build budget wrapper is missing")
        require(shutil.which("strace") is not None, "strace is required for unmap evidence")
        require(shutil.which("/usr/bin/time") is not None, "/usr/bin/time is required for fault counts")
        try:
            self.out.relative_to(self.source)
            raise Failure("output directory must be outside SRC")
        except ValueError:
            pass
        require(not self.out.exists() or not any(self.out.iterdir()),
                "OUT must be fresh and empty: " + str(self.out))
        self.out.mkdir(parents=True, exist_ok=True)
        self.logs.mkdir(parents=True, exist_ok=True)
        self.evidence.mkdir(parents=True, exist_ok=True)
        self.results["source_revision"] = self.git_revision()
        self.results["runtime_source_hashes"] = tree_hashes(self.source / "runtime-rey")
        shutil.copytree(self.source / "runtime-rey", self.runtime)
        self.results["staged_runtime_source_hashes_before_test_hooks"] = tree_hashes(self.runtime)
        with (self.runtime / "regions/main.rey").open("a") as staged_regions:
            staged_regions.write((HERE / "test-hooks.rey").read_text())
        shutil.copyfile(HERE / "fixture.rey", self.fixture_source)
        self.results["staged_runtime_source_hashes"] = tree_hashes(self.runtime)
        self.results["test_hook_sha256"] = sha256(HERE / "test-hooks.rey")
        self.results["instrumented_regions_sha256"] = sha256(self.runtime / "regions/main.rey")
        self.results["harness_hashes"] = {
            str(path.relative_to(self.source)): sha256(path)
            for path in (HERE / "fixture.rey", HERE / "test-hooks.rey", HERE / "run.py", HERE / "run.sh")
        }
        self.save()
        self.build_fixture()

    def git_revision(self):
        result = subprocess.run(["git", "-C", str(self.source), "rev-parse", "HEAD"],
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                timeout=10, check=False)
        return result.stdout.decode().strip() if result.returncode == 0 else "unavailable"

    def save(self):
        path = self.out / "results.json"
        path.write_text(json.dumps(self.results, indent=2, sort_keys=True) + "\n")

    def invoke(self, label, argv, timeout, *, build=False, expected_status=0):
        self.counter += 1
        slug = "%03d-%s" % (self.counter, re.sub(r"[^A-Za-z0-9._-]", "_", label))
        stdout_path = self.logs / (slug + ".stdout")
        stderr_path = self.logs / (slug + ".stderr")
        trace_path = self.logs / (slug + ".execve")
        cgroup_path = self.evidence / (slug + ".cgroup")
        status_path = self.evidence / (slug + ".status")
        marker_path = self.evidence / (slug + ".started")
        require(not marker_path.exists() and not status_path.exists(), "stale command marker for " + label)

        command = list(map(str, argv))
        producer = command[0]
        if build:
            command = ["strace", "-f", "-e", "trace=execve,execveat", "-o", str(trace_path), "--"] + command
            wrapped = self.budget_command(slug, timeout, stdout_path, stderr_path,
                                          status_path, cgroup_path, marker_path, command)
        else:
            shell = "exec nice -n 19 timeout --kill-after=30s %d \"$@\"" % timeout
            wrapped = ["nice", "-n", "19", "timeout", "--kill-after=30s", str(timeout),
                       "sh", "-c", shell, "r02-run", *command]

        started = time.monotonic()
        with stdout_path.open("wb") as out_stream, stderr_path.open("wb") as err_stream:
            try:
                proc = subprocess.Popen(wrapped, stdout=out_stream, stderr=err_stream,
                                        start_new_session=True, pass_fds=(3,) if build else ())
            except OSError as exc:
                raise Failure(label + " could not start: " + str(exc))
            deadline = started + timeout + 60
            next_memory_check = started
            minimum_available = None
            stop_reason = None
            while proc.poll() is None:
                now = time.monotonic()
                if now >= deadline:
                    stop_reason = "outer timeout"
                elif now >= next_memory_check:
                    try:
                        available = memory_available_gib()
                        if minimum_available is None or available < minimum_available:
                            minimum_available = available
                    except (OSError, subprocess.TimeoutExpired, ValueError, Failure):
                        stop_reason = "could not read free -g during command"
                    next_memory_check = now + 5
                if stop_reason:
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
        elapsed = round(time.monotonic() - started, 6)
        stdout = stdout_path.read_bytes()
        stderr = stderr_path.read_bytes()
        require(stop_reason is None, label + " stopped: " + str(stop_reason))
        record = {
            "label": label, "argv": list(map(str, argv)), "status": status,
            "expected_status": expected_status, "elapsed_seconds": elapsed,
            "minimum_free_g_available_gib": minimum_available,
            "stdout_log": str(stdout_path), "stderr_log": str(stderr_path),
            "stdout_sha256": digest_bytes(stdout), "stderr_sha256": digest_bytes(stderr),
            "error": stop_reason,
        }
        if build:
            require(status_path.is_file(), label + " budget wrapper exited without a child status record")
            child_status_text = status_path.read_text().strip()
            require(child_status_text.isdigit() and int(child_status_text) == status,
                    label + " budget wrapper status disagrees with the child status")
            cgroup_text = cgroup_path.read_text()
            oom_events = re.findall(r"^(?:oom|oom_kill|oom_group_kill)\s+(\d+)$",
                                    cgroup_text, re.MULTILINE)
            require(len(oom_events) == 6 and all(int(value) == 0 for value in oom_events),
                    label + " cgroup recorded an OOM event: " + repr(oom_events))
            record["execve_trace"] = str(trace_path)
            record["cgroup_evidence"] = str(cgroup_path)
            record["budget_status"] = child_status_text
            record["cgroup_oom_events"] = [int(value) for value in oom_events]
        self.results["commands"].append(record)
        self.save()
        require(status == expected_status,
                label + ": expected status " + str(expected_status) + ", got " + str(status)
                + "; stdout=" + repr(stdout.decode("utf-8", "replace"))
                + "; stderr=" + repr(stderr.decode("utf-8", "replace")))
        if build:
            self.check_exec_trace(trace_path, producer, label)
        return stdout, stderr, elapsed, slug

    def budget_command(self, slug, timeout, stdout_path, stderr_path,
                       status_path, cgroup_path, marker_path, command):
        capture = (
            'out=$1; err=$2; result=$3; evidence=$4; started=$5; seconds=$6; shift 6; '
            'cap=${REY_CGROUP_CAP:-}; case "$cap" in *G) ;; *) exit 125 ;; esac; '
            'cap_gib=${cap%G}; case "$cap_gib" in ""|*[!0-9]*) exit 125 ;; esac; '
            "avail=$(LC_ALL=C free -g | awk '$1 == \"Mem:\" { print $7; found=1; exit } END { if (!found) exit 1 }') || exit 75; "
            '[ "$avail" -ge 5 ] || { echo "available memory $avail GiB is below the R02 5 GiB build floor"; exit 75; }; '
            'cg_rel=$(sed -n "s/^0:://p" /proc/self/cgroup); [ -n "$cg_rel" ] || exit 125; '
            'cg_dir=/sys/fs/cgroup$cg_rel; [ -r "$cg_dir/memory.max" ] && [ -r "$cg_dir/memory.swap.max" ] || exit 125; '
            'mem_max=$(cat "$cg_dir/memory.max"); swap_max=$(cat "$cg_dir/memory.swap.max"); '
            'cap_bytes=$((cap_gib * 1073741824)); swap_bytes=$((512 * 1048576)); '
            'case "$mem_max:$swap_max" in *[!0-9:]*|max:*|*:max) exit 125 ;; esac; '
            '[ "$mem_max" -gt 0 ] && [ "$mem_max" -le "$cap_bytes" ] || exit 125; '
            '[ "$swap_max" -ge 0 ] && [ "$swap_max" -le "$swap_bytes" ] || exit 125; '
            '{ printf "memory.max=%s\nmemory.swap.max=%s\nmemory.peak.before=%s\n" "$mem_max" "$swap_max" "$(cat "$cg_dir/memory.peak")"; cat "$cg_dir/memory.events"; } >"$evidence"; '
            ': >"$started"; set +e; nice -n 19 timeout --kill-after=30s "$seconds" "$@" >"$out" 2>"$err"; code=$?; set -e; '
            '{ printf "memory.peak.after=%s\n" "$(cat "$cg_dir/memory.peak")"; cat "$cg_dir/memory.events"; printf "command_status=%s\n" "$code"; } >>"$evidence"; '
            'printf "%s\n" "$code" >"$result"; exit "$code"'
        )
        launcher = (
            'capture=$1; out=$2; err=$3; result=$4; evidence=$5; started=$6; seconds=$7; shift 7; cap=${REY_CGROUP_CAP:-}; '
            'run_scope() { mode=$1; shift; if [ "$mode" = user ]; then '
            'set -- systemd-run --user --scope --quiet -p "MemoryMax=$cap" -p MemorySwapMax=512M -- sh -c "$capture" r02-capture '
            '"$out" "$err" "$result" "$evidence" "$started" "$seconds" "$@"; '
            'else set -- sudo -n /usr/bin/systemd-run --system --scope --quiet -p "MemoryMax=$cap" -p MemorySwapMax=512M -- sh -c "$capture" r02-capture '
            '"$out" "$err" "$result" "$evidence" "$started" "$seconds" "$@"; fi; "$@"; }; '
            'set +e; run_scope user "$@"; code=$?; set -e; [ ! -f "$started" ] || exit "$code"; '
            'set +e; run_scope system "$@"; code=$?; set -e; exit "$code"'
        )
        return ["sh", str(self.budget), "run-held", "--seconds", str(timeout),
                "--label", slug, "--", "sh", "-c", launcher, "r02-launcher",
                capture, str(stdout_path), str(stderr_path), str(status_path),
                str(cgroup_path), str(marker_path), str(timeout), *command]

    def check_exec_trace(self, path, producer, label):
        lines = Path(path).read_text(errors="replace").splitlines()
        exec_lines = [line for line in lines if "execve(" in line or "execveat(" in line]
        expected = str(Path(producer).resolve())
        successful = [line for line in exec_lines if " = 0" in line]
        require(len(successful) == 1 and expected in successful[0],
                label + " unexpected successful exec trace: " + repr(exec_lines))
        for line in exec_lines:
            paths = re.findall(r'execve(?:at)?\("([^\"]+)"', line)
            for command_path in paths:
                name = Path(command_path).name.lower()
                require(name not in FORBIDDEN and not name.startswith(("clang-", "python3.", "llvm-", "ld.lld-")),
                        label + " forbidden exec attempt: " + line)
                require(command_path == expected, label + " unexpected exec attempt: " + line)

    def build_fixture(self):
        self.binary.parent.mkdir(parents=True, exist_ok=True)
        command = [str(self.base), "build", str(self.fixture_source), "-o", str(self.binary),
                   "--target", "linux-x64", "--regions", "--managed", "--runtime-root", str(self.runtime)]
        self.invoke("build-current-source-teardown-fixture", command, 600, build=True)
        require(self.binary.is_file(), "fixture build succeeded without an image")
        self.binary.chmod(self.binary.stat().st_mode | 0o111)
        self.results["fixture_sha256"] = sha256(self.binary)
        self.fixture_image = self.binary
        self.save()

    @staticmethod
    def parse_inventory(stdout):
        inventory = []
        walks = []
        for line in stdout.decode("utf-8", "replace").splitlines():
            if line.startswith("owned-map "):
                fields = line.split()
                require(len(fields) == 4, "malformed owned-map line: " + line)
                kind = fields[1]
                address = int(fields[2])
                extent = int(fields[3])
                require(kind in ("context", "program", "slab", "direct"),
                        "unknown owned mapping kind: " + kind)
                inventory.append({"kind": kind, "address": address, "extent": extent})
            elif line.startswith("unowned-map "):
                fields = line.split()
                require(len(fields) == 4, "malformed unowned-map line: " + line)
                inventory.append({"kind": "unowned", "address": int(fields[2]), "extent": int(fields[3])})
            elif line.startswith("walk "):
                fields = line.split()
                item = {"case": fields[1]}
                for field in fields[2:]:
                    key, value = field.split("=", 1)
                    item[key] = int(value)
                walks.append(item)
        return inventory, walks

    @staticmethod
    def parse_unmap_attempts(path):
        result = []
        pattern = re.compile(r"munmap\((0x[0-9a-fA-F]+),\s*(\d+)\)")
        for line in Path(path).read_text(errors="replace").splitlines():
            match = pattern.search(line)
            if match:
                success = re.search(r"\)\s*=\s*0(?:\s|$)", line[match.start():]) is not None
                result.append((int(match.group(1), 16), int(match.group(2)), success))
        return result

    @staticmethod
    def parse_time(path):
        text = Path(path).read_text(errors="replace")
        major = re.search(r"Major \(requiring I/O\) page faults:\s*(\d+)", text)
        minor = re.search(r"Minor \(reclaiming a frame\) page faults:\s*(\d+)", text)
        return {"major_page_faults": int(major.group(1)) if major else None,
                "minor_page_faults": int(minor.group(1)) if minor else None}

    def run_case(self, case, expected_status):
        trace = self.logs / (case + ".munmap")
        timing = self.logs / (case + ".time")
        command = ["/usr/bin/time", "-v", "-o", str(timing), "strace", "-f",
                   "-e", "trace=munmap", "-o", str(trace), "--",
                   str(self.fixture_image), case]
        stdout, stderr, elapsed, slug = self.invoke(
            "run-" + case, command, 300, expected_status=expected_status)
        require(stderr == b"", case + " wrote unexpected stderr: " + repr(stderr))
        text = stdout.decode("utf-8", "replace")
        inventory, walks = self.parse_inventory(stdout)
        attempts = self.parse_unmap_attempts(trace)
        unmaps = [(address, extent) for address, extent, success in attempts if success]
        if case in NEGATIVE:
            require(text.rstrip().endswith("checkpoint: expected-rejection-" + case),
                    case + " did not reach its named pre-destroy rejection checkpoint")
            require("unexpected destroy success" not in text, case + " destroy unexpectedly returned")
            unmap_counts = Counter(unmaps)
            require(not attempts,
                    case + " issued a munmap before rejecting corruption: " + repr(attempts))
            for item in inventory:
                if item["kind"] == "unowned":
                    continue
                require(unmap_counts[(item["address"], item["extent"])] == 0,
                        case + " unmapped a context-owned mapping before rejecting corruption: " + str(item))
            if case == "wrong-base":
                unowned = [item for item in inventory if item["kind"] == "unowned"]
                require(len(unowned) == 1 and (unowned[0]["address"], unowned[0]["extent"]) not in unmaps,
                        "wrong-base rejection unmapped the substituted unowned address")
            self.results["checks"][case] = {
                "status": expected_status, "named_checkpoint": True,
                "owned_mappings_not_unmapped": len([x for x in inventory if x["kind"] != "unowned"]),
                "unmaps": len(unmaps), "stdout_sha256": digest_bytes(stdout),
            }
        else:
            checkpoint = "checkpoint: " + case + "-live-caller"
            require(checkpoint in text, case + " did not reach its live-caller checkpoint after destroy")
            if case != "active-context-destroy":
                require(text.rstrip().endswith(checkpoint), case + " had output after its destroy checkpoint")
            else:
                final_line = text.rstrip().splitlines()[-1]
                require(re.fullmatch(r"walk active-context-destroy-total rounds=128 direct=256 requested-bytes=\d+",
                                     final_line) is not None,
                        "active-context-destroy did not complete all 128 rounds")
            require(inventory, case + " emitted no mapping inventory")
            inventory_counts = Counter((item["address"], item["extent"]) for item in inventory)
            unmap_counts = Counter(unmaps)
            attempt_counts = Counter((address, extent) for address, extent, _success in attempts)
            for item in inventory:
                key = (item["address"], item["extent"])
                require(unmap_counts[key] == inventory_counts[key] and attempt_counts[key] == inventory_counts[key],
                        case + " did not release each owned mapping exactly once: " + str(item))
            self.results["checks"][case] = {
                "status": expected_status, "live_caller_checkpoint": True,
                "owned_mappings": len(inventory), "unmaps": len(unmaps),
                "stdout_sha256": digest_bytes(stdout), "elapsed_seconds": elapsed,
            }
        expected_walks = 2 if case == "active-context-destroy" else 1
        require(len(walks) == expected_walks,
                case + " did not report the expected per-walk counts")
        self.results["walks"][case] = walks
        self.results["faults"][case] = self.parse_time(timing)
        self.results["faults"][case]["elapsed_seconds"] = elapsed
        self.results["faults"][case]["time_file"] = str(timing)
        self.results["checks"][case]["unmap_trace"] = str(trace)
        self.save()

    def run(self):
        self.prepare()
        for case in POSITIVE:
            self.run_case(case, 0)
        for case in NEGATIVE:
            self.run_case(case, 1)
        require(len(self.results["checks"]) == len(POSITIVE) + len(NEGATIVE),
                "not every R02 positive and corruption fixture ran")
        self.results["summary"] = {
            "reclamation_cases": len(POSITIVE),
            "named_rejections": len(NEGATIVE),
            "all_owned_mappings_checked": True,
            "all_runs_report_page_faults": all(
                value.get("major_page_faults") is not None and value.get("minor_page_faults") is not None
                for value in self.results["faults"].values()),
        }
        require(self.results["summary"]["all_runs_report_page_faults"],
                "time -v did not report page faults for every fixture")
        self.save()
        print("R02 reclamation: 5/5 cases, 7/7 named corruption rejections")
        print("R02 mappings: every owned base and extent released once or preserved on rejection")
        print("R02 teardown: checkpoints passed; per-walk counts and page faults in " + str(self.out / "results.json"))
        print("PASS R02 teardown")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("base")
    parser.add_argument("source")
    parser.add_argument("out")
    args = parser.parse_args()
    try:
        Runner(args.base, args.source, args.out).run()
    except Failure as exc:
        print("FAIL R02 teardown: " + str(exc), file=sys.stderr)
        raise SystemExit(1)


if __name__ == "__main__":
    main()
