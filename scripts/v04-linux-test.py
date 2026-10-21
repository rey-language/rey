#!/usr/bin/env python3
"""Run direct Rey-emitted Linux executables in an existing Docker Linux VM."""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import time

ROOT = Path(__file__).resolve().parents[1]
CASES = {"native-branch": 0, "native-loops": 0, "native-arithmetic": 0,
         "overflow": 1, "native-mul-overflow": 1, "native-div-zero": 1,
         "native-rem-overflow": 1, "native-neg-overflow": 1, "native-graph": 0}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--driver", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--target", choices=["linux-arm64", "linux-x64"], default="linux-arm64")
    parser.add_argument("--image", default="ubuntu:22.04", help="must already be present; never pulls")
    parser.add_argument("--allow-emulation", action="store_true", help="permit x64 execution in an ARM64 Linux VM; does not establish a native x64 gate")
    parser.add_argument("--oracle-report", type=Path, help="also execute the exact generated sources/expectations of a passing arithmetic oracle report")
    args = parser.parse_args()
    driver = args.driver.resolve(strict=True)
    output = args.output_dir.resolve()
    if not output.is_relative_to(ROOT / "build/v04"):
        parser.error("output must be under build/v04")
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        parser.error("output directory must be empty")
    docker = shutil.which("docker")
    if not docker:
        parser.error("Docker executable required")
    records = []
    cases = dict(CASES)
    sources = {name: ROOT / "tests/v04/pipeline" / ("graphs/main.rey" if name == "native-graph" else name + ".rey") for name in cases}
    oracle_sha256 = None
    if args.oracle_report:
        oracle_path = args.oracle_report.resolve(strict=True)
        oracle = json.loads(oracle_path.read_text())
        if not oracle.get("passed"):
            parser.error("arithmetic oracle report must have passed")
        oracle_sha256 = digest(oracle_path)
        for item in oracle["programs"]:
            name = "oracle-" + item["name"]
            source = (ROOT / item["source_path"]).resolve(strict=True)
            if not re.fullmatch("[a-z0-9-]+", name) or name in cases or not source.is_relative_to(ROOT / "build/v04"):
                parser.error("invalid or duplicate oracle source identity")
            if digest(source) != item["source_sha256"] or item["expected_status"] not in (0, 1):
                parser.error("oracle source hash or expectation mismatch")
            cases[name], sources[name] = item["expected_status"], source
    source_hashes = {str(p.relative_to(ROOT)): digest(p) for p in sorted((ROOT / "tests/v04/pipeline").rglob("*.rey"))}

    def run(label, command, expected=0, **kwargs):
        started = time.monotonic()
        try:
            result = subprocess.run(command, cwd=ROOT, capture_output=True, timeout=60, **kwargs)
            code, stdout, stderr, error = result.returncode, result.stdout, result.stderr, None
        except (OSError, subprocess.TimeoutExpired) as exc:
            code, stdout, stderr, error = None, b"", b"", str(exc)
        (output / (label + ".stdout")).write_bytes(stdout)
        (output / (label + ".stderr")).write_bytes(stderr)
        record = dict(label=label, returncode=code, expected=expected, passed=code == expected and error is None,
                      error=error, seconds=time.monotonic() - started, command=command)
        records.append(record)
        print(label, "PASS" if record["passed"] else "FAIL", flush=True)
        return record, stdout

    inspection, raw_image = run("image-inspect", [docker, "image", "inspect", args.image, "--format", "{{.Id}}"])
    image_id = raw_image.decode().strip()
    if not inspection["passed"] or not image_id.startswith("sha256:"):
        image_id = None
    empty = output / "empty-path"
    empty.mkdir()
    env = dict(os.environ, PATH=str(empty))
    for key in ("CC", "CXX", "CLANG", "LD", "LLVM_CONFIG", "REY_RUNTIME_DIR"):
        env[key] = str(empty / "absent")
    artifacts = {}
    for name, expected in cases.items():
        source = sources[name]
        artifact = output / (name + ".elf")
        emitted, _ = run(name + "-emit", [str(driver), "build", str(source), "-o", str(artifact), "--target", args.target], env=env)
        machine = b"\xb7\x00" if args.target == "linux-arm64" else b"\x3e\x00"
        valid = emitted["passed"] and artifact.is_file() and artifact.read_bytes()[:6] == b"\x7fELF\x02\x01" and artifact.read_bytes()[18:20] == machine
        records.append(dict(label=name + "-image", passed=valid))
        if valid:
            artifacts[name] = dict(source_sha256=digest(source), image_sha256=digest(artifact), expected=expected)
    execution_mode = "not executed"
    if image_id and len(artifacts) == len(cases):
        payload = io.BytesIO()
        with tarfile.open(fileobj=payload, mode="w") as archive:
            for name in cases:
                data = (output / (name + ".elf")).read_bytes()
                entry = tarfile.TarInfo(name)
                entry.size, entry.mode = len(data), 0o700
                archive.addfile(entry, io.BytesIO(data))
        shell = "tar -xf - -C /work || exit 90; uname -sm; "
        for name in cases:
            shell += "/work/" + name + "; status=$?; printf 'case " + name + " %s\\n' \"$status\"; "
        execution, stdout = run("linux-execute", [docker, "run", "--rm", "--pull", "never", "-i", "--network", "none",
            "--read-only", "--cap-drop", "ALL", "--security-opt", "no-new-privileges", "--memory", "128m",
            "--pids-limit", "32", "--tmpfs", "/work:exec", image_id, "/bin/sh", "-c", shell], input=payload.getvalue())
        lines = stdout.decode(errors="replace").splitlines()
        architecture = "aarch64" if args.target == "linux-arm64" else "x86_64"
        native_host = bool(lines) and lines[0] == "Linux " + architecture
        emulated = bool(lines) and args.allow_emulation and args.target == "linux-x64" and lines[0] == "Linux aarch64"
        execution_mode = "x64 emulation on Linux aarch64" if emulated else "matching Linux VM architecture"
        records.append(dict(label="linux-host", passed=native_host or emulated,
                            observed=lines[0] if lines else None))
        observed = {}
        for line in lines[1:]:
            fields = line.split()
            if len(fields) == 3 and fields[0] == "case" and fields[2].isdigit():
                observed.setdefault(fields[1], []).append(int(fields[2]))
        records.append(dict(label="linux-case-set", passed=set(observed) == set(cases)))
        for name, expected in cases.items():
            records.append(dict(label=name + "-execute", passed=observed.get(name) == [expected],
                                expected=expected, observed=observed.get(name)))
    else:
        records.append(dict(label="linux-execution", passed=False, reason="image inspection or generation failed"))
    report = dict(driver=str(driver), driver_sha256=digest(driver), target=args.target,
                  image=image_id, execution_mode=execution_mode, oracle_report_sha256=oracle_sha256, source_hashes=source_hashes, cases=artifacts, records=records, passed=all(r["passed"] for r in records),
                  limitation="Linux VM execution; this gate does not benchmark performance or prove bare-metal x64 support")
    (output / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
