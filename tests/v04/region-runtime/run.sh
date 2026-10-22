#!/bin/sh
set -eu
# The pinned seed invokes the host Clang; keep Swiftly-managed shims out of PATH.
PATH=/usr/bin:/bin:/usr/sbin:/sbin
CC=/usr/bin/clang
CXX=/usr/bin/clang++
export PATH CC CXX
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
ENGINE="$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME="$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/runtime"
OUT="$ROOT/build/v04/region-runtime"
mkdir -p "$OUT"
STAGE=$(mktemp -d "$OUT/stage.XXXXXX")

test -x "$ENGINE"
test -d "$RUNTIME"
mkdir -p "$STAGE/compiler-src" "$STAGE/source/layout" "$STAGE/source/regions" "$STAGE/source/managed" "$STAGE/source/containers" "$STAGE/fixtures" "$STAGE/out"
cp -R "$ROOT/rey-compiler/src/." "$STAGE/compiler-src/"
cp "$ROOT/tests/v04/region-runtime/main.rey" "$STAGE/compiler-src/main.rey"
cp "$ROOT/runtime-rey/layout/main.rey" "$STAGE/source/layout/main.rey"
cp "$ROOT/runtime-rey/regions/main.rey" "$STAGE/source/regions/main.rey"
cp "$ROOT/runtime-rey/managed/main.rey" "$STAGE/source/managed/main.rey"
cp "$ROOT/runtime-rey/containers/main.rey" "$STAGE/source/containers/main.rey"
cp "$ROOT/tests/v04/region-runtime/fixtures/"*.rey "$STAGE/fixtures/"

"$ENGINE" build "$STAGE/compiler-src/main.rey" -o "$STAGE/region-runtime-driver" --runtime-dir "$RUNTIME"
for case in region-basic program-region group-separation promotion-address-identity promotion-older-source promotion-null promotion-dependency-chain promotion-cycle store-promotes-younger store-records-older destination-construction region-end-lifo region-end-program double-end container-current-region repeated-request-plateau long-lived-graph-retention; do
    (cd "$ROOT" && "$STAGE/region-runtime-driver" "$STAGE/fixtures" "$STAGE/source" "$STAGE/out" "$case" "$@")
done
