#!/bin/sh
# Linux x86_64 native self-host chain for the v0.4 compiler.
# S0 builds H; H builds S1; S1 builds S2; S2 builds S3. S2 and S3 must match.
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
seed=${SEED:-$root/build/rey-linux-x86_64-bin}
seed_runtime=${SEED_RUNTIME_DIR:-/tmp/opencode/rt-linux-no-lto}
work=${WORK:-$root/build/tmp/self-linux-x64}
stage="$work/compiler.<fresh>"
fixtures="$root/tests/v04/selfhost"
through=${THROUGH:-s3}
timeout_sec=${TIMEOUT_SEC:-7200}
dry_run=${DRY_RUN:-0}
host=${HOST:-}

case "$through" in h|s1|s2|s3) ;; *) echo "THROUGH must be h, s1, s2, or s3" >&2; exit 2 ;; esac
if [ "$dry_run" != 1 ] && [ ! -x /usr/bin/time ]; then echo "/usr/bin/time is required" >&2; exit 1; fi
if [ -n "$host" ] && [ "$through" != h ]; then echo "HOST requires THROUGH=h" >&2; exit 2; fi

run_timed() {
    label=$1
    shift
    log="$work/$label.log"
    if [ "$dry_run" = 1 ]; then
        echo "DRY_RUN: timeout $timeout_sec /usr/bin/time -v $* > $log 2>&1"
        return 0
    fi
    mkdir -p "$work"
    echo "== $label"
    set +e
    if command -v flock >/dev/null 2>&1; then
        mkdir -p /tmp/opencode
        flock /tmp/opencode/heavy.lock timeout "$timeout_sec" /usr/bin/time -v "$@" >"$log" 2>&1
    else
        timeout "$timeout_sec" /usr/bin/time -v "$@" >"$log" 2>&1
    fi
    status=$?
    set -e
    sed -n '/^[[:space:]]*Command being timed:/p; /^[[:space:]]*User time (seconds):/p; /^[[:space:]]*System time (seconds):/p; /^[[:space:]]*Elapsed (wall clock) time (h:mm:ss or m:ss):/p; /^[[:space:]]*Maximum resident set size (kbytes):/p' "$log"
    if [ "$status" -ne 0 ]; then cat "$log"; echo "$label failed with status $status" >&2; exit "$status"; fi
}

check_help() {
    compiler=$1
    if [ "$dry_run" = 1 ]; then echo "DRY_RUN: $compiler --help"; return; fi
    "$compiler" --help >/dev/null
}

supports_selfhost_fast_exit() {
    compiler=$1
    if [ "$dry_run" = 1 ]; then return 0; fi
    help=$("$compiler" --help 2>&1) || return 1
    case "$help" in *--selfhost-fast-exit*) return 0 ;; *) return 1 ;; esac
}

expect_status() {
    label=$1 status=$2
    shift 2
    echo "== run $label expect $status"
    set +e
    "$@"
    actual=$?
    set -e
    if [ "$actual" -ne "$status" ]; then echo "$label exited $actual, expected $status" >&2; exit 1; fi
}

compile_fixture() {
    compiler=$1 name=$2
    output="$work/smoke/$name"
    set -- "$compiler" build "$fixtures/$name.rey" -o "$output" --target linux-x64
    if [ "$name" = variadic ]; then set -- "$@" --regions --managed; fi
    run_timed "smoke-$(basename "$compiler")-$name" "$@"
    if [ "$dry_run" != 1 ]; then chmod +x "$output"; fi
}

smoke() {
    compiler=$1
    [ "$dry_run" = 1 ] || mkdir -p "$work/smoke"
    for fixture in add overflow for-range defaults globals variadic; do
        compile_fixture "$compiler" "$fixture"
        case "$fixture" in overflow) expected=1 ;; *) expected=0 ;; esac
        if [ "$dry_run" != 1 ]; then expect_status "$fixture" "$expected" "$work/smoke/$fixture"; fi
    done
}

if [ "$dry_run" != 1 ]; then mkdir -p "$work"; fi
if [ -n "$host" ]; then
    check_help "$host"
    smoke "$host"
    echo "stopped after H ($host)"
    exit 0
fi

if [ "$dry_run" != 1 ]; then
    [ -x "$seed" ] || { echo "seed is missing or not executable: $seed" >&2; exit 1; }
    [ -d "$seed_runtime" ] || { echo "seed runtime directory is missing: $seed_runtime" >&2; exit 1; }
    stage=$(mktemp -d "$work/compiler.XXXXXXXX")
    python3 "$root/scripts/embed-runtime.py"
    python3 "$root/scripts/embed-stdlib.py"
    cp -R "$root/rey-compiler/src/." "$stage/"
    cp "$root/rey-compiler/native-main.rey" "$stage/main.rey"
else
    echo "DRY_RUN: stage sources from $root/rey-compiler into $stage"
fi

build_seed() {
    run_timed S0-build-H "$seed" build "$stage/main.rey" -o "$work/H" --runtime-dir "$seed_runtime"
    [ "$dry_run" = 1 ] || chmod +x "$work/H"
}
build_native_stage() {
    name=$1 input=$2 output=$3
    set -- "$input" build "$stage/main.rey" -o "$output" --target linux-x64 --regions --managed --trust-packed-output
    if supports_selfhost_fast_exit "$input"; then set -- "$@" --selfhost-fast-exit; fi
    run_timed "$name" "$@"
    [ "$dry_run" = 1 ] || chmod +x "$output"
}

build_seed
check_help "$work/H"
smoke "$work/H"
[ "$through" != h ] || { echo "stopped after H"; exit 0; }
build_native_stage H-builds-S1 "$work/H" "$work/S1"
check_help "$work/S1"
[ "$through" != s1 ] || { echo "stopped after S1"; exit 0; }
smoke "$work/S1"
build_native_stage S1-builds-S2 "$work/S1" "$work/S2"
check_help "$work/S2"
[ "$through" != s2 ] || { echo "stopped after S2"; exit 0; }
smoke "$work/S2"
build_native_stage S2-builds-S3 "$work/S2" "$work/S3"
check_help "$work/S3"
if [ "$dry_run" = 1 ]; then echo "DRY_RUN: cmp $work/S2 $work/S3"
elif cmp -s "$work/S2" "$work/S3"; then echo "S2 and S3 are byte-identical"
else echo "S2 and S3 differ" >&2; exit 1; fi
