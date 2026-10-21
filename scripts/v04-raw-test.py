#!/usr/bin/env python3
"""Gate trusted raw primitives, source authority and actual mapped-memory access."""
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

ROOT = Path(__file__).resolve().parents[1]
NORMAL = "zero-initialized i64-min i64-max byte-zero byte-127 byte-128 byte-255 offset-access null-equal null-not-mapping-equal same-mapping-equal distinct-mappings-not-equal separate-mappings raw-call-block-param address-cell-null address-cell-roundtrip".split()
ERRORS = "map-size-zero map-size-negative map-size-too-large offset-null offset-negative offset-overflow load-i64-unaligned store-i64-unaligned store-byte-negative store-byte-overflow unmap-size-zero unmap-syscall-failure load-address-unaligned store-address-unaligned".split()
EXPECTED = {**dict.fromkeys(NORMAL, 2), **dict.fromkeys(ERRORS, 1)}
FAILURE_CASES = {"fail-zero": 1, "fail-17": 1, "fail-negative": 1, "fail-max": 1, "fail-before-99": 1, "conditional-skip": 0}
BUFFER_CASES = {"valid": 0, **dict.fromkeys("negative-create negative-grow negative-length length-over-capacity corrupt-magic corrupt-length corrupt-reserved corrupt-null-data".split(), 1)}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--runtime-dir", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--linux-docker", action="store_true")
    args = parser.parse_args()
    output = args.output_dir.resolve()
    if not output.is_relative_to(ROOT / "build/v04"):
        parser.error("output must be under build/v04")
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        parser.error("output must be empty")
    engine, runtime = args.engine.resolve(strict=True), args.runtime_dir.resolve(strict=True)
    records, snapshots, artifacts = [], {}, {}

    def run(label, command, expected=0, marker="", timeout=120, **kwargs):
        try:
            result = subprocess.run(command, cwd=ROOT, capture_output=True, timeout=timeout, **kwargs)
            code, stdout, stderr, error = result.returncode, result.stdout, result.stderr, None
        except (OSError, subprocess.TimeoutExpired) as exc:
            code, stdout, stderr, error = None, b"", b"", str(exc)
        (output / (label + ".stdout")).write_bytes(stdout)
        (output / (label + ".stderr")).write_bytes(stderr)
        record = dict(label=label, command=command, returncode=code, expected=expected, error=error,
                      passed=code == expected and error is None and marker in (stdout + stderr).decode(errors="replace"))
        records.append(record)
        print(label, "PASS" if record["passed"] else "FAIL", flush=True)
        return record, stdout

    version, stdout = run("version", [str(engine), "--version"])
    version["passed"] = version["passed"] and stdout.strip() == b"rey 0.3.1"
    empty_path = output / "empty-path"
    empty_path.mkdir()
    env = dict(os.environ, PATH=str(empty_path))
    for name in ("CC", "CXX", "CLANG", "LD", "LLVM_CONFIG", "REY_RUNTIME_DIR"):
        env[name] = str(empty_path / "absent")
    markers = {"raw-core": "raw IR core fixtures passed", "raw-checker": "v0.4 raw checker fixtures passed",
               "raw-ir": "v04 raw IR verifier, evaluator, and effects passed", "arm64-raw": "",
               "raw-pipeline": "raw source pipeline fixtures ready",
               "raw-fail": "raw fail source, IR, evaluator, and ARM64 fixtures passed",
               "raw-buffer": "raw byte buffer source fixtures ready"}
    linux_cases = {}
    for name, marker in markers.items():
        project = output / name
        shutil.copytree(ROOT / "rey-compiler/src", project / "src")
        shutil.copyfile(ROOT / "tests/v04" / name / "main.rey", project / "src/main.rey")
        snapshots[name] = {str(p.relative_to(project)): digest(p) for p in sorted((project / "src").rglob("*.rey"))}
        executable = project / "fixture"
        built, _ = run(name + "-build", [str(engine), "build", str(project / "src/main.rey"), "-o", str(executable), "--runtime-dir", str(runtime)])
        if not built["passed"]:
            continue
        command = [str(executable)]
        if name in ("arm64-raw", "raw-pipeline", "raw-fail", "raw-buffer"):
            image_dir = project / "images"
            image_dir.mkdir()
            if name == "raw-pipeline":
                fixtures = project / "fixtures"
                shutil.copytree(ROOT / "tests/v04/raw-pipeline/fixtures", fixtures)
                snapshots[name].update({str(p.relative_to(project)): digest(p) for p in sorted(fixtures.rglob("*.rey"))})
                command.append(str(fixtures))
            if name == "raw-buffer":
                fixtures, sources = project / "fixtures", project / "sources"
                shutil.copytree(ROOT / "tests/v04/raw-buffer/fixtures", fixtures)
                (sources / "buffer").mkdir(parents=True)
                shutil.copyfile(ROOT / "runtime-rey/buffer/main.rey", sources / "buffer/main.rey")
                snapshots[name].update({str(p.relative_to(project)): digest(p) for folder in (fixtures, sources) for p in sorted(folder.rglob("*.rey"))})
                command += [str(fixtures), str(sources)]
            command.append(str(image_dir))
        ran, stdout = run(name + "-run", command, marker=marker, env=env)
        if not ran["passed"] or name not in ("arm64-raw", "raw-pipeline", "raw-fail", "raw-buffer"):
            continue
        if name == "arm64-raw":
            parsed = [(key.decode(), int(status)) for key, status in re.findall(rb"^native-raw-case ([a-z0-9-]+) (\d+)$", stdout, re.M)]
            records.append(dict(label="raw-manifest", passed=len(parsed) == len(EXPECTED) and len(dict(parsed)) == len(parsed) and dict(parsed) == EXPECTED))
            expected_files = {os_name + "-" + case for case in EXPECTED for os_name in ("darwin", "linux")}
            records.append(dict(label="raw-image-set", passed={p.name for p in image_dir.iterdir()} == expected_files and all(p.is_file() for p in image_dir.iterdir())))
            images = [(case, image_dir / ("darwin-" + case), image_dir / ("linux-" + case), status) for case, status in EXPECTED.items()]
        elif name in ("raw-fail", "raw-buffer"):
            cases = FAILURE_CASES if name == "raw-fail" else BUFFER_CASES
            prefix = b"raw-fail-case" if name == "raw-fail" else b"native-raw-buffer-case"
            parsed = [(key.decode(), int(status)) for key, status in re.findall(rb"^" + prefix + rb" ([a-z0-9-]+) (\d+)$", stdout, re.M)]
            records.append(dict(label=name + "-manifest", passed=len(parsed) == len(cases) and len(dict(parsed)) == len(parsed) and dict(parsed) == cases))
            expected_files = {os_name + "-" + case for case in cases for os_name in ("darwin", "linux")}
            if name == "raw-buffer":
                expected_files |= {case + ".ir" for case in cases}
            actual_files = {p.name for p in image_dir.iterdir() if p.is_file()}
            records.append(dict(label=name + "-image-set", passed=actual_files == expected_files))
            snapshots[name].update({str(p.relative_to(project)): digest(p) for p in sorted(project.rglob("*.rey"))})
            images = [(name + "-" + case, image_dir / ("darwin-" + case), image_dir / ("linux-" + case), status) for case, status in cases.items()]
        else:
            images = [("source-" + case, image_dir / (case + ".macho"), image_dir / (case + ".elf"), status) for case, status in {"memory": 0, "bad-byte": 1}.items()]
        for case, darwin, linux, status in images:
            exists = darwin.is_file() and linux.is_file()
            records.append(dict(label=case + "-images", passed=exists))
            if not exists:
                continue
            artifacts[case] = dict(darwin_sha256=digest(darwin), linux_sha256=digest(linux), expected=status)
            correct_format = darwin.read_bytes()[:8] == bytes.fromhex("cffaedfe0c000001") and linux.read_bytes()[:6] == b"\x7fELF\x02\x01" and linux.read_bytes()[18:20] == b"\xb7\x00"
            records.append(dict(label=case + "-formats", passed=correct_format))
            linux_cases[case] = (linux, status)
            if platform.system() == "Darwin" and platform.machine() == "arm64":
                darwin.chmod(0o755)
                execution, target_stdout = run(case + "-darwin-execute", [str(darwin)], expected=status, timeout=10)
                execution["passed"] = execution["passed"] and target_stdout == b""
                run(case + "-signature", ["/usr/bin/codesign", "--verify", "--strict", str(darwin)], timeout=10)
            else:
                records.append(dict(label=case + "-darwin-execute", passed=False, reason="Darwin arm64 host required"))
    if args.linux_docker:
        docker = shutil.which("docker")
        inspected, stdout = run("linux-image-inspect", [docker or "docker", "image", "inspect", "ubuntu:22.04", "--format", "{{.Id}}"], timeout=30)
        image_id = stdout.decode().strip()
        if inspected["passed"] and image_id.startswith("sha256:") and len(linux_cases) == 47:
            payload = io.BytesIO()
            with tarfile.open(fileobj=payload, mode="w") as archive:
                for name, (image, status) in linux_cases.items():
                    data = image.read_bytes()
                    entry = tarfile.TarInfo(name)
                    entry.size, entry.mode = len(data), 0o700
                    archive.addfile(entry, io.BytesIO(data))
            shell = "tar -xf - -C /work || exit 90; uname -sm; "
            for name in linux_cases:
                shell += "/work/" + name + "; status=$?; printf 'case " + name + " %s\\n' \"$status\"; "
            ran, stdout = run("linux-execute", [docker, "run", "--rm", "--pull", "never", "-i", "--network", "none", "--read-only",
                "--cap-drop", "ALL", "--security-opt", "no-new-privileges", "--memory", "128m", "--pids-limit", "32",
                "--tmpfs", "/work:exec", image_id, "/bin/sh", "-c", shell], input=payload.getvalue(), timeout=60)
            lines = stdout.decode(errors="replace").splitlines()
            records.append(dict(label="linux-host", passed=bool(lines) and lines[0] == "Linux aarch64"))
            records.append(dict(label="linux-output-lines", passed=len(lines) == 1 + len(linux_cases)))
            observed = {}
            for line in lines[1:]:
                fields = line.split()
                if len(fields) == 3 and fields[0] == "case" and fields[2].isdigit():
                    observed.setdefault(fields[1], []).append(int(fields[2]))
            records.append(dict(label="linux-case-set", passed=set(observed) == set(linux_cases)))
            for name, (image, status) in linux_cases.items():
                records.append(dict(label=name + "-linux-execute", passed=observed.get(name) == [status], observed=observed.get(name), expected=status))
        else:
            records.append(dict(label="linux-execution", passed=False, reason="missing Docker image or generated raw cases"))
    report = dict(engine_sha256=digest(engine), runtime_sha256=digest(runtime / "librey_rt.a"), snapshots=snapshots,
                  script_sha256=digest(Path(__file__)), artifacts=artifacts, records=records,
                  passed=all(record["passed"] for record in records), linux_requested=args.linux_docker,
                  limitation="trusted raw primitives with manual ownership; no managed collector or inferred regions")
    (output / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
