#!/bin/sh
set -eu

if [ "$#" -ne 7 ] || [ "$5" != "--staged" ] || [ "$6" != "--repeats" ]; then
    echo 'usage: run.sh BASE SEED SRC OUT --staged --repeats N' >&2
    exit 2
fi

BASE=$1
SEED=$2
SRC=$3
OUT=$4
REPEATS=$7
case "$REPEATS" in ''|*[!0-9]*) echo 'repeats must be an integer' >&2; exit 2 ;; esac
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
# Keep descriptor 3 available for compatibility with the staged R01/R02
# subprocess helpers. Parallel build mode deliberately does not lock it.
exec 3</dev/null
REY_PARALLEL_BUILD=1 exec nice -n 19 timeout --kill-after=30s 25200 python3 -B \
    "$ROOT/tests/v04/checked-allocator/run.py" \
    "$BASE" "$SEED" "$SRC" "$OUT" --repeats "$REPEATS"
