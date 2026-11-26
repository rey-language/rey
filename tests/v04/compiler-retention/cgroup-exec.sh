#!/bin/sh
set -eu

if [ "$#" -lt 1 ]; then
    echo "usage: cgroup-exec.sh COMMAND [ARG ...]" >&2
    exit 2
fi

cap=${REY_CGROUP_CAP:-}
[ -n "$cap" ] || { echo "P01 cgroup: missing REY_CGROUP_CAP" >&2; exit 75; }
seconds=${REY_BUDGET_SECONDS:-1800}
case "$seconds" in ''|*[!0-9]*) echo "P01 cgroup: invalid budget timeout" >&2; exit 75 ;; esac
[ "$seconds" -gt 60 ] || { echo "P01 cgroup: timeout must exceed 60 seconds" >&2; exit 75; }
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
check_script="$script_dir/check-cgroup.sh"
monitor_script="$script_dir/monitor-command.sh"

user_scope_works=0
if systemd-run --user --scope --quiet --nice=19 -p "MemoryMax=$cap" \
    -p MemorySwapMax=512M -- sh "$check_script" "$cap" 512M >/dev/null 2>&1; then
    user_scope_works=1
fi

if [ "$user_scope_works" -eq 1 ]; then
    systemd-run --user --scope --quiet --nice=19 -p "MemoryMax=$cap" \
        -p MemorySwapMax=512M -- sh "$monitor_script" "$cap" "$seconds" "$check_script" "$@"
    exit $?
fi

if sudo -n systemd-run --scope --quiet --nice=19 -p "MemoryMax=$cap" \
    -p MemorySwapMax=512M -- sh "$check_script" "$cap" 512M >/dev/null 2>&1; then
    sudo -n systemd-run --scope --quiet --nice=19 -p "MemoryMax=$cap" \
        -p MemorySwapMax=512M -- sh "$monitor_script" "$cap" "$seconds" "$check_script" "$@"
    exit $?
fi

echo "P01 cgroup: neither user nor system scope enforced $cap/512M" >&2
exit 75
