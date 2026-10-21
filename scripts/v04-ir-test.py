#!/usr/bin/env python3
"""Build and exercise the experimental scalar pipeline with an explicit seed."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", required=True, type=Path)
    parser.add_argument("--runtime-dir", required=True, type=Path)
    parser.add_argument("--output-dir", type=Path, default=ROOT / "build/v04/integration")
    parser.add_argument("--timeout", type=float, default=120)
    parser.add_argument("--native", action="store_true", help="also gate direct ARM64 native image generation")
    args = parser.parse_args()
    engine = args.engine.resolve(strict=True)
    runtime = args.runtime_dir.resolve(strict=True)
    archive = runtime / "librey_rt.a"
    if not os.access(engine, os.X_OK) or not archive.is_file() or args.timeout <= 0:
        parser.error("engine must be executable, runtime must contain librey_rt.a, timeout must be positive")
    output = args.output_dir.resolve()
    if not output.is_relative_to(ROOT / "build/v04"):
        parser.error("output must be under build/v04")
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        parser.error("output directory must be empty")
    records: list[dict] = []
    build_inputs: dict[str, dict] = {}

    def run(label: str, command: list[str], expected: int = 0, contains: str = "", env: dict | None = None) -> dict:
        started = time.monotonic()
        try:
            result = subprocess.run(command, cwd=ROOT, capture_output=True, timeout=args.timeout, env=env)
            code, stdout, stderr = result.returncode, result.stdout, result.stderr
            error = None
        except (OSError, subprocess.TimeoutExpired) as exc:
            code, stdout, stderr, error = None, b"", b"", str(exc)
        (output / (label + ".stdout")).write_bytes(stdout)
        (output / (label + ".stderr")).write_bytes(stderr)
        passed = code == expected
        passed = passed and error is None and contains in (stdout + stderr).decode(errors="replace")
        record = dict(label=label, command=command, returncode=code, expected=expected,
                      elapsed_seconds=time.monotonic() - started, error=error,
                      passed=passed, stdout_sha256=hashlib.sha256(stdout).hexdigest(),
                      stderr_sha256=hashlib.sha256(stderr).hexdigest())
        records.append(record)
        print(label + ": " + ("PASS" if passed else "FAIL"), flush=True)
        return dict(record, stdout=stdout)

    version = run("version", [str(engine), "--version"], contains="rey 0.3.1")
    if version["stdout"].strip() != b"rey 0.3.1":
        version["passed"] = False
        records[-1]["passed"] = False

    def build(label: str, main_source: Path) -> Path | None:
        project = output / label
        shutil.copytree(ROOT / "rey-compiler/src", project / "src")
        shutil.copyfile(main_source, project / "src/main.rey")
        build_inputs[label] = {str(p.relative_to(project)): digest(p) for p in sorted((project / "src").rglob("*.rey"))}
        executable = project / "fixture"
        result = run(label + "-build", [str(engine), "build", str(project / "src/main.rey"),
                     "-o", str(executable), "--runtime-dir", str(runtime)])
        return executable if result["passed"] else None

    if version["passed"]:
        suites = ["ir-core", "frontend", "ir-verify", "ir-lower", "ir-loops", "ir-eval", "native-bytes"]
        if args.native:
            suites += ["arm64", "native-image", "native-crosscheck"]
        markers = {"ir-core": "program entry=", "frontend": "v04 frontend ok",
                   "ir-verify": "ir verifier: all fixtures passed", "ir-lower": "v04 ir-lower ok",
                   "ir-eval": "v0.4 IR evaluator and effect analysis fixtures passed", "native-bytes": "native bytes ok",
                   "ir-loops": "v04 ir-loops ok", "arm64": "arm64 emitter fixtures passed", "native-image": "native image ok",
                   "native-crosscheck": "native crosscheck images ready"}
        for suite in suites:
            executable = build(suite, ROOT / "tests/v04" / suite / "main.rey")
            if executable:
                command = [str(executable)]
                if suite == "native-image":
                    (ROOT / "build/v04/native-image").mkdir(parents=True, exist_ok=True)
                if suite == "native-crosscheck":
                    image_dir = output / "native-crosscheck-images"
                    image_dir.mkdir()
                    command.append(str(image_dir))
                result = run(suite + "-run", command, contains=markers.get(suite, ""))
                if suite == "native-crosscheck" and result["passed"]:
                    cases = re.findall(rb"^native-case ([a-z0-9-]+) (\d+)$", result["stdout"], re.MULTILINE)
                    records.append(dict(label="native-crosscheck-count", passed=len(cases) == 21 and len({name for name, _ in cases}) == 21))
                    for raw_name, raw_expected in cases:
                        name, expected = raw_name.decode(), int(raw_expected)
                        artifact = image_dir / (name + ".macho")
                        exists = artifact.is_file()
                        records.append(dict(label=name + "-crosscheck-image", passed=exists, artifact_sha256=digest(artifact) if exists else None))
                        if exists and platform.system() == "Darwin" and platform.machine() == "arm64":
                            artifact.chmod(0o755)
                            run(name + "-crosscheck-execute", [str(artifact)], expected=expected)
                            run(name + "-crosscheck-signature", ["/usr/bin/codesign", "--verify", "--strict", str(artifact)])
                        else:
                            records.append(dict(label=name + "-crosscheck-execute", passed=False, reason="missing artifact or Darwin arm64 host"))
        driver = build("driver", ROOT / "rey-compiler/ir-main.rey")
        if driver:
            run("driver-help", [str(driver), "--help"], contains="Usage:")
            branch = str(ROOT / "tests/v04/pipeline/branch.rey")
            run("branch-check", [str(driver), "check", branch])
            first = run("branch-dump", [str(driver), "dump", branch], contains="program entry=")
            second = run("branch-dump-repeat", [str(driver), "dump", branch], contains="program entry=")
            records.append(dict(label="deterministic-dump", passed=first["stdout"] == second["stdout"] and first["passed"] and second["passed"]))
            run("branch-eval", [str(driver), "eval", branch], contains="void")
            run("overflow-eval", [str(driver), "eval", str(ROOT / "tests/v04/pipeline/overflow.rey")], expected=1, contains="overflow")
            run("unsupported", [str(driver), "check", str(ROOT / "tests/v04/pipeline/unsupported.rey")], expected=1)
            run("missing", [str(driver), "check", str(output / "missing.rey")], expected=1, contains="not found")
            run("usage", [str(driver), "wat"], expected=2, contains="Usage:")
        if args.native:
            native = build("native-driver", ROOT / "rey-compiler/native-main.rey")
            if native:
                run("native-help", [str(native), "--help"], contains="Usage:")
                run("native-target-error", [str(native), "build", str(ROOT / "tests/v04/pipeline/branch.rey"), "-o", str(output / "invalid-target"), "--target", "unknown"], expected=2, contains="unsupported native target")
                run("native-unsupported-source", [str(native), "build", str(ROOT / "tests/v04/pipeline/unsupported.rey"), "-o", str(output / "unsupported-image"), "--target", "darwin-arm64"], expected=1, contains="unsupported")
                clean_env = os.environ.copy()
                empty_path = output / "empty-path"
                empty_path.mkdir()
                clean_env["PATH"] = str(empty_path)
                for variable in ("CC", "CXX", "CLANG", "LD", "LLVM_CONFIG", "REY_RUNTIME_DIR"):
                    clean_env[variable] = str(empty_path / "absent-tool")
                host_can_run = platform.system() == "Darwin" and platform.machine() == "arm64"
                for fixture, expected_exit in (("native-branch", 0), ("native-loops", 0), ("overflow", 1)):
                    artifact = output / (fixture + ".macho")
                    source = ROOT / "tests/v04/pipeline" / (fixture + ".rey")
                    generated = run(fixture + "-emit", [str(native), "build", str(source), "-o", str(artifact), "--target", "darwin-arm64"], env=clean_env)
                    if generated["passed"] and artifact.is_file():
                        records.append(dict(label=fixture + "-image", passed=artifact.read_bytes()[:4] == bytes.fromhex("cffaedfe"), artifact_sha256=digest(artifact)))
                        if host_can_run:
                            artifact.chmod(0o755)
                            run(fixture + "-execute", [str(artifact)], expected=expected_exit)
                            run(fixture + "-signature", ["/usr/bin/codesign", "--verify", "--strict", str(artifact)])
                        else:
                            records.append(dict(label=fixture + "-execute", passed=False, reason="native execution gate requires Darwin arm64"))
                    else:
                        records.append(dict(label=fixture + "-image", passed=False, reason="no generated image"))
                elf = output / "branch.elf"
                generated = run("linux-emit", [str(native), "build", str(ROOT / "tests/v04/pipeline/native-branch.rey"), "-o", str(elf), "--target", "linux-arm64"], env=clean_env)
                valid_elf = generated["passed"] and elf.is_file() and elf.read_bytes()[:6] == b"\x7fELF\x02\x01"
                records.append(dict(label="linux-image-structure", passed=valid_elf, execution="not run on Linux arm64"))
    report = dict(schema_version=1, engine=str(engine), engine_sha256=digest(engine),
                  runtime=str(runtime), runtime_sha256=digest(archive),
                  build_inputs=build_inputs,
                  source_hash_scope="workspace at report time; build_inputs records exact staged snapshots",
                  source_hashes={str(p.relative_to(ROOT)): digest(p) for p in sorted((ROOT / "rey-compiler/src").rglob("*.rey"))},
                  driver_sha256=digest(ROOT / "rey-compiler/ir-main.rey"),
                  native_driver_sha256=digest(ROOT / "rey-compiler/native-main.rey") if args.native else None,
                  fixture_hashes={str(p.relative_to(ROOT)): digest(p) for p in sorted((ROOT / "tests/v04").rglob("*.rey"))},
                  records=records, passed=all(record["passed"] for record in records))
    (output / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
