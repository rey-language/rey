#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
runtime_dir=${REY_RUNTIME_DIR:-$repo_root/rey-compiler/runtime}
scratch_root=${REY_RUNTIME_TEST_SCRATCH:-/tmp/opencode}
build_dir="$scratch_root/rey-runtime-float-format"
cc=${CC:-/usr/bin/clang}

mkdir -p "$build_dir"

runtime_sources="\
$runtime_dir/mem.c \
$runtime_dir/string.c \
$runtime_dir/vec.c \
$runtime_dir/hashmap.c \
$runtime_dir/result.c \
$runtime_dir/enum.c \
$runtime_dir/io.c \
$runtime_dir/value.c"

run_normal() {
    # shellcheck disable=SC2086
    $cc -std=c11 -Wall -Wextra -Wno-unused-parameter -O2 \
        $runtime_sources "$runtime_dir/tests/float_format_test.c" \
        -o "$build_dir/float_format_test"
    "$build_dir/float_format_test"
}

run_sanitize() {
    # shellcheck disable=SC2086
    $cc -std=c11 -Wall -Wextra -Wno-unused-parameter -O1 -g \
        -fno-omit-frame-pointer -fsanitize=address,undefined \
        $runtime_sources "$runtime_dir/tests/float_format_test.c" \
        -o "$build_dir/float_format_test_asan_ubsan"
    ASAN_OPTIONS=${ASAN_OPTIONS:-detect_leaks=0} UBSAN_OPTIONS=${UBSAN_OPTIONS:-halt_on_error=1} \
        "$build_dir/float_format_test_asan_ubsan"
}

if [ "$#" -gt 1 ]; then
    echo "usage: $0 [normal|sanitize|all]" >&2
    exit 2
fi
case "${1:-all}" in
    normal) run_normal ;;
    sanitize) run_sanitize ;;
    all) run_normal; run_sanitize ;;
    *) echo "usage: $0 [normal|sanitize|all]" >&2; exit 2 ;;
esac
