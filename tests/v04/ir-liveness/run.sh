#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
ENGINE="$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME="$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/runtime"
OUT="$ROOT/build/v04/ir-liveness"
mkdir -p "$OUT"
STAGE=$(mktemp -d "$OUT/stage.XXXXXX")

test -x "$ENGINE"
test -d "$RUNTIME"
mkdir -p "$STAGE/compiler-src"
cp -R "$ROOT/rey-compiler/src/." "$STAGE/compiler-src/"
cp "$ROOT/tests/v04/ir-liveness/main.rey" "$STAGE/compiler-src/main.rey"
"$ENGINE" build "$STAGE/compiler-src/main.rey" -o "$STAGE/ir-liveness-fixture" --runtime-dir "$RUNTIME"
"$STAGE/ir-liveness-fixture"
