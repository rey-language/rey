#!/usr/bin/env python3
"""Staged tests for protected inputs and transactional compiler output."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import struct
import subprocess
import sys
import time


SEED_SHA256 = "64c8bbe4e559a2355b79bfa0d357bd6e81d6dee92bbac73fe520165004883ee6"
CODE = bytes((184, 60, 0, 0, 0, 49, 255, 15, 5, 195, 0, 127, 128, 255))
MAP = b"401000 <entry>\n"


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
    def __init__(self, seed: Path, requested_out: Path, source: Path):
        self.seed = seed.resolve(strict=True)
        self.source = source.resolve(strict=True)
        self.requested_out = requested_out.resolve()
        self.out = self.requested_out
        self.results: dict[str, object] = {
            "gate": "O02 output publication",
            "seed": str(self.seed),
            "seed_sha256": sha256(self.seed),
            "source": str(self.source),
            "source_revision": self.git_revision(),
            "platform": platform.platform(),
            "machine": platform.machine(),
            "compiler_builds": [],
            "checks": {},
        }
        self.logs: Path
        self.stage: Path
        self.compiler_source: Path
        self.runtime: Path
        self.std: Path
        self.inputs: Path
        self.driver_source: Path
        self.candidate: Path
        self.writer: Path
        self.output_owned = False

    def git_revision(self) -> str:
        result = subprocess.run(["git", "-C", str(self.source), "rev-parse", "HEAD"],
                                capture_output=True, timeout=10, check=False)
        return result.stdout.decode().strip() if result.returncode == 0 else "unavailable"

    def save(self) -> None:
        (self.out / "results.json").write_text(json.dumps(self.results, indent=2, sort_keys=True) + "\n")

    def prepare(self) -> None:
        require(platform.system() == "Linux" and platform.machine().lower() in ("x86_64", "amd64"),
                "output publication acceptance requires Linux x64")
        require(self.seed.is_file() and os.access(self.seed, os.X_OK), "SEED must be executable")
        require(self.results["seed_sha256"] == SEED_SHA256, "SEED hash does not match pinned S3")
        require(self.source.is_dir() and (self.source / "rey-compiler/native-main.rey").is_file(),
                "SOURCE does not contain the native compiler")
        require(self.source != self.requested_out and self.source not in self.requested_out.parents,
                "OUT must be outside SOURCE")
        if self.out.exists():
            evidence = self.out / "results.json"
            require(evidence.is_file(), "OUT exists without O02 evidence; refusing to reuse it")
            previous = json.loads(evidence.read_text())
            require(previous.get("gate") == "O02 output publication" and
                    previous.get("seed_sha256") == SEED_SHA256 and
                    previous.get("source") == str(self.source),
                    "OUT contains unrelated evidence; refusing to reuse it")
            self.out = self.requested_out / ("attempt-" + time.strftime("%Y%m%dT%H%M%SZ", time.gmtime()))
            suffix = 2
            base = self.out
            while self.out.exists():
                self.out = Path(str(base) + "-%02d" % suffix)
                suffix += 1
            self.results["previous_attempt"] = str(self.requested_out)
        self.out.parent.mkdir(parents=True, exist_ok=True)
        self.out.mkdir()
        self.output_owned = True
        self.logs = self.out / "logs"
        self.stage = self.out / "stage/compiler"
        self.compiler_source = self.stage / "main.rey"
        self.runtime = self.out / "stage/runtime-rey"
        self.std = self.out / "inputs/std"
        self.inputs = self.out / "inputs"
        self.driver_source = self.stage / "o02-publication-driver.rey"
        self.candidate = self.out / "bin/rey-native"
        self.writer = self.out / "bin/publication-driver"
        for directory in (self.logs, self.stage, self.runtime.parent, self.std, self.candidate.parent):
            directory.mkdir(parents=True, exist_ok=True)
        shutil.copytree(self.source / "rey-compiler/src", self.stage, dirs_exist_ok=True)
        shutil.copyfile(self.source / "rey-compiler/native-main.rey", self.compiler_source)
        shutil.copytree(self.source / "runtime-rey", self.runtime)
        self.make_runtime_faults()
        self.make_inputs()
        overlay = self.stage / "frontend/embedded_runtime/main.rey"
        generated = subprocess.run([sys.executable, "-B", str(self.source / "scripts/embed-runtime.py"),
                                    "--root", str(self.source), "--output", str(overlay)],
                                   capture_output=True, text=True, timeout=120, check=False)
        require(generated.returncode == 0, "runtime overlay generation failed: " + generated.stderr)
        self.results.update({
            "resource_policy": "unlimited memory; nice -n 19; timeout; /usr/bin/time -v",
            "runtime_root": str(self.runtime),
            "stdlib_override": str(self.std),
            "source_hashes": {
                "native_main": sha256(self.compiler_source),
                "lowerer": sha256(self.stage / "ir/lower/main.rey"),
                "graph": sha256(self.stage / "frontend/graph/main.rey"),
                "linker": sha256(self.stage / "ir/managed/link/main.rey"),
                "output": sha256(self.stage / "native/output/main.rey"),
                "containers": sha256(self.runtime / "containers/main.rey"),
                "regions": sha256(self.runtime / "regions/main.rey"),
            },
        })
        self.save()

    def make_runtime_faults(self) -> None:
        source = self.runtime / "containers/main.rey"
        text = source.read_text()
        require("func containersNextOutputTempSequence(context: __rey_raw_address): int {" in text,
                "O02 context-backed output state marker is missing")
        region_source = self.runtime / "regions/main.rey"
        region_text = region_source.read_text()
        context_size_anchor = "var context = __reyRawMap(336);"
        context_sequence_anchor = "__reyRawStoreI64(regionAddressAt(context, 328), 1);"
        require(context_size_anchor in region_text and context_sequence_anchor in region_text,
                "O02 region context extension marker is missing")
        region_text = region_text.replace(context_size_anchor, "var context = __reyRawMap(360);", 1)
        region_text = region_text.replace("__reyRawUnmap(context, 336);", "__reyRawUnmap(context, 360);")
        region_text = region_text.replace(context_sequence_anchor,
            context_sequence_anchor + "\n    __reyRawStoreI64(regionAddressAt(context, 336), -1);" +
            "\n    __reyRawStoreI64(regionAddressAt(context, 344), 0);" +
            "\n    __reyRawStoreI64(regionAddressAt(context, 352), 0);", 1)
        region_source.write_text(region_text)
        protected_input_path = str(self.inputs / "fixture.rey")
        protected_input_bytes = protected_input_path.encode()
        protected_input_storage = len(protected_input_bytes) + 1

        def cstring_match(value_name: str, expected: str) -> str:
            encoded = expected.encode()
            checks = [
                "__reyRawLoadByte(containersAddressAt(%s, %d)) == %d" % (value_name, index, byte)
                for index, byte in enumerate(encoded)
            ]
            checks.append("__reyRawLoadByte(containersAddressAt(%s, %d)) == 0" % (value_name, len(encoded)))
            return " &&\n        ".join(checks)

        protected_input_stores = "\n".join(
            "    __reyRawStoreByte(containersAddressAt(value, %d), %d);" % (index, byte)
            for index, byte in enumerate(protected_input_bytes)
        )
        helper = """
func containersO02ProtectedInputPath(): __rey_raw_address {
    var value = __reyRawMap(%d);
%s
    __reyRawStoreByte(containersAddressAt(value, %d), 0);
    return value;
}
""" % (protected_input_storage, protected_input_stores, len(protected_input_bytes))
        sequence_anchor = "func containersNextOutputTempSequence(context: __rey_raw_address): int {"
        text = text.replace(sequence_anchor, helper + "\n" + sequence_anchor, 1)
        write_anchor = "func containersWriteStaging(context: __rey_raw_address, fd: int, staging: __rey_raw_address, count: int): bool {\n    var offset = 0;"
        require(write_anchor in text, "O02 bounded writer hook is missing")
        text = text.replace(write_anchor,
                            "func containersWriteStaging(context: __rey_raw_address, fd: int, staging: __rey_raw_address, count: int): bool {\n"
                            "    if __reyRawToInt(context) > 0 {\n"
                            "        var countdownAddress = containersAddressAt(context, 336);\n"
                            "        var countdown = __reyRawLoadI64(countdownAddress);\n"
                            "        if countdown == 0 { __reyRawStoreI64(countdownAddress, -1); return false; }\n"
                            "        if countdown > 0 { __reyRawStoreI64(countdownAddress, countdown - 1); }\n"
                            "    }\n"
                            "    var offset = 0;", 1)
        image_path = str(self.out / "faults/write-failure/image")
        map_failure_image = str(self.out / "faults/map-write-failure/image")
        path_anchor = "__reyRawStoreByte(containersAddressAt(mapPath, mapPathLength), 0);"
        require(path_anchor in text, "O02 pair path parser is missing")
        text = text.replace(path_anchor,
                            path_anchor + "\n    if " + cstring_match("imagePath", image_path) + " { __reyRawStoreI64(containersAddressAt(context, 336), 0); }" +
                            "\n    if " + cstring_match("imagePath", map_failure_image) + " { __reyRawStoreI64(containersAddressAt(context, 336), 2); }", 1)
        rename_start = "func containersHostRenameat2(context: __rey_raw_address, oldDirectory: int, oldPath: __rey_raw_address, newDirectory: int, newPath: __rey_raw_address, flags: int): int {\n    return hostRenameat2(oldDirectory, oldPath, newDirectory, newPath, flags);\n}"
        require(rename_start in text, "O02 rename hook is missing")
        rename_injected = f'''func containersHostRenameat2(context: __rey_raw_address, oldDirectory: int, oldPath: __rey_raw_address, newDirectory: int, newPath: __rey_raw_address, flags: int): int {{
    if flags == 2 && {cstring_match("newPath", "fail-map")} {{ return -5; }}
    if flags == 2 && {cstring_match("newPath", "rename-image")} {{ return -5; }}
    if flags == 2 && __reyRawLoadI64(containersAddressAt(context, 344)) == 0 && {cstring_match("newPath", "race-image")} {{
        __reyRawStoreI64(containersAddressAt(context, 344), 1);
        containersHostUnlinkat(newDirectory, newPath, 0);
        var protectedInput = containersO02ProtectedInputPath();
        var linked = __reyRawToInt(__reyRawHostSyscall(265, __reyRawFromInt(-100), protectedInput, __reyRawFromInt(newDirectory), newPath, __reyRawFromInt(0), __reyRawNull()));
        __reyRawUnmap(protectedInput, {protected_input_storage});
        if linked != 0 {{ return -5; }}
    }}
    if flags == 2 && __reyRawLoadI64(containersAddressAt(context, 352)) == 0 && {cstring_match("newPath", "race-map")} {{
        __reyRawStoreI64(containersAddressAt(context, 352), 1);
        containersHostUnlinkat(newDirectory, newPath, 0);
        var protectedInput = containersO02ProtectedInputPath();
        var linked = __reyRawToInt(__reyRawHostSyscall(265, __reyRawFromInt(-100), protectedInput, __reyRawFromInt(newDirectory), newPath, __reyRawFromInt(0), __reyRawNull()));
        __reyRawUnmap(protectedInput, {protected_input_storage});
        if linked != 0 {{ return -5; }}
    }}
    return hostRenameat2(oldDirectory, oldPath, newDirectory, newPath, flags);
}}'''
        text = text.replace(rename_start, rename_injected, 1)
        source.write_text(text)

    def make_inputs(self) -> None:
        sentinel = self.std / "sentinel/main.rey"
        sentinel.parent.mkdir(parents=True, exist_ok=True)
        sentinel.write_text("pub func marker(): int { return 42; }\n")
        fixture = self.inputs / "fixture.rey"
        fixture.write_text("import std::sentinel::{marker};\nfunc main(): void { if marker() != 42 { exit(1); } }\n")
        self.image_data = self.inputs / "small-image.elf"
        self.image_data.write_bytes(expected_elf())
        self.runtime_input = self.runtime / "host/main.rey"
        self.std_input = sentinel

    def build(self, label: str, argv: list[str], seconds: int = 7200) -> Path:
        index = len(self.results["compiler_builds"]) + 1
        slug = "%02d-%s" % (index, re.sub(r"[^A-Za-z0-9._-]", "_", label))
        stdout_path = self.logs / (slug + ".stdout")
        stderr_path = self.logs / (slug + ".stderr")
        time_path = self.logs / (slug + ".time-v")
        wrapped = ["nice", "-n", "19", "timeout", "--kill-after=30s", str(seconds),
                   "/usr/bin/time", "-v", "-o", str(time_path), *map(str, argv)]
        with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
            proc = subprocess.run(wrapped, stdout=stdout, stderr=stderr, check=False)
            status = proc.returncode
        time_text = time_path.read_text(errors="replace") if time_path.exists() else ""
        peak = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", time_text)
        record = {
            "label": label,
            "argv": list(map(str, argv)),
            "status": status,
            "stdout": str(stdout_path),
            "stderr": str(stderr_path),
            "time_v": str(time_path),
            "peak_rss_kib": int(peak.group(1)) if peak else None,
        }
        self.results["compiler_builds"].append(record)
        self.save()
        require(peak is not None, label + " did not produce a peak RSS measurement")
        require(status == 0, label + " failed (status %s): %s" % (status, stderr_path.read_text(errors="replace")))
        output = Path(argv[argv.index("-o") + 1])
        require(output.is_file() and file_mode(output) == 0o755, label + " output is missing or not 0755")
        return output

    def run_program(self, argv: list[str], cwd: Path | None = None, timeout: int = 60) -> subprocess.CompletedProcess[bytes]:
        return subprocess.run(["nice", "-n", "19", "timeout", "--kill-after=30s", str(timeout), *map(str, argv)],
                              cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                              timeout=timeout + 60, check=False)

    def build_candidate(self) -> None:
        self.build("seed-build-native-compiler", [str(self.seed), "build", str(self.compiler_source), "-o", str(self.candidate),
                   "--target", "linux-x64", "--managed", "--regions", "--checked-packed-output",
                   "--runtime-root", str(self.runtime)])
        self.candidate = self.out / "bin/rey-native"
        self.results["candidate_sha256"] = sha256(self.candidate)
        self.save()

    def build_success_fixture(self) -> None:
        output = self.out / "artifacts/cli-image"
        symbols = self.out / "artifacts/cli-map"
        output.parent.mkdir(parents=True, exist_ok=True)
        self.build("candidate-build-with-runtime-and-stdlib-overrides", [str(self.candidate), "build", str(self.inputs / "fixture.rey"),
                   "-o", str(output), "--target", "linux-x64", "--managed", "--regions",
                   "--runtime-root", str(self.runtime), "--std-root", str(self.std), "--symbol-map", str(symbols)])
        require(output.is_file() and file_mode(output) == 0o755, "fresh CLI output is not executable")
        require(symbols.is_file() and file_mode(symbols) == 0o644, "symbol map mode or existence is wrong")
        require(b"<entry>" in symbols.read_bytes(), "symbol map did not contain the entry symbol")
        result = self.run_program([str(output)])
        require(result.returncode == 0, "CLI-produced executable did not execute directly: " + result.stderr.decode(errors="replace"))
        self.results["checks"]["explicit_runtime_and_stdlib_overrides"] = {
            "runtime_host_sha256": sha256(self.runtime_input),
            "stdlib_sentinel_sha256": sha256(self.std_input),
            "image_sha256": sha256(output),
            "map_sha256": sha256(symbols),
            "direct_execution": "status 0",
        }
        self.save()

    def build_writer_driver(self) -> None:
        self.driver_source.write_text('''import native::output::{writeVerifiedCompilerOutputs};
func main(): void {
    var argv = args();
    if argv.len() < 7 { exit(2); return; }
    var imageText = readFile(argv[3]);
    var image: Vec<int> = [];
    var index = 0;
    while index < imageText.len() { image.push(int(imageText[index])); index += 1; }
    index = 4;
    while index < argv.len() { var input = readFile(argv[index]); if input.len() == -1 { exit(3); return; } index += 1; }
    var published = writeVerifiedCompilerOutputs(argv[1], image, argv[2], "401000 <entry>\\n");
    if published.isErr() { eprintln(published.unwrapErr()); exit(1); return; }
    println("publication success");
}
''')
        self.build("candidate-build-publication-driver", [str(self.candidate), "build", str(self.driver_source), "-o", str(self.writer),
                   "--target", "linux-x64", "--managed", "--regions", "--runtime-root", str(self.runtime)])

    def writer_inputs(self) -> list[str]:
        return [str(self.image_data), str(self.inputs / "fixture.rey"), str(self.runtime_input), str(self.std_input)]

    def write_pair(self, image: Path, symbol_map: Path, *, cwd: Path | None = None) -> subprocess.CompletedProcess[bytes]:
        image.parent.mkdir(parents=True, exist_ok=True)
        symbol_map.parent.mkdir(parents=True, exist_ok=True)
        command = [str(self.writer), str(image), str(symbol_map), *self.writer_inputs()]
        return self.run_program(command, cwd=cwd)

    def check_executable(self, image: Path, symbol_map: Path) -> None:
        require(image.is_file() and file_mode(image) == 0o755, "published image is absent or not executable: " + str(image))
        require(image.read_bytes() == expected_elf(), "published image bytes differ from ELF oracle")
        require(symbol_map.is_file() and file_mode(symbol_map) == 0o644, "published symbol map mode is not 0644")
        require(symbol_map.read_bytes() == MAP, "published symbol map bytes differ from oracle")
        result = self.run_program([str(image)])
        require(result.returncode == 0, "published image failed direct execution: " + result.stderr.decode(errors="replace"))

    def check_fresh_and_replaced(self) -> None:
        for name, replaced in (("fresh", False), ("replaced-0644", True)):
            directory = self.out / "artifacts" / ("writer-" + name)
            directory.mkdir(parents=True)
            image, symbol_map = directory / "image", directory / "image.map"
            if replaced:
                image.write_bytes(b"old non-executable image")
                os.chmod(image, 0o644)
                symbol_map.write_bytes(b"old map")
            result = self.write_pair(image, symbol_map)
            require(result.returncode == 0 and result.stdout == b"publication success\n",
                    name + " pair publication failed: " + result.stderr.decode(errors="replace"))
            self.check_executable(image, symbol_map)
        directory = self.out / "artifacts/writer-reserved-temp-names"
        directory.mkdir(parents=True)
        image, symbol_map = directory / ".rey-o02-1", directory / ".rey-o02-2"
        image.write_bytes(b"old non-executable image")
        os.chmod(image, 0o644)
        symbol_map.write_bytes(b"old map")
        result = self.write_pair(image, symbol_map)
        require(result.returncode == 0, "reserved temporary names collided with output destinations")
        self.check_executable(image, symbol_map)
        self.results["checks"]["fresh_and_previously_0644_direct_execution"] = True
        self.save()

    def protected_alias_case(self, name: str, destination: str, input_path: Path, link_kind: str,
                             *, relative: bool = False) -> None:
        directory = self.out / "aliases" / name
        directory.mkdir(parents=True)
        image, symbol_map = directory / "image", directory / "map"
        image.write_bytes(b"old image")
        symbol_map.write_bytes(b"old map")
        if destination == "image":
            target = directory / "alias"
        else:
            target = directory / "alias"
        if link_kind == "hardlink":
            os.link(input_path, target)
        else:
            os.symlink(input_path, target)
        before_source = sha256(input_path)
        before_image = image.read_bytes()
        before_map = symbol_map.read_bytes()
        image_arg = str(image)
        map_arg = str(symbol_map)
        if destination == "image":
            image_arg = "./alias" if relative else str(target)
        else:
            map_arg = "./alias" if relative else str(target)
        result = self.run_program([str(self.writer), image_arg, map_arg, *self.writer_inputs()], cwd=directory)
        require(result.returncode != 0 and result.stdout == b"", name + " unexpectedly published an input alias")
        require(sha256(input_path) == before_source, name + " changed a protected input")
        require(image.read_bytes() == before_image and symbol_map.read_bytes() == before_map,
                name + " changed the other prior artifact")

    def check_aliases(self) -> None:
        cases = [
            ("image-hardlink-relative-source", "image", self.inputs / "fixture.rey", "hardlink", True),
            ("map-hardlink-runtime-override", "map", self.runtime_input, "hardlink", False),
            ("image-symlink-runtime-override", "image", self.runtime_input, "symlink", False),
            ("map-symlink-stdlib-override", "map", self.std_input, "symlink", False),
            ("image-hardlink-stdlib-override", "image", self.std_input, "hardlink", False),
            ("map-hardlink-source", "map", self.inputs / "fixture.rey", "hardlink", True),
        ]
        for name, destination, input_path, link_kind, relative in cases:
            self.protected_alias_case(name, destination, input_path, link_kind, relative=relative)

        directory = self.out / "aliases/image-map-hardlink"
        directory.mkdir(parents=True)
        shared = directory / "shared"
        shared.write_bytes(b"shared prior artifact")
        image, symbol_map = directory / "image", directory / "map"
        os.link(shared, image)
        os.link(shared, symbol_map)
        before = shared.read_bytes()
        result = self.write_pair(image, symbol_map)
        require(result.returncode != 0 and shared.read_bytes() == before,
                "image and map aliases were not rejected without changing their shared inode")

        directory = self.out / "aliases/image-map-symlink"
        directory.mkdir(parents=True)
        shared = directory / "shared"
        shared.write_bytes(b"shared prior symlink target")
        image, symbol_map = directory / "image", directory / "map"
        os.symlink(shared, image)
        os.symlink(shared, symbol_map)
        before = shared.read_bytes()
        result = self.write_pair(image, symbol_map)
        require(result.returncode != 0 and shared.read_bytes() == before and
                os.path.samefile(image, symbol_map),
                "image and map symlink aliases were not rejected without changing their target")

        directory = self.out / "aliases/image-map-relative-absolute"
        directory.mkdir(parents=True)
        shared = directory / "same"
        shared.write_bytes(b"shared prior named target")
        result = self.run_program([str(self.writer), "./same", str(shared), *self.writer_inputs()], cwd=directory)
        require(result.returncode != 0 and shared.read_bytes() == b"shared prior named target",
                "relative and absolute names for one destination were not rejected")

        self.results["checks"]["input_aliases"] = [case[0] for case in cases] + [
            "image-map-hardlink", "image-map-symlink", "relative-absolute-same-destination"]
        self.save()

    def prepare_old_pair(self, name: str) -> tuple[Path, Path, bytes, bytes]:
        directory = self.out / "faults" / name
        directory.mkdir(parents=True)
        image, symbol_map = directory / "image", directory / "map"
        image_old, map_old = b"previous executable", b"previous symbol map"
        image.write_bytes(image_old)
        symbol_map.write_bytes(map_old)
        os.chmod(image, 0o755)
        return image, symbol_map, image_old, map_old

    def check_prepublication_failures(self) -> None:
        image, symbol_map, image_old, map_old = self.prepare_old_pair("write-failure")
        result = self.write_pair(image, symbol_map)
        require(result.returncode != 0 and result.stdout == b"", "injected write failure reported success")
        require(image.read_bytes() == image_old and symbol_map.read_bytes() == map_old,
                "write failure changed a prior artifact")

        image, symbol_map, image_old, map_old = self.prepare_old_pair("map-write-failure")
        result = self.write_pair(image, symbol_map)
        require(result.returncode != 0 and result.stdout == b"" and
                image.read_bytes() == image_old and symbol_map.read_bytes() == map_old,
                "map temporary write failure changed a prior artifact")

        image, symbol_map, image_old, map_old = self.prepare_old_pair("map-rename-failure")
        fail_map = image.parent / "fail-map"
        fail_map.write_bytes(map_old)
        image.write_bytes(image_old)
        result = self.write_pair(image, fail_map)
        require(result.returncode != 0 and image.read_bytes() == image_old and fail_map.read_bytes() == map_old,
                "map rename failure did not preserve both prior artifacts")
        self.results["checks"]["prepublication_failures_preserve_both"] = [
            "image write", "map write", "map rename"]
        self.save()

    def check_final_rename_failure(self) -> None:
        directory = self.out / "faults/final-image-rename"
        directory.mkdir(parents=True)
        image, symbol_map = directory / "rename-image", directory / "rename-map"
        image_old, map_old = b"previous executable", b"previous symbol map"
        image.write_bytes(image_old)
        symbol_map.write_bytes(map_old)
        os.chmod(image, 0o755)
        result = self.write_pair(image, symbol_map)
        require(result.returncode != 0, "injected final image rename failure returned success")
        require(image.read_bytes() == image_old, "final image rename failure changed the prior executable")
        require(symbol_map.read_bytes() == MAP, "map was not published before the final image rename")
        require(b"symbol map may be from the failed build" in result.stderr,
                "final image rename failure did not report the map publication state")
        self.results["checks"]["final_image_rename_failure"] = {
            "prior_executable_preserved": True,
            "map_is_new_build": True,
            "diagnostic": "symbol map may be from the failed build",
        }
        self.save()

    def check_path_swaps(self) -> None:
        protected = self.inputs / "fixture.rey"
        protected_hash = sha256(protected)
        directory = self.out / "swaps/map-destination"
        directory.mkdir(parents=True)
        image, symbol_map = directory / "image", directory / "race-map"
        image.write_bytes(b"previous executable")
        symbol_map.write_bytes(b"previous map")
        image_before = image.read_bytes()
        map_before = symbol_map.read_bytes()
        result = self.write_pair(image, symbol_map)
        require(result.returncode != 0, "map path swap into a protected source was accepted")
        require(sha256(protected) == protected_hash and image.read_bytes() == image_before,
                "map destination swap modified a protected input or prior executable")
        require(os.path.samefile(symbol_map, protected) and map_before != protected.read_bytes(),
                "map path swap was not rolled back to the protected input alias")

        directory = self.out / "swaps/image-destination"
        directory.mkdir(parents=True)
        image, symbol_map = directory / "race-image", directory / "map"
        image.write_bytes(b"previous executable")
        symbol_map.write_bytes(b"previous map")
        backup = directory / "prior-executable"
        shutil.copyfile(image, backup)
        result = self.write_pair(image, symbol_map)
        require(result.returncode != 0 and sha256(protected) == protected_hash,
                "image path swap into a protected source was accepted or modified input")
        require(backup.read_bytes() == b"previous executable", "image path swap changed the prior executable bytes")
        self.results["checks"]["controlled_path_swaps"] = ["map destination", "image destination"]
        self.save()

    def run(self) -> None:
        self.prepare()
        self.build_candidate()
        self.build_success_fixture()
        self.build_writer_driver()
        self.check_fresh_and_replaced()
        self.check_aliases()
        self.check_prepublication_failures()
        self.check_final_rename_failure()
        self.check_path_swaps()
        self.results["status"] = "PASS output publication"
        self.save()


def main() -> int:
    if len(sys.argv) != 6 or sys.argv[3] != "--staged" or sys.argv[4] != "--source":
        raise SystemExit("usage: run.py SEED OUT --staged --source SOURCE")
    runner = Runner(Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[5]))
    try:
        runner.run()
    except Exception as error:
        if runner.output_owned:
            runner.results["status"] = "FAIL output publication"
            runner.results["failure"] = str(error)
            runner.save()
        raise
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Failure as error:
        print("output publication: " + str(error), file=sys.stderr)
        raise SystemExit(1)
