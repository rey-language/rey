#!/bin/sh
set -eu

if [ "$#" -lt 6 ]; then
    echo 'usage: uncapped-build.sh TIME_FILE CGROUP_FILE STATUS_FILE SECONDS -- COMMAND [ARG ...]' >&2
    exit 2
fi

TIME_FILE=$1
CGROUP_FILE=$2
STATUS_FILE=$3
SECONDS=$4
shift 4
if [ "$1" != "--" ]; then
    echo 'uncapped-build.sh requires -- before COMMAND' >&2
    exit 2
fi
shift
case "$SECONDS" in ''|*[!0-9]*|0) echo 'SECONDS must be positive' >&2; exit 2 ;; esac
if [ "$#" -eq 0 ]; then
    echo 'uncapped-build.sh requires COMMAND' >&2
    exit 2
fi

CAPTURE='time_file=$1; cgroup_file=$2; status_file=$3; seconds=$4; shift 4; set +e; nice -n 19 timeout --kill-after=30s "$seconds" "$@"; status=$?; set -e; rel=$(sed -n "s/^0:://p" /proc/self/cgroup); [ -n "$rel" ] || exit 125; dir=/sys/fs/cgroup$rel; [ -r "$dir/memory.max" ] && [ -r "$dir/memory.swap.max" ] && [ -r "$dir/memory.peak" ] || exit 125; memory_max=$(cat "$dir/memory.max"); swap_max=$(cat "$dir/memory.swap.max"); [ "$memory_max" = max ] && [ "$swap_max" = max ] || { echo "uncapped cgroup required, got memory.max=$memory_max memory.swap.max=$swap_max" >&2; exit 125; }; peak=$(cat "$dir/memory.peak"); { printf "memory.max=%s\nmemory.swap.max=%s\nmemory.peak=%s\nmemory.accounting=yes\noom_policy=continue\ncommand_status=%s\n" "$memory_max" "$swap_max" "$peak" "$status"; } >"$cgroup_file"; printf "%s\n" "$status" >"$status_file"; exit "$status"'

exec systemd-run --user --scope --quiet --nice=19 \
    -p MemoryMax=infinity -p MemorySwapMax=infinity \
    -p MemoryAccounting=yes -p OOMPolicy=continue -- \
    /bin/sh -c "$CAPTURE" uncapped-build "$TIME_FILE" "$CGROUP_FILE" "$STATUS_FILE" "$SECONDS" "$@"
