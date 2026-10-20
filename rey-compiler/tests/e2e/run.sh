#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 --engine <native-rey> --runtime-dir <directory-containing-librey_rt.a>" >&2
  exit 2
}

fail() {
  echo "e2e: $*" >&2
  exit 1
}

engine_arg="${REY_BIN:-}"
runtime_arg=""
engine_option_seen=0
runtime_option_seen=0
while (($#)); do
  case "$1" in
    --engine)
      ((engine_option_seen == 0)) || usage
      (($# >= 2)) || usage
      engine_option_seen=1
      engine_arg=$2
      shift 2
      ;;
    --runtime-dir)
      ((runtime_option_seen == 0)) || usage
      (($# >= 2)) || usage
      runtime_option_seen=1
      runtime_arg=$2
      shift 2
      ;;
    --help|-h)
      echo "usage: $0 --engine <native-rey> --runtime-dir <directory-containing-librey_rt.a>"
      exit 0
      ;;
    *)
      usage
      ;;
  esac
done

[[ -n "$engine_arg" ]] || usage
[[ -n "$runtime_arg" ]] || usage
if [[ "$engine_arg" == */* ]]; then
  [[ -x "$engine_arg" ]] || fail "engine is missing or not executable: $engine_arg"
  engine="$(cd "$(dirname "$engine_arg")" && pwd)/$(basename "$engine_arg")"
else
  engine_path="$(command -v "$engine_arg" || true)"
  [[ -n "$engine_path" && -x "$engine_path" ]] || fail "engine is missing or not executable: $engine_arg"
  engine=$engine_path
fi
[[ -d "$runtime_arg" ]] || fail "runtime directory is missing: $runtime_arg"
runtime_dir="$(cd "$runtime_arg" && pwd)"
[[ -f "$runtime_dir/librey_rt.a" ]] || fail "runtime directory must contain librey_rt.a: $runtime_dir"
timeout_seconds=${REY_E2E_TIMEOUT_SECONDS:-30}
kill_grace_seconds=${REY_E2E_KILL_GRACE_SECONDS:-1}
[[ "$timeout_seconds" =~ ^[1-9][0-9]*$ ]] || fail "REY_E2E_TIMEOUT_SECONDS must be a positive integer"
[[ "$kill_grace_seconds" =~ ^[1-9][0-9]*$ ]] || fail "REY_E2E_KILL_GRACE_SECONDS must be a positive integer"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../../.." && pwd)"
test_dir="$repo_root/rey-compiler/tests/e2e"
tests=(hello math loops functions structs enums imports strings vec collections io)
((${#tests[@]} > 0)) || fail "fixture list is empty"
for test_name in "${tests[@]}"; do
  source="$test_dir/$test_name.rey"
  expected="$test_dir/$test_name.out"
  [[ -s "$source" ]] || fail "missing or empty source fixture: $source"
  [[ -s "$expected" ]] || fail "missing or empty stdout golden: $expected"
done
[[ -s "$test_dir/imports/util.rey" ]] || fail "missing or empty import fixture: $test_dir/imports/util.rey"

[[ -n "${TMPDIR:-}" && -d "$TMPDIR" ]] || fail "TMPDIR must name an external temporary directory"
temp_root="$(cd "$TMPDIR" && pwd)"
case "$temp_root/" in
  "$repo_root/"*) fail "TMPDIR must be outside the repository" ;;
esac
scratch="$(mktemp -d "${temp_root%/}/rey e2e.XXXXXX")"
cleanup() {
  if [[ "${REY_KEEP_E2E_SCRATCH:-0}" == "1" ]]; then
    printf 'e2e: retained scratch at %s\n' "$scratch" >&2
  else
    rm -rf "$scratch"
  fi
}
trap cleanup EXIT

# Monitor mode gives each async engine command a process group we own.
set -m
terminate_group() {
  local group_id=$1
  kill -TERM -- "-$group_id" 2>/dev/null || true
  sleep "$kill_grace_seconds"
  kill -KILL -- "-$group_id" 2>/dev/null || true
}

run_bounded() {
  local label=$1
  local working_dir=$2
  local stdout_file=$3
  local stderr_file=$4
  shift 4
  local timeout_marker="$scratch/$label.timed-out"
  local engine_pid timer_pid actual_status
  engine_timed_out=0
  engine_left_children=0
  (cd "$working_dir" && exec "$engine" "$@") > "$stdout_file" 2> "$stderr_file" &
  engine_pid=$!
  (
    sleep "$timeout_seconds"
    if kill -0 -- "-$engine_pid" 2>/dev/null; then
      : > "$timeout_marker"
      kill -TERM -- "-$engine_pid" 2>/dev/null || true
      sleep "$kill_grace_seconds"
      kill -KILL -- "-$engine_pid" 2>/dev/null || true
    fi
  ) &
  timer_pid=$!

  if wait "$engine_pid"; then
    actual_status=0
  else
    actual_status=$?
  fi

  if [[ -f "$timeout_marker" ]]; then
    engine_timed_out=1
    wait "$timer_pid" 2>/dev/null || true
    if kill -0 -- "-$engine_pid" 2>/dev/null; then
      terminate_group "$engine_pid"
      engine_left_children=1
    fi
    return "$actual_status"
  fi

  kill -TERM -- "-$timer_pid" 2>/dev/null || true
  wait "$timer_pid" 2>/dev/null || true
  if kill -0 -- "-$engine_pid" 2>/dev/null; then
    terminate_group "$engine_pid"
    engine_left_children=1
  fi
  return "$actual_status"
}

printf 'rey 0.3.0\n' > "$scratch/expected-version.stdout"
: > "$scratch/empty"
set +e
run_bounded engine-version "$repo_root" "$scratch/version.stdout" "$scratch/version.stderr" --version
version_status=$?
set -e
if ((engine_timed_out)); then
  echo "e2e: unsupported engine protocol; --version timed out after ${timeout_seconds}s" >&2
  exit 2
elif ((engine_left_children)); then
  echo "e2e: unsupported engine protocol; --version left child processes" >&2
  exit 2
elif ((version_status != 0)) || ! cmp -s "$scratch/expected-version.stdout" "$scratch/version.stdout" || [[ -s "$scratch/version.stderr" ]]; then
  echo "e2e: unsupported engine protocol; expected --version => 'rey 0.3.0' with exit 0" >&2
  cat "$scratch/version.stdout" "$scratch/version.stderr" >&2
  exit 2
fi

assert_command() {
  local label=$1
  local working_dir=$2
  local expected_status=$3
  local expected_stdout=$4
  local expected_stderr=$5
  shift 5
  local stdout_file="$scratch/$label.stdout"
  local stderr_file="$scratch/$label.stderr"
  local actual_status

  set +e
  run_bounded "$label" "$working_dir" "$stdout_file" "$stderr_file" "$@"
  actual_status=$?
  set -e
  if ((engine_timed_out)); then
    echo "e2e: $label timed out after ${timeout_seconds}s; its process group was terminated" >&2
    cat "$stderr_file" >&2
    return 1
  elif ((engine_left_children)); then
    echo "e2e: $label left child processes after exit; its process group was terminated" >&2
    cat "$stderr_file" >&2
    return 1
  elif ((actual_status != expected_status)); then
    echo "e2e: $label exited $actual_status (expected $expected_status)" >&2
    diff -u "$expected_stdout" "$stdout_file" >&2 || true
    cat "$stderr_file" >&2
    return 1
  fi
  if ! cmp -s "$expected_stdout" "$stdout_file"; then
    echo "e2e: $label stdout differs byte-for-byte" >&2
    diff -u "$expected_stdout" "$stdout_file" >&2 || true
    return 1
  fi
  if ! cmp -s "$expected_stderr" "$stderr_file"; then
    echo "e2e: $label stderr differs byte-for-byte" >&2
    diff -u "$expected_stderr" "$stderr_file" >&2 || true
    return 1
  fi
}

for test_name in "${tests[@]}"; do
  source="$test_dir/$test_name.rey"
  expected="$test_dir/$test_name.out"
  case_dir="$scratch/$test_name"
  mkdir -p "$case_dir"
  source_copy="$case_dir/$test_name.rey"
  expected_copy="$case_dir/$test_name.out"
  cp "$source" "$source_copy"
  cp "$expected" "$expected_copy"
  if [[ "$test_name" == "imports" ]]; then
    cp -R "$test_dir/imports" "$case_dir/imports"
  fi

  : > "$case_dir/expected-empty"
  assert_command "$test_name-check" "$case_dir" 0 "$case_dir/expected-empty" "$case_dir/expected-empty" \
    check "$source_copy" --runtime-dir "$runtime_dir"

  build_output="$case_dir/$test_name built"
  printf '%s\n' "$build_output" > "$case_dir/expected-build.stdout"
  assert_command "$test_name-build" "$case_dir" 0 "$case_dir/expected-build.stdout" "$case_dir/expected-empty" \
    build "$source_copy" -o "$build_output" --runtime-dir "$runtime_dir"
  [[ -x "$build_output" ]] || fail "$test_name build did not create its scratch executable"

  run_output="$case_dir/$test_name run"
  assert_command "$test_name-run" "$case_dir" 0 "$expected_copy" "$case_dir/expected-empty" \
    run "$source_copy" -o "$run_output" --runtime-dir "$runtime_dir"
  [[ -x "$run_output" ]] || fail "$test_name run did not create its scratch executable"
  printf 'PASS %s\n' "$test_name"
done

echo "e2e passed: ${#tests[@]} fixtures"
