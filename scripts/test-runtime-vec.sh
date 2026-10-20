#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
RUNTIME="${REY_RUNTIME_DIR:-$ROOT/rey-compiler/runtime}"
TEST="$RUNTIME/tests/vec_test.c"
MODE=normal

if [ "$#" -gt 1 ]; then
    echo "usage: $0 [--normal|--sanitize]" >&2
    exit 2
fi
case "${1:-}" in
    ""|--normal) ;;
    --sanitize) MODE=sanitize ;;
    *) echo "usage: $0 [--normal|--sanitize]" >&2; exit 2 ;;
esac

COMPILER=${CC:-/usr/bin/clang}
if ! command -v "$COMPILER" >/dev/null 2>&1; then
    echo "error: C compiler not found: $COMPILER" >&2
    exit 2
fi
COMPILER_PATH=$(command -v "$COMPILER")

TMP_ROOT=${REY_TEST_TMPDIR:-/tmp/rey-v03}
mkdir -p "$TMP_ROOT"
BUILD_DIR=$(mktemp -d "$TMP_ROOT/rey-vec-test.XXXXXX")
BIN="$BUILD_DIR/vec-test"

cleanup() {
    rm -f "$BIN"
    rmdir "$BUILD_DIR" 2>/dev/null || true
}
trap cleanup 0
trap 'exit 1' HUP INT TERM

if ! ulimit -t 120; then
    echo "error: unable to set 120-second CPU limit" >&2
    exit 2
fi

set -- -std=c11 -D_POSIX_C_SOURCE=200809L -Wall -Wextra -Wno-unused-parameter -I "$RUNTIME"
if [ "$MODE" = sanitize ]; then
    set -- "$@" -fsanitize=address,undefined -fno-omit-frame-pointer
fi

echo "building vec regression test ($MODE) with $COMPILER_PATH"
"$COMPILER_PATH" "$@" \
    "$RUNTIME/mem.c" \
    "$RUNTIME/string.c" \
    "$RUNTIME/vec.c" \
    "$RUNTIME/hashmap.c" \
    "$RUNTIME/result.c" \
    "$RUNTIME/enum.c" \
    "$RUNTIME/io.c" \
    "$RUNTIME/value.c" \
    "$TEST" \
    -o "$BIN"

if [ "$MODE" = sanitize ]; then
    ASAN_OPTIONS=${ASAN_OPTIONS:-detect_leaks=0} "$BIN"
else
    "$BIN"
fi
