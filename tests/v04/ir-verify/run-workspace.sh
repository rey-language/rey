#!/bin/sh
set -eu

if [ "$#" -ne 3 ]; then
    echo "usage: run-workspace.sh BASE SRC OUT" >&2
    exit 2
fi

BASE=$1
SRC=$2
OUT=$3
CALLER=$(pwd -P)
BASE=$(CDPATH= cd -- "$(dirname -- "$BASE")" && pwd -P)/$(basename -- "$BASE")
SRC=$(CDPATH= cd -- "$SRC" && pwd -P)
case "$OUT" in /*) ;; *) OUT="$CALLER/$OUT" ;; esac
cd "$SRC"
if [ -e "$OUT" ]; then
    echo "FAIL V01: output directory already exists: $OUT" >&2
    exit 1
fi
mkdir -p "$OUT"
OUT=$(CDPATH= cd -- "$OUT" && pwd -P)

test -x "$BASE"
test -f "$SRC/rey-compiler/src/ir/verify/main.rey"
test -f "$SRC/runtime-rey/containers/main.rey"
test -f "$SRC/scripts/v04-linux-x64-budget.sh"
test -f "$SRC/tests/v04/ir-verify/control/main.rey"
test -f "$SRC/tests/v04/ir-verify/main.rey"
test -f "$SRC/tests/v04/ir-verify/workspace.rey"

CONTROL_SHA=9e372c54e0b482b7e8f20fcc553d67ece779a43279cc092a2dd61cce4ccaaae1
actual_control_sha=$(sha256sum "$SRC/tests/v04/ir-verify/control/main.rey" | awk '{print $1}')
if [ "$actual_control_sha" != "$CONTROL_SHA" ]; then
    echo "FAIL V01: checked-in control verifier hash changed" >&2
    exit 1
fi

mkdir -p "$OUT/stage/compiler-src" "$OUT/run/control" "$OUT/run/current" "$OUT/run/workspace"
cp -R "$SRC/rey-compiler/src/." "$OUT/stage/compiler-src/"
cp "$SRC/tests/v04/ir-verify/main.rey" "$OUT/stage/compiler-src/main.rey"
cp "$SRC/tests/v04/ir-verify/control/main.rey" "$OUT/stage/compiler-src/ir/verify/main.rey"

run_build() {
    label=$1
    output=$2
    sh "$SRC/scripts/v04-linux-x64-budget.sh" run --seconds 1800 --label "V01-$label" -- \
        "$BASE" build "$OUT/stage/compiler-src/main.rey" -o "$output" \
        --target linux-x64 --managed --regions --runtime-root "$SRC/runtime-rey"
}

run_build control "$OUT/control-driver"
cp "$SRC/rey-compiler/src/ir/verify/main.rey" "$OUT/stage/compiler-src/ir/verify/main.rey"
run_build current "$OUT/current-driver"
cp "$SRC/tests/v04/ir-verify/workspace.rey" "$OUT/stage/compiler-src/main.rey"
run_build workspace "$OUT/workspace-driver"

if ! timeout --kill-after=10s 120 nice -n 19 "$OUT/control-driver" "$OUT/run/control" >"$OUT/control.stdout"; then
    echo "FAIL V01: control verifier fixture failed" >&2
    exit 1
fi
if ! timeout --kill-after=10s 120 nice -n 19 "$OUT/current-driver" "$OUT/run/current" >"$OUT/current.stdout"; then
    echo "FAIL V01: current verifier fixture failed" >&2
    exit 1
fi
if ! cmp -s "$OUT/control.stdout" "$OUT/current.stdout"; then
    echo "FAIL V01: baseline/current fixture stdout differs" >&2
    diff -u "$OUT/control.stdout" "$OUT/current.stdout" >&2 || true
    exit 1
fi
if ! cmp -s "$OUT/run/control/diagnostics.txt" "$OUT/run/current/diagnostics.txt"; then
    echo "FAIL V01: baseline/current diagnostics differ" >&2
    diff -u "$OUT/run/control/diagnostics.txt" "$OUT/run/current/diagnostics.txt" >&2 || true
    exit 1
fi
if ! cmp -s "$OUT/run/control/emitted.elf" "$OUT/run/current/emitted.elf"; then
    echo "FAIL V01: baseline/current emitted ELF bytes differ" >&2
    exit 1
fi
if [ "$(cat "$OUT/control.stdout")" != "ir verifier: all fixtures passed" ]; then
    echo "FAIL V01: verifier fixture did not report its success marker" >&2
    exit 1
fi
printf '%s\n' "PASS V01 baseline diagnostics and emitted bytes"

if ! timeout --kill-after=10s 120 nice -n 19 "$OUT/workspace-driver" >"$OUT/workspace.stdout"; then
    echo "FAIL V01: many-function workspace fixture failed" >&2
    cat "$OUT/workspace.stdout" >&2
    exit 1
fi
workspace_output=$(cat "$OUT/workspace.stdout")
case "$workspace_output" in
    *"PASS ir verifier workspace") ;;
    *)
    echo "FAIL V01: workspace fixture success marker missing" >&2
    exit 1
    ;;
esac

sha256sum "$BASE" "$SRC/rey-compiler/src/ir/verify/main.rey" \
    "$SRC/runtime-rey/containers/main.rey" "$SRC/tests/v04/ir-verify/main.rey" \
    "$SRC/tests/v04/ir-verify/workspace.rey" "$SRC/tests/v04/ir-verify/control/main.rey" \
    >"$OUT/input-hashes.sha256"
find "$SRC/rey-compiler/src" -type f -print0 | sort -z | xargs -0 sha256sum >"$OUT/compiler-source-hashes.sha256"
find "$SRC/runtime-rey" -type f -print0 | sort -z | xargs -0 sha256sum >"$OUT/runtime-hashes.sha256"
sha256sum "$OUT/run/control/emitted.elf" "$OUT/run/current/emitted.elf" >"$OUT/emitted-hashes.sha256"
cat "$OUT/workspace.stdout"
printf '%s\n' "PASS V01 verifier workspace"
