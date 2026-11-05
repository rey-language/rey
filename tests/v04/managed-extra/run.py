#!/usr/bin/env python3
"""Run focused managed generic-storage fixtures against frozen v0.4 drivers."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[3]
MODES = {
    "heap": ["--no-regions"],
    "placed": [],
    "stress-heap": ["--gc-stress", "--no-regions"],
    "stress-placed": ["--gc-stress"],
    "stress-optimized": ["--gc-stress", "--fold-constants", "--reuse-slots"],
}
FIXTURES = (
    "vec-nullable-node-literal.rey", "hashmap-nullable-node-literal.rey",
    "hashmap-node-identity.rey", "hashmap-nullable-node.rey", "vec-bool-set-pop.rey",
    "vec-char-byte-255.rey", "hashmap-char-get.rey", "vec-nullable-node.rey",
)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def hashes(folder):
    return {str(path.relative_to(folder)): digest(path)
            for path in sorted(folder.rglob("*")) if path.is_file()}


def native_header(data, target):
    if target == "darwin-arm64":
        return len(data) >= 32 and data[:4] == bytes.fromhex("cffaedfe") and struct.unpack_from("<I", data, 4)[0] == 16777228
    return len(data) >= 64 and data[:7] == b"\x7fELF\x02\x01\x01" and struct.unpack_from("<HH", data, 16) == (2, 183)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--drivers", type=Path, required=True,
                        help="frozen v0.4 driver snapshot from the main managed test run")
    parser.add_argument("--linux-docker", action="store_true")
    args = parser.parse_args()
    driver_root = args.drivers.resolve()
    output = args.output_dir.resolve()
    if not output.is_relative_to(ROOT / "build/v04/managed-extra"):
        parser.error("output must be under build/v04/managed-extra")
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        parser.error("output directory must be empty")

    native = driver_root / "native-driver"
    ir = driver_root / "ir-driver"
    teardown = driver_root / "teardown-driver"
    runtime = driver_root / "runtime-rey"
    for path in (native, ir, teardown):
        if not path.is_file():
            raise SystemExit("missing frozen driver: " + str(path))
    if not (runtime / "managed/main.rey").is_file() or not (runtime / "gc/main.rey").is_file():
        raise SystemExit("frozen driver runtime source is incomplete")

    fixture_root = output / "fixtures"
    shutil.copytree(ROOT / "tests/v04/managed-extra/fixtures", fixture_root)
    fixture_hashes = hashes(fixture_root)
    if set(fixture_hashes) != set(FIXTURES):
        raise SystemExit("fixture list differs from the explicit eight-case manifest")
    empty_path = output / "empty-path"
    empty_path.mkdir()
    env = dict(os.environ, PATH=str(empty_path))
    for key in ("CC", "CXX", "CLANG", "LD", "LLVM_CONFIG", "REY_RUNTIME_DIR"):
        env[key] = str(empty_path / "absent")
    images = output / "images"
    images.mkdir()
    records, artifacts, linux_cases, step_counts = [], {}, {}, {}

    def record(label, passed, **fields):
        item = dict(label=label, passed=bool(passed), **fields)
        records.append(item)
        print(label, "PASS" if passed else "FAIL", flush=True)
        return item

    def run(label, command, expected=0, *, input=None, timeout=180, empty=False):
        try:
            result = subprocess.run(command, cwd=ROOT, env=env, input=input,
                                    capture_output=True, timeout=timeout)
            status, stdout, stderr, error = result.returncode, result.stdout, result.stderr, None
        except (OSError, subprocess.TimeoutExpired) as exc:
            status, stdout, stderr, error = None, b"", b"", str(exc)
        (output / (label + ".stdout")).write_bytes(stdout)
        (output / (label + ".stderr")).write_bytes(stderr)
        passed = status == expected and error is None and (not empty or stdout == b"")
        record(label, passed, command=command, status=status, expected=expected, error=error,
               stdout_sha256=hashlib.sha256(stdout).hexdigest(),
               stderr_sha256=hashlib.sha256(stderr).hexdigest())
        return passed, stdout, stderr

    driver_snapshot = {
        "driver_root": str(driver_root),
        "native_driver_sha256": digest(native),
        "ir_driver_sha256": digest(ir),
        "teardown_driver_sha256": digest(teardown),
        "native_compiler_sources": hashes(driver_root / "native-compiler"),
        "ir_compiler_sources": hashes(driver_root / "ir-compiler"),
        "teardown_compiler_sources": hashes(driver_root / "teardown-compiler"),
        "runtime_sources": hashes(runtime),
        "teardown_harness_source_sha256": digest(ROOT / "tests/v04/managed-teardown/main.rey"),
    }
    record("frozen-driver-snapshots", True, native_driver_sha256=driver_snapshot["native_driver_sha256"],
           ir_driver_sha256=driver_snapshot["ir_driver_sha256"],
           teardown_driver_sha256=driver_snapshot["teardown_driver_sha256"])

    base_options = ["--managed", "--runtime-root", str(runtime)]
    for fixture in FIXTURES:
        source = fixture_root / fixture
        source_hash = digest(source)
        source_id = fixture.removesuffix(".rey")
        checked, _, _ = run(source_id + "-check", [str(ir), "check", str(source)] + base_options)
        for mode, flags in MODES.items():
            prefix = images / (source_id + "-" + mode + "-teardown")
            interpreted, stdout, _ = run(source_id + "-" + mode + "-teardown",
                [str(teardown), str(source), str(runtime), str(prefix), mode])
            record(source_id + "-" + mode + "-teardown-marker",
                   interpreted and stdout == b"managed-teardown: verified\n")
            step_path = Path(str(prefix) + ".steps")
            policy_path = Path(str(prefix) + ".evaluation")
            step_text = step_path.read_text().strip() if step_path.is_file() else ""
            policy = policy_path.read_text() if policy_path.is_file() else ""
            valid_steps = step_text.isdigit() and 0 < int(step_text) <= 4000000 and policy == "interpreted\n"
            step_counts[source_id + "/" + mode] = int(step_text) if step_text.isdigit() else None
            record(source_id + "-" + mode + "-interpreted-4m-teardown", interpreted and valid_steps,
                   steps=step_counts[source_id + "/" + mode], evaluator_policy=policy.strip(), cap_steps=4000000)

            for target, image in (("darwin-arm64", Path(str(prefix) + ".darwin")),
                                  ("linux-arm64", Path(str(prefix) + ".linux"))):
                data = image.read_bytes() if image.is_file() else b""
                name = source_id + "-" + mode + "-teardown-" + target
                valid = interpreted and valid_steps and native_header(data, target)
                record(name + "-image", valid)
                if not valid:
                    continue
                artifacts[name] = dict(sha256=digest(image), source_sha256=source_hash,
                                       role="teardown", expected_status=0)
                if target == "linux-arm64":
                    linux_cases[name] = (image, 0)
                else:
                    image.chmod(0o755)
                    run(name + "-signature", ["/usr/bin/codesign", "--verify", "--strict", str(image)], timeout=10)
                    run(name + "-execute", [str(image)], empty=True, timeout=30)

            for target in ("darwin-arm64", "linux-arm64"):
                name = source_id + "-" + mode + "-base-" + target
                image = images / name
                command = [str(native), "build", str(source), "-o", str(image), "--target", target] + base_options + flags
                emitted, _, _ = run(name + "-emit", command)
                data = image.read_bytes() if image.is_file() else b""
                valid = emitted and checked and native_header(data, target)
                record(name + "-image", valid)
                if not valid:
                    continue
                artifacts[name] = dict(sha256=digest(image), source_sha256=source_hash,
                                       role="base", expected_status=0)
                if target == "linux-arm64":
                    linux_cases[name] = (image, 0)
                else:
                    image.chmod(0o755)
                    run(name + "-signature", ["/usr/bin/codesign", "--verify", "--strict", str(image)], timeout=10)
                    run(name + "-execute", [str(image)], empty=True, timeout=30)

    expected_artifacts = len(FIXTURES) * len(MODES) * 4
    record("complete-artifact-set", len(artifacts) == expected_artifacts,
           expected=expected_artifacts, actual=len(artifacts))
    expected_steps = len(FIXTURES) * len(MODES)
    record("teardown-step-count-set", len(step_counts) == expected_steps
           and all(isinstance(value, int) and 0 < value <= 4000000 for value in step_counts.values()),
           expected=expected_steps, actual=len(step_counts))

    linux_image = None
    if args.linux_docker:
        docker = shutil.which("docker") or "docker"
        inspected, stdout, _ = run("linux-image-inspect", [docker, "image", "inspect", "ubuntu:22.04", "--format", "{{.Id}}"], timeout=30)
        linux_image = stdout.decode(errors="replace").strip()
        expected_linux = expected_artifacts // 2
        if inspected and linux_image.startswith("sha256:") and len(linux_cases) == expected_linux:
            payload = io.BytesIO()
            with tarfile.open(fileobj=payload, mode="w") as archive:
                for name, (image, _) in linux_cases.items():
                    data = image.read_bytes()
                    entry = tarfile.TarInfo(name)
                    entry.size, entry.mode = len(data), 0o700
                    archive.addfile(entry, io.BytesIO(data))
            shell = "tar -xf - -C /work || exit 90; uname -sm; "
            for name in linux_cases:
                shell += "/work/" + name + "; status=$?; printf 'case " + name + " %s\\n' \"$status\"; "
            executed, stdout, _ = run("linux-execute", [docker, "run", "--rm", "--pull", "never", "-i",
                "--network", "none", "--read-only", "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
                "--memory", "128m", "--pids-limit", "32", "--tmpfs", "/work:exec", linux_image,
                "/bin/sh", "-c", shell], input=payload.getvalue(), timeout=180)
            expected_lines = ["Linux aarch64"] + ["case " + name + " " + str(status)
                                                    for name, (_, status) in linux_cases.items()]
            record("linux-actual-batch-statuses", executed and stdout.decode(errors="replace").splitlines() == expected_lines,
                   image=linux_image, expected_cases=len(linux_cases),
                   observed_lines=len(stdout.decode(errors="replace").splitlines()))
        else:
            record("linux-actual-batch-statuses", False, reason="Docker image or complete Linux image set missing",
                   image=linux_image, expected_cases=expected_linux, actual_cases=len(linux_cases))

    report = dict(schema="rey-v04-managed-extra-v1", passed=all(item["passed"] for item in records),
        records=records, fixtures=fixture_hashes, driver_snapshot=driver_snapshot,
        artifacts=artifacts, interpreted_steps=step_counts, modes=list(MODES),
        linux_docker_requested=args.linux_docker, linux_image=linux_image,
        expected_artifacts=expected_artifacts, script_sha256=digest(Path(__file__)),
        limitation="native ARM64 behavior; no native x64 or timing claim")
    (output / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print("managed-extra", "PASS" if report["passed"] else "FAIL", len(records), "checks", flush=True)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
