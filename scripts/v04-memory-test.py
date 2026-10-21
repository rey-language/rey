#!/usr/bin/env python3
"""Gate fixed-layout runtime source with an explicit historical seed."""
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

ROOT = Path(__file__).resolve().parents[1]
SEED = ROOT / "build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME = SEED.parent / "runtime"
SEED_HASH = "89f41b7cac5f38018c32b74d7a134d4d32e1314d58ddf292efcea174d182a921"
RUNTIME_HASH = "f3d625762a0ea9dfc403e81b5c3cc8cce49aaec64bd276e5b2dec95fad666667"
MODULES = {"fixed-heap": ("layout", "heap"), "fixed-gc": ("layout", "heap", "gc")}
NATIVE_ONLY = {("fixed-gc", "native-collection-stress-128")}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def hashes(folder):
    return {str(p.relative_to(folder)): digest(p) for p in sorted(folder.rglob("*.rey"))}


def main():
    script_hash = digest(Path(__file__))
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--suites", nargs="+", choices=MODULES, default=list(MODULES))
    parser.add_argument("--linux-docker", action="store_true")
    parser.add_argument("--reuse-slots", action="store_true")
    args = parser.parse_args()
    output = args.output_dir.resolve()
    if not output.is_relative_to(ROOT / "build/v04/memory-integration"):
        parser.error("output must be below build/v04/memory-integration")
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        parser.error("output must be empty")
    records, snapshots, artifacts, linux_cases = [], {}, {}, {}

    def record(label, passed, **fields):
        item = dict(label=label, passed=bool(passed), **fields)
        records.append(item)
        if not passed:
            print(label, "FAIL", flush=True)
        return item

    def run(label, command, expected=0, marker=b"", empty_stdout=False, timeout=120, **kwargs):
        try:
            result = subprocess.run(command, cwd=ROOT, capture_output=True, timeout=timeout, **kwargs)
            code, stdout, stderr, error = result.returncode, result.stdout, result.stderr, None
        except (OSError, subprocess.TimeoutExpired) as exc:
            code, stdout, stderr, error = None, b"", b"", str(exc)
        (output / (label + ".stdout")).write_bytes(stdout)
        (output / (label + ".stderr")).write_bytes(stderr)
        passed = code == expected and error is None and marker in stdout
        item = record(label, passed and (not empty_stdout or not stdout), command=command,
                      returncode=code, expected=expected, error=error,
                      stdout_sha256=hashlib.sha256(stdout).hexdigest(),
                      stderr_sha256=hashlib.sha256(stderr).hexdigest())
        return item, stdout

    seed_ok = SEED.is_file() and (RUNTIME / "librey_rt.a").is_file()
    seed_ok = seed_ok and digest(SEED) == SEED_HASH and digest(RUNTIME / "librey_rt.a") == RUNTIME_HASH
    record("pinned-seed-runtime", seed_ok)
    if not seed_ok:
        raise SystemExit("pinned seed/runtime missing or mismatched")
    version, stdout = run("seed-version", [str(SEED), "--version"])
    version["passed"] = version["passed"] and stdout.strip() == b"rey 0.3.1"
    seed_env = dict(os.environ, PATH="/usr/bin:/bin:/usr/sbin:/sbin")
    empty_path = output / "empty-path"
    empty_path.mkdir()
    native_env = dict(os.environ, PATH=str(empty_path))
    for key in ("CC", "CXX", "CLANG", "LD", "LLVM_CONFIG", "REY_RUNTIME_DIR"):
        native_env[key] = str(empty_path / "absent-tool")

    for suite in args.suites:
        project = output / suite
        compiler, fixtures, sources, images = [project / name for name in ("compiler", "fixtures", "sources", "images")]
        shutil.copytree(ROOT / "rey-compiler/src", compiler)
        shutil.copyfile(ROOT / "tests/v04" / suite / "main.rey", compiler / "main.rey")
        shutil.copytree(ROOT / "tests/v04" / suite / "fixtures", fixtures)
        cases_file = ROOT / "tests/v04" / suite / "cases.json"
        cases = json.loads(cases_file.read_text())
        if not isinstance(cases, dict) or not cases or not all(
            isinstance(name, str) and re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", name)
            and type(code) is int and code >= 0 for name, code in cases.items()
        ):
            raise ValueError("invalid case manifest for " + suite)
        record(suite + "-fixture-set", {p.name for p in fixtures.iterdir()} == {name + ".rey" for name in cases})
        for module in MODULES[suite]:
            (sources / module).mkdir(parents=True)
            shutil.copyfile(ROOT / "runtime-rey" / module / "main.rey", sources / module / "main.rey")
        images.mkdir()
        snapshots[suite] = dict(compiler=hashes(compiler), runtime=hashes(sources), fixtures=hashes(fixtures),
                                cases=cases, cases_sha256=digest(cases_file))
        (project / "cases.json").write_bytes(cases_file.read_bytes())
        executable = project / "fixture"
        built, _ = run(suite + "-build", [str(SEED), "build", str(compiler / "main.rey"), "-o", str(executable),
                                         "--runtime-dir", str(RUNTIME)], env=seed_env)
        if not built["passed"]:
            continue
        for case, code in cases.items():
            label = suite + "-" + case
            marker = (suite + "-case " + case + " " + str(code)).encode()
            command = [str(executable), str(fixtures), str(sources), str(images), case]
            if args.reuse_slots:
                command.append("--reuse-slots")
            ran, stdout = run(label + "-source", command,
                              marker=marker, env=native_env)
            expected_line = marker.decode()
            manifest_lines = [line for line in stdout.decode(errors="replace").splitlines() if line.startswith(suite + "-case ")]
            record(label + "-manifest", ran["passed"] and manifest_lines == [expected_line])
            entry = sources / "main.rey"
            record(label + "-source-snapshot", entry.is_file() and digest(entry) == digest(fixtures / (case + ".rey")))
            steps_file = images / (case + ".steps")
            native_only = (suite, case) in NATIVE_ONLY
            evaluation_file = images / (case + ".evaluation")
            if native_only:
                record(label + "-native-only-evaluation-policy",
                       evaluation_file.is_file() and evaluation_file.read_text() == "native-only\n",
                       evaluator_executed=False,
                       reason="128 forced-collection allocations exceed the historical-seed evaluator work/memory bound; native assertions remain required")
            elif evaluation_file.is_file():
                record(label + "-unexpected-evaluator-skip", False)
            if steps_file.is_file():
                steps_text = steps_file.read_text().strip()
                steps_ok = steps_text == "0" if native_only else steps_text.isdigit() and int(steps_text) > 0
                record(label + "-evaluator-steps", steps_ok,
                       steps=int(steps_text) if steps_text.isdigit() else None, unit="IR evaluator steps; not timing")
            elif native_only:
                record(label + "-evaluator-steps", False)
            expected_status = int(code != 0)
            for target in ("darwin", "linux"):
                image = images / (target + "-" + case)
                data = image.read_bytes() if image.is_file() else b""
                if target == "darwin":
                    header_ok = len(data) >= 32 and data[:4] == bytes.fromhex("cffaedfe")
                    header_ok = header_ok and struct.unpack_from("<I", data, 4)[0] == 16777228
                else:
                    header_ok = len(data) >= 64 and data[:7] == b"\x7fELF\x02\x01\x01"
                    header_ok = header_ok and struct.unpack_from("<HH", data, 16) == (2, 183)
                record(label + "-" + target + "-image", ran["passed"] and header_ok)
                if not image.is_file():
                    continue
                artifacts[label + "-" + target] = dict(sha256=digest(image), expected_status=expected_status,
                                                        source_sha256=digest(fixtures / (case + ".rey")))
                if target == "linux":
                    linux_cases[label] = (image, expected_status)
                elif platform.system() == "Darwin" and platform.machine() == "arm64":
                    image.chmod(0o755)
                    run(label + "-signature", ["/usr/bin/codesign", "--verify", "--strict", str(image)], timeout=10)
                    run(label + "-darwin-execute", [str(image)], expected=expected_status, empty_stdout=True, timeout=30)
                else:
                    record(label + "-darwin-execute", False, reason="Darwin ARM64 host required")
        expected_images = {target + "-" + name for name in cases for target in ("darwin", "linux")}
        actual_images = {p.name for p in images.iterdir()}
        optional_metadata = {name + extension for name in cases for extension in (".ir", ".steps")}
        optional_metadata |= {name + ".evaluation" for name in cases if (suite, name) in NATIVE_ONLY}
        record(suite + "-image-set", expected_images <= actual_images and actual_images <= expected_images | optional_metadata)
        runtime_after = {str(p.relative_to(sources)): digest(p) for module in MODULES[suite] for p in (sources / module).rglob("*.rey")}
        record(suite + "-runtime-immutable", runtime_after == snapshots[suite]["runtime"])

    linux_image = None
    if args.linux_docker:
        docker = shutil.which("docker") or "docker"
        inspected, stdout = run("linux-image-inspect", [docker, "image", "inspect", "ubuntu:22.04", "--format", "{{.Id}}"])
        linux_image = stdout.decode(errors="replace").strip()
        if inspected["passed"] and linux_image.startswith("sha256:") and linux_cases:
            payload = io.BytesIO()
            with tarfile.open(fileobj=payload, mode="w") as archive:
                for name, (path, _) in linux_cases.items():
                    data = path.read_bytes()
                    entry = tarfile.TarInfo(name)
                    entry.size, entry.mode = len(data), 0o700
                    archive.addfile(entry, io.BytesIO(data))
            shell = "tar -xf - -C /work || exit 90; uname -sm; "
            for name in linux_cases:
                shell += "/work/" + name + "; status=$?; printf 'case " + name + " %s\\n' \"$status\"; "
            executed, stdout = run("linux-execute", [docker, "run", "--rm", "--pull", "never", "-i", "--network", "none",
                "--read-only", "--cap-drop", "ALL", "--security-opt", "no-new-privileges", "--memory", "128m",
                "--pids-limit", "32", "--tmpfs", "/work:exec", linux_image, "/bin/sh", "-c", shell],
                input=payload.getvalue(), timeout=120)
            expected_lines = ["Linux aarch64"] + ["case " + name + " " + str(status) for name, (_, status) in linux_cases.items()]
            record("linux-exact-output", executed["passed"] and stdout.decode(errors="replace").splitlines() == expected_lines)
        else:
            record("linux-execution", False, reason="missing local image or generated programs")
    report = dict(schema="rey-v04-memory-integration-v1", records=records, snapshots=snapshots, artifacts=artifacts,
                  seed_sha256=digest(SEED), runtime_sha256=digest(RUNTIME / "librey_rt.a"), script_sha256=script_hash,
                  source_execution_environment={key: native_env[key] for key in ("PATH", "CC", "CXX", "CLANG", "LD", "LLVM_CONFIG", "REY_RUNTIME_DIR")},
                  linux_requested=args.linux_docker, linux_image=linux_image, reuse_slots=args.reuse_slots,
                  passed=all(r["passed"] for r in records),
                  native_only_cases=[suite + "/" + case for suite in args.suites for name, case in NATIVE_ONLY if name == suite],
                  limitation="fixed layouts and explicit construction roots; one labeled 128-allocation forced-collection case is native-only; no automatic compiler roots or RSS claim")
    (output / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print("memory integration", "PASS" if report["passed"] else "FAIL", len(records), "checks", flush=True)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
