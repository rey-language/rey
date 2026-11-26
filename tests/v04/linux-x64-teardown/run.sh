#!/bin/sh
set -eu

if [ "$#" -ne 3 ]; then
    echo 'usage: run.sh BASE SRC OUT' >&2
    exit 2
fi

BASE=$1
SRC=$2
OUT=$3
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
BUDGET="$ROOT/scripts/v04-linux-x64-budget.sh"
RUN_ID=$(date -u +%Y%m%dT%H%M%SZ)-$$
LABEL="R02-teardown-$RUN_ID"
LOG_DIR="$HOME/rey/wave2/logs"
mkdir -p "$LOG_DIR"
OUTER_LOG="$LOG_DIR/$LABEL.log"

if sh "$BUDGET" job --seconds 2400 --label "$LABEL" -- \
    python3 -B "$ROOT/tests/v04/linux-x64-teardown/run.py" "$BASE" "$SRC" "$OUT" \
    >"$OUTER_LOG" 2>&1; then
    cat "$OUTER_LOG"
else
    status=$?
    cat "$OUTER_LOG" >&2
    exit "$status"
fi
