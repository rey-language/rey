#!/bin/sh
set -eu

if [ "$#" -lt 4 ]; then
    echo "usage: monitor-command.sh CAP SECONDS CHECK COMMAND [ARG ...]" >&2
    exit 2
fi

cap=$1
seconds=$2
check_script=$3
shift 3
report_cgroup() {
    status=$1
    cgroup_path=$(sed -n 's/^0:://p' /proc/self/cgroup)
    if [ -z "$cgroup_path" ]; then
        printf 'P01_CGROUP memory.peak=unknown memory.events=unknown memory.swap.events=unknown status=%s\n' "$status"
        return
    fi
    memory_peak=$(cat "/sys/fs/cgroup$cgroup_path/memory.peak" 2>/dev/null || echo unknown)
    memory_events=$(tr '\n' ',' <"/sys/fs/cgroup$cgroup_path/memory.events" 2>/dev/null || echo unknown)
    swap_events=$(tr '\n' ',' <"/sys/fs/cgroup$cgroup_path/memory.swap.events" 2>/dev/null || echo unknown)
    printf 'P01_CGROUP memory.peak=%s memory.events=%s memory.swap.events=%s status=%s\n' \
        "$memory_peak" "$memory_events" "$swap_events" "$status"
}
trap 'exit_status=$?; report_cgroup "$exit_status"' EXIT
sh "$check_script" "$cap" 512M
run_seconds=$((seconds - 60))
timeout --kill-after=30s "$run_seconds" "$@" &
job=$!
printf '%s\n' 'P01_CGROUP monitor=active floor_gib=3'

while :; do
    state=$(ps -o stat= -p "$job" | tr -d ' ' || true)
    case "$state" in ''|*Z*) break ;; esac
    available=$(LC_ALL=C free -g | awk '$1 == "Mem:" { print $7; found = 1; exit } END { if (!found) exit 1 }') || {
        kill -TERM "$job" 2>/dev/null || true
        wait "$job" 2>/dev/null || true
        echo "P01 cgroup: available memory reading failed" >&2
        exit 75
    }
    case "$available" in
        ''|*[!0-9]*)
            kill -TERM "$job" 2>/dev/null || true
            wait "$job" 2>/dev/null || true
            echo "P01 cgroup: invalid available-memory reading" >&2
            exit 75
            ;;
    esac
    if [ "$available" -lt 3 ]; then
        kill -TERM "$job" 2>/dev/null || true
        wait "$job" 2>/dev/null || true
        echo "P01 cgroup: available memory fell below 3 GiB; stopped owned command" >&2
        exit 75
    fi
    sleep 2
done

set +e
wait "$job"
status=$?
set -e
exit "$status"
