#!/usr/bin/env python3
"""Build and exercise the experimental scalar pipeline with an explicit seed."""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
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
    fixture_snapshots: dict[str, dict] = {}

    def run(label: str, command: list[str], expected: int = 0, contains: str = "", env: dict | None = None,
            timeout: float | None = None) -> dict:
        started = time.monotonic()
        try:
            result = subprocess.run(command, cwd=ROOT, capture_output=True,
                                    timeout=args.timeout if timeout is None else timeout, env=env)
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

    def build(label: str, main_source: Path, fixture_source: Path | None = None) -> Path | None:
        project = output / label
        shutil.copytree(ROOT / "rey-compiler/src", project / "src")
        shutil.copyfile(main_source, project / "src/main.rey")
        build_inputs[label] = {str(p.relative_to(project)): digest(p) for p in sorted((project / "src").rglob("*.rey"))}
        if fixture_source is not None:
            fixture_root = project / "fixtures"
            shutil.copytree(fixture_source, fixture_root)
            fixture_snapshots[label] = dict(
                source=str(fixture_source.relative_to(ROOT)),
                staged_root=str(fixture_root.relative_to(ROOT)),
                files={str(p.relative_to(fixture_root)): digest(p) for p in sorted(fixture_root.rglob("*")) if p.is_file()},
            )
        executable = project / "fixture"
        result = run(label + "-build", [str(engine), "build", str(project / "src/main.rey"),
                     "-o", str(executable), "--runtime-dir", str(runtime)])
        return executable if result["passed"] else None

    if version["passed"]:
        suites = ["ir-core", "frontend", "ir-verify", "ir-lower", "ir-loops", "ir-eval", "native-bytes",
                  "frontend-graph", "ir-arithmetic", "ir-source-coverage", "ir-liveness"]
        if args.native:
            suites += ["arm64", "native-image", "native-crosscheck", "arm64-arithmetic"]
        markers = {"ir-core": "program entry=", "frontend": "v04 frontend ok",
                   "ir-verify": "ir verifier: all fixtures passed", "ir-lower": "v04 ir-lower ok",
                   "ir-eval": "v0.4 IR evaluator and effect analysis fixtures passed", "native-bytes": "native bytes ok",
                   "ir-loops": "v04 ir-loops ok", "arm64": "arm64 emitter fixtures passed", "native-image": "native image ok",
                   "native-crosscheck": "native crosscheck images ready",
                   "frontend-graph": "v04 frontend graph ok",
                   "ir-arithmetic": "v04 scalar arithmetic verification and evaluation passed",
                   "ir-source-coverage": "v04 ir source coverage ok", "ir-liveness": "v04 ir liveness ok",
                   "arm64-arithmetic": "arm64 arithmetic fixture images ready"}
        for suite in suites:
            fixture_source = ROOT / "tests/v04" / suite / "fixtures" if suite in ("frontend-graph", "ir-source-coverage") else None
            executable = build(suite, ROOT / "tests/v04" / suite / "main.rey", fixture_source)
            if executable:
                command = [str(executable)]
                if fixture_source is not None:
                    command.append(str(output / suite / "fixtures"))
                if suite == "native-image":
                    (ROOT / "build/v04/native-image").mkdir(parents=True, exist_ok=True)
                if suite == "native-crosscheck":
                    image_dir = output / "native-crosscheck-images"
                    image_dir.mkdir()
                    command.append(str(image_dir))
                if suite == "arm64-arithmetic":
                    image_dir = output / "arm64-arithmetic-images"
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
                if suite == "arm64-arithmetic" and result["passed"]:
                    verifier_path = ROOT / "tests/v04/arm64-arithmetic/verify.py"
                    spec = importlib.util.spec_from_file_location("v04_arm64_arithmetic_verify", verifier_path)
                    if spec is None or spec.loader is None:
                        raise RuntimeError("cannot load arm64 arithmetic expected-case oracle")
                    verifier = importlib.util.module_from_spec(spec)
                    spec.loader.exec_module(verifier)
                    cases = verifier.expected_cases()
                    names = re.findall(rb"^native-arithmetic-case ([a-z0-9-]+)$", result["stdout"], re.MULTILINE)
                    emitted = [name.decode() for name in names]
                    records.append(dict(label="arm64-arithmetic-emitted-cases",
                                        passed=len(emitted) == 28 and len(set(emitted)) == 28 and set(emitted) == set(cases),
                                        count=len(emitted), names=emitted, expected_count=len(cases),
                                        expected_names=sorted(cases)))
                    entries = list(image_dir.iterdir())
                    actual_files = {path.name for path in entries if path.is_file()}
                    exact_set = all(path.is_file() for path in entries) and actual_files == set(cases)
                    records.append(dict(label="arm64-arithmetic-image-set", passed=exact_set,
                                        expected_names=sorted(cases), actual_files=sorted(actual_files),
                                        non_file_entries=sorted(path.name for path in entries if not path.is_file())))
                    host_can_run = platform.system() == "Darwin" and platform.machine() == "arm64"
                    for name, value in cases.items():
                        artifact = image_dir / name
                        exists = artifact.is_file()
                        records.append(dict(label=name + "-arithmetic-image", passed=exists,
                                            artifact_sha256=digest(artifact) if exists else None))
                        if exists and host_can_run:
                            artifact.chmod(0o755)
                            expected_status = 1 if value is None else value % 256
                            run(name + "-arithmetic-execute", [str(artifact)], expected=expected_status,
                                timeout=min(args.timeout, 10))
                            run(name + "-arithmetic-signature", ["/usr/bin/codesign", "--verify", "--strict", str(artifact)],
                                timeout=min(args.timeout, 10))
                        else:
                            records.append(dict(label=name + "-arithmetic-execute", passed=False,
                                                reason="missing artifact or Darwin arm64 host"))
                            records.append(dict(label=name + "-arithmetic-signature", passed=False,
                                                reason="missing artifact or Darwin arm64 host"))
        std_source = ROOT / "tests/v04/frontend-graph/fixtures/std-valid/main.rey"
        std_root = ROOT / "tests/v04/frontend-graph/fixtures/stdroot"
        driver = build("driver", ROOT / "rey-compiler/ir-main.rey")
        if driver:
            run("driver-help", [str(driver), "--help"], contains="Usage:")
            run("driver-std-root", [str(driver), "check", str(std_source), "--std-root", str(std_root)])
            branch = str(ROOT / "tests/v04/pipeline/branch.rey")
            run("branch-check", [str(driver), "check", branch])
            first = run("branch-dump", [str(driver), "dump", branch], contains="program entry=")
            second = run("branch-dump-repeat", [str(driver), "dump", branch], contains="program entry=")
            records.append(dict(label="deterministic-dump", passed=first["stdout"] == second["stdout"] and first["passed"] and second["passed"]))
            run("branch-eval", [str(driver), "eval", branch], contains="void")
            run("graph-eval", [str(driver), "eval", str(ROOT / "tests/v04/pipeline/graphs/main.rey")], contains="void")
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
                native_cases = [
                    ("native-branch", ROOT / "tests/v04/pipeline/native-branch.rey", 0, []),
                    ("native-loops", ROOT / "tests/v04/pipeline/native-loops.rey", 0, []),
                    ("overflow", ROOT / "tests/v04/pipeline/overflow.rey", 1, []),
                    ("native-arithmetic", ROOT / "tests/v04/pipeline/native-arithmetic.rey", 0, []),
                    ("native-mul-overflow", ROOT / "tests/v04/pipeline/native-mul-overflow.rey", 1, []),
                    ("native-div-zero", ROOT / "tests/v04/pipeline/native-div-zero.rey", 1, []),
                    ("native-rem-overflow", ROOT / "tests/v04/pipeline/native-rem-overflow.rey", 1, []),
                    ("native-neg-overflow", ROOT / "tests/v04/pipeline/native-neg-overflow.rey", 1, []),
                    ("native-graph", ROOT / "tests/v04/pipeline/graphs/main.rey", 0, []),
                    ("native-std-root", std_source, 0, ["--std-root", str(std_root)]),
                ]
                for fixture, source, expected_exit, extra_args in native_cases:
                    artifact = output / (fixture + ".macho")
                    generated = run(fixture + "-emit", [str(native), "build", str(source), "-o", str(artifact),
                                                          "--target", "darwin-arm64"] + extra_args, env=clean_env)
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
                  native_driver_sha256=build_inputs.get("native-driver", {}).get("src/main.rey") if args.native else None,
                  fixture_snapshots=fixture_snapshots,
                  fixture_hashes={str(p.relative_to(ROOT)): digest(p) for p in sorted((ROOT / "tests/v04").rglob("*.rey"))},
                  records=records, passed=all(record["passed"] for record in records))
    (output / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
