#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
ENGINE=${REY_ENGINE:-"$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"}
RUNTIME=${REY_RUNTIME:-"$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/runtime"}
test -x "$ENGINE"
test -d "$RUNTIME"
OUT="$ROOT/build/v04/raw-offset-x64"
mkdir -p "$OUT"
STAGE=$(mktemp -d "$OUT/stage.XXXXXX")
mkdir -p "$STAGE/src" "$STAGE/images"
cp -R "$ROOT/rey-compiler/src/." "$STAGE/src/"
cp "$ROOT/tests/v04/raw-offset-x64/main.rey" "$STAGE/src/main.rey"
if [ -d "$ROOT/std/src" ]; then
    export REY_STD_PATH="$ROOT/std/src"
fi
"$ENGINE" build "$STAGE/src/main.rey" -o "$STAGE/raw-offset-x64-fixture" --runtime-dir "$RUNTIME"
"$STAGE/raw-offset-x64-fixture" "$STAGE/images"
python3 "$ROOT/tests/v04/raw-offset-x64/verify.py" "$STAGE/images"
