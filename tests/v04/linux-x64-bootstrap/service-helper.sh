#!/bin/sh
# Runs inside the bounded system service. The tracer and chroot utility remain
# outside the producer's minimal root; only the Rey binary enters that root.
set -eu

if [ "$#" -ne 11 ]; then
    echo 'service-helper: invalid argument count' >&2
    exit 2
fi
root=$1
work=$2
cap=$3
seed_sha=$4
input=$5
output=$6
trace=$7
stdout_file=$8
stderr_file=$9
expected_stdout=${10}
mode=${11}

case "$cap" in *G) cap_gb=${cap%G} ;; *) echo "invalid memory cap: $cap" >&2; exit 2 ;; esac
case "$cap_gb" in ''|*[!0-9]*) echo "invalid memory cap: $cap" >&2; exit 2 ;; esac
cap_bytes=$((cap_gb * 1073741824))
swap_bytes=$((512 * 1048576))

cgroup_rel=$(sed -n 's/^0:://p' /proc/self/cgroup)
[ -n "$cgroup_rel" ] || { echo 'service has no cgroup v2 membership' >&2; exit 1; }
cgroup_dir="/sys/fs/cgroup$cgroup_rel"
[ -r "$cgroup_dir/memory.max" ] || { echo 'effective memory.max is unreadable' >&2; exit 1; }
[ -r "$cgroup_dir/memory.swap.max" ] || { echo 'effective memory.swap.max is unreadable' >&2; exit 1; }
memory_max=$(cat "$cgroup_dir/memory.max")
swap_max=$(cat "$cgroup_dir/memory.swap.max")
case "$memory_max:$swap_max" in *[!0-9:]*|max:*|*:max) echo 'cgroup memory limits are not finite numeric values' >&2; exit 1 ;; esac
[ "$memory_max" -gt 0 ] && [ "$memory_max" -le "$cap_bytes" ] || {
    echo "effective memory.max $memory_max exceeds requested cap $cap_bytes" >&2
    exit 1
}
[ "$swap_max" -ge 0 ] && [ "$swap_max" -le "$swap_bytes" ] || {
    echo "effective memory.swap.max $swap_max exceeds 512 MiB" >&2
    exit 1
}
seccomp=$(awk '$1 == "Seccomp:" { print $2; exit }' /proc/self/status)
[ "$seccomp" = 2 ] || { echo 'address-family seccomp filter is not active' >&2; exit 1; }

[ -d "$root/seed" ] && [ -d "$root/src" ] && [ -d "$root/out" ] || {
    echo 'minimal root is incomplete' >&2
    exit 1
}
[ -x "$root/seed/rey" ] || { echo 'minimal root has no executable seed' >&2; exit 1; }
actual_seed_sha=$(sha256sum "$root/seed/rey" | awk '{ print $1 }')
[ "$actual_seed_sha" = "$seed_sha" ] || { echo 'minimal-root seed hash changed' >&2; exit 1; }
for forbidden in "$root/bin" "$root/usr" "$root/lib" "$root/lib64" "$root/opt" "$root/run" "$root/var"; do
    [ ! -e "$forbidden" ] || { echo "minimal root contains forbidden host tree: $forbidden" >&2; exit 1; }
done

mkdir -p "$work/evidence" "$work/logs"
{
    printf 'cgroup=%s\nmemory.max=%s\nmemory.swap.max=%s\nseccomp=%s\n' \
        "$cgroup_rel" "$memory_max" "$swap_max" "$seccomp"
    printf 'memory.peak.before=%s\n' "$(cat "$cgroup_dir/memory.peak")"
    printf 'memory.events.before:\n'
    cat "$cgroup_dir/memory.events"
} >"$work/evidence/cgroup-before.txt"

time_file="$work/evidence/producer.time.txt"
checker=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)/check-exec-trace.sh
# chroot(2) keeps the caller's cwd. Enter the new root first so the producer
# cannot retain a host-side working directory across chroot.
cd "$root"
set +e
HOME=/root LANG=C LC_ALL=C PATH= /usr/bin/time -v -o "$time_file" \
    /usr/bin/strace -f -qq -e trace=execve,execveat -o "$trace" -- \
    /usr/sbin/chroot "$root" /seed/rey build "$input" -o "$output" \
    --target linux-x64 --regions --managed
build_status=$?
set -e
if [ "$build_status" -ne 0 ]; then
    printf 'producer_status=%s\n' "$build_status" >"$work/evidence/producer.status"
    exit "$build_status"
fi

sh "$checker" "$trace" /usr/sbin/chroot /seed/rey
[ -x "$root$out" ] || { echo "producer output is absent or not executable: $output" >&2; exit 1; }

set +e
if [ "$mode" = candidate ]; then
    HOME=/root LANG=C LC_ALL=C PATH= /usr/sbin/chroot "$root" "$output" --help >"$stdout_file" 2>"$stderr_file"
else
    HOME=/root LANG=C LC_ALL=C PATH= /usr/sbin/chroot "$root" "$output" >"$stdout_file" 2>"$stderr_file"
fi
run_status=$?
set -e
[ "$run_status" -eq 0 ] || { echo "smoke executable exited $run_status" >&2; exit 1; }
if [ "$mode" = smoke ]; then
    cmp -s "$stdout_file" "$expected_stdout" || { echo 'smoke stdout differs from the exact expected bytes' >&2; exit 1; }
else
    grep -q '^Usage: rey-native build ' "$stdout_file" || { echo 'candidate compiler --help output is invalid' >&2; exit 1; }
fi
[ ! -s "$stderr_file" ] || { echo 'smoke executable wrote unexpected stderr' >&2; exit 1; }

{
    printf 'producer_status=0\nsmoke_status=0\n'
    printf 'memory.peak.after=%s\n' "$(cat "$cgroup_dir/memory.peak")"
    printf 'memory.events.after:\n'
    cat "$cgroup_dir/memory.events"
} >"$work/evidence/cgroup-after.txt"
