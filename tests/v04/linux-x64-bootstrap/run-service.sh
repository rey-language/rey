#!/bin/sh
# Launch the bounded host-side tracer and sandbox setup service.
set -eu

if [ "$#" -lt 3 ]; then
    echo 'usage: run-service.sh MODE SECONDS WORK [ARGS ...]' >&2
    exit 2
fi
mode=$1
seconds=$2
work=$3
shift 3
cap=${REY_CGROUP_CAP:-}
case "$cap" in *G) ;; *) echo 'budget wrapper did not provide a cgroup memory cap' >&2; exit 1 ;; esac
case "$seconds" in ''|*[!0-9]*|0) echo 'invalid systemd service timeout' >&2; exit 2 ;; esac
[ "$seconds" -le 2400 ] || { echo 'systemd service timeout exceeds 2400 seconds' >&2; exit 2; }

case "$mode" in
    build)
        [ "$#" -eq 11 ] || { echo 'run-service build expects eleven arguments' >&2; exit 2; }
        helper=$1
        root=$2
        work_root=$3
        seed_sha=$4
        input=$5
        output=$6
        trace=$7
        stdout_file=$8
        stderr_file=$9
        expected_stdout=${10}
        build_mode=${11}
        ;;
    network)
        [ "$#" -eq 2 ] || { echo 'run-service network expects trace and helper paths' >&2; exit 2; }
        trace=$1
        helper=$2
        ;;
    failed-isolation)
        [ "$#" -eq 3 ] || { echo 'run-service failed-isolation expects trace, missing root, and helper paths' >&2; exit 2; }
        trace=$1
        missing_root=$2
        helper=$3
        ;;
    *) echo "unknown service mode: $mode" >&2; exit 2 ;;
esac

set -- sudo -n /usr/bin/systemd-run --system --wait --pipe --collect --unit="rey-b01-${mode}-$$" \
    -p "MemoryMax=$cap" \
    -p MemorySwapMax=512M \
    -p "RuntimeMaxSec=${seconds}s" \
    -p TimeoutStopSec=30s \
    -p KillMode=control-group \
    -p RestrictAddressFamilies=AF_UNIX \
    -p RestrictNamespaces=yes \
    -p NoNewPrivileges=yes \
    -p ProtectSystem=strict

case "$mode" in
    build)
        set -- "$@" -p "ReadOnlyPaths=$root/seed $root/src" \
            -p "ReadWritePaths=$root/out $work_root/logs $work_root/evidence" \
            -- /bin/sh "$helper" "$root" "$work_root" "$cap" "$seed_sha" \
            "$input" "$output" "$trace" "$stdout_file" "$stderr_file" "$expected_stdout" "$build_mode"
        ;;
    network)
        set -- "$@" -p "ReadWritePaths=$work/logs $work/evidence" -- /bin/sh "$helper" "$trace"
        ;;
    failed-isolation)
        set -- "$@" -p "ReadWritePaths=$work/logs $work/evidence" -- /bin/sh "$helper" "$trace" "$missing_root"
        ;;
esac

exec "$@"
