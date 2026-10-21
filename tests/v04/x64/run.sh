#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
ENGINE="$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME="$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/runtime"
OUT="$ROOT/build/v04/x64"
mkdir -p "$OUT"
STAGE=$(mktemp -d "$OUT/stage.XXXXXX")
IMAGES="$STAGE/fixtures"

test -x "$ENGINE"
test -d "$RUNTIME"
mkdir -p "$STAGE/src" "$IMAGES"
cp -R "$ROOT/rey-compiler/src/." "$STAGE/src/"
cp "$ROOT/tests/v04/x64/main.rey" "$STAGE/src/main.rey"
"$ENGINE" build "$STAGE/src/main.rey" -o "$STAGE/x64-emitter-fixture" --runtime-dir "$RUNTIME"
(cd "$ROOT" && "$STAGE/x64-emitter-fixture" "$IMAGES")
python3 "$ROOT/tests/v04/x64/verify.py" "$IMAGES"
