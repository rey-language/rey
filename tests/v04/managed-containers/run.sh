#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
ENGINE="$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME="$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/runtime"
OUT="$ROOT/build/v04/managed-containers"
mkdir -p "$OUT"
STAGE=$(mktemp -d "$OUT/stage.XXXXXX")

test -x "$ENGINE"
test -d "$RUNTIME"
mkdir -p "$STAGE/compiler-src" "$STAGE/source/layout" "$STAGE/source/heap" "$STAGE/source/gc" "$STAGE/source/managed" "$STAGE/source/containers" "$STAGE/fixtures" "$STAGE/out"
cp -R "$ROOT/rey-compiler/src/." "$STAGE/compiler-src/"
cp "$ROOT/tests/v04/managed-containers/main.rey" "$STAGE/compiler-src/main.rey"
cp "$ROOT/runtime-rey/layout/main.rey" "$STAGE/source/layout/main.rey"
cp "$ROOT/runtime-rey/heap/main.rey" "$STAGE/source/heap/main.rey"
cp "$ROOT/runtime-rey/gc/main.rey" "$STAGE/source/gc/main.rey"
cp "$ROOT/runtime-rey/managed/main.rey" "$STAGE/source/managed/main.rey"
cp "$ROOT/runtime-rey/containers/main.rey" "$STAGE/source/containers/main.rey"
cp "$ROOT/tests/v04/managed-containers/fixtures/"*.rey "$STAGE/fixtures/"

"$ENGINE" build "$STAGE/compiler-src/main.rey" -o "$STAGE/managed-containers-driver" --runtime-dir "$RUNTIME"
for case in core-strings refvec-gc refvec-gc-small map-cycles map-cycles-small reject-region-value missing-map-key malformed-string-to-int out-of-range-string-to-int intmax-string-to-int; do
    (cd "$ROOT" && "$STAGE/managed-containers-driver" "$STAGE/fixtures" "$STAGE/source" "$STAGE/out" "$case" "$@")
done
