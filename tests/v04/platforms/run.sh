#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
ENGINE=${REY_ENGINE:-"$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"}
RUNTIME=${REY_RUNTIME:-"$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/runtime"}
export REY_ENGINE="$ENGINE"
export REY_RUNTIME="$RUNTIME"
export PATH=/usr/bin:/bin:/usr/sbin:/sbin
export CC=/usr/bin/clang
export CXX=/usr/bin/clang++
test -x "$ENGINE"
test -d "$RUNTIME"
OUT="$ROOT/build/v04/platforms"
mkdir -p "$OUT"
STAGE=$(mktemp -d "$OUT/stage.XXXXXX")
mkdir -p "$STAGE/src"
cp -R "$ROOT/rey-compiler/src/." "$STAGE/src/"
cp "$ROOT/tests/v04/platforms/main.rey" "$STAGE/src/main.rey"
if [ -d "$ROOT/std/src" ]; then
    export REY_STD_PATH="$ROOT/std/src"
fi
"$ENGINE" build "$STAGE/src/main.rey" -o "$STAGE/platform-fixture" --runtime-dir "$RUNTIME"
"$STAGE/platform-fixture"
python3 "$ROOT/scripts/v04-platform-check.py"
