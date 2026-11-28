#!/bin/sh
set -eu

if [ "$#" -ne 4 ] || [ "$3" != "--suite" ] || { [ "$4" != "core" ] && [ "$4" != "format" ]; }; then
    echo 'usage: run.sh REY OUT --suite core|format' >&2
    exit 2
fi

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
REY=$1
case "$REY" in /*) ;; *) REY="$(pwd -P)/$REY" ;; esac
OUT=$2
case "$OUT" in /*) ;; *) OUT="$(pwd -P)/$OUT" ;; esac
test -x "$REY" || { echo "FAIL f64 core: compiler is not executable: $REY" >&2; exit 1; }
if [ -e "$OUT" ]; then
    attempt=2
    while [ -e "$OUT/attempt-$attempt" ]; do attempt=$((attempt + 1)); done
    OUT="$OUT/attempt-$attempt"
fi
mkdir -p "$OUT"
SUITE=$4

build() {
    label=$1
    source=$2
    target=$3
    shift 3
    if /usr/bin/time -v timeout --kill-after=30s 600 "$REY" build "$SCRIPT_DIR/$source" \
        -o "$OUT/$label" --target "$target" "$@" >"$OUT/$label.stdout" 2>"$OUT/$label.time"; then
        chmod +x "$OUT/$label"
    else
        return $?
    fi
}

expect_build_failure() {
    label=$1
    pattern=$2
    shift 2
    if build "$label" "$@"; then
        echo "FAIL f64 core: unsupported case unexpectedly built: $label" >&2
        exit 1
    fi
    if ! grep -Eiq "$pattern" "$OUT/$label.time"; then
        echo "FAIL f64 core: unsupported case failed without a visible diagnostic: $label" >&2
        cat "$OUT/$label.time" >&2
        exit 1
    fi
}

run_status() {
    label=$1
    executable=$2
    expected=$3
    set +e
    timeout --kill-after=5s 30 "$executable" >"$OUT/$label.run.stdout" 2>"$OUT/$label.run.stderr"
    actual=$?
    set -e
    if [ "$actual" -ne "$expected" ]; then
        echo "FAIL f64 core: $label exited $actual, expected $expected" >&2
        cat "$OUT/$label.run.stderr" >&2
        exit 1
    fi
    test ! -s "$OUT/$label.run.stdout" || { echo "FAIL f64 core: $label wrote unexpected stdout" >&2; exit 1; }
}

run_format_suite() {
    if /usr/bin/time -v timeout --kill-after=30s 1200 "$REY" build "$SCRIPT_DIR/format.rey" \
        -o "$OUT/format" --target linux-x64 --managed --regions >"$OUT/format.stdout" 2>"$OUT/format.time"; then
        chmod +x "$OUT/format"
    else
        echo 'FAIL f64 format: format fixture did not build' >&2
        cat "$OUT/format.time" >&2
        exit 1
    fi
    timeout --kill-after=5s 30 "$OUT/format" >"$OUT/format.run.stdout" 2>"$OUT/format.run.stderr" || {
        echo 'FAIL f64 format: format fixture failed at runtime' >&2
        cat "$OUT/format.run.stderr" >&2
        exit 1
    }
    if ! cmp -s "$SCRIPT_DIR/format.expected" "$OUT/format.run.stdout"; then
        echo 'FAIL f64 format: formatted output differs from SPEC §9.1 oracle' >&2
        diff -u "$SCRIPT_DIR/format.expected" "$OUT/format.run.stdout" >&2 || true
        exit 1
    fi

    if /usr/bin/time -v timeout --kill-after=30s 1200 "$REY" build "$SCRIPT_DIR/../../../examples/basic/variables.rey" \
        -o "$OUT/variables" --target linux-x64 --managed --regions >"$OUT/variables.stdout" 2>"$OUT/variables.time"; then
        chmod +x "$OUT/variables"
    else
        echo 'FAIL f64 format: variables fixture did not build' >&2
        cat "$OUT/variables.time" >&2
        exit 1
    fi
    timeout --kill-after=5s 30 "$OUT/variables" >"$OUT/variables.run.stdout" 2>"$OUT/variables.run.stderr" || {
        echo 'FAIL f64 format: variables fixture failed at runtime' >&2
        cat "$OUT/variables.run.stderr" >&2
        exit 1
    }
    if ! cmp -s "$SCRIPT_DIR/variables.expected" "$OUT/variables.run.stdout"; then
        echo 'FAIL f64 format: variables example output differs' >&2
        diff -u "$SCRIPT_DIR/variables.expected" "$OUT/variables.run.stdout" >&2 || true
        exit 1
    fi

    for case_name in invalid-float-string float-overflow-string float-underflow-string; do
        if /usr/bin/time -v timeout --kill-after=30s 1200 "$REY" build "$SCRIPT_DIR/$case_name.rey" \
            -o "$OUT/$case_name" --target linux-x64 --managed --regions >"$OUT/$case_name.stdout" 2>"$OUT/$case_name.time"; then
            chmod +x "$OUT/$case_name"
        else
            echo "FAIL f64 format: conversion failure fixture did not build: $case_name" >&2
            cat "$OUT/$case_name.time" >&2
            exit 1
        fi
        run_status "$case_name" "$OUT/$case_name" 1
    done

    sha256sum "$REY" "$SCRIPT_DIR"/*.rey >"$OUT/input-hashes.sha256"
    max_rss_kib=0
    for timing in "$OUT"/*.time; do
        rss=$(awk '/Maximum resident set size \(kbytes\):/ { print $6 }' "$timing" | tail -n 1)
        if [ -n "$rss" ] && [ "$rss" -gt "$max_rss_kib" ]; then max_rss_kib=$rss; fi
    done
    printf '%s\n' "$max_rss_kib" >"$OUT/peak-rss-kib.txt"
    printf '%s\n' 'PASS f64 format'
}

if [ "$SUITE" = "format" ]; then
    run_format_suite
    exit 0
fi

build core core.rey linux-x64
run_status core "$OUT/core" 0

mkdir -p "$OUT/evaluator-src"
cp -R "$SCRIPT_DIR/../../../rey-compiler/src/." "$OUT/evaluator-src/"
cp "$SCRIPT_DIR/evaluator.rey" "$OUT/evaluator-src/main.rey"
/usr/bin/time -v timeout --kill-after=30s 7200 "$REY" build "$OUT/evaluator-src/main.rey" \
    -o "$OUT/evaluator" --target linux-x64 --managed --regions >"$OUT/evaluator-build.stdout" 2>"$OUT/evaluator-build.time"
chmod +x "$OUT/evaluator"
run_status evaluator "$OUT/evaluator" 0

build integer-abi integer-abi.rey linux-x64
run_status integer-abi "$OUT/integer-abi" 0

for case_name in div-positive-zero div-negative-zero rem-positive-zero rem-negative-zero conversion-overflow; do
    build "$case_name" "$case_name.rey" linux-x64
    run_status "$case_name" "$OUT/$case_name" 1
done

expect_build_failure managed-unsupported 'float|unsupported' managed-unsupported.rey linux-x64 --managed --regions
expect_build_failure arm64-unsupported 'f64|float' arm64-unsupported.rey linux-arm64
expect_build_failure windows-unsupported 'f64|float' windows-unsupported.rey windows-x64

sha256sum "$REY" "$SCRIPT_DIR"/*.rey >"$OUT/input-hashes.sha256"
max_rss_kib=0
for timing in "$OUT"/*.time; do
    rss=$(awk '/Maximum resident set size \(kbytes\):/ { print $6 }' "$timing" | tail -n 1)
    if [ -n "$rss" ] && [ "$rss" -gt "$max_rss_kib" ]; then max_rss_kib=$rss; fi
done
printf '%s\n' "$max_rss_kib" >"$OUT/peak-rss-kib.txt"
printf '%s\n' 'PASS f64 core'
