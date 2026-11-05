#!/bin/sh
set -eu
PATH=/usr/bin:/bin:/usr/sbin:/sbin
CC=/usr/bin/clang
CXX=/usr/bin/clang++
export PATH CC CXX
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
ENGINE=${REY_ENGINE:-"$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"}
RUNTIME=${REY_RUNTIME:-"$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/runtime"}
VM_LIMIT_KB=${REY_VM_LIMIT_KB:-12582912}
test -x "$ENGINE"
test -d "$RUNTIME"
OUT="$ROOT/build/v04/packed-buffer"
mkdir -p "$OUT"
STAGE=$(mktemp -d "$OUT/stage.XXXXXX")
mkdir -p "$STAGE/src"
cp -R "$ROOT/rey-compiler/src/." "$STAGE/src/"
cp "$ROOT/tests/v04/packed-buffer/main.rey" "$STAGE/src/main.rey"
if [ -d "$ROOT/std/src" ]; then
    export REY_STD_PATH="$ROOT/std/src"
fi
(ulimit -v "$VM_LIMIT_KB"; timeout 300 "$ENGINE" build "$STAGE/src/main.rey" -o "$STAGE/packed-buffer-fixture" --runtime-dir "$RUNTIME")
(cd "$STAGE" && ./packed-buffer-fixture)
