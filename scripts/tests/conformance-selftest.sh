#!/usr/bin/env bash

set -u

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P) || exit 2
repo_root=$(cd "$script_dir/../.." && pwd -P) || exit 2
runner="$repo_root/scripts/conformance.sh"
tmpdir=$(mktemp -d "${TMPDIR:-/tmp}/rey-conformance-selftest.XXXXXX") || exit 2
runner_scratch="$tmpdir/runner-scratch"
output_file="$tmpdir/output"
fake_engine="$tmpdir/fake-engine.sh"
child_pid=
SELFTEST_TIMEOUT=
SELFTEST_RC=0
SELFTEST_OUTPUT=

process_is_live() {
  local pid=$1
  local state
  state=$(ps -o stat= -p "$pid" 2>/dev/null | tr -d '[:space:]')
  [[ -n "$state" && "$state" != Z* ]]
}

cleanup() {
  if [[ -n "$child_pid" ]] && process_is_live "$child_pid"; then
    kill -KILL "$child_pid" 2>/dev/null || true
  fi
  rm -rf "$tmpdir"
}
trap cleanup EXIT HUP INT TERM

fail() {
  printf 'FAIL selftest: %s\n' "$1" >&2
  if [[ -s "$output_file" ]]; then
    cat "$output_file" >&2
  fi
  exit 1
}

assert_exit() {
  if [[ "$SELFTEST_RC" != "$1" ]]; then
    fail "expected runner exit $1, got $SELFTEST_RC"
  fi
}

assert_contains() {
  case "$SELFTEST_OUTPUT" in
    *"$1"*) ;;
    *) fail "runner output did not contain: $1" ;;
  esac
}

assert_not_contains() {
  case "$SELFTEST_OUTPUT" in
    *"$1"*) fail "runner output unexpectedly contained: $1" ;;
  esac
}

run_runner() {
  local rc
  if [[ -n "$SELFTEST_TIMEOUT" ]]; then
    REY_CONFORMANCE_SCRATCH="$runner_scratch" CONFORMANCE_TIMEOUT_SECONDS="$SELFTEST_TIMEOUT" "$runner" "$@" > "$output_file" 2>&1
    rc=$?
  else
    REY_CONFORMANCE_SCRATCH="$runner_scratch" "$runner" "$@" > "$output_file" 2>&1
    rc=$?
  fi
  SELFTEST_RC=$rc
  SELFTEST_OUTPUT=$(cat "$output_file")
}

cat > "$fake_engine" <<'ENGINE'
#!/usr/bin/env bash
mode=${FAKE_MODE:-correct}

if [[ "$1" == check ]]; then
  if [[ "$mode" == legacy ]]; then
    printf "error: File not found: 'check'\n" >&2
    exit 1
  fi
  if [[ "$mode" == timeout-check ]]; then
    exec /bin/sleep 30
  fi
  if grep -Fq 'var preflightValue: int = 1;' main.rey; then
    if [[ "$mode" == noisy-preflight ]]; then
      printf 'check passed\n'
    fi
    exit 0
  fi
  if grep -Fq 'var x: int = "s";' main.rey; then
    printf 'error: expected int\n' >&2
    if [[ "$mode" == usage2 ]]; then
      exit 2
    fi
    exit 1
  fi
  if grep -Fq 'println("hello");' main.rey; then
    if [[ "$mode" == noisy ]]; then
      printf 'warning: noisy check\n'
    fi
    exit 0
  fi
  exit 0
fi

if [[ "$1" == run ]]; then
  if [[ -n "${FAKE_RUN_MARKER:-}" ]]; then
    : > "$FAKE_RUN_MARKER"
  fi
  case "$mode" in
    wrong-stdout) printf 'not hello\n' ;;
    wrong-exit) printf 'hello\n'; exit 7 ;;
    timeout)
      /bin/sleep 30 &
      child=$!
      printf '%s\n' "$child" > "$FAKE_CHILD_PID_FILE"
      wait "$child"
      ;;
    *) printf 'hello\n' ;;
  esac
fi
ENGINE
chmod +x "$fake_engine"

mkdir -p "$tmpdir/nonrepo"
cd "$tmpdir/nonrepo" || fail 'could not enter non-repository working directory'

run_runner --engine "$fake_engine" --filter _smoke
assert_exit 0
assert_contains 'Engine preflight PASS'
assert_contains 'PASS _smoke/hello'
assert_contains 'PASS _smoke/type-error'
assert_contains 'Summary: 2 passed, 0 failed, 2 total'
printf 'PASS selftest: correct engine, silent preflight, positive and compile-negative smoke\n'

run_runner --engine "FAKE_MODE=legacy $fake_engine" --filter _smoke
assert_exit 1
assert_contains 'Engine preflight FAIL: check must exit 0 on a valid program; got 1.'
assert_contains "error: File not found: 'check'"
assert_contains 'No fixture cases were counted or run.'
assert_not_contains 'PASS _smoke/'
assert_not_contains 'Summary:'
printf 'PASS selftest: legacy generic-error CLI rejected before fixture counts or passes\n'

run_runner --engine "FAKE_MODE=usage2 $fake_engine" --filter _smoke/type-error
assert_exit 1
assert_contains 'expected compile-error check exit 1, got 2'
assert_contains 'Summary: 0 passed, 1 failed, 1 total'
printf 'PASS selftest: compile-negative usage exit 2 is not accepted\n'

noisy_run_marker="$tmpdir/noisy-run-called"
run_runner --engine "FAKE_MODE=noisy FAKE_RUN_MARKER=$noisy_run_marker $fake_engine" --filter _smoke/hello
assert_exit 1
assert_contains 'successful check must be silent'
assert_contains 'warning: noisy check'
if [[ -e "$noisy_run_marker" ]]; then
  fail 'runner executed a positive case after noisy check output'
fi
printf 'PASS selftest: positive successful check must be silent\n'

run_runner --engine "FAKE_MODE=noisy-preflight $fake_engine" --filter _smoke
assert_exit 1
assert_contains 'Engine preflight FAIL: successful check must be silent.'
assert_contains 'No fixture cases were counted or run.'
printf 'PASS selftest: preflight successful check must be silent\n'

run_runner --engine "FAKE_MODE=wrong-stdout $fake_engine" --filter _smoke/hello
assert_exit 1
assert_contains 'stdout differs byte-for-byte'
assert_contains '+not hello'
printf 'PASS selftest: stdout golden mismatch detected\n'

run_runner --engine "FAKE_MODE=wrong-exit $fake_engine" --filter _smoke/hello
assert_exit 1
assert_contains 'expected run exit 0, got 7'
printf 'PASS selftest: runtime exit mismatch detected\n'

child_pid_file="$tmpdir/timeout-child.pid"
SELFTEST_TIMEOUT=1
run_runner --engine "FAKE_MODE=timeout FAKE_CHILD_PID_FILE=$child_pid_file $fake_engine" --filter _smoke/hello
SELFTEST_TIMEOUT=
assert_exit 1
assert_contains 'run timed out after 1 seconds'
if [[ ! -s "$child_pid_file" ]]; then
  fail 'timeout fake did not record its child pid'
fi
child_pid=$(cat "$child_pid_file")
if [[ ! "$child_pid" =~ ^[0-9]+$ ]] || process_is_live "$child_pid"; then
  fail "timeout left a live child process ($child_pid)"
fi
printf 'PASS selftest: timeout failed the case and left no live child process\n'

SELFTEST_TIMEOUT=1
run_runner --engine "FAKE_MODE=timeout-check $fake_engine" --filter _smoke
SELFTEST_TIMEOUT=
assert_exit 1
assert_contains 'Engine preflight FAIL: check timed out after 1 seconds.'
assert_contains 'No fixture cases were counted or run.'
printf 'PASS selftest: preflight timeout aborts before fixture counts\n'

mkdir -p "$tmpdir/engine path"
cp "$fake_engine" "$tmpdir/engine path/fake engine.sh"
chmod +x "$tmpdir/engine path/fake engine.sh"
run_runner --engine "$tmpdir/engine path/fake engine.sh" --filter _smoke
assert_exit 0
assert_contains 'Summary: 2 passed, 0 failed, 2 total'
printf 'PASS selftest: executable path containing spaces\n'

run_runner --engine "$tmpdir/missing-engine" --filter _smoke
assert_exit 1
assert_contains 'Engine preflight FAIL: check must exit 0 on a valid program; got 127.'
assert_contains 'No fixture cases were counted or run.'
printf 'PASS selftest: missing executable rejected during preflight\n'

run_runner --engine "$fake_engine" --filter definitely-no-such-case
assert_exit 1
assert_contains 'No conformance cases matched'
assert_contains 'empty selection is a failure'
printf 'PASS selftest: empty filter selection is not green\n'

printf 'PASS all conformance runner self-tests\n'
