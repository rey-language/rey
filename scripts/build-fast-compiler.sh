#!/bin/sh
# Build the fast Linux x64 v0.4 compiler.
#
# Stage H of the documented self-host chain (scripts/v04-selfhost-linux-x64.sh):
# the v0.3.1 seed compiles rey-compiler/native-main.rey through LLVM/clang.
# H compiles ~16x faster than the native S1..S3 images because clang optimizes
# the same Rey source; the native backend emits unoptimized code today.
#
# The output is a standalone compiler binary. It embeds the managed runtime
# (scripts/embed-runtime.py) and writes executable images (io.c chmods images).
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
seed=${SEED:-$root/build/rey-linux-x86_64-bin}
runtime=${RUNTIME_DIR:-$root/build/runtime}
out=${OUT:-$root/build/v04/rey}
stage=${STAGE:-$root/build/tmp/fast-compiler}

if [ ! -x "$seed" ]; then
    echo "error: seed compiler not found: $seed" >&2
    exit 2
fi

# rebuild every time - a stale archive skips runtime fixes like executable images
"$root/scripts/build-runtime.sh" --output-dir "$runtime" >/dev/null

python3 "$root/scripts/embed-runtime.py"
python3 "$root/scripts/embed-stdlib.py"

rm -rf "$stage"
mkdir -p "$stage" "$(dirname "$out")"
cp -R "$root/rey-compiler/src/." "$stage/"
cp "$root/rey-compiler/native-main.rey" "$stage/main.rey"

"$seed" build "$stage/main.rey" -o "$out" --runtime-dir "$runtime"
chmod +x "$out"
"$out" --help >/dev/null
echo "$out"
