#!/bin/sh
# Native self-host chain for the v0.4 compiler.
# S0 is the pinned v0.3.1 seed. S0 builds H from the compiler sources.
# H builds S1, S1 builds S2, and S2 builds S3. S2 and S3 must be byte-identical.
# Every compiler invocation runs under /tmp/memcap.sh at 3072 MB and 600 seconds.
#
# THROUGH=h|s1|s2|s3 stops after that stage. The default is s3.
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
seed="$root/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
seed_runtime="$root/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/runtime"
work="$root/build/tmp/self"
stage="$work/compiler"
fixtures="$root/tests/v04/selfhost"
through=${THROUGH:-s3}

export PATH="/usr/bin:/bin:/usr/sbin:/sbin"
export CC="/usr/bin/clang"
export CXX="/usr/bin/clang++"

if [ ! -x "$seed" ]; then
    echo "pinned seed is missing: $seed" >&2
    exit 1
fi
if [ ! -x /tmp/memcap.sh ]; then
    echo "/tmp/memcap.sh is missing" >&2
    exit 1
fi

case "$through" in
    h|s1|s2|s3) ;;
    *) echo "THROUGH must be h, s1, s2, or s3" >&2; exit 2 ;;
esac

run_memcap() {
    label=$1
    shift
    echo "== $label"
    MEMCAP_MB=3072 MEMCAP_TIMEOUT_SEC=600 /tmp/memcap.sh "$@"
}

stage_sources() {
    rm -rf "$stage"
    mkdir -p "$stage"
    python3 "$root/scripts/embed-runtime.py"
    python3 "$root/scripts/embed-stdlib.py"
    cp -R "$root/rey-compiler/src/." "$stage/"
    cp "$root/rey-compiler/native-main.rey" "$stage/main.rey"
}

compile_with() {
    compiler=$1
    source=$2
    output=$3
    shift 3
    run_memcap "compile $(basename "$source")" "$compiler" build "$source" -o "$output" --target darwin-arm64 "$@"
    chmod +x "$output"
}

expect_status() {
    label=$1
    status=$2
    shift 2
    echo "== run $label expect $status"
    set +e
    "$@"
    actual=$?
    set -e
    if [ "$actual" -ne "$status" ]; then
        echo "$label exited $actual, expected $status" >&2
        exit 1
    fi
}

smoke_h() {
    out="$work/smoke"
    rm -rf "$out"
    mkdir -p "$out"
    compile_with "$work/H" "$fixtures/add.rey" "$out/add"
    expect_status add 0 "$out/add"
    compile_with "$work/H" "$fixtures/overflow.rey" "$out/overflow"
    expect_status overflow 1 "$out/overflow"
    compile_with "$work/H" "$fixtures/for-range.rey" "$out/for-range"
    expect_status for-range 0 "$out/for-range"
    compile_with "$work/H" "$fixtures/defaults.rey" "$out/defaults"
    expect_status defaults 0 "$out/defaults"
    compile_with "$work/H" "$fixtures/globals.rey" "$out/globals"
    expect_status globals 0 "$out/globals"
    compile_with "$work/H" "$fixtures/variadic.rey" "$out/variadic" --managed --regions
    expect_status variadic 0 "$out/variadic"
}

supports_selfhost_fast_exit() {
    compiler=$1
    help=$("$compiler" --help 2>&1) || return 1
    case "$help" in *--selfhost-fast-exit*) return 0 ;; *) return 1 ;; esac
}

build_stage() {
    name=$1
    input=$2
    output=$3
    set -- "$input" build "$stage/main.rey" -o "$output" --target darwin-arm64 --regions --managed --trust-packed-output
    if supports_selfhost_fast_exit "$input"; then set -- "$@" --selfhost-fast-exit; fi
    run_memcap "$name" "$@"
    chmod +x "$output"
}

echo "== stage compiler sources"
stage_sources
run_memcap "S0 builds H" "$seed" build "$stage/main.rey" -o "$work/H" --runtime-dir "$seed_runtime"
chmod +x "$work/H"
smoke_h
if [ "$through" = "h" ]; then
    echo "stopped after H"
    exit 0
fi

build_stage "H builds S1" "$work/H" "$work/S1"
if [ "$through" = "s1" ]; then
    echo "stopped after S1"
    exit 0
fi

build_stage "S1 builds S2" "$work/S1" "$work/S2"
if [ "$through" = "s2" ]; then
    echo "stopped after S2"
    exit 0
fi

build_stage "S2 builds S3" "$work/S2" "$work/S3"
if ! cmp -s "$work/S2" "$work/S3"; then
    echo "S2 and S3 differ" >&2
    exit 1
fi
echo "S2 and S3 are byte-identical"
