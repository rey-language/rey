#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
BUDGET="$ROOT/scripts/v04-linux-x64-budget.sh"
RUN_ID=$(date -u +%Y%m%dT%H%M%SZ)-$$
LABEL="R01-region-$RUN_ID"
LOG_DIR="$HOME/rey/wave1/logs"
mkdir -p "$LOG_DIR"
WRAPPER_STDOUT="$LOG_DIR/R01-$RUN_ID-wrapper.stdout"
WRAPPER_STDERR="$LOG_DIR/R01-$RUN_ID-wrapper.stderr"

if sh "$BUDGET" job --seconds 2400 --label "$LABEL" -- \
    python3 -B "$ROOT/tests/v04/linux-x64-region/run.py" "$@" \
    >"$WRAPPER_STDOUT" 2>"$WRAPPER_STDERR"; then
    :
else
    status=$?
    cat "$WRAPPER_STDOUT" >&2
    cat "$WRAPPER_STDERR" >&2
    exit "$status"
fi

RUN_LOG=
MATCHES=0
for candidate in "$LOG_DIR/B01-$LABEL-"*.log; do
    [ -f "$candidate" ] || continue
    RUN_LOG=$candidate
    MATCHES=$((MATCHES + 1))
done
[ "$MATCHES" -eq 1 ] || {
    echo "FAIL region gate: expected one B01 job log for $LABEL, found $MATCHES" >&2
    exit 1
}
tail -n 2 "$RUN_LOG"
