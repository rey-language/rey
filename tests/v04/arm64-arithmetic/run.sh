#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
ENGINE="$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME="$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/runtime"
OUT="$ROOT/build/v04/arm64-arithmetic"
mkdir -p "$OUT"
STAGE=$(mktemp -d "$OUT/staged.XXXXXX")
IMAGES="$STAGE/images"

test -x "$ENGINE"
test -d "$RUNTIME"
mkdir -p "$STAGE/src" "$IMAGES"
cp -R "$ROOT/rey-compiler/src/." "$STAGE/src/"
cp "$ROOT/tests/v04/arm64-arithmetic/main.rey" "$STAGE/src/main.rey"
"$ENGINE" build "$STAGE/src/main.rey" -o "$STAGE/arm64-arithmetic-fixture" --runtime-dir "$RUNTIME"
(cd "$ROOT" && "$STAGE/arm64-arithmetic-fixture" "$IMAGES")
python3 "$ROOT/tests/v04/arm64-arithmetic/verify.py" "$IMAGES"
