#!/bin/sh
set -eu

if [ "$#" -lt 2 ] || [ "$#" -gt 3 ]; then
    echo "usage: sh tests/native-cli/run.sh <native-rey> <runtime-dir> [scratch-dir]" >&2
    exit 2
fi

compiler=$1
runtime_source=$2
scratch_root=${3:-/tmp/rey-v03/native-cli-tests}
mkdir -p "$scratch_root"
scratch="$scratch_root/run-$$"
runtime_copy="$scratch/runtime path"
output_dir="$scratch/output path"

if [ ! -f "$runtime_source/librey_rt.a" ]; then
    echo "runtime directory must contain librey_rt.a: $runtime_source" >&2
    exit 2
fi
mkdir -p "$scratch" "$runtime_copy" "$output_dir"
cp -R "$runtime_source"/. "$runtime_copy"/

source_file="$scratch/source with spaces.rey"
broken_file="$scratch/broken source.rey"
cat > "$source_file" <<'REY'
println("native-cli-run");
REY
printf 'var broken = @;\n' > "$broken_file"

expect_status() {
    expected=$1
    name=$2
    shift 2
    set +e
    "$compiler" "$@" > "$scratch/$name.stdout" 2> "$scratch/$name.stderr"
    actual=$?
    set -e
    if [ "$actual" -ne "$expected" ]; then
        echo "$name: expected exit $expected, got $actual" >&2
        cat "$scratch/$name.stdout" "$scratch/$name.stderr" >&2
        return 1
    fi
}

expect_status 0 help --help
grep -F 'Usage: rey' "$scratch/help.stdout" >/dev/null
test ! -s "$scratch/help.stderr"
expect_status 0 version --version
grep -Fx 'rey 0.3.0' "$scratch/version.stdout" >/dev/null
test ! -s "$scratch/version.stderr"
expect_status 2 usage
grep -F 'Usage: rey' "$scratch/usage.stderr" >/dev/null
expect_status 2 unknown-command nope "$source_file"
grep -F 'unsupported command: nope' "$scratch/unknown-command.stderr" >/dev/null
expect_status 2 missing-output-value build "$source_file" -o
expect_status 2 missing-runtime-value check "$source_file" --runtime-dir
expect_status 2 unknown-option build "$source_file" --unknown
expect_status 2 option-used-before-value build "$source_file" -o --runtime-dir "$runtime_copy"

expect_status 0 check-silent check "$source_file"
test ! -s "$scratch/check-silent.stdout"
test ! -s "$scratch/check-silent.stderr"
test ! -e "$scratch/source with spaces"
test ! -e "$scratch/source with spaces.ll"
test ! -e "$scratch/source with spaces.o"
expect_status 0 check-runtime check "$source_file" --runtime-dir "$runtime_copy"
test ! -s "$scratch/check-runtime.stdout"
test ! -s "$scratch/check-runtime.stderr"
expect_status 1 invalid-runtime check "$source_file" --runtime-dir "$scratch/missing runtime"
grep -F 'does not contain librey_rt.a' "$scratch/invalid-runtime.stderr" >/dev/null
test ! -s "$scratch/invalid-runtime.stdout"

missing_file="$scratch/missing input with spaces.rey"
expect_status 1 missing-input check "$missing_file"
grep -F "$missing_file:?:?: error: file not found" "$scratch/missing-input.stderr" >/dev/null
expect_status 1 lexical-error check "$broken_file"
grep -F "$broken_file:1:" "$scratch/lexical-error.stderr" >/dev/null

cycle_a="$scratch/cycle a.rey"
cycle_b="$scratch/cycle b.rey"
printf 'import "./cycle b.rey";\n' > "$cycle_a"
printf 'import "./cycle a.rey";\n' > "$cycle_b"
expect_status 1 import-cycle check "$cycle_a"
grep -F 'circular import:' "$scratch/import-cycle.stderr" >/dev/null
duplicate="$scratch/duplicate imports.rey"
common="$scratch/common module.rey"
printf '// shared module\n' > "$common"
printf 'import "./common module.rey";\nimport "./common module.rey";\n' > "$duplicate"
expect_status 0 duplicate-import check "$duplicate"
test ! -s "$scratch/duplicate-import.stdout"

build_output="$output_dir/built rey"
expect_status 0 build-spaces build "$source_file" --output "$build_output" --runtime-dir "$runtime_copy"
printf '%s\n' "$build_output" > "$scratch/expected-build.stdout"
cmp -s "$scratch/expected-build.stdout" "$scratch/build-spaces.stdout"
test -f "$build_output"

run_output="$output_dir/run rey"
expect_status 0 run-spaces run "$source_file" -o "$run_output" --runtime-dir "$runtime_copy"
printf 'native-cli-run\n' > "$scratch/expected-run.stdout"
cmp -s "$scratch/expected-run.stdout" "$scratch/run-spaces.stdout"
test -f "$run_output"

echo 'native CLI checks passed'
