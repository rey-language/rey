#!/bin/sh

set -u

SCRIPT_DIR=$(CDPATH= cd "$(dirname "$0")" 2>/dev/null && pwd) || exit 2
REPO_ROOT=$(CDPATH= cd "$SCRIPT_DIR/.." 2>/dev/null && pwd) || exit 2
SCRATCH=/tmp/rey-v03

echo '=== Rey v0.3 machine gate (transient peer state possible) ==='

mkdir -p "$SCRATCH" || { echo "FAIL: cannot create scratch directory: $SCRATCH" >&2; exit 2; }
TMPDIR="$SCRATCH"
export TMPDIR
ci_work=$(mktemp -d "$SCRATCH/ci.XXXXXX") || { echo 'FAIL: cannot create CI work directory' >&2; exit 2; }
failures=0
skips=0
build_ok=0
v1_bin="$ci_work/cargo-target/release/rey-v0"

run_bounded_command() {
    timeout_seconds=$1
    timeout_marker=$2
    shift 2
    perl - "$timeout_seconds" "$timeout_marker" "$@" <<'PERL'
use POSIX qw(setpgid WNOHANG);
use Time::HiRes qw(time sleep);

my ($timeout, $marker, @command) = @ARGV;
my $pid = fork();
defined $pid or die "fork failed: $!\n";
if ($pid == 0) {
    setpgid(0, 0) == 0 or exit 125;
    exec @command or die "exec failed: $!\n";
}

my $deadline = time() + $timeout;
while (1) {
    my $waited = waitpid($pid, WNOHANG);
    if ($waited == $pid) {
        my $status = $?;
        exit(($status & 127) ? 128 + ($status & 127) : ($status >> 8));
    }
    if (time() >= $deadline) {
        kill 'TERM', -$pid;
        my $grace_deadline = time() + 1;
        while (time() < $grace_deadline) {
            last if waitpid($pid, WNOHANG) == $pid;
            sleep(0.05);
        }
        kill 'KILL', -$pid;
        waitpid($pid, 0);
        open my $timeout_file, '>', $marker;
        close $timeout_file if $timeout_file;
        exit 124;
    }
    sleep(0.05);
}
PERL
}

run_persistent_gate() {
    gate_name=$1
    gate_path=$2
    gate_shell=$3
    gate_timeout=${4:-}
    if [ ! -f "$gate_path" ]; then
        echo "SKIP: $gate_name (missing: $gate_path)"
        skips=$((skips + 1))
        return 0
    fi
    echo "--- persistent gate: $gate_name ---"
    gate_log="$ci_work/$gate_name.log"
    if [ -n "$gate_timeout" ]; then
        gate_timeout_marker="$ci_work/$gate_name.timeout"
        if [ -x "$gate_path" ]; then
            run_bounded_command "$gate_timeout" "$gate_timeout_marker" "$gate_path" > "$gate_log" 2>&1
        elif [ "$gate_shell" = bash ]; then
            run_bounded_command "$gate_timeout" "$gate_timeout_marker" bash "$gate_path" > "$gate_log" 2>&1
        else
            run_bounded_command "$gate_timeout" "$gate_timeout_marker" sh "$gate_path" > "$gate_log" 2>&1
        fi
    elif [ -x "$gate_path" ]; then
        "$gate_path" > "$gate_log" 2>&1
    elif [ "$gate_shell" = bash ]; then
        bash "$gate_path" > "$gate_log" 2>&1
    else
        sh "$gate_path" > "$gate_log" 2>&1
    fi
    gate_status=$?
    cat "$gate_log"
    if [ "$gate_status" -eq 0 ]; then
        echo "PASS: $gate_name"
    elif [ -n "$gate_timeout" ] && [ -f "$gate_timeout_marker" ]; then
        echo "FAIL: $gate_name timed out after ${gate_timeout}s (exit $gate_status)"
        failures=$((failures + 1))
    else
        if grep -F -q 'overall: INCOMPLETE' "$gate_log"; then
            echo "FAIL: $gate_name (aggregate INCOMPLETE, exit $gate_status)"
        elif grep -F -q 'overall: FAIL' "$gate_log"; then
            echo "FAIL: $gate_name (aggregate FAIL, exit $gate_status)"
        else
            echo "FAIL: $gate_name (exit $gate_status)"
        fi
        failures=$((failures + 1))
    fi
}

echo '--- (1) environment sanity ---'
clang_path=$(which clang 2>/dev/null || true)
if [ "$clang_path" != /usr/bin/clang ]; then
    if [ -n "$clang_path" ]; then
        echo "WARN: which clang resolves to $clang_path, not /usr/bin/clang; using /usr/bin/clang"
    else
        echo 'WARN: clang is missing from PATH; using /usr/bin/clang if available'
    fi
    if [ -x /usr/bin/clang ]; then
        PATH="/usr/bin:$PATH"
        export PATH
        CC=/usr/bin/clang
        export CC
    else
        echo 'WARN: /usr/bin/clang is unavailable; build/toolchain may fail'
    fi
else
    CC=/usr/bin/clang
    export CC
fi

echo '--- (1b) tiered quick gate ---'
if [ -f "$REPO_ROOT/scripts/gate.sh" ]; then
    quick_log="$ci_work/gate-quick.log"
    quick_timeout_marker="$ci_work/gate-quick.timeout"
    if run_bounded_command 600 "$quick_timeout_marker" bash "$REPO_ROOT/scripts/gate.sh" quick > "$quick_log" 2>&1; then
        cat "$quick_log"
        echo 'PASS: gate quick'
    else
        quick_status=$?
        cat "$quick_log"
        if [ -f "$quick_timeout_marker" ]; then
            echo 'FAIL: gate quick timed out after 600s'
            failures=$((failures + 1))
        elif [ "$quick_status" -eq 2 ]; then
            echo 'SKIP: gate quick (incomplete; not a full release pass)'
            skips=$((skips + 1))
        else
            echo "FAIL: gate quick (exit $quick_status)"
            failures=$((failures + 1))
        fi
    fi
else
    echo "SKIP: gate quick (missing: $REPO_ROOT/scripts/gate.sh)"
    skips=$((skips + 1))
fi

echo '--- (2) cargo build --release --jobs 1 (isolated CARGO_TARGET_DIR) ---'
if (cd "$REPO_ROOT/compiler/v1" && CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR="$ci_work/cargo-target" cargo build --release --jobs 1) > "$ci_work/build.log" 2>&1; then
    cat "$ci_work/build.log"
    build_ok=1
else
    build_status=$?
    echo "FAIL: cargo build --release (exit $build_status); last 60 lines follow:"
    tail -n 60 "$ci_work/build.log"
    failures=$((failures + 1))
fi

echo '--- (2a) cargo test --release --jobs 1 (bounded 300s) ---'
test_log="$ci_work/cargo-test.log"
test_timeout_marker="$ci_work/cargo-test.timeout"
if (cd "$REPO_ROOT/compiler/v1" && run_bounded_command 300 "$test_timeout_marker" env CARGO_TARGET_DIR="$ci_work/cargo-target" cargo test --release --jobs 1) > "$test_log" 2>&1; then
    cat "$test_log"
    echo 'PASS: cargo test'
else
    test_status=$?
    if [ -f "$test_timeout_marker" ]; then
        echo 'FAIL: cargo test timed out after 300 seconds; last 60 lines follow:'
    else
        echo "FAIL: cargo test (exit $test_status); last 60 lines follow:"
    fi
    tail -n 60 "$test_log"
    failures=$((failures + 1))
fi

echo '--- (2b) persistent harness self-tests ---'
run_persistent_gate conformance-selftest "$REPO_ROOT/scripts/tests/conformance-selftest.sh" bash 120
run_persistent_gate differential-selftest "$REPO_ROOT/scripts/tests/differential-selftest.sh" bash 120
run_persistent_gate native-e2e-runner-selftest "$REPO_ROOT/rey-compiler/tests/e2e/runner-selftest.sh" bash 120

echo '--- (2c) aggregate C runtime gate (default: normal + sanitize) ---'
run_persistent_gate runtime-aggregate "$REPO_ROOT/scripts/test-runtime.sh" sh
run_persistent_gate runtime-float-format "$REPO_ROOT/scripts/test-runtime-float-format.sh" sh 120

echo '--- (3) conformance ---'
if [ "$build_ok" -eq 1 ]; then
    if [ -f "$REPO_ROOT/scripts/conformance.sh" ]; then
        "$REPO_ROOT/scripts/conformance.sh" --engine "$v1_bin"
        conformance_status=$?
        if [ "$conformance_status" -eq 0 ]; then
            echo 'PASS: conformance'
        else
            echo "FAIL: conformance (exit $conformance_status)"
            failures=$((failures + 1))
        fi
    else
        echo "FAIL: conformance runner missing: $REPO_ROOT/scripts/conformance.sh"
        skips=$((skips + 1))
    fi
else
    echo 'SKIP: conformance (oracle build failed; no freshly built engine)'
fi

echo '--- (4) native differential ---'
native_bin=${REY_BIN:-$REPO_ROOT/rey-compiler/main}
case "$native_bin" in
    /*) ;;
    *) native_bin="$REPO_ROOT/$native_bin" ;;
esac
if [ -x "$native_bin" ]; then
    if [ "$build_ok" -ne 1 ]; then
        echo 'SKIP: native differential (oracle build failed; no freshly built engine)'
    elif [ -f "$REPO_ROOT/scripts/differential.sh" ]; then
        "$REPO_ROOT/scripts/differential.sh" --a "$v1_bin" --b "$native_bin"
        differential_status=$?
        if [ "$differential_status" -eq 0 ]; then
            echo 'PASS: native differential'
        else
            echo "FAIL: native differential (exit $differential_status)"
            failures=$((failures + 1))
        fi
    else
        echo "SKIP: native differential (runner missing: $REPO_ROOT/scripts/differential.sh; NOT a full release pass)"
        skips=$((skips + 1))
    fi
else
    echo "SKIP: native differential ($native_bin not found/executable; native compiler is being rebuilt in Phase B; this is NOT a full release pass)"
    skips=$((skips + 1))
fi

echo '--- (5) native E2E behavior ---'
native_runtime_dir=${REY_RUNTIME_DIR:-$REPO_ROOT/build/runtime}
case "$native_runtime_dir" in
    /*) ;;
    *) native_runtime_dir="$REPO_ROOT/$native_runtime_dir" ;;
esac
e2e_runner="$REPO_ROOT/rey-compiler/tests/e2e/run.sh"
if [ ! -x "$native_bin" ]; then
    echo "SKIP: native E2E (native binary missing/not executable: $native_bin; NOT a full release pass)"
    skips=$((skips + 1))
elif [ ! -d "$native_runtime_dir" ] || [ ! -f "$native_runtime_dir/librey_rt.a" ]; then
    echo "SKIP: native E2E (runtime archive missing: $native_runtime_dir/librey_rt.a; set REY_RUNTIME_DIR to its containing directory; NOT a full release pass)"
    skips=$((skips + 1))
elif [ ! -f "$e2e_runner" ]; then
    echo "SKIP: native E2E (runner missing: $e2e_runner; NOT a full release pass)"
    skips=$((skips + 1))
else
    echo "native E2E config: engine=$native_bin runtime-dir=$native_runtime_dir REY_E2E_TIMEOUT_SECONDS=${REY_E2E_TIMEOUT_SECONDS:-30}"
    e2e_log="$ci_work/native-e2e.log"
    e2e_timeout_marker="$ci_work/native-e2e.timeout"
    if run_bounded_command 1200 "$e2e_timeout_marker" env TMPDIR="$SCRATCH" \
        "$e2e_runner" --engine "$native_bin" --runtime-dir "$native_runtime_dir" > "$e2e_log" 2>&1; then
        cat "$e2e_log"
        echo 'PASS: native E2E behavior'
    else
        e2e_status=$?
        if [ -f "$e2e_timeout_marker" ]; then
            echo 'FAIL: native E2E behavior exceeded the 1200-second aggregate bound; last 60 lines follow:'
        else
            echo "FAIL: native E2E behavior (exit $e2e_status); last 60 lines follow:"
        fi
        tail -n 60 "$e2e_log"
        failures=$((failures + 1))
    fi
fi

echo '=== CI summary (transient peer state possible) ==='
if [ "$failures" -ne 0 ]; then
    echo "CI RESULT: FAIL ($failures real failure(s))"
    exit 1
fi
if [ "$skips" -ne 0 ]; then
    echo "CI RESULT: INCOMPLETE ($skips gate(s) skipped); NOT a full release pass"
    exit 2
else
    echo 'CI RESULT: PASS (all configured gates passed)'
fi
