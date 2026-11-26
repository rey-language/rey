#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo 'usage: failed-isolation-probe.sh TRACE MISSING_ROOT' >&2
    exit 2
fi
trace=$1
missing_root=$2
[ ! -e "$missing_root" ] || { echo 'failed-isolation root unexpectedly exists' >&2; exit 1; }

set +e
/usr/bin/strace -f -qq -e trace=execve,execveat -o "$trace" -- \
    /usr/sbin/chroot "$missing_root" /seed/rey build /src/compiler/main.rey \
    -o /out/S1 --target linux-x64 --regions --managed
status=$?
set -e
[ "$status" -ne 0 ] || { echo 'missing-root isolation unexpectedly succeeded' >&2; exit 1; }
if grep -Fq '"/seed/rey"' "$trace"; then
    echo 'producer ran after isolation setup failed' >&2
    exit 1
fi
printf 'failed isolation prevented producer exec\n'
