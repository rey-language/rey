#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "usage: check-cgroup.sh MEMORY_MAX SWAP_MAX" >&2
    exit 2
fi

expected_memory=$1
expected_swap=$2
cgroup_path=$(sed -n 's/^0:://p' /proc/self/cgroup)
[ -n "$cgroup_path" ] || { echo "P01 cgroup: cgroup v2 membership missing" >&2; exit 75; }
actual_memory=$(cat "/sys/fs/cgroup$cgroup_path/memory.max")
actual_swap=$(cat "/sys/fs/cgroup$cgroup_path/memory.swap.max")
to_bytes() {
    value=$1
    case "$value" in
        *G) amount=${value%G}; echo $((amount * 1024 * 1024 * 1024)) ;;
        *M) amount=${value%M}; echo $((amount * 1024 * 1024)) ;;
        *K) amount=${value%K}; echo $((amount * 1024)) ;;
        *) echo "$value" ;;
    esac
}
if [ "$actual_memory" != "$(to_bytes "$expected_memory")" ] || [ "$actual_swap" != "$(to_bytes "$expected_swap")" ]; then
    echo "P01 cgroup: effective limits $actual_memory/$actual_swap do not match $expected_memory/$expected_swap" >&2
    exit 75
fi
nice_value=$(ps -o ni= -p "$$" | tr -d ' ')
case "$nice_value" in ''|*[!0-9]*) echo "P01 cgroup: cannot confirm nice level" >&2; exit 75 ;; esac
[ "$nice_value" -ge 19 ] || { echo "P01 cgroup: command nice level is $nice_value, expected 19" >&2; exit 75; }
printf 'P01_CGROUP memory.max=%s memory.swap.max=%s nice=%s\n' "$actual_memory" "$actual_swap" "$nice_value"
