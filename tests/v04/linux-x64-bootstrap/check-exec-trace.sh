#!/bin/sh
set -eu

if [ "$#" -ne 3 ]; then
    echo 'usage: check-exec-trace.sh TRACE CHROOT_PATH REY_PATH' >&2
    exit 2
fi
trace=$1
expected_chroot=$2
expected_rey=$3
[ -r "$trace" ] || { echo "trace is missing: $trace" >&2; exit 1; }

awk -v chroot_path="$expected_chroot" -v rey_path="$expected_rey" '
    function exec_path(line, path) {
        if (line ~ /execve\("/) {
            sub(/^.*execve\("/, "", line)
            sub(/".*$/, "", line)
            return line
        }
        if (line ~ /execveat\([^,]+, "/) {
            sub(/^.*execveat\([^,]+, "/, "", line)
            sub(/".*$/, "", line)
            return line
        }
        return ""
    }
    /execve(at)?\(/ {
        path = exec_path($0)
        if (path == "") {
            printf "unparsed exec attempt: %s\n", $0 > "/dev/stderr"
            bad = 1
            next
        }
        if (path == chroot_path) {
            chroot_count++
            next
        }
        if (path == rey_path) {
            rey_count++
            next
        }
        printf "unexpected exec attempt: %s\n", path > "/dev/stderr"
        bad = 1
    }
    END {
        if (chroot_count != 1) {
            printf "expected one chroot exec, saw %d\n", chroot_count > "/dev/stderr"
            bad = 1
        }
        if (rey_count != 1) {
            printf "expected one Rey exec, saw %d\n", rey_count > "/dev/stderr"
            bad = 1
        }
        if (bad) exit 1
    }
' "$trace"
