#!/usr/bin/env python3
"""Run managed source integration only after the global code-complete barrier."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import re
import shutil
import struct
import subprocess
import tarfile
import time

ROOT = Path(__file__).resolve().parents[1]
SEED = ROOT / "build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME = SEED.parent / "runtime"
MODES = {
    "heap": ["--no-regions"],
    "placed": [],
    "stress-heap": ["--gc-stress", "--no-regions"],
    "stress-placed": ["--gc-stress"],
    "stress-optimized": ["--gc-stress", "--fold-constants", "--reuse-slots"],
}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def hashes(folder):
    return {str(path.relative_to(folder)): digest(path) for path in sorted(folder.rglob("*")) if path.is_file()}


def native_header(data, target):
    if target == "darwin-arm64":
        return len(data) >= 32 and data[:4] == bytes.fromhex("cffaedfe") and struct.unpack_from("<I", data, 4)[0] == 16777228
    return len(data) >= 64 and data[:7] == b"\x7fELF\x02\x01\x01" and struct.unpack_from("<HH", data, 16) == (2, 183)


def main():
    script_hash = digest(Path(__file__))
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--code-complete", action="store_true", help="coordinator has frozen ALL implementation sectors")
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--linux-docker", action="store_true")
    parser.add_argument("--modes", nargs="+", choices=MODES, default=list(MODES))
    args = parser.parse_args()
    if not args.code_complete:
        parser.error("global code-complete barrier must be explicitly acknowledged")
    output = args.output_dir.resolve()
    if not output.is_relative_to(ROOT / "build/v04/managed-integration"):
        parser.error("output must be under build/v04/managed-integration")
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        parser.error("output directory must be empty")
    records, snapshots, artifacts, linux_cases = [], {}, {}, {}

    def record(label, passed, **fields):
        item = dict(label=label, passed=bool(passed), **fields)
        records.append(item)
        print(label, "PASS" if passed else "FAIL", flush=True)
        return item

    def run(label, command, expected=0, *, env=None, input=None, timeout=180, empty=False):
        started = time.monotonic()
        try:
            result = subprocess.run(command, cwd=ROOT, env=env, input=input, capture_output=True, timeout=timeout)
            status, stdout, stderr, error = result.returncode, result.stdout, result.stderr, None
        except (OSError, subprocess.TimeoutExpired) as exc:
            status, stdout, stderr, error = None, b"", b"", str(exc)
        (output / (label + ".stdout")).write_bytes(stdout)
        (output / (label + ".stderr")).write_bytes(stderr)
        item = record(label, status == expected and error is None and (not empty or stdout == b""),
                      command=command, status=status, expected=expected, error=error,
                      elapsed_seconds=time.monotonic() - started,
                      stdout_sha256=hashlib.sha256(stdout).hexdigest(), stderr_sha256=hashlib.sha256(stderr).hexdigest())
        return item, stdout, stderr

    # Keep the exact seed identity independent of a user's PATH/default runtime.
    pinned = SEED.is_file() and digest(SEED) == "89f41b7cac5f38018c32b74d7a134d4d32e1314d58ddf292efcea174d182a921" and (RUNTIME / "librey_rt.a").is_file()
    pinned = pinned and digest(RUNTIME / "librey_rt.a") == "f3d625762a0ea9dfc403e81b5c3cc8cce49aaec64bd276e5b2dec95fad666667"
    record("pinned-seed-runtime", pinned)
    if not pinned:
        raise SystemExit("pinned seed/runtime mismatch")
    seed_env = dict(os.environ, PATH="/usr/bin:/bin:/usr/sbin:/sbin")
    empty_path = output / "empty-path"
    empty_path.mkdir()
    native_env = dict(os.environ, PATH=str(empty_path))
    for key in ("CC", "CXX", "CLANG", "LD", "LLVM_CONFIG", "REY_RUNTIME_DIR"):
        native_env[key] = str(empty_path / "absent")

    runtime_root = output / "runtime-rey"
    fixtures = output / "fixtures"
    shutil.copytree(ROOT / "runtime-rey", runtime_root)
    shutil.copytree(ROOT / "tests/v04/managed-aggregates/fixtures", fixtures)
    cases_file = ROOT / "tests/v04/managed-aggregates/cases.json"
    cases = json.loads(cases_file.read_text())
    pattern_cases_file = ROOT / "tests/v04/managed-patterns/cases.json"
    shutil.copytree(ROOT / "tests/v04/managed-patterns/fixtures", fixtures / "patterns")
    pattern_cases = json.loads(pattern_cases_file.read_text())
    for name, specification in pattern_cases.items():
        cases["patterns/" + name] = specification
    smoke_cases_file = ROOT / "tests/v04/managed-smoke/cases.json"
    shutil.copytree(ROOT / "tests/v04/managed-smoke/fixtures", fixtures / "smoke")
    for name, specification in json.loads(smoke_cases_file.read_text()).items():
        cases["smoke/" + name] = specification
    if not isinstance(cases, dict) or not cases:
        raise ValueError("nonempty managed case manifest required")
    snapshots.update(runtime=hashes(runtime_root), fixtures=hashes(fixtures),
                     cases_sha256=digest(cases_file), pattern_cases_sha256=digest(pattern_cases_file),
                     smoke_cases_sha256=digest(smoke_cases_file))
    (output / "cases.json").write_text(json.dumps(cases, indent=2) + "\n")
    for unit in ("managed-roots", "managed-placement", "managed-link"):
        compiler = output / (unit + "-compiler")
        shutil.copytree(ROOT / "rey-compiler/src", compiler)
        shutil.copyfile(ROOT / "tests/v04" / unit / "main.rey", compiler / "main.rey")
        snapshots[unit + "-compiler"] = hashes(compiler)
        binary = output / (unit + "-fixture")
        built, _, _ = run(unit + "-build", [str(SEED), "build", str(compiler / "main.rey"), "-o", str(binary),
            "--runtime-dir", str(RUNTIME)], env=seed_env)
        if built["passed"]:
            executed, stdout, _ = run(unit + "-execute", [str(binary)], env=native_env)
            record(unit + "-assertions", executed["passed"] and stdout == (unit + ": static assertions passed\n").encode())
    drivers = {}
    for kind, entry in (("ir", "ir-main.rey"), ("native", "native-main.rey"), ("teardown", "../tests/v04/managed-teardown/main.rey")):
        compiler = output / (kind + "-compiler")
        shutil.copytree(ROOT / "rey-compiler/src", compiler)
        shutil.copyfile(ROOT / "rey-compiler" / entry, compiler / "main.rey")
        snapshots[kind + "-compiler"] = hashes(compiler)
        binary = output / (kind + "-driver")
        built, _, _ = run(kind + "-build", [str(SEED), "build", str(compiler / "main.rey"), "-o", str(binary),
                                           "--runtime-dir", str(RUNTIME)], env=seed_env)
        if built["passed"]:
            drivers[kind] = binary
    if "ir" in drivers and "native" in drivers:
        images = output / "images"
        images.mkdir()
        for case_index, (relative, specification) in enumerate(cases.items()):
            if not isinstance(specification, dict) or specification.get("expect") not in ("ok", "check-error", "lower-error", "runtime-error"):
                raise ValueError("invalid manifest case " + relative)
            source = (fixtures / relative).resolve()
            if not source.is_relative_to(fixtures) or not source.is_file():
                raise ValueError("invalid case entry path " + relative)
            case = "case-" + str(case_index)
            expectation = specification["expect"]
            diagnostic = specification.get("diagnostic", "")
            if expectation in ("check-error", "lower-error") and not diagnostic:
                raise ValueError("negative compile cases require a stage-specific diagnostic: " + relative)
            if type(specification.get("teardown", False)) is not bool:
                raise ValueError("teardown must be boolean: " + relative)
            base_options = ["--managed", "--runtime-root", str(runtime_root)]
            check_status = int(expectation in ("check-error", "lower-error"))
            checked, _, stderr = run(case + "-check", [str(drivers["ir"]), "check", str(source)] + base_options,
                                     expected=check_status, env=native_env)
            if diagnostic and check_status:
                record(case + "-diagnostic", diagnostic.encode() in stderr)
            if check_status:
                continue
            native_only = specification.get("native_only", False)
            if type(native_only) is not bool:
                raise ValueError("native_only must be boolean")
            if native_only and not specification.get("reason"):
                raise ValueError("native-only cases require an explicit workload and evaluator-limit reason")
            teardown_native_only = specification.get("teardown_native_only", False)
            if type(teardown_native_only) is not bool or (teardown_native_only and
                    (not native_only or not specification.get("teardown") or not specification.get("teardown_reason"))):
                raise ValueError("native-only teardown requires a native-only case, teardown, and measured limit reason")
            eval_via_teardown = specification.get("eval_via_teardown", False)
            if type(eval_via_teardown) is not bool or (eval_via_teardown and
                    (native_only or not specification.get("teardown") or not specification.get("reason"))):
                raise ValueError("shared teardown interpretation requires interpreted teardown and a measured reason")
            expected_status = int(expectation == "runtime-error")
            for mode in args.modes:
                flags = MODES[mode]
                teardown_interpreted_ok = False
                if expectation == "ok" and specification.get("teardown", False):
                    prefix = images / (case + "-" + mode + "-teardown")
                    if "teardown" not in drivers:
                        record(case + "-" + mode + "-teardown", False, reason="teardown harness build failed")
                    else:
                        teardown_command = [str(drivers["teardown"]), str(source), str(runtime_root), str(prefix), mode]
                        if teardown_native_only:
                            teardown_command.append("native-only-bounded-workload")
                        checked, stdout, _ = run(case + "-" + mode + "-teardown", teardown_command, env=native_env)
                        record(case + "-" + mode + "-teardown-marker", checked["passed"] and stdout == b"managed-teardown: verified\n")
                        steps = Path(str(prefix) + ".steps")
                        step_text = steps.read_text().strip() if steps.is_file() else ""
                        policy_file = Path(str(prefix) + ".evaluation")
                        policy = policy_file.read_text() if policy_file.is_file() else ""
                        valid_steps = step_text.isdigit() and 0 < int(step_text) <= 4000000 and policy == "interpreted\n"
                        if teardown_native_only:
                            valid_steps = step_text == "0" and policy == "native-only\n"
                        teardown_interpreted_ok = checked["passed"] and valid_steps and not teardown_native_only
                        record(case + "-" + mode + "-teardown-steps", checked["passed"] and valid_steps,
                               steps=int(step_text) if step_text.isdigit() else None,
                               evaluator_executed=not teardown_native_only,
                               reason=specification.get("teardown_reason") if teardown_native_only else None)
                        for os_name in ("darwin", "linux"):
                            image = Path(str(prefix) + "." + os_name)
                            name = case + "-" + mode + "-teardown-" + os_name
                            data = image.read_bytes() if image.is_file() else b""
                            valid = checked["passed"] and native_header(data, os_name + "-arm64")
                            record(name + "-image", valid)
                            if not valid:
                                continue
                            artifacts[name] = dict(sha256=digest(image), source_sha256=digest(source), expected_status=0, teardown_checks=True)
                            if os_name == "linux":
                                linux_cases[name] = (image, 0)
                            else:
                                image.chmod(0o755)
                                run(name + "-signature", ["/usr/bin/codesign", "--verify", "--strict", str(image)], timeout=10)
                                run(name + "-execute", [str(image)], empty=True, timeout=30)
                if eval_via_teardown:
                    record(case + "-" + mode + "-evaluator-policy", teardown_interpreted_ok,
                           evaluator_executed=teardown_interpreted_ok,
                           via=case + "-" + mode + "-teardown-steps", reason=specification["reason"])
                elif native_only:
                    record(case + "-" + mode + "-evaluator-policy", True, evaluator_executed=False,
                           reason=specification.get("reason", "explicit native-only long-churn fixture"))
                else:
                    ir_flags = [flag for flag in flags if flag != "--reuse-slots"]
                    evaluated, stdout, stderr = run(case + "-" + mode + "-eval",
                        [str(drivers["ir"]), "eval", str(source)] + base_options + ir_flags,
                        expected=expected_status, env=native_env)
                    if expectation == "ok":
                        record(case + "-" + mode + "-eval-void", evaluated["passed"] and stdout == b"void\n")
                    elif diagnostic:
                        record(case + "-" + mode + "-runtime-diagnostic", diagnostic.encode() in stderr)
                for target in ("darwin-arm64", "linux-arm64"):
                    name = case + "-" + mode + "-" + target
                    image = images / name
                    emitted, _, _ = run(name + "-emit", [str(drivers["native"]), "build", str(source), "-o", str(image),
                        "--target", target] + base_options + flags, env=native_env)
                    data = image.read_bytes() if image.is_file() else b""
                    valid = native_header(data, target)
                    record(name + "-image", emitted["passed"] and valid)
                    if not emitted["passed"] or not valid:
                        continue
                    artifacts[name] = dict(sha256=digest(image), source_sha256=digest(source), expected_status=expected_status)
                    if target == "linux-arm64":
                        linux_cases[name] = (image, expected_status)
                    elif platform.system() == "Darwin" and platform.machine() == "arm64":
                        image.chmod(0o755)
                        run(name + "-signature", ["/usr/bin/codesign", "--verify", "--strict", str(image)], timeout=10)
                        run(name + "-execute", [str(image)], expected=expected_status, empty=True, timeout=30)
                    else:
                        record(name + "-execute", False, reason="Darwin ARM64 host required")
        invalid_options = (["--gc-stress", "--gc-stress"], ["--no-regions", "--no-regions"],
                           ["--managed", "--managed"], ["--runtime-root"], ["--runtime-root", ""],
                           ["--runtime-root", "--gc-stress"])
        first_case = str(fixtures / next(iter(cases)))
        for index, options in enumerate(invalid_options):
            run("ir-invalid-options-" + str(index), [str(drivers["ir"]), "check", first_case] + options, expected=2, env=native_env)
            run("native-invalid-options-" + str(index), [str(drivers["native"]), "build", first_case, "-o", str(images / "invalid"),
                "--target", "darwin-arm64"] + options, expected=2, env=native_env)

    linux_image = None
    if args.linux_docker and linux_cases:
        docker = shutil.which("docker") or "docker"
        inspected, stdout, _ = run("docker-image", [docker, "image", "inspect", "ubuntu:22.04", "--format", "{{.Id}}"])
        linux_image = stdout.decode(errors="replace").strip()
        if inspected["passed"] and linux_image.startswith("sha256:"):
            payload = io.BytesIO()
            with tarfile.open(fileobj=payload, mode="w") as archive:
                for name, (path, _) in linux_cases.items():
                    data = path.read_bytes()
                    item = tarfile.TarInfo(name)
                    item.size, item.mode = len(data), 0o700
                    archive.addfile(item, io.BytesIO(data))
            shell = "tar -xf - -C /work || exit 90; uname -sm; "
            for name in linux_cases:
                shell += "/work/" + name + "; status=$?; printf 'case " + name + " %s\\n' \"$status\"; "
            executed, stdout, _ = run("linux-execute", [docker, "run", "--rm", "--pull", "never", "-i", "--network", "none",
                "--read-only", "--cap-drop", "ALL", "--security-opt", "no-new-privileges", "--memory", "128m", "--pids-limit", "32",
                "--tmpfs", "/work:exec", linux_image, "/bin/sh", "-c", shell], input=payload.getvalue(), timeout=180)
            expected = ["Linux aarch64"] + ["case " + name + " " + str(status) for name, (_, status) in linux_cases.items()]
            record("linux-exact-output", executed["passed"] and stdout.decode(errors="replace").splitlines() == expected)
        else:
            record("linux-execute", False, reason="local Linux image unavailable")
    elif args.linux_docker:
        record("linux-execute", False, reason="no generated Linux cases")
    record("runtime-source-immutable", hashes(runtime_root) == snapshots["runtime"])
    record("fixture-source-immutable", hashes(fixtures) == snapshots["fixtures"])
    for name, snapshot in snapshots.items():
        if name.endswith("-compiler"):
            record(name + "-immutable", hashes(output / name) == snapshot)
    report = dict(schema="rey-v04-managed-source-integration-v1", passed=all(item["passed"] for item in records),
        records=records, snapshots=snapshots, artifacts=artifacts, modes=args.modes, linux_image=linux_image,
        script_sha256=script_hash, seed_sha256=digest(SEED), runtime_sha256=digest(RUNTIME / "librey_rt.a"),
        code_complete_barrier=True, limitation="ARM64 native correctness; explicit bounded-evaluator omissions recorded; no native x64/RSS/timing claim")
    (output / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print("managed integration", "PASS" if report["passed"] else "FAIL", len(records), "checks", flush=True)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
