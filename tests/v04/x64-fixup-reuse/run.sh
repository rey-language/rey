#!/bin/sh
set -eu

if [ "$#" -ne 3 ]; then
    echo 'usage: run.sh BASE SRC W/fixups' >&2
    exit 2
fi

BASE=$(realpath "$1")
SRC=$(realpath "$2")
OUT=$3
case "$OUT" in /*) ;; *) echo 'X01 output directory must be absolute' >&2; exit 2 ;; esac
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
BUDGET="$ROOT/scripts/v04-linux-x64-budget.sh"
MANIFEST="$ROOT/tests/v04/x64-fixup-reuse/manifest.json"
FIXTURES="$ROOT/tests/v04/x64-fixup-reuse/fixtures.rey"

test -x "$BASE"
test -d "$SRC/rey-compiler/src"
test -x "$BUDGET"
if [ -e "$OUT" ]; then
    if [ -n "$(find "$OUT" -mindepth 1 -print -quit)" ]; then
        echo "X01 output directory is not fresh: $OUT" >&2
        exit 2
    fi
else
    mkdir -p "$OUT"
fi

mkdir -p "$OUT/current-stage/src/x64_fixup_reuse" "$OUT/control" "$OUT/recycled"
cp "$MANIFEST" "$OUT/manifest.json"
cp -R "$SRC/rey-compiler/src/." "$OUT/current-stage/src/"
cp "$FIXTURES" "$OUT/current-stage/src/x64_fixup_reuse/main.rey"
cp "$ROOT/tests/v04/x64-fixup-reuse/main.rey" "$OUT/current-stage/src/main.rey"

CONTROL_REV=$(python3 -B -c 'import json,sys; print(json.load(open(sys.argv[1]))["baseline_revision"])' "$MANIFEST")
CONTROL_SOURCE_HASH=$(git -C "$SRC" show "$CONTROL_REV:rey-compiler/src/native/x64/main.rey" | sha256sum | awk '{print $1}')
EXPECTED_CONTROL_HASH=$(python3 -B -c 'import json,sys; print(json.load(open(sys.argv[1]))["baseline_x64_sha256"])' "$MANIFEST")
if [ "$CONTROL_SOURCE_HASH" != "$EXPECTED_CONTROL_HASH" ]; then
    echo "X01 control source hash mismatch: $CONTROL_SOURCE_HASH" >&2
    exit 1
fi

SOURCE_REV=$(git -C "$SRC" rev-parse HEAD)
RUNTIME_TREE=$(git -C "$SRC" rev-parse HEAD:runtime-rey)
{
    printf 'source_revision=%s\n' "$SOURCE_REV"
    printf 'runtime_tree=%s\n' "$RUNTIME_TREE"
    printf 'control_revision=%s\n' "$CONTROL_REV"
    printf 'control_x64_sha256=%s\n' "$CONTROL_SOURCE_HASH"
    printf 'current_x64_sha256=%s\n' "$(sha256sum "$SRC/rey-compiler/src/native/x64/main.rey" | awk '{print $1}')"
    sha256sum "$BASE" "$SRC/rey-compiler/src/native/x64/main.rey" "$FIXTURES" "$MANIFEST"
} > "$OUT/provenance.txt"

run_build() {
    stage=$1
    label=$2
    log="$HOME/rey/wave2/logs/X01-${label}-$(date -u +%Y%m%dT%H%M%SZ)-$$.log"
    mkdir -p "$(dirname "$log")"
    set +e
    "$BUDGET" run --seconds 600 --label "X01-${label}" -- \
        "$BASE" build "$stage/src/main.rey" -o "$stage/x64-fixup-driver" \
        --target linux-x64 --regions --managed --runtime-root "$SRC/runtime-rey" > "$log" 2>&1
    status=$?
    set -e
    cat "$log"
    if [ "$status" -ne 0 ]; then
        echo "X01 ${label} build failed with status $status (log: $log)" >&2
        return "$status"
    fi
    printf 'command_log=%s\n' "$log" >> "$OUT/provenance.txt"
}

run_driver() {
    binary=$1
    output=$2
    label=$3
    log="$OUT/${label}.run.log"
    set +e
    nice -n 19 timeout --kill-after=30s 600 "$binary" "$output" > "$log" 2>&1
    status=$?
    set -e
    cat "$log"
    if [ "$status" -ne 0 ]; then
        echo "X01 ${label} fixture failed with status $status (log: $log)" >&2
        return "$status"
    fi
}

cd "$SRC"
run_build "$OUT/current-stage" current
run_driver "$OUT/current-stage/x64-fixup-driver" "$OUT" current

python3 -B "$ROOT/tests/v04/x64-fixup-reuse/verify.py" \
    "$OUT/manifest.json" "$OUT" "$OUT/current.run.log"
echo 'PASS x64 fixup reuse'
