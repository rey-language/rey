#!/usr/bin/env python3
"""Build the scalar-fold fixtures with the pinned Rey seed and check an independent i64 oracle."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[3]
SEED = ROOT / "build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME = SEED.parent / "runtime"
EXPECTED_SEED = "89f41b7cac5f38018c32b74d7a134d4d32e1314d58ddf292efcea174d182a921"
EXPECTED_RUNTIME = "f3d625762a0ea9dfc403e81b5c3cc8cce49aaec64bd276e5b2dec95fad666667"
MIN_I64 = -(1 << 63)
MAX_I64 = (1 << 63) - 1
EXPECTED_PAIRS = [
    (MIN_I64, 0), (MIN_I64, 1), (MIN_I64, -1), (MIN_I64, 2), (MIN_I64, -2),
    (MIN_I64 + 1, -1), (MIN_I64 + 1, 1), (MAX_I64, 0), (MAX_I64, 1),
    (MAX_I64, -1), (MAX_I64, 2), (MAX_I64, -2), (MAX_I64 - 1, 1),
    (MAX_I64 - 1, -1), (-1, 0), (-1, 1), (-1, -1), (-1, 3), (-7, 3),
    (7, -3), (-7, -3), (7, 3), (3037000499, 3037000499),
    (-3037000500, -3037000500), (3037000500, 3037000500),
    (-3037000500, 3037000500), (3037000500, -3037000500), (0, MIN_I64),
    (MIN_I64, MAX_I64), (MAX_I64, MIN_I64),
]


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def trunc_div(left: int, right: int) -> int:
    magnitude = abs(left) // abs(right)
    return -magnitude if (left < 0) != (right < 0) else magnitude


def expected(op: str, left: int, right: int) -> tuple[int | None, str | None]:
    if op == "add":
        value = left + right
    elif op == "sub":
        value = left - right
    elif op == "mul":
        value = left * right
    elif op == "div":
        if right == 0:
            return None, "zero"
        if left == MIN_I64 and right == -1:
            return None, "overflow"
        value = trunc_div(left, right)
    elif op == "rem":
        if right == 0:
            return None, "zero"
        if left == MIN_I64 and right == -1:
            return None, "overflow"
        quotient = trunc_div(left, right)
        value = left - quotient * right
    else:
        raise ValueError(op)
    if value < MIN_I64 or value > MAX_I64:
        return None, "overflow"
    return value, None


def run(command: list[str], cwd: Path, timeout: float = 300) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(command, cwd=cwd, capture_output=True, timeout=timeout)


def output_root(value: str | None) -> Path:
    base = ROOT / "build/v04/ir-fold"
    base.mkdir(parents=True, exist_ok=True)
    if value:
        path = Path(value).expanduser()
        if not path.is_absolute():
            path = ROOT / path
        path = path.resolve()
        try:
            path.relative_to(base.resolve())
        except ValueError as exc:
            raise ValueError(f"output must stay under {base}") from exc
    else:
        path = base / time.strftime("run-%Y%m%dT%H%M%S-")
        path = Path(str(path) + str(os.getpid()))
    if path.exists() and any(path.iterdir()):
        raise ValueError(f"output directory is not empty: {path}")
    path.mkdir(parents=True, exist_ok=True)
    return path


def check_case_records(stdout: str) -> tuple[int, list[dict[str, object]]]:
    records: list[dict[str, object]] = []
    seen: set[tuple[str, int, int]] = set()
    for line in stdout.splitlines():
        if not line.startswith("FOLD_CASE,"):
            continue
        fields = line.split(",", 5)
        if len(fields) != 6:
            raise ValueError(f"malformed arithmetic case record: {line}")
        _, op, left_text, right_text, status, actual_text = fields
        left, right = int(left_text), int(right_text)
        key = (op, left, right)
        if key in seen:
            raise ValueError(f"duplicate arithmetic case: {key}")
        seen.add(key)
        expected_value, trap = expected(op, left, right)
        if trap is None:
            if status != "value" or int(actual_text) != expected_value:
                raise ValueError(f"{key}: expected value {expected_value}, got {status} {actual_text}")
        else:
            if status != "trap":
                raise ValueError(f"{key}: expected {trap} trap, got {status} {actual_text}")
            lowered = actual_text.lower()
            if trap == "zero" and "by zero" not in lowered:
                raise ValueError(f"{key}: expected division/remainder-by-zero error, got {actual_text}")
            if trap == "overflow" and "overflow" not in lowered:
                raise ValueError(f"{key}: expected overflow error, got {actual_text}")
        records.append(dict(op=op, left=left, right=right, status=status,
                            expected=expected_value, trap=trap))
    if len(records) != 150:
        raise ValueError(f"expected 150 independently checked arithmetic cases, found {len(records)}")
    if {item["op"] for item in records} != {"add", "sub", "mul", "div", "rem"}:
        raise ValueError("arithmetic case records do not cover every checked operation")
    expected_cases = {(op, left, right) for op in ("add", "sub", "mul", "div", "rem")
                      for left, right in EXPECTED_PAIRS}
    if seen != expected_cases:
        missing = sorted(expected_cases - seen)
        extra = sorted(seen - expected_cases)
        raise ValueError(f"arithmetic input set differs: missing={missing}, extra={extra}")
    return len(records), records


def check_native(output: Path, stdout: str) -> list[dict[str, object]]:
    expected_names = {"safe-original": 122, "safe-folded": 122,
                      "overflow-original": 1, "overflow-folded": 1}
    emitted = {line.split(",", 1)[1] for line in stdout.splitlines() if line.startswith("NATIVE_CASE,")}
    if emitted != set(expected_names):
        raise ValueError(f"native image records differ: expected {sorted(expected_names)}, got {sorted(emitted)}")
    records: list[dict[str, object]] = []
    can_execute = platform.system() == "Darwin" and platform.machine() == "arm64"
    for name, expected_status in sorted(expected_names.items()):
        image = output / (name + ".macho")
        if not image.is_file() or image.read_bytes()[:4] != bytes.fromhex("cffaedfe"):
            raise ValueError(f"{name}: missing or invalid Mach-O arm64 image")
        record: dict[str, object] = dict(name=name, image=str(image.relative_to(ROOT)), sha256=digest(image),
                                         structurally_valid=True, executed=False, codesign_verified=False)
        if can_execute:
            image.chmod(0o755)
            execution = run([str(image)], ROOT, timeout=10)
            record["executed"] = execution.returncode == expected_status
            record["returncode"] = execution.returncode
            record["expected_returncode"] = expected_status
            if execution.returncode != expected_status:
                raise ValueError(f"{name}: exit {execution.returncode}, expected {expected_status}; stderr={execution.stderr!r}")
            if execution.stdout != b"":
                raise ValueError(f"{name}: expected empty stdout, got {execution.stdout!r}")
            signature = run(["/usr/bin/codesign", "--verify", "--strict", str(image)], ROOT, timeout=10)
            record["codesign_verified"] = signature.returncode == 0
            if signature.returncode != 0:
                raise ValueError(f"{name}: codesign verification failed: {signature.stderr!r}")
        records.append(record)
    return records


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", help="fresh output path under build/v04/ir-fold")
    args = parser.parse_args()
    output = output_root(args.output_dir)
    if not SEED.is_file() or not (RUNTIME / "librey_rt.a").is_file():
        raise ValueError("pinned v0.3.1 seed and its adjacent runtime archive are required")
    seed_hash = digest(SEED)
    runtime_hash = digest(RUNTIME / "librey_rt.a")
    if seed_hash != EXPECTED_SEED or runtime_hash != EXPECTED_RUNTIME:
        raise ValueError("pinned v0.3.1 seed/runtime hashes do not match the v0.4 baseline")
    version = run([str(SEED), "--version"], ROOT)
    if version.returncode != 0 or version.stdout.strip() != b"rey 0.3.1":
        raise ValueError(f"pinned seed version check failed: {version.stdout!r} {version.stderr!r}")

    stage = output / "staged-compiler"
    shutil.copytree(ROOT / "rey-compiler/src", stage / "src")
    shutil.copyfile(Path(__file__).with_name("main.rey"), stage / "src/main.rey")
    build = run([str(SEED), "build", str(stage / "src/main.rey"), "-o", str(stage / "ir-fold"),
                 "--runtime-dir", str(RUNTIME)], ROOT)
    (output / "build.stdout").write_bytes(build.stdout)
    (output / "build.stderr").write_bytes(build.stderr)
    if build.returncode != 0 or not (stage / "ir-fold").is_file():
        raise ValueError(f"staged full compiler build failed (exit {build.returncode}); see {output / 'build.stderr'}")

    images = output / "native-images"
    images.mkdir()
    execution = run([str(stage / "ir-fold"), str(images)], ROOT, timeout=300)
    (output / "fixture.stdout").write_bytes(execution.stdout)
    (output / "fixture.stderr").write_bytes(execution.stderr)
    stdout = execution.stdout.decode(errors="replace")
    if execution.returncode != 0 or "ir-fold: all fixture assertions passed" not in stdout:
        raise ValueError(f"fixture execution failed (exit {execution.returncode}); see {output / 'fixture.stderr'}")
    case_count, cases = check_case_records(stdout)
    native = check_native(images, stdout)
    report = {
        "schema": "rey-v04-ir-fold-v1",
        "status": "pass",
        "seed": str(SEED.relative_to(ROOT)),
        "seed_sha256": seed_hash,
        "runtime_archive": str((RUNTIME / "librey_rt.a").relative_to(ROOT)),
        "runtime_sha256": runtime_hash,
        "staged_source_hashes": {
            path.relative_to(stage).as_posix(): digest(path)
            for path in sorted((stage / "src").rglob("*.rey"))
        },
        "arithmetic_oracle": {"checked_cases": case_count, "records": cases},
        "native_images": native,
        "host": {"system": platform.system(), "machine": platform.machine()},
        "fixture_stdout_sha256": hashlib.sha256(execution.stdout).hexdigest(),
    }
    (output / "result.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(f"ir-fold: PASS ({case_count} arbitrary-precision arithmetic cases; {len(native)} ARM64 images)")
    print(f"report: {output / 'result.json'}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.TimeoutExpired) as exc:
        raise SystemExit(f"ir-fold: FAIL: {exc}")
