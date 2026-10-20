#!/bin/sh
set -eu
LC_ALL=C
export LC_ALL

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd -P)
runtime_dir=$repo_root/rey-compiler/runtime
compiler=${CC:-/usr/bin/clang}
scratch_root=/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03
total_timeout=${REY_RUNTIME_TOTAL_TIMEOUT_SECONDS:-900}
gate_timeout=${REY_RUNTIME_GATE_TIMEOUT_SECONDS:-120}
gates="hashmap string vec io result stderr typed-value value overflow matrix float-format"

case "$total_timeout" in ''|*[!0-9]*|0) echo "total timeout must be a positive integer" >&2; exit 2 ;; esac
case "$gate_timeout" in ''|*[!0-9]*|0) echo "gate timeout must be a positive integer" >&2; exit 2 ;; esac
if [ "$total_timeout" -gt 86400 ] || [ "$gate_timeout" -gt 3600 ]; then
    echo "timeout exceeds supported limit" >&2
    exit 2
fi
if ! command -v "$compiler" >/dev/null 2>&1; then
    echo "C compiler not found: $compiler" >&2
    exit 2
fi
if [ ! -x /usr/bin/perl ] || [ ! -x /usr/bin/shasum ]; then
    echo "required system tools /usr/bin/perl and /usr/bin/shasum are unavailable" >&2
    exit 2
fi

case "${1:-}" in
    "") modes="normal sanitize" ;;
    --normal) modes=normal ;;
    --sanitize) modes=sanitize ;;
    *) echo "usage: $0 [--normal|--sanitize]" >&2; exit 2 ;;
esac
if [ "$#" -gt 1 ]; then echo "usage: $0 [--normal|--sanitize]" >&2; exit 2; fi

mkdir -p "$scratch_root"
scratch_root=$(CDPATH= cd -- "$scratch_root" && pwd -P)
case "$scratch_root" in
    "$repo_root"|"$repo_root"/*) echo "refusing temp output root inside source tree: $scratch_root" >&2; exit 2 ;;
esac
work_dir=$(mktemp -d "$scratch_root/runtime-gate.XXXXXX")
case "$work_dir" in
    "$scratch_root"/runtime-gate.*) ;;
    *) echo "unexpected temp output path: $work_dir" >&2; exit 2 ;;
esac
cleanup() {
    case "$work_dir" in
        "$scratch_root"/runtime-gate.*) rm -rf "$work_dir" ;;
        *) echo "refusing to remove unexpected path: $work_dir" >&2; return 1 ;;
    esac
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM

copy_runtime_snapshot() {
    dest=$1
    rm -rf "$dest"
    mkdir -p "$dest/tests"
    cp "$runtime_dir"/*.c "$runtime_dir"/*.h "$dest/"
    cp "$runtime_dir"/tests/*.c "$dest/tests/"
}

hash_snapshot() {
    dir=$1
    ( cd "$dir" && find . -type f -print | sort | xargs /usr/bin/shasum -a 256 ) \
        | /usr/bin/shasum -a 256 | awk '{print $1}'
}

# Copy the runtime once, and again, until the two copies match. A later edit
# of the live tree must not flip this run to INCOMPLETE or mix old and new
# sources in one binary. The quick gate calls this script while other agents
# may still be writing runtime C.
snapshot_dir=$work_dir/src
snapshot_check=$work_dir/src-check
snapshot_stable=0
snapshot_attempt=1
while [ "$snapshot_attempt" -le 5 ]; do
    copy_runtime_snapshot "$snapshot_dir"
    copy_runtime_snapshot "$snapshot_check"
    hash_one=$(hash_snapshot "$snapshot_dir")
    hash_two=$(hash_snapshot "$snapshot_check")
    if [ -n "$hash_one" ] && [ "$hash_one" = "$hash_two" ]; then
        snapshot_stable=1
        break
    fi
    snapshot_attempt=$((snapshot_attempt + 1))
done
rm -rf "$snapshot_check"
if [ "$snapshot_stable" -ne 1 ]; then
    copy_runtime_snapshot "$snapshot_dir"
fi
if [ ! -f "$snapshot_dir/rey_rt.h" ] || [ ! -f "$snapshot_dir/string.c" ]; then
    echo "INCOMPLETE: unable to snapshot runtime sources" >&2
    exit 2
fi
export REY_RUNTIME_DIR=$snapshot_dir

source_fingerprint() {
    hash_snapshot "$snapshot_dir"
}

run_bounded() {
    seconds=$1
    shift
    /usr/bin/perl -e '
        use POSIX qw(:sys_wait_h);
        my $seconds = shift @ARGV;
        my $pid = fork();
        die "gate timeout wrapper fork failed: $!\n" unless defined $pid;
        if ($pid == 0) {
            setpgrp(0, 0) or die "gate timeout wrapper setpgrp failed: $!\n";
            exec { $ARGV[0] } @ARGV or die "gate timeout wrapper exec failed: $!\n";
        }
        $SIG{ALRM} = sub {
            kill "TERM", -$pid;
            kill "KILL", -$pid;
            waitpid($pid, 0);
            print STDERR "gate timed out after $seconds seconds\n";
            exit 124;
        };
        alarm($seconds);
        waitpid($pid, 0);
        alarm(0);
        my $status = $?;
        if (WIFEXITED($status)) { exit WEXITSTATUS($status); }
        if (WIFSIGNALED($status)) { exit 128 + WTERMSIG($status); }
        exit 1;
    ' "$seconds" "$@"
}

if ! initial_fingerprint=$(source_fingerprint); then
    echo "INCOMPLETE: unable to fingerprint runtime C/header sources" >&2
    exit 2
fi
start_epoch=$(/bin/date +%s)
records=$work_dir/gate-records
: > "$records"
source_changed=0
total_expired=0

record_not_run() {
    gate=$1
    mode=$2
    detail=$3
    printf '%s|%s|INCOMPLETE|124|%s\n' "$gate" "$mode" "$detail" >> "$records"
    total_expired=1
}

run_gate() {
    gate=$1
    mode=$2
    gate_script=$script_dir/test-runtime-$gate.sh
    stdout_log=$work_dir/$gate.$mode.stdout
    stderr_log=$work_dir/$gate.$mode.stderr
    before_gate=$(source_fingerprint) || before_gate=unavailable

    now=$(/bin/date +%s)
    remaining=$((total_timeout - (now - start_epoch)))
    if [ "$remaining" -lt 4 ]; then
        record_not_run "$gate" "$mode" "total timeout budget exhausted before gate"
        return
    fi
    this_timeout=$gate_timeout
    if [ "$remaining" -lt "$this_timeout" ]; then this_timeout=$remaining; fi
    inner_timeout=$((this_timeout / 4))
    if [ "$inner_timeout" -lt 1 ]; then inner_timeout=1; fi
    if [ "$inner_timeout" -gt 30 ]; then inner_timeout=30; fi

    if [ "$gate" = hashmap ] || [ "$gate" = vec ]; then
        runner_mode=--$mode
    else
        runner_mode=$mode
    fi

    if [ ! -f "$gate_script" ]; then
        printf 'missing required gate script: %s\n' "$gate_script" > "$stderr_log"
        exit_code=127
    else
        if [ "$mode" = sanitize ]; then
            if run_bounded "$this_timeout" /usr/bin/env \
                REY_RUNTIME_DIR="$snapshot_dir" \
                REY_TEST_TMPDIR="$work_dir" \
                REY_RUNTIME_TEST_TIMEOUT_SECONDS="$inner_timeout" \
                ASAN_OPTIONS=detect_leaks=0 UBSAN_OPTIONS=halt_on_error=1 \
                /bin/sh "$gate_script" "$runner_mode" >"$stdout_log" 2>"$stderr_log"; then
                exit_code=0
            else
                exit_code=$?
            fi
        else
            if run_bounded "$this_timeout" /usr/bin/env \
                REY_RUNTIME_DIR="$snapshot_dir" \
                REY_TEST_TMPDIR="$work_dir" \
                REY_RUNTIME_TEST_TIMEOUT_SECONDS="$inner_timeout" \
                /bin/sh "$gate_script" "$runner_mode" >"$stdout_log" 2>"$stderr_log"; then
                exit_code=0
            else
                exit_code=$?
            fi
        fi
    fi

    after_gate=$(source_fingerprint) || after_gate=unavailable
    if [ "$before_gate" = unavailable ] || [ "$after_gate" = unavailable ] || [ "$before_gate" != "$after_gate" ]; then
        source_changed=1
        state=INCOMPLETE
        detail="runtime source fingerprint changed/unavailable during gate"
    elif [ "$exit_code" -eq 0 ]; then
        state=PASS
        detail=ok
    else
        state=FAIL
        detail=gate-exit
    fi
    printf '%s|%s|%s|%s|%s\n' "$gate" "$mode" "$state" "$exit_code" "$detail" >> "$records"
}

for mode in $modes; do
    for gate in $gates; do
        run_gate "$gate" "$mode"
    done
done

if ! final_fingerprint=$(source_fingerprint); then
    final_fingerprint=unavailable
fi
if [ "$initial_fingerprint" != "$final_fingerprint" ]; then source_changed=1; fi

passes=0
failures=0
incomplete=0
total=0
source_status=STABLE
if [ "$source_changed" -eq 1 ]; then source_status=CHANGED; fi

echo "=== runtime C gate summary ==="
echo "modes: $modes"
echo "scope: runtime C gates only; not a language/compiler conformance claim"
case " $modes " in
    *" sanitize "*) echo "sanitize scope: ASan/UBSan memory-safety checks; leak detection is disabled, not a leak-free proof" ;;
esac
echo "known ABI limitation: legacy rey_val_to_str remains ambiguous; typed callers must supply kind metadata (docs/STRING-ABI-NOTES.md)"
echo "source fingerprint before: $initial_fingerprint"
echo "source fingerprint after:  $final_fingerprint"
echo "source stability: $source_status"
while IFS='|' read -r gate mode state exit_code detail; do
    [ -n "$gate" ] || continue
    if [ "$source_changed" -eq 1 ] && [ "$state" = PASS ]; then
        state=INCOMPLETE
        detail="source versions differed during overall run"
    fi
    total=$((total + 1))
    case "$state" in
        PASS) passes=$((passes + 1)) ;;
        FAIL) failures=$((failures + 1)) ;;
        INCOMPLETE) incomplete=$((incomplete + 1)) ;;
    esac
    printf '%-8s %-8s %-10s exit=%s (%s)\n' "$gate" "$mode" "$state" "$exit_code" "$detail"
done < "$records"
echo "counts: pass=$passes fail=$failures incomplete=$incomplete total=$total"

overall=PASS
if [ "$failures" -gt 0 ]; then overall=FAIL; fi
if [ "$incomplete" -gt 0 ] || [ "$source_changed" -eq 1 ] || [ "$total_expired" -eq 1 ]; then overall=INCOMPLETE; fi
echo "overall: $overall"

if [ "$overall" != PASS ]; then
    echo "=== non-pass gate logs ===" >&2
    while IFS='|' read -r gate mode state exit_code detail; do
        [ -n "$gate" ] || continue
        if [ -s "$work_dir/$gate.$mode.stdout" ] || [ -s "$work_dir/$gate.$mode.stderr" ]; then
            echo "--- $gate $mode (exit=$exit_code; $detail) ---" >&2
            [ ! -s "$work_dir/$gate.$mode.stdout" ] || cat "$work_dir/$gate.$mode.stdout" >&2
            [ ! -s "$work_dir/$gate.$mode.stderr" ] || cat "$work_dir/$gate.$mode.stderr" >&2
        fi
    done < "$records"
    exit 1
fi
