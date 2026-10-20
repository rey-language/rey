#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
RUNTIME="$ROOT/rey-compiler/runtime"
OUTPUT_DIR="$ROOT/build/runtime"

usage() {
    echo "usage: $0 [--output-dir DIR]" >&2
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --output-dir|-o)
            if [ "$#" -lt 2 ]; then usage; exit 2; fi
            OUTPUT_DIR=$2
            shift 2
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            usage
            echo "error: unknown argument: $1" >&2
            exit 2
            ;;
    esac
done

COMPILER=${CC:-/usr/bin/clang}
if ! command -v "$COMPILER" >/dev/null 2>&1; then
    echo "error: C compiler not found: $COMPILER" >&2
    exit 2
fi
COMPILER_PATH=$(command -v "$COMPILER")

ARCHIVER=${AR:-/usr/bin/ar}
if ! command -v "$ARCHIVER" >/dev/null 2>&1; then
    echo "error: archiver not found: $ARCHIVER" >&2
    exit 2
fi
ARCHIVER_PATH=$(command -v "$ARCHIVER")

MAKE=${MAKE:-/usr/bin/make}
if ! command -v "$MAKE" >/dev/null 2>&1; then
    echo "error: make not found: $MAKE" >&2
    exit 2
fi
MAKE_PATH=$(command -v "$MAKE")

mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR=$(CDPATH= cd -- "$OUTPUT_DIR" && pwd -P)
RUNTIME=$(CDPATH= cd -- "$RUNTIME" && pwd -P)
case "$OUTPUT_DIR/" in
    "$RUNTIME/"*)
        echo "error: output directory must be outside the runtime source directory" >&2
        exit 2
        ;;
esac

if ! ulimit -t 120; then
    echo "error: unable to set 120-second CPU limit" >&2
    exit 2
fi

"$MAKE_PATH" -C "$RUNTIME" \
    "OUT=$OUTPUT_DIR" \
    "CC=$COMPILER_PATH" \
    "AR=$ARCHIVER_PATH" \
    all

ARCHIVE="$OUTPUT_DIR/librey_rt.a"
if [ ! -f "$ARCHIVE" ]; then
    echo "error: build succeeded without producing $ARCHIVE" >&2
    exit 1
fi
printf '%s\n' "$ARCHIVE"
