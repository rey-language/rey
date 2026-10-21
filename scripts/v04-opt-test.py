#!/usr/bin/env python3
"""Independently gate optional scalar constant folding through source drivers."""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import tarfile
import time


ROOT = Path(__file__).resolve().parents[1]
OUTPUT_ROOT = (ROOT / "build/v04/opt-integration").resolve()
SEED = ROOT / "build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME = SEED.parent / "runtime"
BASELINE = ROOT / "build/v04/seed/baseline-seed-winruntime/result.json"
PINNED_SEED_SHA256 = "89f41b7cac5f38018c32b74d7a134d4d32e1314d58ddf292efcea174d182a921"
PINNED_RUNTIME_FILES = {
    "enum.o": "afb37262963bfe125a7e8b03551d6aba9a53d23c38b8674c5fd0228c6fcbcc7d",
    "hashmap.o": "ed65df03614c98814bfbe499634d64e236e8047ad5e9246d1a7e9daa92fbfb59",
    "io.o": "97a3805f7cf6fa0c479d3ed1b7a79f7a24def88b43b0cab2c6432ebbeed304f3",
    "librey_rt.a": "f3d625762a0ea9dfc403e81b5c3cc8cce49aaec64bd276e5b2dec95fad666667",
    "math.o": "fee05d5cd19ea00517fbc287db442ab908779819b4e9ca2e3ada35fdd6dc0b0a",
    "mem.o": "f26c459886fc2c4058b330f35f07ce243b975d6a7db52f045e752d659a949da5",
    "result.o": "b0ee437c29ae0a72a592414768a462aa90a4eaa6c7d83c33baab7d5db116343b",
    "string.o": "2c4d6a571434e8372566c84e06ec59cd6dec63b09c3738c9a44d24d228861322",
    "value.o": "012b7f1240a24d390143d4ea4c1b6f92c965fed3ede883e1d4f4c453d4f3fbb7",
    "vec.o": "ef6c7ee17e45f752ba0959af80d7c38cab249cea5399202ce0303f08933cd47a",
}
TRAPS = {
    "add-overflow": "traps/add-overflow.rey",
    "sub-overflow": "traps/sub-overflow.rey",
    "mul-overflow": "traps/mul-overflow.rey",
    "div-zero": "traps/div-zero.rey",
    "div-min-overflow": "traps/div-min-overflow.rey",
    "rem-min-overflow": "traps/rem-min-overflow.rey",
    "neg-min-overflow": "traps/neg-min-overflow.rey",
    "dead-result-overflow": "traps/dead-result-overflow.rey",
    "first-trap-before-later-call": "traps/first-trap.rey",
}
NATIVE_CASES = {"safe": ("main.rey", 0), "graph": ("graph/main.rey", 0),
                "std-root": ("std-main.rey", 0), "storage": ("storage.rey", 0),
                **{name: (path, 1) for name, path in TRAPS.items()}}


def digest_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def digest(path: Path) -> str:
    return digest_bytes(path.read_bytes())


def tree_hashes(root: Path) -> dict[str, str]:
    return {path.relative_to(root).as_posix(): digest(path)
            for path in sorted(root.rglob("*")) if path.is_file() and not path.is_symlink()}


def add_record(records: list[dict], label: str, passed: bool, **fields) -> dict:
    record = {"kind": "assertion", "label": label, "passed": bool(passed), **fields}
    records.append(record)
    print(label, "PASS" if record["passed"] else "FAIL", flush=True)
    return record


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", required=True, type=Path,
                        help="fresh run directory below build/v04/opt-integration")
    parser.add_argument("--linux-docker", action="store_true",
                        help="execute generated Linux images in an already-present local ubuntu:22.04 image")
    parser.add_argument("--reuse-slots", action="store_true",
                        help="also opt every native emission into conservative slot reuse")
    parser.add_argument("--image", default="ubuntu:22.04",
                        help="Docker image name for --linux-docker; never pulled")
    args = parser.parse_args()

    output = args.output_dir.expanduser().resolve()
    if output == OUTPUT_ROOT or not output.is_relative_to(OUTPUT_ROOT):
        parser.error("--output-dir must be a fresh child of build/v04/opt-integration")
    if output.exists() and (not output.is_dir() or any(output.iterdir())):
        parser.error("--output-dir must be empty")
    output.mkdir(parents=True, exist_ok=True)

    records: list[dict] = []
    snapshots: dict[str, dict] = {}
    artifacts: dict[str, dict] = {}
    modes = [("reuse" if args.reuse_slots else "default", False),
             ("folded-reuse" if args.reuse_slots else "folded", True)]
    fixture_source = ROOT / "tests/v04/opt-integration"
    staged_fixtures = output / "fixtures"
    shutil.copytree(fixture_source, staged_fixtures)
    snapshots["fixtures"] = {"source": str(fixture_source.relative_to(ROOT)),
                             "staged_root": str(staged_fixtures.relative_to(ROOT)),
                             "files": tree_hashes(staged_fixtures)}
    empty_path = output / "empty-path"
    empty_path.mkdir()
    no_tools_env = dict(os.environ, PATH=str(empty_path))
    for key in ("CC", "CXX", "CLANG", "LD", "LLVM_CONFIG", "REY_RUNTIME_DIR"):
        no_tools_env[key] = str(empty_path / ("missing-" + key.lower()))

    def run(label: str, command: list[str], expected: int = 0, *, env: dict | None = None,
            timeout: float = 120, stdout_empty: bool | None = None, contains: str = "",
            input_data: bytes | None = None) -> tuple[dict, bytes, bytes]:
        started = time.monotonic()
        error = None
        try:
            result = subprocess.run(command, cwd=ROOT, capture_output=True, timeout=timeout,
                                    env=env, input=input_data)
            code, stdout, stderr = result.returncode, result.stdout, result.stderr
        except (OSError, subprocess.TimeoutExpired) as exc:
            code, stdout, stderr, error = None, b"", b"", str(exc)
        stdout_path = output / (label + ".stdout")
        stderr_path = output / (label + ".stderr")
        stdout_path.parent.mkdir(parents=True, exist_ok=True)
        stdout_path.write_bytes(stdout)
        stderr_path.write_bytes(stderr)
        passed = code == expected and error is None
        if stdout_empty is not None:
            passed = passed and ((len(stdout) == 0) == stdout_empty)
        if contains:
            passed = passed and contains in (stdout + stderr).decode(errors="replace")
        record = {"kind": "process", "label": label, "command": command,
                  "returncode": code, "expected_returncode": expected, "error": error,
                  "elapsed_seconds": time.monotonic() - started,
                  "environment": (None if env is None else {
                      "path": env.get("PATH"),
                      "compiler_variables": {key: env[key] for key in
                          ("CC", "CXX", "CLANG", "LD", "LLVM_CONFIG", "REY_RUNTIME_DIR") if key in env},
                  }),
                  "stdout": {"path": str(stdout_path.relative_to(ROOT)), "bytes": len(stdout),
                             "sha256": digest_bytes(stdout)},
                  "stderr": {"path": str(stderr_path.relative_to(ROOT)), "bytes": len(stderr),
                             "sha256": digest_bytes(stderr)}, "passed": bool(passed)}
        records.append(record)
        print(label, "PASS" if passed else "FAIL", flush=True)
        return record, stdout, stderr

    baseline_ok = False
    baseline_data: dict = {}
    try:
        baseline_data = json.loads(BASELINE.read_text())
        provenance = baseline_data["provenance"]
        baseline_engine = provenance["engine_sha256"]
        baseline_runtime = provenance["runtime_files_sha256"]
        baseline_ok = baseline_engine == PINNED_SEED_SHA256 and baseline_runtime == PINNED_RUNTIME_FILES
    except (OSError, KeyError, TypeError, json.JSONDecodeError):
        provenance = {}
    actual_seed = digest(SEED) if SEED.is_file() else None
    actual_runtime = tree_hashes(RUNTIME) if RUNTIME.is_dir() else {}
    provenance_ok = (baseline_ok and actual_seed == PINNED_SEED_SHA256
                     and actual_runtime == PINNED_RUNTIME_FILES and os.access(SEED, os.X_OK))
    add_record(records, "pinned-seed-runtime-provenance", provenance_ok,
               baseline_report=str(BASELINE.relative_to(ROOT)),
               baseline_report_sha256=digest(BASELINE) if BASELINE.is_file() else None,
               baseline_engine_sha256=provenance.get("engine_sha256"),
               expected_engine_sha256=PINNED_SEED_SHA256, actual_engine_sha256=actual_seed,
               baseline_runtime_files_sha256=baseline_runtime if baseline_ok else None,
               expected_runtime_files_sha256=PINNED_RUNTIME_FILES,
               actual_runtime_files_sha256=actual_runtime)

    def stage_driver(label: str, source_file: Path) -> tuple[Path, Path]:
        project = output / label
        source_root = project / "src"
        shutil.copytree(ROOT / "rey-compiler/src", source_root)
        shutil.copyfile(source_file, source_root / "main.rey")
        snapshots[label] = {"source_driver": str(source_file.relative_to(ROOT)),
                            "staged_root": str(source_root.relative_to(ROOT)),
                            "files": tree_hashes(source_root)}
        return source_root / "main.rey", project / label

    ir_source, ir_exe = stage_driver("ir-driver", ROOT / "rey-compiler/ir-main.rey")
    native_source, native_exe = stage_driver("native-driver", ROOT / "rey-compiler/native-main.rey")
    # The bootstrap compiler uses the host system Clang; /usr/bin is deliberately first.
    seed_env = dict(os.environ, PATH="/usr/bin:/bin:/usr/sbin:/sbin")
    for key in ("CC", "CXX", "CLANG", "LD", "LLVM_CONFIG"):
        seed_env.pop(key, None)

    if provenance_ok:
        for label, source, executable in (("ir-driver", ir_source, ir_exe),
                                          ("native-driver", native_source, native_exe)):
            build, _, _ = run(label + "-seed-build",
                              [str(SEED), "build", str(source), "-o", str(executable),
                               "--runtime-dir", str(RUNTIME)], env=seed_env)
            build["artifact_exists"] = executable.is_file()
            build["artifact_sha256"] = digest(executable) if executable.is_file() else None
            if not executable.is_file():
                build["passed"] = False
    else:
        add_record(records, "seed-driver-builds-skipped", False,
                   reason="pinned seed or runtime did not match the recorded baseline")

    driver_ready = provenance_ok and ir_exe.is_file() and native_exe.is_file()
    source = lambda rel: staged_fixtures / rel
    stdroot = staged_fixtures / "stdroot"
    safe_source = source("main.rey")
    graph_source = source("graph/main.rey")
    std_source = source("std-main.rey")
    fold_source = source("fold-dump.rey")

    def ir_command(driver: Path, action: str, file: Path, folded: bool = False,
                   std: Path | None = None, flag_order: str = "fold-then-std") -> list[str]:
        command = [str(driver), action, str(file)]
        if folded and flag_order == "fold-then-std":
            command.append("--fold-constants")
        if std is not None:
            command += ["--std-root", str(std)]
        if folded and flag_order == "std-then-fold":
            command.append("--fold-constants")
        return command

    def native_command(driver: Path, file: Path, artifact: Path, target: str,
                       folded: bool = False, std: Path | None = None,
                       flag_order: str = "fold-then-std") -> list[str]:
        command = [str(driver), "build", str(file), "-o", str(artifact), "--target", target]
        if args.reuse_slots:
            command.append("--reuse-slots")
        if folded and flag_order == "fold-then-std":
            command.append("--fold-constants")
        if std is not None:
            command += ["--std-root", str(std)]
        if folded and flag_order == "std-then-fold":
            command.append("--fold-constants")
        return command

    def compare_processes(label: str, original: tuple[dict, bytes, bytes],
                          folded: tuple[dict, bytes, bytes]) -> dict:
        before, before_out, before_err = original
        after, after_out, after_err = folded
        return add_record(records, label,
                          before["passed"] and after["passed"]
                          and before["returncode"] == after["returncode"]
                          and before_out == after_out and before_err == after_err,
                          original_label=before["label"], folded_label=after["label"],
                          original_returncode=before["returncode"], folded_returncode=after["returncode"],
                          stdout_identical=before_out == after_out,
                          stderr_identical=before_err == after_err)

    if driver_ready:
        for label, command in (("ir-driver-help", [str(ir_exe), "--help"]),
                               ("native-driver-help", [str(native_exe), "--help"])):
            run(label, command, contains="Usage:")

        default_dump = run("ir-default-dump", ir_command(ir_exe, "dump", fold_source))
        repeated_dump = run("ir-default-dump-repeat", ir_command(ir_exe, "dump", fold_source))
        folded_dump = run("ir-folded-dump", ir_command(ir_exe, "dump", fold_source, folded=True))
        add_record(records, "deterministic-default-dump",
                   default_dump[0]["passed"] and repeated_dump[0]["passed"]
                   and default_dump[1] == repeated_dump[1],
                   first_sha256=digest_bytes(default_dump[1]), repeat_sha256=digest_bytes(repeated_dump[1]))
        original_text = default_dump[1].decode(errors="replace")
        folded_text = folded_dump[1].decode(errors="replace")
        original_adds = original_text.count("add.checked.i64")
        folded_adds = folded_text.count("add.checked.i64")
        fold_source_text = fold_source.read_text()
        safe_span = ":{}..{}".format(fold_source_text.index("19 + 23"),
                                      fold_source_text.index("19 + 23") + len("19 + 23"))
        overflow_expr = "9223372036854775807 + 1"
        overflow_start = fold_source_text.index(overflow_expr)
        overflow_span = ":{}..{}".format(overflow_start, overflow_start + len(overflow_expr))
        folded_add_lines = [line for line in folded_text.splitlines() if "add.checked.i64" in line]
        add_record(records, "dump-folds-safe-op-and-retains-traps-and-spans",
                   default_dump[0]["passed"] and folded_dump[0]["passed"]
                   and original_adds > folded_adds and folded_adds >= 1
                   and "div.checked.i64" in folded_text
                   and "fold-dump.rey:" in folded_text and ".." in folded_text
                   and ("const.i64 42 @" in folded_text and safe_span in folded_text)
                   and any(overflow_span in line for line in folded_add_lines)
                   and all(safe_span not in line for line in folded_add_lines),
                   original_checked_adds=original_adds, folded_checked_adds=folded_adds,
                   retained_division="div.checked.i64" in folded_text,
                   folded_source_span="fold-dump.rey:" in folded_text and ".." in folded_text,
                   folded_safe_const_span=safe_span, retained_overflow_span=overflow_span)

        for action in ("check", "eval"):
            before = run("safe-" + action + "-default", ir_command(ir_exe, action, safe_source))
            after = run("safe-" + action + "-folded", ir_command(ir_exe, action, safe_source, folded=True))
            compare_processes("safe-" + action + "-same-result", before, after)
            before = run("storage-" + action + "-default", ir_command(ir_exe, action, source("storage.rey")))
            after = run("storage-" + action + "-folded", ir_command(ir_exe, action, source("storage.rey"), folded=True))
            compare_processes("storage-" + action + "-same-result", before, after)
        for name, file in (("graph", graph_source), ("std-root", std_source)):
            for order in ("fold-then-std", "std-then-fold"):
                before = run(f"{name}-eval-{order}-default",
                             ir_command(ir_exe, "eval", file, std=stdroot if name == "std-root" else None))
                after = run(f"{name}-eval-{order}-folded",
                            ir_command(ir_exe, "eval", file, folded=True,
                                       std=stdroot if name == "std-root" else None, flag_order=order))
                compare_processes(f"{name}-eval-{order}-same-result", before, after)

        for name, relative in TRAPS.items():
            file = source(relative)
            before = run(name + "-eval-default", ir_command(ir_exe, "eval", file), expected=1)
            after = run(name + "-eval-folded", ir_command(ir_exe, "eval", file, folded=True), expected=1)
            compare_processes(name + "-trap-preserved", before, after)
            if name == "first-trap-before-later-call":
                error_text = (after[1] + after[2]).decode(errors="replace")
                add_record(records, "first-trap-precedes-later-trapping-call",
                           after[0]["passed"] and "overflow" in error_text
                           and "division by zero" not in error_text,
                           expected_first_failure="checked signed overflow",
                           observed_error_sha256=digest_bytes(after[1] + after[2]))

        for name in ("invalid-source", "invalid-literal"):
            file = source(name + ".rey")
            before = run(name + "-check-default", ir_command(ir_exe, "check", file), expected=1)
            after = run(name + "-check-folded", ir_command(ir_exe, "check", file, folded=True), expected=1)
            compare_processes(name + "-diagnostic-preserved", before, after)

        ir_bad_commands = {
            "ir-duplicate-fold": [str(ir_exe), "check", str(safe_source), "--fold-constants", "--fold-constants"],
            "ir-missing-std-root": [str(ir_exe), "check", str(safe_source), "--std-root"],
            "ir-empty-std-root": [str(ir_exe), "check", str(safe_source), "--std-root", ""],
            "ir-missing-std-root-before-fold": [str(ir_exe), "check", str(safe_source),
                                                  "--std-root", "--fold-constants"],
            "ir-duplicate-std-root": [str(ir_exe), "check", str(safe_source), "--std-root",
                                      str(stdroot), "--std-root", str(stdroot)],
            "ir-unknown-option": [str(ir_exe), "check", str(safe_source), "--unknown"],
            "ir-missing-source": [str(ir_exe), "check"],
            "native-duplicate-fold": [str(native_exe), "build", str(safe_source), "-o", str(output / "bad-image"),
                                      "--target", "darwin-arm64", "--fold-constants", "--fold-constants"],
            "native-missing-std-root": [str(native_exe), "build", str(safe_source), "-o", str(output / "bad-image"),
                                        "--target", "darwin-arm64", "--std-root"],
            "native-empty-std-root": [str(native_exe), "build", str(safe_source), "-o",
                                      str(output / "bad-image"), "--target", "darwin-arm64",
                                      "--std-root", ""],
            "native-missing-std-root-before-fold": [str(native_exe), "build", str(safe_source), "-o",
                                                     str(output / "bad-image"), "--target", "darwin-arm64",
                                                     "--std-root", "--fold-constants"],
            "native-duplicate-std-root": [str(native_exe), "build", str(safe_source), "-o",
                                          str(output / "bad-image"), "--target", "darwin-arm64",
                                          "--std-root", str(stdroot), "--std-root", str(stdroot)],
            "native-unknown-option": [str(native_exe), "build", str(safe_source), "-o", str(output / "bad-image"),
                                      "--target", "darwin-arm64", "--unknown"],
            "native-duplicate-reuse": [str(native_exe), "build", str(safe_source), "-o", str(output / "bad-image"),
                                       "--target", "darwin-arm64", "--reuse-slots", "--reuse-slots"],
        }
        for label, command in ir_bad_commands.items():
            run(label, command, expected=2)

        # Every source-to-native-image invocation runs with no compiler on PATH.
        for target in ("darwin-arm64", "linux-arm64", "linux-x64"):
            for name, (relative, expected) in NATIVE_CASES.items():
                file = source(relative)
                for mode, folded in modes:
                    artifact = output / "images" / target / (mode + "-" + name)
                    artifact.parent.mkdir(parents=True, exist_ok=True)
                    order = "fold-then-std"
                    if name == "std-root" and folded and target == "darwin-arm64":
                        order = "fold-then-std"
                    command = native_command(native_exe, file, artifact, target, folded=folded,
                                             std=stdroot if name == "std-root" else None,
                                             flag_order=order)
                    emitted, _, _ = run(f"{target}-{mode}-{name}-emit", command, env=no_tools_env)
                    expected_machine = {"darwin-arm64": bytes.fromhex("cffaedfe"),
                                        "linux-arm64": b"\x7fELF\x02\x01\x01" + b"\x00" * 9 + b"\x02\x00\xb7\x00",
                                        "linux-x64": b"\x7fELF\x02\x01\x01" + b"\x00" * 9 + b"\x02\x00\x3e\x00"}[target]
                    header = artifact.read_bytes()[:len(expected_machine)] if artifact.is_file() else b""
                    image_ok = emitted["passed"] and artifact.is_file() and header == expected_machine
                    image_sha = digest(artifact) if artifact.is_file() else None
                    add_record(records, f"{target}-{mode}-{name}-image-header", image_ok,
                               artifact=str(artifact.relative_to(ROOT)) if artifact.exists() else None,
                               artifact_sha256=image_sha, expected_header_hex=expected_machine.hex(),
                               observed_header_hex=header.hex() if artifact.is_file() else None,
                               source_sha256=digest(file))
                    artifacts[f"{target}/{mode}/{name}"] = {
                        "source": str(file.relative_to(ROOT)), "source_sha256": digest(file),
                        "image": str(artifact.relative_to(ROOT)) if artifact.is_file() else None,
                        "image_sha256": image_sha, "expected_status": expected,
                        "folded": folded, "target": target,
                        "slot_reuse": args.reuse_slots,
                    }

                    if target == "darwin-arm64":
                        host_ok = platform.system() == "Darwin" and platform.machine().lower() in ("arm64", "aarch64")
                        if host_ok and image_ok:
                            artifact.chmod(0o755)
                            run(f"darwin-{mode}-{name}-codesign",
                                ["/usr/bin/codesign", "--verify", "--strict", str(artifact)])
                            executed, stdout, stderr = run(f"darwin-{mode}-{name}-execute", [str(artifact)],
                                                          expected=expected, stdout_empty=True, timeout=20)
                            add_record(records, f"darwin-{mode}-{name}-execution-evidence",
                                       executed["passed"] and len(stdout) == 0,
                                       expected_status=expected, observed_status=executed["returncode"],
                                       stdout_bytes=len(stdout), stderr_sha256=digest_bytes(stderr))
                        else:
                            add_record(records, f"darwin-{mode}-{name}-native-execution-unavailable", False,
                                       host=f"{platform.system()} {platform.machine()}", image_valid=image_ok)

        # Rebuild std-root with the alternate trailing option order for Darwin, folded mode.
        for order in ("fold-then-std", "std-then-fold"):
            artifact = output / "images" / "darwin-arm64" / ("std-order-" + order)
            command = native_command(native_exe, std_source, artifact, "darwin-arm64", folded=True,
                                     std=stdroot, flag_order=order)
            generated, _, _ = run("darwin-std-root-" + order + "-emit", command, env=no_tools_env)
            valid = generated["passed"] and artifact.is_file() and artifact.read_bytes()[:4] == bytes.fromhex("cffaedfe")
            add_record(records, "darwin-std-root-" + order + "-option-order", valid,
                       artifact_sha256=digest(artifact) if artifact.is_file() else None,
                       fold_flag_order=order)
            if valid and platform.system() == "Darwin" and platform.machine().lower() in ("arm64", "aarch64"):
                artifact.chmod(0o755)
                run("darwin-std-root-" + order + "-codesign",
                    ["/usr/bin/codesign", "--verify", "--strict", str(artifact)])
                run("darwin-std-root-" + order + "-execute", [str(artifact)], stdout_empty=True)

    linux_image_id = None
    if args.linux_docker:
        docker = shutil.which("docker")
        if not docker:
            add_record(records, "docker-available", False, reason="Docker executable not found")
        else:
            inspected, image_info, _ = run("docker-image-inspect",
                                           [docker, "image", "inspect", args.image, "--format", "{{.Id}} {{.Architecture}}"])
            fields = image_info.decode(errors="replace").strip().split()
            if inspected["passed"] and len(fields) == 2 and fields[0].startswith("sha256:"):
                linux_image_id = fields[0]
                add_record(records, "docker-image-identity", True, image=linux_image_id,
                           architecture=fields[1], pull_policy="never")
            else:
                add_record(records, "docker-image-identity", False,
                           observed=image_info.decode(errors="replace").strip(), required=args.image)

    if driver_ready:
        for target in ("linux-arm64", "linux-x64"):
            case_files = [(name, source(relative), expected)
                          for name, (relative, expected) in NATIVE_CASES.items()]
            for mode, _ in modes:
                case_specs = []
                all_images = True
                for name, file, expected in case_files:
                    artifact = output / "images" / target / (mode + "-" + name)
                    case_specs.append((name, artifact, expected))
                    all_images = all_images and artifact.is_file()
                if not args.linux_docker:
                    add_record(records, f"{target}-{mode}-execution-not-requested", True,
                               evidence="images emitted and structurally checked; Docker execution was not requested")
                elif not linux_image_id or not all_images:
                    add_record(records, f"{target}-{mode}-docker-execution", False,
                               reason="missing local image identity or one or more generated images")
                else:
                    payload = io.BytesIO()
                    with tarfile.open(fileobj=payload, mode="w") as archive:
                        for name, artifact, _ in case_specs:
                            data = artifact.read_bytes()
                            entry = tarfile.TarInfo(name)
                            entry.size, entry.mode = len(data), 0o700
                            archive.addfile(entry, io.BytesIO(data))
                    shell = "tar -xf - -C /work || exit 90; uname -sm; "
                    for name, _, _ in case_specs:
                        shell += "/work/" + name + "; status=$?; printf 'case " + name + " %s\\n' \"$status\"; "
                    execution, stdout, _ = run(
                        f"{target}-{mode}-docker-run",
                        [docker, "run", "--rm", "--pull", "never", "-i", "--network", "none",
                         "--read-only", "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
                         "--memory", "128m", "--pids-limit", "32", "--tmpfs", "/work:exec",
                         linux_image_id, "/bin/sh", "-c", shell], input_data=payload.getvalue(), timeout=180)
                    lines = stdout.decode(errors="replace").splitlines()
                    observed_host = lines[0] if lines else None
                    target_arch = "aarch64" if target == "linux-arm64" else "x86_64"
                    matching_vm = observed_host == "Linux " + target_arch
                    x64_emulation = target == "linux-x64" and observed_host == "Linux aarch64"
                    host_ok = matching_vm or x64_emulation
                    mode_text = ("matching Linux VM architecture" if matching_vm else
                                 "x86_64 user-mode emulation in Linux aarch64 VM" if x64_emulation else "unknown")
                    add_record(records, f"{target}-{mode}-linux-host", execution["passed"] and host_ok,
                               observed=observed_host, execution_mode=mode_text,
                               x64_emulation_explicitly_recorded=x64_emulation)
                    expected_output_lines = [f"case {name} {expected}" for name, _, expected in case_specs]
                    output_shape_ok = (len(lines) == len(case_specs) + 1
                                       and lines[1:] == expected_output_lines)
                    add_record(records, f"{target}-{mode}-linux-stdout-exact",
                               execution["passed"] and host_ok and output_shape_ok,
                               observed_line_count=len(lines), expected_line_count=len(case_specs) + 1,
                               observed_case_lines=lines[1:], expected_case_lines=expected_output_lines)
                    found: dict[str, list[int]] = {}
                    for line in lines[1:]:
                        match = re.fullmatch(r"case ([a-z0-9-]+) (-?\d+)", line.strip())
                        if match:
                            found.setdefault(match.group(1), []).append(int(match.group(2)))
                    add_record(records, f"{target}-{mode}-linux-case-set",
                               set(found) == {name for name, _, _ in case_specs},
                               expected=sorted(name for name, _, _ in case_specs), observed=found)
                    for name, _, expected in case_specs:
                        add_record(records, f"{target}-{mode}-{name}-execute",
                                   host_ok and found.get(name) == [expected],
                                   expected_status=expected, observed_statuses=found.get(name),
                                   execution_mode=mode_text)

    emitted_source_builds = [record for record in records
                             if record.get("kind") == "process"
                             and record.get("label", "").endswith("-emit")]
    no_tools_values = {key: str(empty_path / ("missing-" + key.lower()))
                       for key in ("CC", "CXX", "CLANG", "LD", "LLVM_CONFIG", "REY_RUNTIME_DIR")}
    isolated_builds = bool(emitted_source_builds) and all(
        record.get("environment", {}).get("path") == str(empty_path)
        and record.get("environment", {}).get("compiler_variables") == no_tools_values
        for record in emitted_source_builds)
    add_record(records, "emitted-source-builds-have-empty-path-and-bogus-tools",
               isolated_builds, emitted_build_count=len(emitted_source_builds),
               empty_path=str(empty_path), compiler_variables=no_tools_values)

    # Every process invocation must have complete, hashed stdout and stderr evidence.
    process_records = [record for record in records if record.get("kind") == "process"]
    evidence_complete = all(
        record.get("returncode") is not None and record.get("error") is None
        and record.get("stdout", {}).get("sha256")
        and record.get("stderr", {}).get("sha256")
        and (ROOT / record["stdout"]["path"]).is_file()
        and (ROOT / record["stderr"]["path"]).is_file()
        for record in process_records)
    add_record(records, "process-evidence-complete", evidence_complete,
               process_record_count=len(process_records),
               rule="each process has return code plus persisted SHA-256 stdout and stderr")
    report = {
        "schema": "rey-v04-opt-integration-v1", "seed": str(SEED),
        "seed_sha256": actual_seed, "runtime": str(RUNTIME),
        "runtime_files_sha256": actual_runtime,
        "baseline_report": str(BASELINE.relative_to(ROOT)),
        "baseline_report_sha256": digest(BASELINE) if BASELINE.is_file() else None,
        "driver_snapshots": snapshots,
        "fixture_sha256": snapshots["fixtures"]["files"],
        "artifacts": artifacts, "docker_image": linux_image_id,
        "linux_docker_requested": args.linux_docker,
        "slot_reuse_requested": args.reuse_slots,
        "linux_execution_note": "linux-x64 runs in a Linux aarch64 VM are labeled user-mode emulation",
        "records": records, "process_record_count": len(process_records),
        "passed": all(record.get("passed", False) for record in records),
        "limitation": "scalar folding integration correctness only; this is not a performance or full v0.4 claim",
    }
    result_path = output / "result.json"
    result_path.write_text(json.dumps(report, indent=2) + "\n")
    print("result", result_path, "PASS" if report["passed"] else "FAIL", flush=True)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
