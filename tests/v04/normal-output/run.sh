#!/bin/sh
set -eu

if [ "$#" -ne 5 ] || [ "$3" != "--staged" ] || [ "$4" != "--source" ]; then
    echo 'usage: run.sh SEED OUT --staged --source SOURCE' >&2
    exit 2
fi

SEED=$1
OUT=$2
SOURCE=$5
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
RUN_ID=$(date -u +%Y%m%dT%H%M%SZ)-$$
LABEL="O01-normal-output-$RUN_ID"
LOG_DIR="$HOME/rey/wave3/logs"
mkdir -p "$LOG_DIR"
OUTER_LOG="$LOG_DIR/$LABEL.log"

if timeout --kill-after=30s 7200 \
    python3 -B "$ROOT/tests/v04/normal-output/run.py" "$SEED" "$OUT" --staged --source "$SOURCE" \
    >"$OUTER_LOG" 2>&1; then
    printf '%s\n' 'PASS normal output'
else
    status=$?
    cat "$OUTER_LOG" >&2
    exit "$status"
fi
