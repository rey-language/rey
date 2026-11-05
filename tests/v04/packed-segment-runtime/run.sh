#!/bin/sh
set -eu
PATH=/usr/bin:/bin:/usr/sbin:/sbin
CC=/usr/bin/clang
CXX=/usr/bin/clang++
export PATH CC CXX
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
ENGINE=${REY_ENGINE:-"$ROOT/build/rey-linux-x86_64-bin"}
RUNTIME=${REY_RUNTIME:-/tmp/opencode/rt-linux-no-lto}
OUT=${REY_OUTPUT:-"$ROOT/build/v04/packed-segment-runtime"}
VM_LIMIT_KB=${REY_VM_LIMIT_KB:-12582912}
mkdir -p "$OUT"
STAGE=$(mktemp -d "$OUT/stage.XXXXXX")
test -x "$ENGINE"
test -f "$RUNTIME/librey_rt.a"
mkdir -p "$STAGE/compiler-src" "$STAGE/source/layout" "$STAGE/source/regions" "$STAGE/source/managed" "$STAGE/source/containers" "$STAGE/source/host" "$STAGE/out"
cp -R "$ROOT/rey-compiler/src/." "$STAGE/compiler-src/"
cp "$ROOT/tests/v04/packed-segment-runtime/main.rey" "$STAGE/compiler-src/main.rey"
cp "$ROOT/runtime-rey/layout/main.rey" "$STAGE/source/layout/main.rey"
cp "$ROOT/runtime-rey/regions/main.rey" "$STAGE/source/regions/main.rey"
cp "$ROOT/runtime-rey/managed/main.rey" "$STAGE/source/managed/main.rey"
cp "$ROOT/runtime-rey/containers/main.rey" "$STAGE/source/containers/main.rey"
cp "$ROOT/runtime-rey/host/main.rey" "$STAGE/source/host/main.rey"
if [ -d "$ROOT/std/src" ]; then export REY_STD_PATH="$ROOT/std/src"; fi
(ulimit -v "$VM_LIMIT_KB"; timeout 300 "$ENGINE" build "$STAGE/compiler-src/main.rey" -o "$STAGE/packed-segment-runtime-driver" --runtime-dir "$RUNTIME")
(cd "$ROOT" && "$STAGE/packed-segment-runtime-driver" "$ROOT/tests/v04/packed-segment-runtime/fixtures/packed-segments.rey" "$STAGE/source" "$STAGE/out")
chmod +x "$STAGE/out/packed-segment-fixture"
python3 "$ROOT/tests/v04/packed-segment-runtime/verify.py" "$STAGE/out"
