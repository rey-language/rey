#!/bin/sh
set -eu

fail() {
    printf 'FAIL embed runtime+stdlib: %s\n' "$*" >&2
    exit 1
}

if [ "$#" -ne 3 ]; then
    printf 'usage: sh tests/v04/embedded-source/run.sh SEED SOURCE WORK\n' >&2
    exit 2
fi

seed=$1
source_root=$(CDPATH= cd "$(dirname "$2")" && cd "$(basename "$2")" && pwd -P)
work_root=$3
test_root=$(CDPATH= cd "$(dirname "$0")" && pwd -P)
budget="$source_root/scripts/v04-linux-x64-budget.sh"
tool_source="$source_root/tools/v04-embed/main.rey"
fixture="$test_root/fixtures/source"
tag=$(date -u +%Y%m%dT%H%M%SZ)-$$
job="$work_root/acceptance-$tag"
generator="$job/native/rey-embed"

[ -x "$seed" ] || fail "seed is not executable: $seed"
[ -f "$tool_source" ] || fail "generator source is missing: $tool_source"
[ -f "$budget" ] || fail "budget wrapper is missing: $budget"
[ -d "$fixture/runtime-rey" ] || fail "runtime fixture is missing"
[ -d "$fixture/std/src" ] || fail "stdlib fixture is missing"
command -v strace >/dev/null 2>&1 || fail "strace is unavailable"
command -v readelf >/dev/null 2>&1 || fail "readelf is unavailable"
command -v sha256sum >/dev/null 2>&1 || fail "sha256sum is unavailable"

if [ "${REY_EMBED_ACCEPTANCE_LOCKED:-0}" != 1 ]; then
    exec sh "$budget" job --seconds 2400 --label E01-embed-acceptance -- \
        sh -c 'export REY_EMBED_ACCEPTANCE_LOCKED=1; exec sh "$1" "$2" "$3" "$4"' \
        sh "$test_root/run.sh" "$seed" "$source_root" "$work_root"
fi

mkdir -p "$work_root"
mkdir "$job"
mkdir "$job/native"

audit_exec() {
    trace=$1
    expected=$2
    [ -s "$trace" ] || fail "empty exec trace: $trace"
    while IFS= read -r event; do
        case "$event" in
            *execve*)
                case "$event" in
                    *"execve(\"$expected\""*) ;;
                    *) fail "unexpected exec in $trace: $event" ;;
                esac
                ;;
        esac
    done < "$trace"
    count=$(grep -F -c "execve(\"$expected\"" "$trace" || :)
    [ "$count" -eq 1 ] || fail "expected one exec of $expected in $trace; saw $count"
}

prepare_stage() {
    stage=$1
    mkdir -p "$stage/rey-compiler/src/frontend/embedded_runtime"
    mkdir -p "$stage/rey-compiler/src/frontend/embedded_stdlib"
    mkdir -p "$stage/embedded-source"
}

run_tool() {
    run_label=$1
    run_source=$2
    run_stage=$3
    run_mode=${4:-normal}
    trace="$job/$run_label.exec"
    log="$job/$run_label.budget.log"
    prepare_stage "$run_stage"
    if [ "$run_mode" = reverse ]; then
        if ! sh "$budget" run-held --seconds 2400 --label "E01-$run_label" -- \
            strace -f -e trace=execve,execveat -o "$trace" -- \
            "$generator" "$run_source" "$run_stage" --reverse-enumeration >"$log" 2>&1; then
            cat "$log" >&2
            fail "generator run failed: $run_label"
        fi
    else
        if ! sh "$budget" run-held --seconds 2400 --label "E01-$run_label" -- \
            strace -f -e trace=execve,execveat -o "$trace" -- \
            "$generator" "$run_source" "$run_stage" >"$log" 2>&1; then
            cat "$log" >&2
            fail "generator run failed: $run_label"
        fi
    fi
    audit_exec "$trace" "$generator"
}

assert_same() {
    cmp -s "$1" "$2" || fail "bytes differ: $1 and $2"
}

assert_different() {
    if cmp -s "$1" "$2"; then fail "expected changed output: $1"; fi
}

assert_no_generated_files() {
    stage=$1
    [ ! -f "$stage/rey-compiler/src/frontend/embedded_runtime/main.rey" ] || fail "rejected input left a runtime overlay"
    [ ! -f "$stage/rey-compiler/src/frontend/embedded_stdlib/main.rey" ] || fail "rejected input left a stdlib overlay"
    [ ! -f "$stage/embedded-source/runtime.inventory" ] || fail "rejected input left a runtime inventory"
    [ ! -f "$stage/embedded-source/stdlib.inventory" ] || fail "rejected input left a stdlib inventory"
}

if ! sh "$budget" run-held --seconds 2400 --label E01-embed-build -- \
    strace -f -e trace=execve,execveat -o "$job/build.exec" -- \
    "$seed" build "$tool_source" -o "$generator" --target linux-x64 --regions --managed >"$job/build.budget.log" 2>&1; then
    cat "$job/build.budget.log" >&2
    fail "seed could not build the native generator"
fi
audit_exec "$job/build.exec" "$seed"
[ -x "$generator" ] || fail "native generator is not executable"

elf_header=$(readelf -h "$generator" 2>&1) || fail "cannot inspect native generator ELF"
printf '%s\n' "$elf_header" | grep -Eq 'Class:[[:space:]]+ELF64' || fail "generator is not ELF64"
printf '%s\n' "$elf_header" | grep -Eq 'Type:[[:space:]]+EXEC ' || fail "generator is not ET_EXEC"
printf '%s\n' "$elf_header" | grep -Eq 'Machine:[[:space:]]+Advanced Micro Devices X86-64' || fail "generator is not x86-64"
program_headers=$(readelf -l "$generator" 2>&1) || fail "cannot inspect generator segments"
case "$program_headers" in *INTERP*|*DYNAMIC*) fail "generator requests a dynamic loader or segment" ;; esac
dynamic_report=$(readelf -d "$generator" 2>&1) || fail "cannot inspect generator dynamic section"
case "$dynamic_report" in *"There is no dynamic section"*) ;; *) fail "generator has a dynamic section" ;; esac
native_files=$(find "$job/native" -type f -print)
[ "$native_files" = "$generator" ] || fail "unexpected native generator sidecars: $native_files"
sidecars=$(find "$job/native" -type f \( -name '*.so' -o -name '*.so.*' -o -name '*.a' -o -name '*.o' \) -print)
[ -z "$sidecars" ] || fail "native generator has library or object sidecars: $sidecars"

full_one="$job/full-one"
full_two="$job/full-two"
full_reverse="$job/full-reverse"
run_tool full-one "$source_root" "$full_one"
run_tool full-two "$source_root" "$full_two"
run_tool full-reverse "$source_root" "$full_reverse" reverse

runtime_relative=rey-compiler/src/frontend/embedded_runtime/main.rey
stdlib_relative=rey-compiler/src/frontend/embedded_stdlib/main.rey
runtime_inventory=embedded-source/runtime.inventory
stdlib_inventory=embedded-source/stdlib.inventory
assert_same "$full_one/$runtime_relative" "$full_two/$runtime_relative"
assert_same "$full_one/$stdlib_relative" "$full_two/$stdlib_relative"
assert_same "$full_one/$runtime_inventory" "$full_two/$runtime_inventory"
assert_same "$full_one/$stdlib_inventory" "$full_two/$stdlib_inventory"
assert_same "$full_one/$runtime_relative" "$full_reverse/$runtime_relative"
assert_same "$full_one/$stdlib_relative" "$full_reverse/$stdlib_relative"
assert_same "$full_one/$runtime_inventory" "$full_reverse/$runtime_inventory"
assert_same "$full_one/$stdlib_inventory" "$full_reverse/$stdlib_inventory"
assert_same "$full_one/$runtime_relative" "$source_root/$runtime_relative"
assert_same "$full_one/$stdlib_relative" "$source_root/$stdlib_relative"
assert_same "$full_one/$runtime_inventory" "$test_root/baseline/runtime.inventory"
assert_same "$full_one/$stdlib_inventory" "$test_root/baseline/stdlib.inventory"

fixture_one="$job/fixture-one"
fixture_reverse="$job/fixture-reverse"
run_tool fixture-one "$fixture" "$fixture_one"
run_tool fixture-reverse "$fixture" "$fixture_reverse" reverse
assert_same "$fixture_one/$runtime_relative" "$test_root/baseline/fixture-runtime.rey"
assert_same "$fixture_one/$stdlib_relative" "$test_root/baseline/fixture-stdlib.rey"
assert_same "$fixture_one/$runtime_inventory" "$test_root/baseline/fixture-runtime.inventory"
assert_same "$fixture_one/$stdlib_inventory" "$test_root/baseline/fixture-stdlib.inventory"
assert_same "$fixture_one/$runtime_relative" "$fixture_reverse/$runtime_relative"
assert_same "$fixture_one/$stdlib_relative" "$fixture_reverse/$stdlib_relative"
assert_same "$fixture_one/$runtime_inventory" "$fixture_reverse/$runtime_inventory"
assert_same "$fixture_one/$stdlib_inventory" "$fixture_reverse/$stdlib_inventory"

addition_source="$job/addition-source"
mkdir -p "$addition_source"
cp -R "$fixture/." "$addition_source/"
mkdir -p "$addition_source/runtime-rey/new"
cp "$test_root/fixtures/edits/added.rey" "$addition_source/runtime-rey/new/added.rey"
addition_output="$job/addition-output"
run_tool addition "$addition_source" "$addition_output"
assert_same "$addition_output/$runtime_relative" "$test_root/baseline/addition-runtime.rey"
assert_same "$addition_output/$stdlib_relative" "$test_root/baseline/addition-stdlib.rey"
assert_same "$addition_output/$runtime_inventory" "$test_root/baseline/addition-runtime.inventory"
assert_same "$addition_output/$stdlib_inventory" "$test_root/baseline/addition-stdlib.inventory"
assert_different "$addition_output/$runtime_relative" "$fixture_one/$runtime_relative"
assert_same "$addition_output/$stdlib_relative" "$fixture_one/$stdlib_relative"

edited_source="$job/edited-source"
mkdir -p "$edited_source"
cp -R "$fixture/." "$edited_source/"
cp "$test_root/fixtures/edits/runtime-main.rey" "$edited_source/runtime-rey/main.rey"
edited_output="$job/edited-output"
run_tool edited "$edited_source" "$edited_output"
assert_same "$edited_output/$runtime_relative" "$test_root/baseline/edited-runtime.rey"
assert_same "$edited_output/$stdlib_relative" "$test_root/baseline/edited-stdlib.rey"
assert_same "$edited_output/$runtime_inventory" "$test_root/baseline/edited-runtime.inventory"
assert_same "$edited_output/$stdlib_inventory" "$test_root/baseline/edited-stdlib.inventory"
assert_different "$edited_output/$runtime_relative" "$fixture_one/$runtime_relative"
assert_same "$edited_output/$stdlib_relative" "$fixture_one/$stdlib_relative"

missing_runtime="$job/missing-runtime-source"
mkdir -p "$missing_runtime/std/src/core"
cp "$fixture/std/src/core/main.rey" "$missing_runtime/std/src/core/main.rey"
missing_runtime_stage="$job/missing-runtime-stage"
prepare_stage "$missing_runtime_stage"
if sh "$budget" run-held --seconds 2400 --label E01-missing-runtime -- \
    strace -f -e trace=execve,execveat -o "$job/missing-runtime.exec" -- \
    "$generator" "$missing_runtime" "$missing_runtime_stage" >"$job/missing-runtime.budget.log" 2>&1; then
    fail "generator accepted a missing runtime input"
fi
audit_exec "$job/missing-runtime.exec" "$generator"
grep -F 'missing runtime source directory' "$job/missing-runtime.budget.log" >/dev/null || fail "missing runtime rejection was not diagnosed"
assert_no_generated_files "$missing_runtime_stage"

missing_stdlib="$job/missing-stdlib-source"
mkdir -p "$missing_stdlib/runtime-rey"
cp "$fixture/runtime-rey/main.rey" "$missing_stdlib/runtime-rey/main.rey"
missing_stdlib_stage="$job/missing-stdlib-stage"
prepare_stage "$missing_stdlib_stage"
if sh "$budget" run-held --seconds 2400 --label E01-missing-stdlib -- \
    strace -f -e trace=execve,execveat -o "$job/missing-stdlib.exec" -- \
    "$generator" "$missing_stdlib" "$missing_stdlib_stage" >"$job/missing-stdlib.budget.log" 2>&1; then
    fail "generator accepted a missing stdlib input"
fi
audit_exec "$job/missing-stdlib.exec" "$generator"
grep -F 'missing stdlib source directory' "$job/missing-stdlib.budget.log" >/dev/null || fail "missing stdlib rejection was not diagnosed"
assert_no_generated_files "$missing_stdlib_stage"

{
    printf 'acceptance=PASS embed runtime+stdlib\n'
    printf 'seed=%s\n' "$seed"
    printf 'generator=%s\n' "$generator"
    printf 'baseline_runtime_bytes=PASS\n'
    printf 'baseline_stdlib_bytes=PASS\n'
    printf 'pinned_inventories=PASS\n'
    printf 'repeat_and_reverse_determinism=PASS\n'
    printf 'staged_addition_and_edit=PASS\n'
    printf 'missing_runtime_and_stdlib_rejected=PASS\n'
    printf 'exec_allowlist_and_static_elf=PASS\n'
} > "$job/evidence.txt"
sha256sum "$seed" "$tool_source" "$generator" \
    "$full_one/$runtime_relative" "$full_one/$stdlib_relative" \
    "$full_one/$runtime_inventory" "$full_one/$stdlib_inventory" > "$job/sha256.txt"
(
    cd "$source_root"
    find runtime-rey -type f -name '*.rey' -print0 | LC_ALL=C sort -z | xargs -0 sha256sum
) > "$job/runtime-source.sha256"
(
    cd "$source_root"
    find std/src -type f -name '*.rey' -print0 | LC_ALL=C sort -z | xargs -0 sha256sum
) > "$job/stdlib-source.sha256"

printf 'PASS embed runtime+stdlib\n'
