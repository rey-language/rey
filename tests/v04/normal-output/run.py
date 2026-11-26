#!/usr/bin/env python3
"""Bounded staged checks for the Linux x64 image writer."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import signal
import struct
import subprocess
import sys
import time


HERE = Path(__file__).resolve().parent
SEED_SHA256 = "64c8bbe4e559a2355b79bfa0d357bd6e81d6dee92bbac73fe520165004883ee6"
CODE = bytes((184, 60, 0, 0, 0, 49, 255, 15, 5, 195, 0, 127, 128, 255))
SYMBOLS = b"401000 <entry>\n401000 main\n401009 helper\n"
FORBIDDEN_EXEC_NAMES = {
    "clang", "clang++", "cc", "gcc", "g++", "llc", "rustc", "cargo",
    "python", "python2", "python3", "zig", "go", "ld", "ld.lld", "as",
    "llvm-as", "llvm-objcopy",
}


class Failure(RuntimeError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise Failure(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def file_mode(path: Path) -> int:
    return path.stat().st_mode & 0o777


def expected_elf() -> bytes:
    length = 4096 + len(CODE)
    ident = b"\x7fELF\x02\x01\x01" + bytes(9)
    header = struct.pack("<16sHHIQQQIHHHHHH", ident, 2, 62, 1, 0x401000, 64, 0, 0,
                         64, 56, 2, 0, 0, 0)
    load = struct.pack("<IIQQQQQQ", 1, 5, 0, 0x400000, 0x400000, length, length, 4096)
    stack = struct.pack("<IIQQQQQQ", 1685382481, 6, 0, 0, 0, 0, 0, 16)
    return header + load + stack + bytes(4096 - len(header) - len(load) - len(stack)) + CODE


class Runner:
    def __init__(self, seed: Path, out: Path, source: Path):
        self.seed = seed.resolve(strict=True)
        self.source = source.resolve(strict=True)
        self.requested_out = out.resolve()
        self.out = self.requested_out
        self.budget = self.source / "scripts/v04-linux-x64-budget.sh"
        self.results: dict[str, object] = {
            "gate": "O01 normal output",
            "seed": str(self.seed),
            "seed_sha256": sha256(self.seed),
            "source": str(self.source),
            "source_revision": self.git_revision(),
            "platform": platform.platform(),
            "machine": platform.machine(),
            "commands": [],
            "checks": {},
            "images": {},
        }
        self.logs = self.out / "logs"
        self.evidence = self.out / "evidence"
        self.stage = self.out / "staged"
        self.compiler_source = self.stage / "main.rey"
        self.counter = 0

    def git_revision(self) -> str:
        result = subprocess.run(["git", "-C", str(self.source), "rev-parse", "HEAD"],
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=10, check=False)
        return result.stdout.decode().strip() if result.returncode == 0 else "unavailable"

    def save(self) -> None:
        (self.out / "results.json").write_text(json.dumps(self.results, indent=2, sort_keys=True) + "\n")

    def prepare(self) -> None:
        require(platform.system() == "Linux" and platform.machine().lower() in ("x86_64", "amd64"),
                "O01 requires native Linux x64")
        require(self.seed.is_file() and os.access(self.seed, os.X_OK), "SEED must be executable")
        require(self.results["seed_sha256"] == SEED_SHA256, "SEED hash does not match pinned S3")
        require(self.budget.is_file() and shutil.which("strace") is not None,
                "B01 budget wrapper and strace are required")
        require(self.source.is_dir() and (self.source / "rey-compiler/native-main.rey").is_file(),
                "SOURCE does not contain the native compiler")
        require(self.source != self.requested_out and self.source not in self.requested_out.parents,
                "OUT must be outside SOURCE")
        if self.out.exists():
            prior_path = self.out / "results.json"
            require(prior_path.is_file(), "OUT exists without O01 evidence; refusing to reuse it")
            prior = json.loads(prior_path.read_text())
            require(prior.get("gate") == "O01 normal output" and
                    prior.get("seed_sha256") == SEED_SHA256 and
                    prior.get("source") == str(self.source),
                    "OUT contains unrelated evidence; refusing to reuse it")
            attempt = 2
            while (self.requested_out / ("attempt-%02d" % attempt)).exists():
                attempt += 1
            self.out = self.requested_out / ("attempt-%02d" % attempt)
            self.results["previous_attempt"] = str(self.requested_out)
            self.logs = self.out / "logs"
            self.evidence = self.out / "evidence"
            self.stage = self.out / "staged"
            self.compiler_source = self.stage / "main.rey"
        self.out.parent.mkdir(parents=True, exist_ok=True)
        self.out.mkdir()
        self.logs.mkdir()
        self.evidence.mkdir()
        self.stage.mkdir()
        self.results["build_lock"] = str(Path.home() / "rey/build.lock")
        self.results["resource_policy"] = "scripts/v04-linux-x64-budget.sh, 8 GiB host-derived cap, 512 MiB swap cap"
        self.save()

    def stage_modules(self, *, faulted: bool = False) -> Path:
        root = self.stage if not faulted else self.out / "fault-stage"
        if faulted:
            shutil.copytree(self.source / "rey-compiler/src", root)
            shutil.copyfile(self.source / "rey-compiler/native-main.rey", root / "main.rey")
            runtime = root / "runtime-rey"
            shutil.copytree(self.source / "runtime-rey", runtime)
            self.inject_faults(runtime / "host/main.rey")
            embed_root = root
        else:
            shutil.copytree(self.source / "rey-compiler/src", root, dirs_exist_ok=True)
            shutil.copyfile(self.source / "rey-compiler/native-main.rey", root / "main.rey")
            embed_root = self.source
        overlay = root / "frontend/embedded_runtime/main.rey"
        result = subprocess.run([sys.executable, "-B", str(self.source / "scripts/embed-runtime.py"),
                                 "--root", str(embed_root), "--output", str(overlay)],
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=120, check=False)
        require(result.returncode == 0,
                "runtime overlay generation failed: " + result.stderr.decode("utf-8", "replace"))
        return root

    @staticmethod
    def inject_faults(host: Path) -> None:
        text = host.read_text()
        insert = '''
var o01FaultMode: int = 0;

func o01FaultPath(path: __rey_raw_address, suffixLength: int, expectedHash: int): bool {
    var length = 0;
    while length < 4096 && __reyRawLoadByte(__reyRawOffset(path, length)) != 0 { length = length + 1; }
    if length < suffixLength { return false; }
    var index = 0;
    var hash = 0;
    while index < suffixLength {
        hash = hash * 31 + __reyRawLoadByte(__reyRawOffset(path, length - suffixLength + index));
        index = index + 1;
    }
    return hash == expectedHash;
}
'''
        text = text.replace("func hostRawInt(value: int): __rey_raw_address {", insert + "\nfunc hostRawInt(value: int): __rey_raw_address {", 1)
        changes = (
            ('''pub func hostOpen(path: __rey_raw_address, flags: int, mode: int): int {
    return __reyRawToInt(__reyRawHostSyscall(2, path, hostRawInt(flags), hostRawInt(mode), __reyRawNull(), __reyRawNull(), __reyRawNull()));
}''',
             '''pub func hostOpen(path: __rey_raw_address, flags: int, mode: int): int {
    if flags == 577 {
        if o01FaultPath(path, 10, 1312259019416084) { o01FaultMode = 1; }
        else if o01FaultPath(path, 10, 1312259000772653) { o01FaultMode = 2; }
        else if o01FaultPath(path, 9, 42330935948481) { o01FaultMode = 3; }
    }
    return __reyRawToInt(__reyRawHostSyscall(2, path, hostRawInt(flags), hostRawInt(mode), __reyRawNull(), __reyRawNull(), __reyRawNull()));
}''', "hostOpen"),
            ('''pub func hostWrite(fd: int, base: __rey_raw_address, length: int): int {
    return __reyRawToInt(__reyRawHostSyscall(1, hostRawInt(fd), base, hostRawInt(length), __reyRawNull(), __reyRawNull(), __reyRawNull()));
}''',
             '''pub func hostWrite(fd: int, base: __rey_raw_address, length: int): int {
    if o01FaultMode == 1 { return -5; }
    return __reyRawToInt(__reyRawHostSyscall(1, hostRawInt(fd), base, hostRawInt(length), __reyRawNull(), __reyRawNull(), __reyRawNull()));
}''', "hostWrite"),
            ('''pub func hostRead(fd: int, base: __rey_raw_address, length: int): int {
    return __reyRawToInt(__reyRawHostSyscall(0, hostRawInt(fd), base, hostRawInt(length), __reyRawNull(), __reyRawNull(), __reyRawNull()));
}''',
             '''pub func hostRead(fd: int, base: __rey_raw_address, length: int): int {
    if o01FaultMode == 3 { return -5; }
    return __reyRawToInt(__reyRawHostSyscall(0, hostRawInt(fd), base, hostRawInt(length), __reyRawNull(), __reyRawNull(), __reyRawNull()));
}''', "hostRead"),
            ('''pub func hostClose(fd: int): int {
    return __reyRawToInt(__reyRawHostSyscall(3, hostRawInt(fd), __reyRawNull(), __reyRawNull(), __reyRawNull(), __reyRawNull(), __reyRawNull()));
}''',
             '''pub func hostClose(fd: int): int {
    var result = __reyRawToInt(__reyRawHostSyscall(3, hostRawInt(fd), __reyRawNull(), __reyRawNull(), __reyRawNull(), __reyRawNull(), __reyRawNull()));
    if o01FaultMode == 2 { return -5; }
    return result;
}''', "hostClose"),
        )
        for old, new, label in changes:
            require(old in text, "could not inject " + label + " failure")
            text = text.replace(old, new, 1)
        host.write_text(text)

    def invoke_build(self, label: str, argv: list[str], timeout: int = 600,
                     expected_status: int = 0) -> tuple[bytes, bytes]:
        self.counter += 1
        slug = "%03d-%s" % (self.counter, re.sub(r"[^A-Za-z0-9._-]", "_", label))
        stdout_path = self.logs / (slug + ".stdout")
        stderr_path = self.logs / (slug + ".stderr")
        wrapper_stdout_path = self.logs / (slug + ".wrapper.stdout")
        wrapper_stderr_path = self.logs / (slug + ".wrapper.stderr")
        trace_path = self.evidence / (slug + ".execve")
        cgroup_path = self.evidence / (slug + ".cgroup")
        timing_path = self.evidence / (slug + ".time")
        started_path = self.evidence / (slug + ".started")
        status_path = self.evidence / (slug + ".status")
        stdout_path.touch()
        stderr_path.touch()
        capture = (
            'set -eu; out=$1; err=$2; trace=$3; cgroup=$4; timing=$5; started=$6; statusfile=$7; '
            'seconds=$8; shift 8; cap=${REY_CGROUP_CAP:-}; case "$cap" in *G) ;; *) exit 125 ;; esac; '
            'cap_gib=${cap%G}; case "$cap_gib" in ""|*[!0-9]*) exit 125 ;; esac; '
            'cg_rel=$(sed -n "s/^0:://p" /proc/self/cgroup); [ -n "$cg_rel" ] || exit 125; '
            'cg_dir=/sys/fs/cgroup$cg_rel; [ -r "$cg_dir/memory.max" ] && [ -r "$cg_dir/memory.swap.max" ] || exit 125; '
            'mem_max=$(cat "$cg_dir/memory.max"); swap_max=$(cat "$cg_dir/memory.swap.max"); '
            'cap_bytes=$((cap_gib * 1073741824)); swap_bytes=$((512 * 1048576)); '
            'case "$mem_max:$swap_max" in *[!0-9:]*|max:*|*:max) exit 125 ;; esac; '
            '[ "$mem_max" -gt 0 ] && [ "$mem_max" -le "$cap_bytes" ] || exit 125; '
            '[ "$swap_max" -ge 0 ] && [ "$swap_max" -le "$swap_bytes" ] || exit 125; '
            '{ printf "memory.max=%s\\nmemory.swap.max=%s\\nmemory.peak.before=%s\\n" "$mem_max" "$swap_max" "$(cat "$cg_dir/memory.peak")"; cat "$cg_dir/memory.events"; } >"$cgroup"; '
            ': >"$started"; set +e; nice -n 19 timeout --kill-after=30s "$seconds" '
            '/usr/bin/time -v -o "$timing" strace -f -e trace=execve,execveat -o "$trace" -- "$@" >"$out" 2>"$err"; '
            'code=$?; set -e; { printf "memory.peak.after=%s\\n" "$(cat "$cg_dir/memory.peak")"; '
            'cat "$cg_dir/memory.events"; printf "command_status=%s\\n" "$code"; } >>"$cgroup"; '
            'printf "%s\\n" "$code" >"$statusfile"; exit "$code"'
        )
        launcher = (
            'capture=$1; out=$2; err=$3; trace=$4; cgroup=$5; timing=$6; started=$7; statusfile=$8; '
            'seconds=$9; shift 9; cap=${REY_CGROUP_CAP:-}; run_scope() { which=$1; shift; '
            'if [ "$which" = user ]; then set -- systemd-run --user --scope --quiet -p "MemoryMax=$cap" '
            '-p MemorySwapMax=512M -- sh -c "$capture" o01-capture "$out" "$err" "$trace" "$cgroup" '
            '"$timing" "$started" "$statusfile" "$seconds" "$@"; else '
            'set -- sudo -n /usr/bin/systemd-run --system --scope --quiet -p "MemoryMax=$cap" '
            '-p MemorySwapMax=512M -- sh -c "$capture" o01-capture "$out" "$err" "$trace" "$cgroup" '
            '"$timing" "$started" "$statusfile" "$seconds" "$@"; fi; "$@"; }; '
            'set +e; run_scope user "$@"; code=$?; set -e; [ ! -f "$started" ] || exit "$code"; '
            'set +e; run_scope system "$@"; code=$?; set -e; exit "$code"'
        )
        wrapped = ["sh", str(self.budget), "run", "--seconds", str(timeout + 60), "--label", slug, "--",
                   "sh", "-c", launcher, "o01-launch", capture, str(stdout_path), str(stderr_path),
                   str(trace_path), str(cgroup_path), str(timing_path), str(started_path), str(status_path),
                   str(timeout), *map(str, argv)]
        start = time.monotonic()
        with wrapper_stdout_path.open("wb") as out_stream, wrapper_stderr_path.open("wb") as err_stream:
            try:
                proc = subprocess.Popen(wrapped, stdout=out_stream, stderr=err_stream,
                                        start_new_session=True)
            except OSError as exc:
                raise Failure(label + " could not start: " + str(exc))
            try:
                status = proc.wait(timeout=timeout + 150)
            except subprocess.TimeoutExpired:
                os.killpg(proc.pid, signal.SIGTERM)
                try:
                    proc.wait(timeout=30)
                except subprocess.TimeoutExpired:
                    os.killpg(proc.pid, signal.SIGKILL)
                    proc.wait()
                raise Failure(label + " exceeded its outer timeout")
        stdout, stderr = stdout_path.read_bytes(), stderr_path.read_bytes()
        require(status_path.is_file(), label + " budget wrapper did not record producer status: " +
                wrapper_stderr_path.read_text(errors="replace"))
        require(status_path.read_text().strip() == str(status),
                label + " budget and producer statuses differ")
        require(cgroup_path.is_file() and timing_path.is_file(), label + " omitted resource evidence")
        cgroup_text = cgroup_path.read_text()
        oom = re.findall(r"^(?:oom|oom_kill|oom_group_kill)\s+(\d+)$", cgroup_text, re.MULTILINE)
        require(len(oom) == 6 and all(int(value) == 0 for value in oom), label + " cgroup recorded an OOM")
        record = {"label": label, "argv": list(map(str, argv)), "status": status,
                  "expected_status": expected_status, "elapsed_seconds": round(time.monotonic() - start, 6),
                  "stdout_log": str(stdout_path), "stderr_log": str(stderr_path),
                  "wrapper_stdout_log": str(wrapper_stdout_path),
                  "wrapper_stderr_log": str(wrapper_stderr_path),
                  "execve_trace": str(trace_path), "cgroup_evidence": str(cgroup_path),
                  "time_evidence": str(timing_path)}
        commands = self.results["commands"]
        assert isinstance(commands, list)
        commands.append(record)
        self.save()
        require(status == expected_status,
                label + ": expected status " + str(expected_status) + ", got " + str(status)
                + "; stdout=" + repr(stdout.decode("utf-8", "replace"))
                + "; stderr=" + repr(stderr.decode("utf-8", "replace")))
        self.check_exec_trace(trace_path, Path(argv[0]), label)
        return stdout, stderr

    @staticmethod
    def check_exec_trace(path: Path, producer: Path, label: str) -> None:
        expected = str(producer.resolve())
        lines = [line for line in path.read_text(errors="replace").splitlines()
                 if "execve(" in line or "execveat(" in line]
        successes = [line for line in lines if re.search(r"\)\s*=\s*0(?:\s|$)", line)]
        require(len(successes) == 1 and expected in successes[0],
                label + " unexpected successful execs: " + repr(lines))
        for line in lines:
            match = re.search(r'execve(?:at)?\("([^\"]+)"', line)
            if match:
                command = match.group(1)
                name = Path(command).name.lower()
                require(command == expected and name not in FORBIDDEN_EXEC_NAMES and
                        not name.startswith(("clang-", "python3.", "llvm-", "ld.lld-")),
                        label + " unexpected exec attempt: " + line)

    def run_program(self, label: str, argv: list[str], expected_status: int | None = 0,
                    expected_stdout: bytes | None = None) -> tuple[bytes, bytes]:
        self.counter += 1
        slug = "%03d-%s" % (self.counter, re.sub(r"[^A-Za-z0-9._-]", "_", label))
        stdout_path, stderr_path = self.logs / (slug + ".stdout"), self.logs / (slug + ".stderr")
        with stdout_path.open("wb") as out_stream, stderr_path.open("wb") as err_stream:
            proc = subprocess.run(["nice", "-n", "19", "timeout", "--kill-after=30s", "30", *map(str, argv)],
                                  stdout=out_stream, stderr=err_stream, check=False,
                                  preexec_fn=lambda: os.umask(0o022))
        stdout, stderr = stdout_path.read_bytes(), stderr_path.read_bytes()
        if expected_status is None:
            require(proc.returncode != 0,
                    label + ": expected a nonzero status, got 0; stderr="
                    + repr(stderr.decode("utf-8", "replace")))
        else:
            require(proc.returncode == expected_status,
                    label + ": expected status " + str(expected_status) + ", got " + str(proc.returncode)
                    + "; stderr=" + repr(stderr.decode("utf-8", "replace")))
        if expected_stdout is not None:
            require(stdout == expected_stdout,
                    label + ": stdout differs: " + repr(stdout.decode("utf-8", "replace")))
        runs = self.results.setdefault("runs", [])
        assert isinstance(runs, list)
        runs.append({"label": label, "argv": list(map(str, argv)), "status": proc.returncode,
                     "expected_status": "nonzero" if expected_status is None else expected_status,
                     "stdout_log": str(stdout_path), "stderr_log": str(stderr_path)})
        self.save()
        return stdout, stderr

    def build_staged_driver(self, compiler: Path, source: Path, output: Path,
                            runtime: Path | None, label: str, *, trusted: bool) -> None:
        output.parent.mkdir(parents=True, exist_ok=True)
        command = [str(compiler), "build", str(source), "-o", str(output),
                   "--target", "linux-x64", "--managed", "--regions"]
        if trusted:
            command.append("--trust-packed-output")
        if runtime is not None:
            command.extend(["--runtime-root", str(runtime)])
        self.invoke_build(label, command, timeout=1950)
        require(output.is_file() and file_mode(output) == 0o755,
                label + " did not produce an immediately executable staged driver")
        self.check_elf(output)
        self.results["images"][output.name] = {"sha256": sha256(output), "mode": oct(file_mode(output))}
        self.save()

    @staticmethod
    def check_elf(path: Path) -> None:
        data = path.read_bytes()
        require(data[:7] == b"\x7fELF\x02\x01\x01", path.name + " is not ELF64 little-endian")
        fields = struct.unpack_from("<16sHHIQQQIHHHHHH", data)
        require(fields[1:4] == (2, 62, 1) and fields[8:11] == (64, 56, 2),
                path.name + " has an unexpected ELF header")

    def check_cli_source(self) -> None:
        path = self.source / "rey-compiler/native-main.rey"
        text = path.read_text()
        for old in ("--selfhost-fast-exit", "--trust-packed-output", "--gc-stress", "--no-regions",
                    "--experimental-escape-summary", "--experimental-packed-x64", "selfhostFastExit",
                    "trustPackedOutput", "buildManagedProgram"):
            require(old not in text, "released CLI still contains legacy path " + old)
        require("setCheckedPackedOutput(checkedPackedOutput)" in text,
                "Linux x64 checked writer binding is not enabled by the current CLI source")
        require("buildCompletedRegionProgram(checked.unwrap(), runtimeRoot, stdRoot, promotionStress)" in text,
                "managed CLI no longer selects verified region lowering")
        lowerer = (self.source / "rey-compiler/src/ir/lower/main.rey").read_text()
        require("var checkedPackedOutput: bool = false;" in lowerer and
                "setCheckedPackedOutput(enabled: bool)" in lowerer and
                "var trustedPackedOutput" not in lowerer,
                "checked writer binding is still coupled to the retired trust flag")
        require(lowerer.count("lowerer.managed && checkedPackedOutput && target.sourceName") == 2,
                "checked writer binding does not cover both byte and packed-segment outputs")
        require(re.search(r"unknown or duplicate option:[^}]*?exit\(2\)", text, re.DOTALL) is not None,
                "released CLI no longer rejects removed options with status 2")
        self.results["checks"]["cli_source"] = {
            "sha256": sha256(path), "lowerer_sha256": sha256(self.source / "rey-compiler/src/ir/lower/main.rey"),
            "removed_options_exit": 2,
            "removed_options": ["--selfhost-fast-exit", "--trust-packed-output", "--gc-stress",
                                "--no-regions", "--experimental-escape-summary", "--experimental-packed-x64"],
            "managed_regions": True,
        }
        self.save()

    def check_runtime_writer_source(self) -> None:
        containers = (self.source / "runtime-rey/containers/main.rey").read_text()
        host_path = self.source / "runtime-rey/host/main.rey"
        host = host_path.read_text()
        require(containers.count("__reyRawMap(4096)") == 2,
                "Linux x64 image writers do not share the fixed 4096-byte staging bound")
        require(containers.count("hostFchmod(fd, 493)") == 2,
                "byte and packed writers do not set executable mode")
        require(containers.count("if extra != 0 { readOk = false; }") == 2,
                "byte and packed writers do not read back through EOF")
        require(containers.count("return readOk && readClose == 0;") == 2,
                "byte and packed writers do not check readback close")
        require("pub func hostFchmod(fd: int, mode: int): int {" in host and
                "__reyRawHostSyscall(77, hostRawInt(fd), hostRawInt(mode)" in host,
                "Linux x64 host fchmod syscall wrapper is missing")
        require("hostOpen(containersPathData(path), 577, 420)" in containers and
                "if hostClose(fd) != 0 { containersFail(2208); }" in containers,
                "plain writeFile permissions or close handling changed")
        self.results["checks"]["runtime_writer_source"] = {
            "containers_sha256": sha256(self.source / "runtime-rey/containers/main.rey"),
            "host_sha256": sha256(host_path),
            "staging_bytes": 4096, "readback_to_eof": True,
            "checked_close": True, "linux_x64_mode": "0755",
            "plain_write_file_mode": "0644 on fresh paths; preserve existing mode",
        }
        self.save()

    def check_image_output(self, driver: Path, producer_name: str, kind: str, destination: Path) -> bytes:
        stdout, _ = self.run_program(producer_name + "-" + kind + "-" + destination.name,
                                     [str(driver), kind, str(destination)], expected_stdout=b"writer success\n")
        require(stdout == b"writer success\n", "writer fixture did not emit exactly one success report")
        require(destination.is_file() and file_mode(destination) == 0o755,
                kind + " output is missing executable mode")
        data = destination.read_bytes()
        require(data == expected_elf(), kind + " image bytes differ from fixed ELF oracle")
        symbol_path = Path(str(destination) + ".map")
        require(symbol_path.read_bytes() == SYMBOLS, kind + " symbol offsets differ from fixed oracle")
        require(file_mode(symbol_path) == 0o644, "symbol map should retain plain data permissions")
        self.run_program(producer_name + "-" + kind + "-direct", [str(destination)], expected_stdout=b"")
        self.results["images"][destination.name] = {
            "sha256": sha256(destination), "mode": oct(file_mode(destination)),
            "size": len(data), "symbol_sha256": sha256(symbol_path),
        }
        self.save()
        return data

    def run(self) -> None:
        self.prepare()
        module_root = self.stage_modules()
        for name in ("native-main.rey", "ir/lower/main.rey", "native/bytes/main.rey",
                     "native/packed/main.rey", "native/output/main.rey", "native/image/x64/main.rey"):
            path = module_root / name if name != "native-main.rey" else module_root / "main.rey"
            require(path.is_file(), "staged writer module is missing: " + name)
        self.results["source_hashes"] = {
            "native_main": sha256(self.compiler_source),
            "lowerer": sha256(module_root / "ir/lower/main.rey"),
            "bytes": sha256(module_root / "native/bytes/main.rey"),
            "packed": sha256(module_root / "native/packed/main.rey"),
            "output": sha256(module_root / "native/output/main.rey"),
            "image_x64": sha256(module_root / "native/image/x64/main.rey"),
            "containers": sha256(self.source / "runtime-rey/containers/main.rey"),
            "host": sha256(self.source / "runtime-rey/host/main.rey"),
            "embedded_runtime": sha256(module_root / "frontend/embedded_runtime/main.rey"),
        }
        self.check_cli_source()
        self.check_runtime_writer_source()
        self.save()

        driver_source = module_root / "normal-output-driver.rey"
        shutil.copyfile(HERE / "driver.rey", driver_source)
        writer_driver = self.out / "producers/seed-built-staged-writer"
        writer_driver.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(self.seed, writer_driver)
        seed_writer_hash = sha256(writer_driver)
        self.build_staged_driver(self.seed, driver_source, writer_driver, None,
                                 "build-seed-staged-writer-driver", trusted=True)
        require(file_mode(writer_driver) == 0o755 and sha256(writer_driver) != seed_writer_hash,
                "seed-to-candidate writer transition did not replace the executable destination")
        self.results["checks"]["seed_to_staged_writer_binding"] = {
            "seed_sha256": seed_writer_hash, "driver_sha256": sha256(writer_driver),
            "driver_binding": "pinned S3 seed compiles current output modules with its legacy bootstrap hook",
            "current_source_binding": "checkedPackedOutput-gated hooks verified by source invariants",
            "runtime_sha256": "embedded in pinned S3 seed for the executable writer fixtures",
            "runtime_overlay_sha256": sha256(module_root / "frontend/embedded_runtime/main.rey"),
            "rebuilt_candidate_recheck": "pending §4 memory gate",
        }

        images: dict[tuple[str, str], tuple[bytes, bytes]] = {}
        for kind in ("plain", "managed", "packed"):
            fresh = self.out / "images" / (kind + "-fresh")
            fresh.parent.mkdir(parents=True, exist_ok=True)
            data = self.check_image_output(writer_driver, "staged-driver", kind, fresh)
            symbols = Path(str(fresh) + ".map").read_bytes()
            require(data == expected_elf(), kind + " image bytes differ from fixed ELF oracle")
            images[(kind, "fresh")] = (data, symbols)

            replaced = self.out / "images" / (kind + "-replaced-0644")
            replaced.write_bytes(b"old non-executable contents")
            os.chmod(replaced, 0o644)
            replaced_data = self.check_image_output(writer_driver, "staged-driver", kind, replaced)
            replaced_symbols = Path(str(replaced) + ".map").read_bytes()
            require(replaced_data == data and replaced_symbols == symbols,
                    kind + " bytes or symbol offsets changed at a 0644 destination")

        require(images[("plain", "fresh")] == images[("managed", "fresh")] == images[("packed", "fresh")],
                "plain, managed and packed images or symbol offsets differ")

        data_fresh = self.out / "data/write-file-fresh"
        data_fresh.parent.mkdir(parents=True, exist_ok=True)
        self.run_program("plain-write-file-fresh", [str(writer_driver), "data", str(data_fresh)],
                         expected_stdout=b"writer success\n")
        require(data_fresh.read_bytes() == b"plain writeFile data\n" and file_mode(data_fresh) == 0o644,
                "plain writeFile fresh bytes or permissions differ")
        data_existing = self.out / "data/write-file-existing"
        data_existing.write_bytes(b"old data")
        os.chmod(data_existing, 0o600)
        self.run_program("plain-write-file-existing", [str(writer_driver), "data", str(data_existing)],
                         expected_stdout=b"writer success\n")
        require(data_existing.read_bytes() == b"plain writeFile data\n" and file_mode(data_existing) == 0o600,
                "plain writeFile changed existing data permissions")

        fault_root = self.stage_modules(faulted=True)
        fault_source = fault_root / "fault-writer.rey"
        fault_source.write_text('''import native::output::{writeVerifiedCompilerOutputBytes};
func main(): void {
    var argv = args();
    if argv.len() == 3 && argv[1] == "data" {
        writeFile(argv[2], "plain writeFile data\\n");
        println("writer success");
        return;
    }
    if argv.len() != 2 { exit(2); return; }
    var result = writeVerifiedCompilerOutputBytes(argv[1], [0, 127, 128, 255, 0, 42]);
    if result.isErr() { eprintln("writer failed"); exit(1); return; }
    println("writer success");
}
''')
        fault_driver = self.out / "producers/fault-injection-driver"
        shutil.copy2(writer_driver, fault_driver)
        trusted_fault_hash = sha256(fault_driver)
        self.build_staged_driver(self.seed, fault_source, fault_driver, fault_root / "runtime-rey",
                                 "build-current-runtime-fault-driver", trusted=False)
        require(file_mode(fault_driver) == 0o755 and sha256(fault_driver) != trusted_fault_hash,
                "current-runtime fault driver transition did not preserve an executable destination")
        self.results["checks"]["fault_driver_transition"] = {
            "seed_embedded_sha256": trusted_fault_hash,
            "current_runtime_sha256": sha256(fault_driver),
            "runtime_source_hashes": {
                "containers": sha256(fault_root / "runtime-rey/containers/main.rey"),
                "host": sha256(fault_root / "runtime-rey/host/main.rey"),
            },
            "destination_mode_preserved": oct(file_mode(fault_driver)),
        }
        self.save()
        for fault in ("write", "close", "read"):
            destination = self.out / "faults" / ("image.o01-" + fault)
            destination.parent.mkdir(parents=True, exist_ok=True)
            stdout, _stderr = self.run_program("injected-" + fault,
                                               [str(fault_driver), str(destination) + ".o01-" + fault],
                                               expected_status=None)
            require(stdout == b"" and
                    str(destination).encode() not in stdout,
                    fault + " failure returned success or printed a success report")

        current_runtime_success = self.out / "faults/current-runtime-boundary-bytes"
        self.run_program("current-runtime-verified-write",
                         [str(fault_driver), str(current_runtime_success)],
                         expected_stdout=b"writer success\n")
        require(current_runtime_success.read_bytes() == bytes((0, 127, 128, 255, 0, 42)) and
                file_mode(current_runtime_success) == 0o644,
                "current runtime verified byte write or plain data permissions differ")
        current_data_fresh = self.out / "data/current-write-file-fresh"
        self.run_program("current-runtime-write-file-fresh",
                         [str(fault_driver), "data", str(current_data_fresh)],
                         expected_stdout=b"writer success\n")
        require(current_data_fresh.read_bytes() == b"plain writeFile data\n" and
                file_mode(current_data_fresh) == 0o644,
                "current runtime plain writeFile fresh bytes or mode differ")
        current_data_existing = self.out / "data/current-write-file-existing"
        current_data_existing.write_bytes(b"old data")
        os.chmod(current_data_existing, 0o600)
        self.run_program("current-runtime-write-file-existing",
                         [str(fault_driver), "data", str(current_data_existing)],
                         expected_stdout=b"writer success\n")
        require(current_data_existing.read_bytes() == b"plain writeFile data\n" and
                file_mode(current_data_existing) == 0o600,
                "current runtime plain writeFile changed an existing mode")
        raw_output = self.out / "raw-authority-output"
        stdout, _stderr = self.invoke_build("reject-ordinary-raw-authority",
                                            [str(self.seed), "build", str(HERE / "raw-authority.rey"), "-o",
                                             str(raw_output), "--target", "linux-x64", "--managed", "--regions",
                                             "--runtime-root", str(self.source / "runtime-rey")],
                                            timeout=600, expected_status=1)
        require(not raw_output.exists() and str(raw_output).encode() not in stdout,
                "ordinary source acquired raw writer authority or printed success")
        self.results["checks"].update({
            "plain_managed_packed_bytes": {kind: sha256(self.out / "images" / (kind + "-fresh"))
                                           for kind in ("plain", "managed", "packed")},
            "symbol_offsets": SYMBOLS.decode().splitlines(),
            "write_close_readback_failures": ["write", "close", "read"],
            "ordinary_raw_writer_authority": "rejected",
            "fresh_and_0644_execution": True,
            "candidate_recheck": "pending §4 memory gate",
        })
        self.results["status"] = "PASS normal output"
        self.save()


def main() -> int:
    if len(sys.argv) != 6 or sys.argv[3] != "--staged" or sys.argv[4] != "--source":
        raise SystemExit("usage: run.py SEED OUT --staged --source SOURCE")
    Runner(Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[5])).run()
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Failure as exc:
        print("FAIL normal output: " + str(exc), file=sys.stderr)
        raise SystemExit(1)
