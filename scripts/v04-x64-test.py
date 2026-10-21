#!/usr/bin/env python3
"""Build frozen x64-sector snapshots and independently inspect the images."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import shutil
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--runtime-dir", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--base-ref", help="optional frozen compiler base; overlay only x64 modules and driver")
    args = parser.parse_args()
    output = args.output_dir.resolve()
    if not output.is_relative_to(ROOT / "build/v04"):
        parser.error("output must be under build/v04")
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        parser.error("output must be empty")
    engine, runtime = args.engine.resolve(strict=True), args.runtime_dir.resolve(strict=True)
    base = None
    base_commit = None
    if args.base_ref:
        base_commit = subprocess.run(["git", "rev-parse", "--verify", args.base_ref + "^{commit}"], cwd=ROOT,
                                     check=True, capture_output=True, timeout=10).stdout.decode().strip()
        base = subprocess.run(["git", "archive", base_commit, "rey-compiler/src"], cwd=ROOT,
                              check=True, capture_output=True, timeout=10).stdout
    records, snapshots = [], {}

    def run(label, command, expected=0, marker=""):
        try:
            result = subprocess.run(command, cwd=ROOT, capture_output=True, timeout=120)
            code, stdout, stderr, error = result.returncode, result.stdout, result.stderr, None
        except (OSError, subprocess.TimeoutExpired) as exc:
            code, stdout, stderr, error = None, b"", b"", str(exc)
        (output / (label + ".stdout")).write_bytes(stdout)
        (output / (label + ".stderr")).write_bytes(stderr)
        record = dict(label=label, command=command, returncode=code, expected=expected, error=error,
                      passed=code == expected and error is None and marker in (stdout + stderr).decode(errors="replace"))
        records.append(record)
        print(label, "PASS" if record["passed"] else "FAIL", flush=True)
        return record["passed"]

    run("version", [str(engine), "--version"], marker="rey 0.3.1")
    mains = {"image": ROOT / "tests/v04/x64-image/main.rey", "emitter": ROOT / "tests/v04/x64/main.rey",
             "driver": ROOT / "rey-compiler/native-main.rey"}
    for name, main_source in mains.items():
        src = output / name / "src"
        if base is None:
            shutil.copytree(ROOT / "rey-compiler/src", src)
        else:
            stage = output / name / "base"
            stage.mkdir(parents=True)
            with tarfile.open(fileobj=io.BytesIO(base)) as archive:
                archive.extractall(stage, filter="data")
            shutil.copytree(stage / "rey-compiler/src", src)
            for relative in ("native/x64/main.rey", "native/image/x64/main.rey"):
                destination = src / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(ROOT / "rey-compiler/src" / relative, destination)
        shutil.copyfile(main_source, src / "main.rey")
        snapshots[name] = {str(p.relative_to(src)): digest(p) for p in sorted(src.rglob("*.rey"))}
    for name in mains:
        project = output / name
        executable = project / "fixture"
        if not run(name + "-build", [str(engine), "build", str(project / "src/main.rey"), "-o", str(executable), "--runtime-dir", str(runtime)]):
            continue
        if name == "driver":
            continue
        artifact = project / "return42.elf" if name == "image" else project / "images"
        if name == "emitter":
            artifact.mkdir()
        marker = "x64 ELF image fixtures passed" if name == "image" else "x64 scalar emission fixtures ready"
        if run(name + "-run", [str(executable), str(artifact)], marker=marker):
            verifier = ROOT / "tests/v04" / ("x64-image" if name == "image" else "x64") / "verify.py"
            run(name + "-structure", [shutil.which("python3"), str(verifier), str(artifact)])
    report = dict(base_commit=base_commit, engine_sha256=digest(engine), runtime_sha256=digest(runtime / "librey_rt.a"),
                  script_sha256=digest(Path(__file__)), snapshots=snapshots, records=records,
                  verifier_hashes={name: digest(ROOT / "tests/v04" / name / "verify.py") for name in ("x64", "x64-image")},
                  passed=all(record["passed"] for record in records), limitation="byte/image gate only; Linux execution is separate")
    (output / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
