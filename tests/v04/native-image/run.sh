#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
ENGINE="$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME="$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/runtime"
OUT="$ROOT/build/v04/native-image"
STAGE="$OUT/staged"

mkdir -p "$STAGE/src" "$OUT"
cp -R "$ROOT/rey-compiler/src/." "$STAGE/src/"
cp "$ROOT/tests/v04/native-image/main.rey" "$STAGE/src/main.rey"
"$ENGINE" build "$STAGE/src/main.rey" -o "$OUT/native-image-test" --runtime-dir "$RUNTIME"
(cd "$ROOT" && "$OUT/native-image-test")
python3 "$ROOT/tests/v04/native-image/verify.py"
