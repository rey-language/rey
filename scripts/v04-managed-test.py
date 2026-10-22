#!/usr/bin/env python3
"""Run region-only managed source integration after the global code-complete barrier.

The production path is the compiler-owned region runtime. The harness rejects
collector-stress options, never selects the legacy heap path, and executes the
same frozen source under the default, promotion-stress, optimized, and combined
region modes.
"""
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
import sys
import tarfile
import time

ROOT = Path(__file__).resolve().parents[1]
SEED = ROOT / "build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME = SEED.parent / "runtime"
MODES = {
    "region": [],
    "stress": ["--promotion-stress"],
    "optimized": ["--fold-constants", "--reuse-slots"],
    "stress-optimized": ["--promotion-stress", "--fold-constants", "--reuse-slots"],
}
SUITES = (
    ("aggregates", ROOT / "tests/v04/managed-aggregates"),
    ("patterns", ROOT / "tests/v04/managed-patterns"),
    ("smoke", ROOT / "tests/v04/managed-smoke"),
    ("region-source", ROOT / "tests/v04/region-source"),
)
EXPECTATIONS = ("ok", "check-error", "lower-error", "runtime-error")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def hashes(folder):
    return {str(path.relative_to(folder)): digest(path)
            for path in sorted(folder.rglob("*")) if path.is_file()}


def slug(value):
    result = re.sub(r"[^A-Za-z0-9]+", "-", value).strip("-")
    return result or "case"


def native_header(data, target):
    if target == "darwin-arm64":
        return (len(data) >= 32 and data[:4] == bytes.fromhex("cffaedfe")
                and struct.unpack_from("<I", data, 4)[0] == 16777228)
    return (len(data) >= 64 and data[:7] == b"\x7fELF\x02\x01\x01"
            and struct.unpack_from("<HH", data, 16) == (2, 183))


def main():
    script_hash = digest(Path(__file__))
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--code-complete", action="store_true",
                        help="coordinator has frozen ALL implementation sectors")
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

    def run(label, command, expected=0, *, env=None, input=None, timeout=180,
            empty=False, nonzero=False):
        started = time.monotonic()
        try:
            result = subprocess.run(command, cwd=ROOT, env=env, input=input,
                                    capture_output=True, timeout=timeout)
            status, stdout, stderr, error = result.returncode, result.stdout, result.stderr, None
        except (OSError, subprocess.TimeoutExpired) as exc:
            status, stdout, stderr, error = None, b"", b"", str(exc)
        (output / (label + ".stdout")).write_bytes(stdout)
        (output / (label + ".stderr")).write_bytes(stderr)
        status_ok = status is not None and ((status != 0) if nonzero else (status == expected))
        passed = status_ok and error is None and (not empty or stdout == b"")
        item = record(label, passed, command=command, status=status,
                      expected=("nonzero" if nonzero else expected), error=error,
                      elapsed_seconds=time.monotonic() - started,
                      stdout_sha256=hashlib.sha256(stdout).hexdigest(),
                      stderr_sha256=hashlib.sha256(stderr).hexdigest())
        return item, stdout, stderr

    # Keep the exact seed identity independent of a user's PATH/default runtime.
    pinned = (SEED.is_file()
              and digest(SEED) == "89f41b7cac5f38018c32b74d7a134d4d32e1314d58ddf292efcea174d182a921"
              and (RUNTIME / "librey_rt.a").is_file())
    pinned = pinned and digest(RUNTIME / "librey_rt.a") == "f3d625762a0ea9dfc403e81b5c3cc8cce49aaec64bd276e5b2dec95fad666667"
    record("pinned-seed-runtime", pinned)
    if not pinned:
        raise SystemExit("pinned seed/runtime mismatch")

    seed_env = dict(os.environ, PATH="/usr/bin:/bin:/usr/sbin:/sbin",
                    CC="/usr/bin/clang", CXX="/usr/bin/clang++")
    empty_path = output / "empty-path"
    empty_path.mkdir()
    native_env = dict(os.environ, PATH=str(empty_path))
    for key in ("CC", "CXX", "CLANG", "LD", "LLVM_CONFIG", "REY_RUNTIME_DIR"):
        native_env[key] = str(empty_path / "absent")

    contract = run("region-contract-check",
                   [sys.executable, str(ROOT / "scripts/v04-region-check.py")],
                   env=dict(os.environ))
    if not contract[0]["passed"]:
        raise SystemExit("region contract check failed")

    runtime_root = output / "runtime-rey"
    fixtures_root = output / "fixtures"
    shutil.copytree(ROOT / "runtime-rey", runtime_root)
    fixtures_root.mkdir()
    cases = {}
    for suite, base in SUITES:
        shutil.copytree(base / "fixtures", fixtures_root / suite)
        suite_cases = json.loads((base / "cases.json").read_text())
        if not isinstance(suite_cases, dict) or not suite_cases:
            raise ValueError("nonempty managed case manifest required: " + suite)
        for relative, specification in suite_cases.items():
            cases[suite + "/" + relative] = specification
        snapshots[suite + "-cases"] = digest(base / "cases.json")
    (output / "cases.json").write_text(json.dumps(cases, indent=2) + "\n")
    snapshots["runtime"] = hashes(runtime_root)
    snapshots["fixtures"] = hashes(fixtures_root)

    # Compiler-owned region plan assertions.
    compiler = output / "managed-regions-compiler"
    shutil.copytree(ROOT / "rey-compiler/src", compiler)
    shutil.copyfile(ROOT / "tests/v04/managed-regions/main.rey", compiler / "main.rey")
    snapshots["managed-regions-compiler"] = hashes(compiler)
    static_binary = output / "managed-regions-fixture"
    built, _, _ = run("managed-regions-build",
                      [str(SEED), "build", str(compiler / "main.rey"), "-o", str(static_binary),
                       "--runtime-dir", str(RUNTIME)], env=seed_env)
    if built["passed"]:
        executed, stdout, _ = run("managed-regions-execute", [str(static_binary)], env=native_env)
        record("managed-regions-assertions", executed["passed"]
               and stdout == b"managed-regions: static assertions passed\n")

    # Direct region runtime contract gate (evaluator, no collector path).
    runtime_gate, _, _ = run("region-runtime-gate",
                             ["/bin/sh", str(ROOT / "tests/v04/region-runtime/run.sh")],
                             env=seed_env, timeout=300)
    if runtime_gate["passed"]:
        gate_text = (output / "region-runtime-gate.stdout").read_text()
        record("region-runtime-cases", gate_text.count("region-runtime-case ") >= 16)

    drivers = {}
    for kind, entry in (("ir", "ir-main.rey"), ("native", "native-main.rey")):
        compiler = output / (kind + "-compiler")
        shutil.copytree(ROOT / "rey-compiler/src", compiler)
        shutil.copyfile(ROOT / "rey-compiler" / entry, compiler / "main.rey")
        snapshots[kind + "-compiler"] = hashes(compiler)
        binary = output / (kind + "-driver")
        built, _, _ = run(kind + "-build",
                          [str(SEED), "build", str(compiler / "main.rey"), "-o", str(binary),
                           "--runtime-dir", str(RUNTIME)], env=seed_env)
        if built["passed"]:
            drivers[kind] = binary
    if "ir" not in drivers or "native" not in drivers:
        raise SystemExit("managed driver build failed")

    images = output / "images"
    images.mkdir()
    for case_index, (relative, specification) in enumerate(cases.items()):
        if not isinstance(specification, dict) or specification.get("expect") not in EXPECTATIONS:
            raise ValueError("invalid manifest case " + relative)
        source = (fixtures_root / relative).resolve()
        if not source.is_relative_to(fixtures_root) or not source.is_file():
            raise ValueError("invalid case entry path " + relative)
        label = "case-%03d-%s" % (case_index, slug(relative))
        expectation = specification["expect"]
        diagnostic = specification.get("diagnostic", "")
        if expectation in ("check-error", "lower-error") and not diagnostic:
            raise ValueError("negative compile cases require a stage-specific diagnostic: " + relative)
        native_only = specification.get("native_only", False)
        if type(native_only) is not bool:
            raise ValueError("native_only must be boolean: " + relative)
        if native_only and not specification.get("reason"):
            raise ValueError("native-only cases require an explicit evaluator-limit reason: " + relative)
        base_options = ["--managed", "--runtime-root", str(runtime_root)]
        if expectation == "check-error":
            checked, _, stderr = run(label + "-check",
                                     [str(drivers["ir"]), "check", str(source)] + base_options,
                                     env=native_env, nonzero=True)
            record(label + "-diagnostic", checked["passed"] and diagnostic.encode() in stderr,
                   fixture=relative)
            continue
        if expectation == "lower-error":
            lowered, _, stderr = run(label + "-lower",
                                     [str(drivers["native"]), "build", str(source), "-o",
                                      str(images / (label + "-lower")), "--target", "darwin-arm64"]
                                     + base_options, env=native_env, nonzero=True)
            record(label + "-diagnostic", lowered["passed"] and diagnostic.encode() in stderr,
                   fixture=relative)
            continue
        expected_status = int(expectation == "runtime-error")
        for mode in args.modes:
            flags = MODES[mode]
            mode_label = label + "-" + mode
            if native_only:
                record(mode_label + "-evaluator-policy", True, evaluator_executed=False,
                       fixture=relative, reason=specification["reason"])
            else:
                eval_flags = [flag for flag in flags if flag != "--reuse-slots"]
                evaluated, stdout, stderr = run(mode_label + "-eval",
                    [str(drivers["ir"]), "eval", str(source)] + base_options + eval_flags,
                    expected=expected_status, env=native_env, timeout=300)
                if expectation == "ok":
                    record(mode_label + "-eval-void", evaluated["passed"] and stdout == b"void\n",
                           fixture=relative)
                elif diagnostic:
                    record(mode_label + "-runtime-diagnostic",
                           evaluated["passed"] and diagnostic.encode() in stderr,
                           fixture=relative)
            for target in ("darwin-arm64", "linux-arm64"):
                name = mode_label + "-" + target
                image = images / name
                emitted, _, _ = run(name + "-emit",
                    [str(drivers["native"]), "build", str(source), "-o", str(image),
                     "--target", target] + base_options + flags, env=native_env)
                data = image.read_bytes() if image.is_file() else b""
                valid = native_header(data, target)
                record(name + "-image", emitted["passed"] and valid, fixture=relative)
                if not emitted["passed"] or not valid:
                    continue
                artifacts[name] = dict(sha256=digest(image), source_sha256=digest(source),
                                       expected_status=expected_status, fixture=relative)
                if target == "linux-arm64":
                    linux_cases[name] = (image, expected_status)
                elif platform.system() == "Darwin" and platform.machine() == "arm64":
                    image.chmod(0o755)
                    run(name + "-signature",
                        ["/usr/bin/codesign", "--verify", "--strict", str(image)],
                        timeout=10, env=native_env)
                    run(name + "-execute", [str(image)], expected=expected_status,
                        empty=True, timeout=30, env=native_env)
                else:
                    record(name + "-execute", False, reason="Darwin ARM64 host required")

    first_case = str(fixtures_root / next(iter(cases)))
    invalid_options = (
        (["--gc-stress"], 2),
        (["--managed", "--managed"], 2),
        (["--runtime-root"], 2),
        (["--runtime-root", ""], 2),
        (["--runtime-root", "--gc-stress"], 2),
    )
    for index, (options, expected) in enumerate(invalid_options):
        run("ir-invalid-options-" + str(index),
            [str(drivers["ir"]), "check", first_case, "--managed"] + options,
            expected=expected, env=native_env)
        run("native-invalid-options-" + str(index),
            [str(drivers["native"]), "build", first_case, "-o", str(images / "invalid"),
             "--target", "darwin-arm64", "--managed"] + options,
            expected=expected, env=native_env)

    linux_image = None
    if args.linux_docker and linux_cases:
        docker = shutil.which("docker") or "docker"
        inspected, stdout, _ = run("docker-image",
                                   [docker, "image", "inspect", "ubuntu:22.04",
                                    "--format", "{{.Id}}"], timeout=30)
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
            executed, stdout, _ = run("linux-execute",
                [docker, "run", "--rm", "--pull", "never", "-i", "--network", "none",
                 "--read-only", "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
                 "--memory", "128m", "--pids-limit", "32", "--tmpfs", "/work:exec",
                 linux_image, "/bin/sh", "-c", shell],
                input=payload.getvalue(), timeout=300)
            expected = ["Linux aarch64"] + [
                "case " + name + " " + str(status)
                for name, (_, status) in linux_cases.items()
            ]
            record("linux-exact-output",
                   executed["passed"] and stdout.decode(errors="replace").splitlines() == expected)
        else:
            record("linux-execute", False, reason="local Linux image unavailable")
    elif args.linux_docker:
        record("linux-execute", False, reason="no generated Linux cases")

    record("runtime-source-immutable", hashes(runtime_root) == snapshots.get("runtime", hashes(runtime_root)))
    record("fixture-source-immutable", hashes(fixtures_root) == snapshots.get("fixtures", hashes(fixtures_root)))
    for name, snapshot in snapshots.items():
        if name.endswith("-compiler"):
            record(name + "-immutable", hashes(output / name) == snapshot)

    report = dict(
        schema="rey-v04-region-source-integration-v1",
        passed=all(item["passed"] for item in records),
        records=records,
        snapshots=snapshots,
        artifacts=artifacts,
        modes=args.modes,
        linux_image=linux_image,
        script_sha256=script_hash,
        seed_sha256=digest(SEED),
        runtime_sha256=digest(RUNTIME / "librey_rt.a"),
        code_complete_barrier=True,
        limitation=("ARM64 native correctness; conservative program-region placement for "
                    "unknown callees and container hooks; no x64/RSS/timing claim"),
    )
    (output / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print("region integration", "PASS" if report["passed"] else "FAIL",
          len(records), "checks", flush=True)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
