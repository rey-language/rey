#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
if [ "${1-}" = "--self-test" ]; then
    [ "$#" -eq 3 ] && [ "$2" = "--work" ] || {
        echo "usage: $0 --self-test --work DIR" >&2
        exit 2
    }
    exec python3 -B "$root/tests/v04/compiler-profile/run.py" self-test --work "$3"
fi

if [ "${1-}" = "--log" ]; then
    [ "$#" -eq 4 ] && [ "$3" = "--out" ] || {
        echo "usage: $0 --log EVENTS.jsonl --out REPORT.md" >&2
        exit 2
    }
    exec python3 -B "$root/tests/v04/compiler-profile/run.py" report --log "$2" --out "$4"
fi

echo "usage: $0 --self-test --work DIR | --log EVENTS.jsonl --out REPORT.md" >&2
exit 2
