#!/usr/bin/env python3
"""Independent, deterministic source-level i64 oracle for the v0.4 drivers."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import shutil
import signal
import subprocess
import time
from typing import Any


ROOT = Path(__file__).resolve().parents[3]
MIN_I64 = -(1 << 63)
MAX_I64 = (1 << 63) - 1
SEED = 0x52455904
DEFAULT_SEED = ROOT / "build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
EXPECTED_SEED_SHA256 = "89f41b7cac5f38018c32b74d7a134d4d32e1314d58ddf292efcea174d182a921"
EXPECTED_RUNTIME_SHA256 = "f3d625762a0ea9dfc403e81b5c3cc8cce49aaec64bd276e5b2dec95fad666667"


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def hash_file(path: Path) -> str:
    return sha256(path.read_bytes())


def trunc_div(a: int, b: int) -> int:
    magnitude = abs(a) // abs(b)
    return -magnitude if (a < 0) != (b < 0) else magnitude


def checked(op: str, a: int, b: int | None = None) -> tuple[int | None, str | None]:
    if op == "neg":
        result = -a
        if result > MAX_I64:
            return None, "overflow"
        return result, None
    assert b is not None
    if op == "+":
        result = a + b
    elif op == "-":
        result = a - b
    elif op == "*":
        result = a * b
    elif op == "/":
        if b == 0:
            return None, "zero"
        if a == MIN_I64 and b == -1:
            return None, "overflow"
        result = trunc_div(a, b)
    elif op == "%":
        if b == 0:
            return None, "zero"
        if a == MIN_I64 and b == -1:
            return None, "overflow"
        q = trunc_div(a, b)
        result = a - q * b
    else:
        raise ValueError(op)
    if result < MIN_I64 or result > MAX_I64:
        return None, "overflow"
    return result, None


def literal(value: int) -> str:
    if value == MIN_I64:
        return "(-9223372036854775807 - 1)"
    if value < 0:
        return "(-" + str(-value) + ")"
    return str(value)


def case(op: str, a: int, b: int | None = None, origin: str = "boundary") -> dict[str, Any]:
    value, trap = checked(op, a, b)
    return {"op": op, "a": a, "b": b, "expected": value, "trap": trap, "origin": origin}


def build_cases() -> tuple[list[dict[str, Any]], list[dict[str, Any]], list[dict[str, Any]]]:
    safe: list[dict[str, Any]] = []
    traps: list[dict[str, Any]] = []
    bool_cases: list[dict[str, Any]] = []

    boundary = [
        ("+", MAX_I64, 0), ("+", MAX_I64 - 1, 1), ("+", MIN_I64, 0),
        ("+", MIN_I64 + 1, -1), ("+", MAX_I64, -1), ("+", MIN_I64, 1),
        ("+", 0, MIN_I64), ("+", -1, MIN_I64 + 1),
        ("-", MAX_I64, 0), ("-", MIN_I64, 0), ("-", MIN_I64 + 1, 1),
        ("-", MAX_I64 - 1, -1), ("-", MAX_I64, MAX_I64),
        ("-", MIN_I64, MIN_I64), ("-", 0, MAX_I64), ("-", 0, MIN_I64 + 1),
        ("*", 0, MIN_I64), ("*", 1, MAX_I64), ("*", -1, MAX_I64),
        ("*", 1, MIN_I64), ("*", 3037000499, 3037000499),
        ("*", -3037000499, 3037000499), ("*", 65536, 140737488355327),
        ("/", MIN_I64, 1), ("/", MIN_I64, 2), ("/", MIN_I64, 3),
        ("/", -7, 3), ("/", 7, -3), ("/", -7, -3), ("/", MAX_I64, -2),
        ("/", 0, -3), ("/", MAX_I64, MAX_I64),
        ("%", MIN_I64, 1), ("%", MIN_I64, 3), ("%", -7, 3),
        ("%", 7, -3), ("%", -7, -3), ("%", MAX_I64, 97),
        ("%", MIN_I64 + 1, MAX_I64), ("%", 0, -7),
        ("neg", 0), ("neg", 1), ("neg", -1), ("neg", MIN_I64 + 1),
        ("neg", MAX_I64),
    ]
    boundary_traps = [
        ("+", MAX_I64, 1), ("+", MIN_I64, -1),
        ("-", MIN_I64, 1), ("-", MAX_I64, -1), ("-", 0, MIN_I64),
        ("*", MAX_I64, 2), ("*", MIN_I64, 2), ("*", MIN_I64, -1),
        ("/", MIN_I64, -1), ("/", 9, 0),
        ("%", MIN_I64, -1), ("%", -9, 0), ("neg", MIN_I64),
    ]
    for op, a, *rest in boundary:
        entry = case(op, a, rest[0] if rest else None)
        assert entry["trap"] is None
        safe.append(entry)
    for op, a, *rest in boundary_traps:
        entry = case(op, a, rest[0] if rest else None)
        assert entry["trap"] is not None
        traps.append(entry)

    rng = random.Random(SEED)
    edge_values = [MIN_I64, MIN_I64 + 1, MIN_I64 + 2, -MAX_I64, -3037000499,
                   -65537, -3, -2, -1, 0, 1, 2, 3, 65537, 3037000499,
                   MAX_I64 - 2, MAX_I64 - 1, MAX_I64]
    for op in ("+", "-", "*", "/", "%"):
        for index in range(5):
            a = rng.choice(edge_values) if index % 2 == 0 else rng.randint(MIN_I64, MAX_I64)
            if op == "*":
                b = rng.randint(-3, 3) if index % 2 == 0 else rng.randint(-100000, 100000)
            elif op in ("/", "%"):
                b = rng.choice(edge_values)
            elif index % 2 == 0:
                b = rng.choice(edge_values)
            else:
                b = rng.randint(-100000, 100000)
            entry = case(op, a, b, "seeded-random")
            (traps if entry["trap"] else safe).append(entry)
    for _ in range(6):
        a = rng.choice(edge_values) if rng.randrange(2) == 0 else rng.randint(MIN_I64, MAX_I64)
        entry = case("neg", a, origin="seeded-random")
        (traps if entry["trap"] else safe).append(entry)

    bool_cases.extend([
        {"name": "false-and-skips-trap", "expr": "false && trapBool()", "expected": "false"},
        {"name": "true-or-skips-trap", "expr": "true || trapBool()", "expected": "true"},
        {"name": "true-and-calls-safe", "expr": "true && true", "expected": "true"},
        {"name": "false-or-calls-safe", "expr": "false || false", "expected": "false"},
        {"name": "false-and-nested-skips-trap", "expr": "false && (true || trapBool())", "expected": "false"},
        {"name": "true-or-nested-skips-trap", "expr": "true || (false && trapBool())", "expected": "true"},
    ])
    return safe, traps, bool_cases


def program_for_cases(entries: list[dict[str, Any]], batch_name: str) -> str:
    lines = [
        "func failNow(value: int): int { return value + 1; }",
    ]
    for index, entry in enumerate(entries):
        if entry["op"] == "neg":
            lines.append(f"func op{index}(a: int): int {{ return -a; }}")
        else:
            op = entry["op"]
            lines.append(f"func op{index}(a: int, b: int): int {{ return a {op} b; }}")
    lines.append("func main(): void {")
    for index, entry in enumerate(entries):
        left = literal(entry["a"])
        if entry["op"] == "neg":
            expression = f"op{index}({left})"
        else:
            right = literal(entry["b"])
            expression = f"op{index}({left}, {right})"
        if entry["trap"]:
            lines.append(f"    var observed{index}: int = {expression};")
        else:
            lines.append(f"    if {expression} != {entry['expected']} {{ var mismatch{index}: int = failNow(9223372036854775807); }}")
    lines.append("    return;")
    lines.append("}")
    return "\n".join(lines) + "\n"


def program_for_bool(entries: list[dict[str, Any]]) -> str:
    lines = [
        "func failInt(value: int): int { return value + 1; }",
        "func trapBool(): bool { var doomed: int = failInt(9223372036854775807); return true; }",
        "func failNow(value: int): int { return value + 1; }",
        "func main(): void {",
    ]
    for index, entry in enumerate(entries):
        lines.append(f"    if ({entry['expr']}) != {entry['expected']} {{ var mismatch{index}: int = failNow(9223372036854775807); }}")
    lines.extend(["    return;", "}"])
    return "\n".join(lines) + "\n"


def expected_diagnostic(entry: dict[str, Any]) -> str:
    if entry["trap"] == "zero":
        return "zero"
    return "overflow"


def prepare_output(path: Path) -> Path:
    output = path.resolve()
    allowed = (ROOT / "build/v04/arithmetic-oracle").resolve()
    if not output.is_relative_to(allowed):
        raise ValueError(f"output must be within {allowed}")
    if output.exists() and any(output.iterdir()):
        raise ValueError(f"output directory is not empty: {output}")
    output.mkdir(parents=True, exist_ok=True)
    return output


class Runner:
    def __init__(self, root: Path, timeout: float):
        self.root = root
        self.timeout = timeout
        self.records: list[dict[str, Any]] = []

    def run(self, label: str, argv: list[str], expected_status: int | None,
            output: Path, env: dict[str, str] | None = None,
            diagnostic: str | None = None) -> dict[str, Any]:
        stdout_path = output / f"{label}.stdout"
        stderr_path = output / f"{label}.stderr"
        started = time.monotonic()
        status: int | None = None
        timed_out = False
        error: str | None = None
        child_env = os.environ.copy() if env is None else env.copy()
        with stdout_path.open("wb") as stdout_file, stderr_path.open("wb") as stderr_file:
            try:
                process = subprocess.Popen(argv, cwd=self.root, env=child_env,
                                           stdout=stdout_file, stderr=stderr_file,
                                           start_new_session=True)
            except OSError as exc:
                error = str(exc)
            else:
                try:
                    status = process.wait(timeout=self.timeout)
                except subprocess.TimeoutExpired:
                    timed_out = True
                    try:
                        os.killpg(process.pid, signal.SIGTERM)
                    except ProcessLookupError:
                        pass
                    try:
                        status = process.wait(timeout=1.0)
                    except subprocess.TimeoutExpired:
                        try:
                            os.killpg(process.pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass
                        status = process.wait()
        elapsed = time.monotonic() - started
        stdout = stdout_path.read_bytes()
        stderr = stderr_path.read_bytes()
        combined = (stdout + stderr).decode("utf-8", errors="replace")
        passed = error is None and not timed_out and status == expected_status
        if diagnostic is not None:
            passed = passed and diagnostic.lower() in combined.lower()
        record = {
            "label": label,
            "argv": argv,
            "status": status,
            "expected_status": expected_status,
            "timed_out": timed_out,
            "timeout_seconds": self.timeout,
            "elapsed_seconds": round(elapsed, 6),
            "error": error,
            "stdout_path": str(stdout_path.relative_to(self.root)),
            "stdout_sha256": sha256(stdout),
            "stderr_path": str(stderr_path.relative_to(self.root)),
            "stderr_sha256": sha256(stderr),
            "diagnostic_required": diagnostic,
            "passed": passed,
        }
        self.records.append(record)
        print(f"{label}: {'PASS' if passed else 'FAIL'} (status={status}, expected={expected_status})", flush=True)
        return record


def stage_driver(driver_source: Path, name: str, seed: Path, runtime: Path,
                 output: Path, runner: Runner, manifest: dict[str, Any]) -> Path | None:
    staged = output / "staged" / name
    src = staged / "src"
    shutil.copytree(ROOT / "rey-compiler/src", src)
    shutil.copyfile(driver_source, src / "main.rey")
    manifest[name] = {
        p.relative_to(staged).as_posix(): hash_file(p)
        for p in sorted(staged.rglob("*")) if p.is_file()
    }
    executable = staged / name
    record = runner.run(f"driver-{name}-build",
                        [str(seed), "build", str(src / "main.rey"), "-o", str(executable),
                         "--runtime-dir", str(runtime)], 0, output)
    return executable if record["passed"] and executable.is_file() else None


def make_groups(items: list[dict[str, Any]], size: int) -> list[list[dict[str, Any]]]:
    return [items[index:index + size] for index in range(0, len(items), size)]


def mismatch_source(value: str) -> str:
    return (
        "func main(): void {\n"
        f"    var outOfRange: int = {value};\n"
        "    return;\n"
        "}\n"
    )


def run_suite(args: argparse.Namespace) -> int:
    seed = args.seed.resolve(strict=True)
    runtime = seed.parent / "runtime"
    archive = runtime / "librey_rt.a"
    if not seed.is_file() or not os.access(seed, os.X_OK) or not archive.is_file():
        raise ValueError("seed must be executable and have adjacent runtime/librey_rt.a")
    seed_hash = hash_file(seed)
    runtime_hash = hash_file(archive)
    if args.require_pinned and (seed_hash != EXPECTED_SEED_SHA256 or runtime_hash != EXPECTED_RUNTIME_SHA256):
        raise ValueError("seed/runtime hashes do not match the pinned v0.3.1 pair")
    output = prepare_output(args.output_dir)
    runner = Runner(ROOT, args.timeout)
    staged_hashes: dict[str, Any] = {}
    version = runner.run("seed-version", [str(seed), "--version"], 0, output)
    version_output = (output / "seed-version.stdout").read_text(errors="replace").strip()
    if version_output != "rey 0.3.1":
        version["passed"] = False

    ir_driver = stage_driver(ROOT / "rey-compiler/ir-main.rey", "ir-driver", seed, runtime, output, runner, staged_hashes)
    native_driver = stage_driver(ROOT / "rey-compiler/native-main.rey", "native-driver", seed, runtime, output, runner, staged_hashes)
    if ir_driver is None or native_driver is None:
        ir_driver = None
        native_driver = None

    safe, traps, bool_cases = build_cases()
    sources = output / "sources"
    sources.mkdir()
    source_manifest: dict[str, dict[str, Any]] = {}
    clean_env = os.environ.copy()
    empty_path = output / "empty-path"
    empty_path.mkdir()
    clean_env["PATH"] = str(empty_path)
    for tool_name in ("CC", "CXX", "CLANG", "LD", "LLVM_CONFIG", "REY_RUNTIME_DIR"):
        clean_env[tool_name] = str(empty_path / "absent-tool")

    generated: list[tuple[str, str, list[dict[str, Any]], bool]] = []
    for index, batch in enumerate(make_groups(safe, 2)):
        generated.append((f"safe-{index:02d}", program_for_cases(batch, f"safe-{index:02d}"), batch, False))
    for index, entry in enumerate(traps):
        generated.append((f"trap-{index:02d}", program_for_cases([entry], f"trap-{index:02d}"), [entry], False))
    for index, batch in enumerate(make_groups(bool_cases, 2)):
        generated.append((f"bool-{index:02d}", program_for_bool(batch), batch, True))

    program_records: list[dict[str, Any]] = []
    for name, source, entries, is_bool in generated:
        source_path = sources / f"{name}.rey"
        source_path.write_text(source, encoding="utf-8")
        src_hash = hash_file(source_path)
        base = {
            "name": name,
            "source_path": str(source_path.relative_to(ROOT)),
            "source_sha256": src_hash,
            "cases": entries,
            "expected_status": 1 if any(entry.get("trap") for entry in entries) else 0,
        }
        source_manifest[name] = base
        if ir_driver is None or native_driver is None:
            program_records.append(dict(base, passed=False, reason="driver build failed"))
            continue
        check = runner.run(f"{name}-ir-check", [str(ir_driver), "check", str(source_path)], 0, output)
        native_image = output / "images" / f"{name}.macho"
        native_image.parent.mkdir(exist_ok=True)
        emitted = runner.run(f"{name}-native-emit",
                             [str(native_driver), "build", str(source_path), "-o", str(native_image),
                              "--target", "darwin-arm64"], 0, output, env=clean_env)
        if emitted["passed"] and native_image.is_file():
            native_image.chmod(0o755)
            native_run = runner.run(f"{name}-native-run", [str(native_image)], base["expected_status"], output)
        else:
            native_run = {"passed": False, "status": None, "expected_status": base["expected_status"]}
        ir_expected = 1 if base["expected_status"] == 1 else 0
        diagnostic = None
        if ir_expected == 1:
            diagnostic = "zero" if any(entry.get("trap") == "zero" for entry in entries) else "overflow"
        ir_run = runner.run(f"{name}-ir-eval", [str(ir_driver), "eval", str(source_path)], ir_expected,
                            output, diagnostic=diagnostic)
        item_passed = check["passed"] and emitted["passed"] and native_run["passed"] and ir_run["passed"]
        program_records.append(dict(base, native_image_sha256=hash_file(native_image) if native_image.is_file() else None,
                                    ir_check=check, native_emit=emitted, native_run=native_run,
                                    ir_eval=ir_run, passed=item_passed))

    invalid_sources = [
        ("positive-magnitude-above-max", "9223372036854775808"),
        ("negative-magnitude-below-min", "-9223372036854775809"),
    ]
    invalid_records: list[dict[str, Any]] = []
    for name, expression in invalid_sources:
        source_path = sources / f"{name}.rey"
        source_path.write_text(mismatch_source(expression), encoding="utf-8")
        record = {"name": name, "expression": expression, "source_sha256": hash_file(source_path),
                  "source_path": str(source_path.relative_to(ROOT))}
        if ir_driver is not None and native_driver is not None:
            ir_reject = runner.run(f"{name}-ir-reject", [str(ir_driver), "check", str(source_path)], 1, output)
            native_reject = runner.run(f"{name}-native-reject",
                                       [str(native_driver), "build", str(source_path), "-o",
                                        str(output / "images" / f"{name}.macho"), "--target", "darwin-arm64"],
                                       1, output, env=clean_env)
            record.update(ir_reject=ir_reject, native_reject=native_reject,
                          passed=ir_reject["passed"] and native_reject["passed"])
        else:
            record.update(passed=False, reason="driver build failed")
        invalid_records.append(record)

    report = {
        "schema_version": 1,
        "oracle": "Python arbitrary precision; truncating division computed with integer magnitudes, never floating point",
        "seeded_random_seed": SEED,
        "seed": str(seed),
        "seed_sha256": seed_hash,
        "expected_seed_sha256": EXPECTED_SEED_SHA256,
        "runtime_archive": str(archive),
        "runtime_sha256": runtime_hash,
        "expected_runtime_sha256": EXPECTED_RUNTIME_SHA256,
        "host": {"system": platform.system(), "machine": platform.machine()},
        "timeout_seconds": args.timeout,
        "native_generation_path": "PATH contains only an empty directory; compiler-tool variables point to absent paths",
        "staged_compiler_source_hashes": staged_hashes,
        "programs": program_records,
        "invalid_literals": invalid_records,
        "process_records": runner.records,
        "case_counts": {"safe": len(safe), "traps": len(traps), "boolean": len(bool_cases),
                        "programs": len(generated), "invalid_literals": len(invalid_sources)},
    }
    report["passed"] = (version["passed"] and ir_driver is not None and native_driver is not None
                        and all(item.get("passed", False) for item in program_records)
                        and all(item.get("passed", False) for item in invalid_records))
    (output / "result.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"result: {'PASS' if report['passed'] else 'FAIL'}; artifacts: {output}")
    print(f"programs={len(generated)} safe_cases={len(safe)} trap_cases={len(traps)} bool_cases={len(bool_cases)} invalid_literals={len(invalid_records)}")
    return 0 if report["passed"] else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seed", type=Path, default=DEFAULT_SEED)
    parser.add_argument("--output-dir", type=Path,
                        default=ROOT / "build/v04/arithmetic-oracle/run")
    parser.add_argument("--timeout", type=float, default=20.0)
    parser.add_argument("--allow-unpinned-seed", action="store_true")
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("--timeout must be positive")
    try:
        return run_suite(argparse.Namespace(**vars(args), require_pinned=not args.allow_unpinned_seed))
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
