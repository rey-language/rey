#!/usr/bin/env python3
"""Execute direct-IR slot-clobber regressions in both emitter modes."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[3]
SEED = ROOT / "build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME = SEED.parent / "runtime"
CASES = ("dead-result", "dead-parameters", "dead-block-parameters", "parallel-swap", "chain", "raw-across-call")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--linux-docker", action="store_true")
    parser.add_argument("--suite", choices=("storage", "arguments"), default="storage")
    args = parser.parse_args()
    output = args.output_dir.resolve()
    suite = "native-" + args.suite
    cases = CASES if args.suite == "storage" else ("eight", "nine", "ten", "sixteen", "large-frame", "hidden-pointers")
    raw_case = "raw-across-call" if args.suite == "storage" else "hidden-pointers"
    if not output.is_relative_to(ROOT / "build/v04" / suite):
        parser.error("output must be under build/v04/" + suite)
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        parser.error("output must be empty")
    records = []

    def record(label, passed, **fields):
        records.append(dict(label=label, passed=bool(passed), **fields))
        if not passed:
            print(label, "FAIL", flush=True)

    def run(label, command, *, env=None, input=None, timeout=120, empty=False):
        try:
            result = subprocess.run(command, cwd=ROOT, env=env, input=input, capture_output=True, timeout=timeout)
            status, stdout, stderr, error = result.returncode, result.stdout, result.stderr, None
        except (OSError, subprocess.TimeoutExpired) as exc:
            status, stdout, stderr, error = None, b"", b"", str(exc)
        (output / (label + ".stdout")).write_bytes(stdout)
        (output / (label + ".stderr")).write_bytes(stderr)
        passed = status == 0 and error is None and (not empty or stdout == b"")
        record(label, passed, command=command, status=status, error=error,
               stdout_sha256=hashlib.sha256(stdout).hexdigest(), stderr_sha256=hashlib.sha256(stderr).hexdigest())
        return passed, stdout

    seed_ok = digest(SEED) == "89f41b7cac5f38018c32b74d7a134d4d32e1314d58ddf292efcea174d182a921"
    seed_ok = seed_ok and digest(RUNTIME / "librey_rt.a") == "f3d625762a0ea9dfc403e81b5c3cc8cce49aaec64bd276e5b2dec95fad666667"
    record("pinned-seed-runtime", seed_ok)
    if not seed_ok:
        raise SystemExit("seed/runtime mismatch")
    stage, images = output / "compiler", output / "images"
    shutil.copytree(ROOT / "rey-compiler/src", stage)
    shutil.copyfile(ROOT / "tests/v04" / suite / "main.rey", stage / "main.rey")
    source_hashes = {str(p.relative_to(stage)): digest(p) for p in sorted(stage.rglob("*.rey"))}
    executable = output / "fixture"
    seed_env = dict(os.environ, PATH="/usr/bin:/bin:/usr/sbin:/sbin")
    built, _ = run("build", [str(SEED), "build", str(stage / "main.rey"), "-o", str(executable), "--runtime-dir", str(RUNTIME)], env=seed_env)
    images.mkdir()
    empty_path = output / "empty-path"
    empty_path.mkdir()
    native_env = dict(os.environ, PATH=str(empty_path))
    for key in ("CC", "CXX", "CLANG", "LD", "LLVM_CONFIG", "REY_RUNTIME_DIR"):
        native_env[key] = str(empty_path / "absent")
    expected = {(name, mode, target) for name in cases for mode in ("default", "reuse")
                for target in ("darwin", "linux", "x64") if not (name == raw_case and target == "x64")}
    stdout = b""
    if built:
        ready, stdout = run("generate", [str(executable), str(images)], env=native_env)
        marker = b"native storage fixtures ready" if args.suite == "storage" else b"native arguments ready"
        record("fixture-marker", ready and marker in stdout)
    parsed = re.findall(rb"^" + suite.encode() + rb"-image ([a-z0-9-]+) (default|reuse) (darwin|linux|x64)$", stdout, re.M)
    found = [tuple(part.decode() for part in entry) for entry in parsed]
    record("manifest", len(found) == len(expected) and set(found) == expected)
    record("image-set", {p.name for p in images.iterdir()} == {"-".join(parts) for parts in expected})
    if args.suite == "storage":
        frames = re.findall(rb"^native-storage-frame chain (default|reuse) ([0-9]+)$", stdout, re.M)
        record("actual-arm64-frame-reduction", sorted(frames) == [(b"default", b"336"), (b"reuse", b"16")],
               observed=[(mode.decode(), int(size)) for mode, size in frames], unit="bytes; not execution timing")
        plans = re.findall(rb"^native-storage-plan chain 0 ([0-9]+) ([0-9]+)$", stdout, re.M)
        record("chain-slot-plan", plans == [(b"41", b"2")])
    else:
        frames = re.findall(rb"^native-arguments-frame large-frame (default|reuse) ([0-9]+)$", stdout, re.M)
        record("actual-large-arm64-frames", sorted(frames) == [(b"default", b"9600"), (b"reuse", b"16")],
               observed=[(mode.decode(), int(size)) for mode, size in frames], unit="bytes; not execution timing")
    linux = {}
    artifacts = {}
    for name, mode, target in sorted(expected):
        key = "-".join((name, mode, target))
        path = images / key
        if not path.is_file():
            record(key + "-present", False)
            continue
        data = path.read_bytes()
        if target == "darwin":
            header_ok = len(data) >= 32 and data[:4] == bytes.fromhex("cffaedfe") and struct.unpack_from("<I", data, 4)[0] == 16777228
        else:
            machine = 62 if target == "x64" else 183
            header_ok = len(data) >= 64 and data[:7] == b"\x7fELF\x02\x01\x01" and struct.unpack_from("<HH", data, 16) == (2, machine)
        record(key + "-header", header_ok)
        artifacts[key] = digest(path)
        if target == "darwin":
            path.chmod(0o755)
            run(key + "-signature", ["/usr/bin/codesign", "--verify", "--strict", str(path)], timeout=10)
            run(key + "-execute", [str(path)], empty=True, timeout=10)
        else:
            linux[key] = path
    if args.linux_docker:
        docker = shutil.which("docker") or "docker"
        inspected, stdout = run("docker-image", [docker, "image", "inspect", "ubuntu:22.04", "--format", "{{.Id}}"])
        image = stdout.decode(errors="replace").strip()
        if inspected and image.startswith("sha256:") and len(linux) == len(expected) - len(cases) * 2:
            payload = io.BytesIO()
            with tarfile.open(fileobj=payload, mode="w") as archive:
                for key, path in linux.items():
                    data = path.read_bytes()
                    item = tarfile.TarInfo(key)
                    item.size, item.mode = len(data), 0o700
                    archive.addfile(item, io.BytesIO(data))
            shell = "tar -xf - -C /work || exit 90; uname -sm; "
            for key in linux:
                shell += "/work/" + key + "; status=$?; printf 'case " + key + " %s\\n' \"$status\"; "
            executed, stdout = run("linux-execute", [docker, "run", "--rm", "--pull", "never", "-i", "--network", "none", "--read-only",
                "--cap-drop", "ALL", "--security-opt", "no-new-privileges", "--memory", "128m", "--pids-limit", "32", "--tmpfs", "/work:exec",
                image, "/bin/sh", "-c", shell], input=payload.getvalue(), timeout=60)
            record("linux-exact-output", executed and stdout.decode(errors="replace").splitlines() == ["Linux aarch64"] + ["case " + key + " 0" for key in linux])
        else:
            record("linux-execute", False, reason="missing image or ELF artifacts")
    report = dict(records=records, passed=all(r["passed"] for r in records), snapshots=source_hashes,
                  artifacts=artifacts, script_sha256=digest(Path(__file__)), linux_requested=args.linux_docker,
                  execution="Darwin arm64 and Linux aarch64 VM; x64 artifacts use user-mode emulation",
                  limitation="fixed slot-frame reduction is not runtime or compiler performance evidence")
    (output / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(suite, "PASS" if report["passed"] else "FAIL", len(records), "checks")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
