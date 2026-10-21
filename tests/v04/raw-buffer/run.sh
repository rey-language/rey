#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
ENGINE="$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME="$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/runtime"
OUT="$ROOT/build/v04/raw-buffer"
mkdir -p "$OUT"
STAGE=$(mktemp -d "$OUT/stage.XXXXXX")

test -x "$ENGINE"
test -d "$RUNTIME"
mkdir -p "$STAGE/compiler-src" "$STAGE/source/buffer" "$STAGE/fixtures" "$STAGE/out"
cp -R "$ROOT/rey-compiler/src/." "$STAGE/compiler-src/"
cp "$ROOT/tests/v04/raw-buffer/main.rey" "$STAGE/compiler-src/main.rey"
cp "$ROOT/runtime-rey/buffer/main.rey" "$STAGE/source/buffer/main.rey"
cp "$ROOT/tests/v04/raw-buffer/fixtures/"*.rey "$STAGE/fixtures/"

"$ENGINE" build "$STAGE/compiler-src/main.rey" -o "$STAGE/raw-buffer-driver" --runtime-dir "$RUNTIME"
(cd "$ROOT" && "$STAGE/raw-buffer-driver" "$STAGE/fixtures" "$STAGE/source" "$STAGE/out")
python3 "$ROOT/tests/v04/raw-buffer/verify.py" "$STAGE/out"
