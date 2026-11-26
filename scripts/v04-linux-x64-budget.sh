#!/bin/sh
# Resource and single-builder lock wrapper for v0.4 Linux x64 build jobs.
set -eu

usage() {
    cat >&2 <<'EOF'
Usage:
  v04-linux-x64-budget.sh run --seconds N --label NAME -- COMMAND [ARG ...]
  v04-linux-x64-budget.sh test-run --seconds N --lock PATH [--available-gb N] -- COMMAND [ARG ...]

"run" uses ~/rey/build.lock, requires at least 6 GiB available, and records
the complete command output under ~/rey/wave1/logs/.
"test-run" is reserved for bootstrap self-tests of lock and memory failures.
EOF
}

fail() { printf '%s\n' "B01 budget: $*" >&2; exit 2; }

mode=${1:-}
case "$mode" in run|test-run) shift ;; *) usage; exit 2 ;; esac

seconds=
label=
lock="$HOME/rey/build.lock"
available_override=
while [ "$#" -gt 0 ]; do
    case "$1" in
        --seconds) [ "$#" -ge 2 ] || fail 'missing --seconds value'; seconds=$2; shift 2 ;;
        --label) [ "$#" -ge 2 ] || fail 'missing --label value'; label=$2; shift 2 ;;
        --lock)
            [ "$mode" = test-run ] || fail '--lock is available only to self-tests'
            [ "$#" -ge 2 ] || fail 'missing --lock value'; lock=$2; shift 2 ;;
        --available-gb)
            [ "$mode" = test-run ] || fail '--available-gb is available only to self-tests'
            [ "$#" -ge 2 ] || fail 'missing --available-gb value'; available_override=$2; shift 2 ;;
        --) shift; break ;;
        *) fail "unknown option: $1" ;;
    esac
done
[ -n "$seconds" ] || fail 'a timeout is required'
case "$seconds" in ''|*[!0-9]*|0) fail 'timeout must be a positive integer' ;; esac
[ "$seconds" -le 2400 ] || fail 'timeout exceeds the 2400 second cap'
[ "$#" -gt 0 ] || fail 'missing command'
if [ "$mode" = run ]; then
    [ -n "$label" ] || fail 'a log label is required'
    case "$label" in *[!A-Za-z0-9._-]*) fail 'log label contains unsupported characters' ;; esac
fi
if [ "$mode" = test-run ]; then
    [ "${REY_BUDGET_SELF_TEST:-0}" = 1 ] || fail 'test-run is reserved for the harness self-test'
    case "$available_override" in ''|*[!0-9]*) [ -z "$available_override" ] || fail 'invalid simulated memory value' ;; esac
fi

if [ "$mode" = run ]; then
    stamp=$(date -u +%Y%m%dT%H%M%SZ)
    log_dir="$HOME/rey/wave1/logs"
    mkdir -p "$log_dir"
    log="$log_dir/B01-${label}-${stamp}-$$.log"
else
    log_dir="$HOME/rey/wave1/logs"
    mkdir -p "$log_dir"
    stamp=$(date -u +%Y%m%dT%H%M%SZ)
    log="$log_dir/B01-selftest-${stamp}-$$.log"
fi

mkdir -p "$(dirname "$lock")"
set +e
if [ -n "$available_override" ]; then
    REY_BUDGET_SECONDS="$seconds" REY_BUDGET_AVAILABLE_OVERRIDE="$available_override" \
        flock -n -E 73 "$lock" sh -c '
            set -eu
            avail=$REY_BUDGET_AVAILABLE_OVERRIDE
            [ "$avail" -ge 6 ] || { printf "available memory %s GiB is below the 6 GiB admission floor\n" "$avail"; exit 75; }
            cap=$((avail - 3))
            [ "$cap" -le 12 ] || cap=12
            export REY_CGROUP_CAP="${cap}G"
            printf "available_gb=%s cap=%s swap_cap=512M\n" "$avail" "$REY_CGROUP_CAP"
            exec nice -n 19 timeout --kill-after=30s "$REY_BUDGET_SECONDS" "$@"
        ' b01-budget "$@" >"$log" 2>&1
else
    REY_BUDGET_SECONDS="$seconds" flock -n -E 73 "$lock" sh -c '
        set -eu
        avail=$(LC_ALL=C free -g | awk '\''$1 == "Mem:" { print $7; found = 1; exit } END { if (!found) exit 1 }'\'') || {
            printf "could not read available memory from free -g\n"
            exit 75
        }
        case "$avail" in ""|*[!0-9]*) printf "invalid available-memory reading: %s\n" "$avail"; exit 75 ;; esac
        [ "$avail" -ge 6 ] || { printf "available memory %s GiB is below the 6 GiB admission floor\n" "$avail"; exit 75; }
        cap=$((avail - 3))
        [ "$cap" -le 12 ] || cap=12
        export REY_CGROUP_CAP="${cap}G"
        printf "available_gb=%s cap=%s swap_cap=512M\n" "$avail" "$REY_CGROUP_CAP"
        exec nice -n 19 timeout --kill-after=30s "$REY_BUDGET_SECONDS" "$@"
    ' b01-budget "$@" >"$log" 2>&1
fi
status=$?
set -e

if [ "$status" -eq 73 ]; then
    printf '%s\n' 'B01 budget: build lock is already held; command was not invoked' >>"$log"
fi
if [ "$status" -ne 0 ]; then
    cat "$log" >&2
    printf 'B01 budget: command failed with status %s (log: %s)\n' "$status" "$log" >&2
    exit "$status"
fi
printf 'B01 budget log: %s\n' "$log"
