#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
ENGINE="$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME="$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/runtime"
OUT="$ROOT/build/v04/layout-registry"
mkdir -p "$OUT"
STAGE=$(mktemp -d "$OUT/stage.XXXXXX")

test -x "$ENGINE"
test -d "$RUNTIME"
mkdir -p "$STAGE/compiler-src" "$STAGE/source/layout" "$STAGE/fixtures" "$STAGE/out"
cp -R "$ROOT/rey-compiler/src/." "$STAGE/compiler-src/"
cp "$ROOT/tests/v04/layout-registry/main.rey" "$STAGE/compiler-src/main.rey"
cp "$ROOT/runtime-rey/layout/main.rey" "$STAGE/source/layout/main.rey"
cp "$ROOT/tests/v04/layout-registry/fixtures/"*.rey "$STAGE/fixtures/"

"$ENGINE" build "$STAGE/compiler-src/main.rey" -o "$STAGE/layout-registry-driver" --runtime-dir "$RUNTIME"
for CASE in \
    valid \
    capacity-negative \
    capacity-over-id-range \
    descriptor-size-overflow \
    payload-negative \
    reference-count-negative \
    null-reference-offsets \
    duplicate-offsets \
    unsorted-offsets \
    negative-offset \
    unaligned-offset \
    out-of-bounds-offset \
    reference-count-exceeds-payload \
    id-negative \
    id-zero \
    id-too-large \
    index-negative \
    index-too-large \
    full-registry \
    zero-capacity-full \
    corrupt-magic \
    corrupt-capacity \
    corrupt-count \
    corrupt-length \
    corrupt-reserved \
    corrupt-descriptor-payload \
    corrupt-descriptor-count \
    corrupt-descriptor-reserved \
    corrupt-descriptor-offset \
    corrupt-duplicate-slot
do
    "$STAGE/layout-registry-driver" "$STAGE/fixtures" "$STAGE/source" "$STAGE/out" "$CASE"
done
python3 "$ROOT/tests/v04/layout-registry/verify.py" "$STAGE/out"
