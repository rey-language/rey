#!/bin/bash

set -u

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P) || exit 2
repo_root=$(cd "$script_dir/../.." && pwd -P) || exit 2
scratch=/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03
mkdir -p "$scratch" || exit 2
work=$(mktemp -d "$scratch/differential-selftest.XXXXXX") || exit 2
test_repo="$work/repo"
runner="$test_repo/scripts/differential.sh"
output_file="$work/output"
test_number=0

cleanup() {
    if [[ -n "${timeout_child:-}" ]]; then
        state=$(ps -o stat= -p "$timeout_child" 2>/dev/null | tr -d '[:space:]')
        if [[ -n "$state" && "$state" != Z* ]]; then
            kill -KILL "$timeout_child" 2>/dev/null || true
        fi
    fi
    rm -rf "$work"
}
trap cleanup EXIT HUP INT TERM

fail() {
    printf 'FAIL differential selftest: %s\n' "$1" >&2
    if [[ -s "$output_file" ]]; then
        cat "$output_file" >&2
    fi
    exit 1
}

run_diff() {
    expected_status=$1
    expected_text=$2
    shift 2
    test_number=$((test_number + 1))
    if "$runner" "$@" > "$output_file" 2>&1; then
        actual_status=0
    else
        actual_status=$?
    fi
    if [[ "$actual_status" -ne "$expected_status" ]]; then
        fail "test $test_number expected exit $expected_status, got $actual_status"
    fi
    if [[ -n "$expected_text" ]] && ! grep -F -q -e "$expected_text" "$output_file"; then
        fail "test $test_number output missing: $expected_text"
    fi
}

passed() {
    printf 'PASS differential selftest: %s\n' "$1"
}

mkdir -p "$test_repo/scripts" \
    "$test_repo/tests/conformance/smoke/positive" \
    "$test_repo/tests/conformance/errors/negative" \
    "$test_repo/tests/conformance/smoke/exit-divergent"
cp "$repo_root/scripts/differential.sh" "$runner" || fail 'could not copy differential runner'
chmod +x "$runner"

cat > "$test_repo/tests/conformance/smoke/positive/main.rey" <<'REY'
println("hello");
REY
printf 'hello\n' > "$test_repo/tests/conformance/smoke/positive/expected.stdout"

cat > "$test_repo/tests/conformance/errors/negative/main.rey" <<'REY'
var x: int = "s";
REY
: > "$test_repo/tests/conformance/errors/negative/expected.stdout"
printf 'type mismatch\n' > "$test_repo/tests/conformance/errors/negative/expect_check_fail"

cat > "$test_repo/tests/conformance/smoke/exit-divergent/main.rey" <<'REY'
println("exit-check");
REY
printf 'exit-check\n' > "$test_repo/tests/conformance/smoke/exit-divergent/expected.stdout"

engine="$work/fake-engine"
cat > "$engine" <<'ENGINE'
#!/bin/sh
mode=${FAKE_MODE:-normal}
action=$1
source=$2

if [ -n "${FAKE_CALL_LOG:-}" ]; then
    case "$source" in
        */negative/main.rey) case_name=negative ;;
        */positive/main.rey) case_name=positive ;;
        */exit-divergent/main.rey) case_name=exit-divergent ;;
        */preflight-*/main.rey) case_name=preflight ;;
        *) case_name=other ;;
    esac
    printf '%s %s\n' "$action" "$case_name" >> "$FAKE_CALL_LOG"
fi

if [ "$action" = check ]; then
    case "$mode" in
        legacy)
            echo 'error: unknown command; expected a file' >&2
            exit 2
            ;;
        noisy-preflight)
            echo 'check passed'
            exit 0
            ;;
    esac
    if grep -F -q 'var preflightValue: int = 1;' "$source"; then
        exit 0
    fi
    if grep -F -q 'var x: int = "s";' "$source"; then
        echo 'type mismatch: expected int' >&2
        case "$mode" in
            usage2) exit 2 ;;
            signal) kill -TERM $$ ;;
        esac
        exit 1
    fi
    if [ "$mode" = noisy-positive ] && grep -F -q 'println("hello");' "$source"; then
        echo 'warning: noisy successful check'
    fi
    exit 0
fi

if [ "$action" = run ]; then
    if grep -F -q 'exit-check' "$source"; then
        printf 'exit-check\n'
        [ "$mode" = wrong-exit ] && exit 7
        exit 0
    fi
    case "$mode" in
        wrong-output) printf 'not hello\n' ;;
        timeout)
            /bin/sleep 120 &
            child=$!
            printf '%s\n' "$child" > "$FAKE_CHILD_PID_FILE"
            wait "$child"
            ;;
        *) printf 'hello\n' ;;
    esac
fi
ENGINE
chmod +x "$engine"

good="/usr/bin/env FAKE_MODE=normal $engine"

call_log="$work/calls.log"
: > "$call_log"
run_diff 0 'Pass count: 3/3' --a "/usr/bin/env FAKE_MODE=normal FAKE_CALL_LOG=$call_log $engine" \
    --b "/usr/bin/env FAKE_MODE=normal FAKE_CALL_LOG=$call_log $engine"
if grep -F -q 'run negative' "$call_log"; then
    fail 'compile-negative fixture was run instead of check-only'
fi
passed 'valid identity and negative check-only behavior'

legacy="/usr/bin/env FAKE_MODE=legacy $engine"
run_diff 1 'No conformance cases were counted or run.' --a "$legacy" --b "$legacy" --filter negative
if ! grep -F -q 'Engine preflight FAIL: engine-a' "$output_file" || \
   ! grep -F -q 'Engine preflight FAIL: engine-b' "$output_file" || \
   grep -F -q 'Pass count:' "$output_file"; then
    fail 'identical legacy errors passed preflight or counted fixtures'
fi
passed 'identical legacy errors rejected by both engine preflights'

usage2="/usr/bin/env FAKE_MODE=usage2 $engine"
run_diff 1 'check=exit:2 (expected exit 1)' --a "$usage2" --b "$usage2" --filter negative
passed 'negative check usage exit 2 rejected'

noisy="/usr/bin/env FAKE_MODE=noisy-preflight $engine"
run_diff 1 'successful check must be silent' --a "$noisy" --b "$noisy" --filter negative
run_diff 1 'No conformance cases were counted or run.' --a "$noisy" --b "$noisy" --filter negative
passed 'noisy successful check rejected before fixture counting'

positive_call_log="$work/noisy-positive.log"
run_diff 1 'check=noisy (expected silent success)' --a "/usr/bin/env FAKE_MODE=noisy-positive FAKE_CALL_LOG=$positive_call_log $engine" \
    --b "/usr/bin/env FAKE_MODE=noisy-positive FAKE_CALL_LOG=$positive_call_log $engine" --filter positive
if grep -F -q 'run positive' "$positive_call_log"; then
    fail 'positive fixture ran after a noisy successful check'
fi
passed 'noisy successful positive check fails without running the case'

wrong="/usr/bin/env FAKE_MODE=wrong-output $engine"
run_diff 1 'stdout=mismatch,parity=match' --a "$wrong" --b "$wrong" --filter positive
passed 'matching wrong stdout still fails the golden fixture'

run_diff 1 'parity=different' --a "$good" --b "$wrong" --filter positive
passed 'divergent stdout reported'

wrong_exit="/usr/bin/env FAKE_MODE=wrong-exit $engine"
run_diff 1 'run=7,stdout=match,parity=different' --a "$good" --b "$wrong_exit" --filter exit-divergent
passed 'divergent runtime exit reported'

run_diff 1 'check=exit:143 (expected exit 1)' --a "/usr/bin/env FAKE_MODE=signal $engine" \
    --b "/usr/bin/env FAKE_MODE=signal $engine" --filter negative
passed 'negative check signal rejected'

mkdir -p "$work/engine path"
spaced_engine="$work/engine path/fake engine"
cp "$engine" "$spaced_engine"
chmod +x "$spaced_engine"
run_diff 0 'Pass count: 3/3' --a "$spaced_engine" --b "$spaced_engine"
passed 'engine executable path containing spaces'

run_diff 2 'no conformance cases matched filter' --a "$good" --b "$good" --filter no-such-case
passed 'empty fixture selection rejected'

missing="$work/not-an-engine"
run_diff 2 "missing engine binary for --b: $missing" --a "$good" --b "$missing" --filter negative
passed 'missing engine binary rejected before fixture counting'

timeout_pid_file="$work/timeout-child.pid"
timeout_engine="/usr/bin/env FAKE_MODE=timeout FAKE_CHILD_PID_FILE=$timeout_pid_file $engine"
run_diff 1 'run=timeout,parity=not-compared' --a "$good" --b "$timeout_engine" --filter positive
if [[ ! -s "$timeout_pid_file" ]]; then
    fail 'timeout engine did not record its child process'
fi
timeout_child=$(cat "$timeout_pid_file")
if [[ ! "$timeout_child" =~ ^[0-9]+$ ]]; then
    fail 'timeout engine recorded an invalid child pid'
fi
child_state=$(ps -o stat= -p "$timeout_child" 2>/dev/null | tr -d '[:space:]')
if [[ -n "$child_state" && "$child_state" != Z* ]]; then
    fail "timeout left a live child process ($timeout_child)"
fi
passed '30-second timeout terminates the engine process group'

printf 'PASS all differential self-tests\n'
