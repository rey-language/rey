#!/bin/sh
set -eu
# The pinned seed invokes the host Clang; keep Swiftly-managed shims out of PATH.
PATH=/usr/bin:/bin:/usr/sbin:/sbin
CC=/usr/bin/clang
CXX=/usr/bin/clang++
export PATH CC CXX
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
ENGINE=${REY_ENGINE:-"$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"}
RUNTIME=${REY_RUNTIME:-"$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/runtime"}
OUT=${REY_OUTPUT:-"$ROOT/build/v04/region-runtime-reclamation"}
mkdir -p "$OUT"
STAGE=$(mktemp -d "$OUT/stage.XXXXXX")

test -x "$ENGINE"
test -d "$RUNTIME"
mkdir -p "$STAGE/compiler-src" "$STAGE/source/layout" "$STAGE/source/regions" "$STAGE/source/managed" "$STAGE/source/containers" "$STAGE/source/host" "$STAGE/fixtures" "$STAGE/out"
cp -R "$ROOT/rey-compiler/src/." "$STAGE/compiler-src/"
cp "$ROOT/tests/v04/region-runtime-reclamation/main.rey" "$STAGE/compiler-src/main.rey"
cp "$ROOT/runtime-rey/layout/main.rey" "$STAGE/source/layout/main.rey"
cp "$ROOT/runtime-rey/regions/main.rey" "$STAGE/source/regions/main.rey"
cp "$ROOT/runtime-rey/managed/main.rey" "$STAGE/source/managed/main.rey"
cp "$ROOT/runtime-rey/containers/main.rey" "$STAGE/source/containers/main.rey"
cp "$ROOT/runtime-rey/host/main.rey" "$STAGE/source/host/main.rey"
cp "$ROOT/tests/v04/region-runtime-reclamation/fixtures/"*.rey "$STAGE/fixtures/"

"$ENGINE" build "$STAGE/compiler-src/main.rey" -o "$STAGE/region-runtime-reclamation-driver" --runtime-dir "$RUNTIME"
for case in reclaim-three-slabs direct-mapping-metrics opaque-handle-reuse active-context-destroy teardown-many-direct-chunks; do
    (cd "$ROOT" && "$STAGE/region-runtime-reclamation-driver" "$STAGE/fixtures" "$STAGE/source" "$STAGE/out" "$case" "$@")
    if [ "$(uname -s)" = Linux ] && [ "$(uname -m)" = x86_64 ]; then
        chmod +x "$STAGE/out/linux-x64-$case"
        if [ "$case" = active-context-destroy ]; then
            (ulimit -v 16384; "$STAGE/out/linux-x64-$case")
        else
            "$STAGE/out/linux-x64-$case"
        fi
        echo "region-runtime-reclamation-case $case native pass"
    elif [ "$case" = reclaim-three-slabs ] || [ "$case" = active-context-destroy ]; then
        echo "$case requires Linux x64 native execution" >&2
        exit 1
    fi
done
