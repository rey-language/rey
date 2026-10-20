#!/usr/bin/env bash
set -euo pipefail

usage() {
  printf 'Usage: bash tests/native-parser/run.sh --engine /path/to/rey-v0 [--timeout-seconds N]\n' >&2
}

engine=
timeout_seconds=30
while (($#)); do
  case "$1" in
    --engine)
      (($# >= 2)) || { usage; exit 2; }
      engine=$2
      shift 2
      ;;
    --timeout-seconds)
      (($# >= 2)) || { usage; exit 2; }
      timeout_seconds=$2
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage
      exit 2
      ;;
  esac
done

if [[ -z "$engine" || ! "$timeout_seconds" =~ ^[1-9][0-9]*$ ]]; then
  usage
  exit 2
fi

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
repo_root=$(cd "$script_dir/../../.." && pwd -P)
scratch_root=${REY_NATIVE_PARSER_SCRATCH:-/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/native-parser}
mkdir -p "$scratch_root"

python3 - "$engine" "$timeout_seconds" "$repo_root" "$scratch_root" <<'PY'
import hashlib
import json
import os
import pathlib
import signal
import subprocess
import sys
import tempfile
import datetime

requested, timeout_text, repo_text, scratch_text = sys.argv[1:]
timeout = int(timeout_text)
repo = pathlib.Path(repo_text)
scratch = pathlib.Path(scratch_text)
engine = pathlib.Path(requested).expanduser()
if not engine.is_absolute():
    engine = pathlib.Path.cwd() / engine
try:
    engine = engine.resolve(strict=True)
except FileNotFoundError:
    print(f"BLOCKED engine executable not found: {requested}", file=sys.stderr)
    raise SystemExit(2)
if not engine.is_file() or not os.access(engine, os.X_OK):
    print(f"BLOCKED engine is not an executable file: {engine}", file=sys.stderr)
    raise SystemExit(2)

log_dir = pathlib.Path(tempfile.mkdtemp(prefix="run.", dir=scratch))
version_out = log_dir / "version.stdout"
version_err = log_dir / "version.stderr"
run_out = log_dir / "harness.stdout"
run_err = log_dir / "harness.stderr"

def run_bounded(argv, cwd, out_path, err_path):
    with out_path.open("wb") as out, err_path.open("wb") as err:
        proc = subprocess.Popen(argv, cwd=cwd, stdout=out, stderr=err, start_new_session=True)
        try:
            status = proc.wait(timeout=timeout)
            try:
                os.killpg(proc.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            return status, False
        except subprocess.TimeoutExpired:
            try:
                os.killpg(proc.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                proc.wait(timeout=1)
            except subprocess.TimeoutExpired:
                try:
                    os.killpg(proc.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                proc.wait()
            return None, True

digest = hashlib.sha256(engine.read_bytes()).hexdigest()
engine_mtime = engine.stat().st_mtime
print(f"ENGINE {engine} sha256={digest}")
version_status, version_timeout = run_bounded([str(engine), "--version"], repo, version_out, version_err)
if version_timeout:
    print(f"BLOCKED engine --version timed out after {timeout}s; child process group killed; logs={log_dir}", file=sys.stderr)
    raise SystemExit(2)
version = version_out.read_text(errors="replace").strip()
print(f"ENGINE_VERSION {version or '(empty)'}")
if version_status != 0 or "rey-v0" not in version:
    print(f"BLOCKED expected a rey-v0 oracle (--version exit={version_status}); logs={log_dir}", file=sys.stderr)
    if version_err.exists():
        sys.stderr.write(version_err.read_text(errors="replace"))
    raise SystemExit(2)

rust_sources = list((repo / "compiler/v1/src").rglob("*.rs"))
newest_source = max((p.stat().st_mtime for p in rust_sources), default=0)
newest_path = max(rust_sources, key=lambda p: p.stat().st_mtime) if rust_sources else None
if engine_mtime < newest_source:
    print(
        f"BLOCKED stale oracle: executable mtime={datetime.datetime.fromtimestamp(engine_mtime).astimezone().isoformat(timespec='seconds')}; "
        f"newest Rust source={newest_path} mtime={datetime.datetime.fromtimestamp(newest_source).astimezone().isoformat(timespec='seconds')}; "
        f"no harness cases run; logs={log_dir}",
        file=sys.stderr,
    )
    raise SystemExit(2)

source_root = (repo / "rey-compiler/src").resolve(strict=True)
stage_root = (log_dir / "project-root").resolve()
stage_root.mkdir()

def contains(root, path):
    try:
        path.relative_to(root)
        return True
    except ValueError:
        return False

if contains(source_root, stage_root) or contains(stage_root, source_root):
    print(f"BLOCKED staging root is not source-disjoint: {stage_root} vs {source_root}", file=sys.stderr)
    raise SystemExit(2)

project_root = stage_root / "rey-compiler"
project_root.mkdir()
source_link = project_root / "src"
if source_link.exists() or source_link.is_symlink():
    print(f"BLOCKED staging source link already exists: {source_link}", file=sys.stderr)
    raise SystemExit(2)
os.symlink(str(source_root), str(source_link), target_is_directory=True)
if (
    not source_link.is_symlink()
    or os.readlink(source_link) != str(source_root)
    or source_link.resolve(strict=True) != source_root
):
    print(f"BLOCKED staging source link target mismatch: {source_link}", file=sys.stderr)
    raise SystemExit(2)

harness = (repo / "rey-compiler/tests/native-parser/harness.rey").resolve(strict=True)
entry = project_root / "native-parser-entry.rey"
if entry.exists() or entry.is_symlink():
    print(f"BLOCKED staging entry already exists: {entry}", file=sys.stderr)
    raise SystemExit(2)
entry.write_text(
    f"import {json.dumps(str(harness))};\n"
    "\n"
    "func main(): void {\n"
    "  runTests();\n"
    "}\n",
    encoding="utf-8",
)
if contains(repo, entry.resolve(strict=True)):
    print(f"BLOCKED staging entry is inside the source repository: {entry}", file=sys.stderr)
    raise SystemExit(2)

print(f"PROJECT_ROOT {project_root}")
print(f"SOURCE_LINK {source_link} -> {os.readlink(source_link)}")
print("SOURCE_DEST_DISJOINT true")
print(f"ROOT_ENTRY {entry}")
command = [str(engine), "run", str(entry)]
status, timed_out = run_bounded(command, repo, run_out, run_err)
stdout = run_out.read_text(errors="replace")
stderr = run_err.read_text(errors="replace")
if stdout:
    sys.stdout.write(stdout)
if stderr:
    sys.stderr.write(stderr)
if timed_out:
    print(f"BLOCKED harness timed out after {timeout}s; child process group killed; logs={log_dir}", file=sys.stderr)
    raise SystemExit(124)
if status != 0:
    print(f"FAIL harness engine exit={status}; logs={log_dir}", file=sys.stderr)
    raise SystemExit(status or 1)
print(f"HARNESS_EXIT 0 logs={log_dir}")
PY
