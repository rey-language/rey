#!/bin/sh
set -eu

if [ "$#" -lt 4 ]; then
    echo "usage: run.sh BASE SEED SRC OUT --staged --repeats N" >&2
    exit 2
fi

BASE=$1
SEED=$2
SRC=$3
OUT=$4
shift 4
STAGED=0
REPEATS=
while [ "$#" -gt 0 ]; do
    case "$1" in
        --staged) STAGED=1; shift ;;
        --repeats)
            [ "$#" -ge 2 ] || { echo "missing --repeats value" >&2; exit 2; }
            REPEATS=$2
            shift 2
            ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
done
[ "$STAGED" -eq 1 ] || { echo "compiler scratch acceptance requires --staged" >&2; exit 2; }
[ "$REPEATS" = 3 ] || { echo "compiler scratch acceptance requires --repeats 3" >&2; exit 2; }

CALLER=$(pwd -P)
case "$OUT" in /*) ;; *) OUT="$CALLER/$OUT" ;; esac
SRC=$(CDPATH= cd -- "$SRC" && pwd -P)
case "$BASE" in /*) ;; *) BASE="$CALLER/$BASE" ;; esac
SEED=$(CDPATH= cd -- "$(dirname -- "$SEED")" && pwd -P)/$(basename -- "$SEED")
DRIVER_SOURCE="$SRC/tests/v04/compiler-scratch/main.rey"
REGION_DRIVER_SOURCE="$SRC/tests/v04/compiler-scratch/region-lowering.rey"
RUNTIME_DRIVER_SOURCE="$SRC/tests/v04/compiler-scratch/runtime.rey"

if [ -d "$BASE" ]; then
    test -d "$BASE/rey-compiler/src"
elif [ ! -x "$BASE" ]; then
    echo "FAIL scratch lifetime: BASE must be a source tree or executable: $BASE" >&2
    exit 1
fi
test -x "$SEED"
test -f "$DRIVER_SOURCE"
test -f "$REGION_DRIVER_SOURCE"
test -f "$RUNTIME_DRIVER_SOURCE"

if [ -e "$OUT" ]; then
    mkdir -p "$OUT"
    OUT="$OUT/attempt-$(date -u +%Y%m%dT%H%M%SZ)-$$"
fi
mkdir -p "$OUT/stage/compiler" "$OUT/stage/region" "$OUT/stage/runtime" "$OUT/stage/compiler-runtime" "$OUT/bin" "$OUT/logs" "$OUT/results" "$OUT/rss"
OUT=$(CDPATH= cd -- "$OUT" && pwd -P)
STAGE="$OUT/stage/compiler"
REGION_STAGE="$OUT/stage/region"
RUNTIME_STAGE="$OUT/stage/runtime"
COMPILER_RUNTIME_STAGE="$OUT/stage/compiler-runtime"
cp -R "$SRC/rey-compiler/src/." "$STAGE/"
cp "$DRIVER_SOURCE" "$STAGE/main.rey"
cp -R "$SRC/rey-compiler/src/." "$REGION_STAGE/"
cp "$REGION_DRIVER_SOURCE" "$REGION_STAGE/main.rey"
cp "$RUNTIME_DRIVER_SOURCE" "$RUNTIME_STAGE/main.rey"
python3 -B "$SRC/tests/v04/compiler-scratch/stage-runtime.py" \
    "$SRC/runtime-rey" "$RUNTIME_STAGE" >"$OUT/logs/stage-runtime.stdout"
find "$RUNTIME_STAGE" -type f -print0 | sort -z | xargs -0 sha256sum >"$OUT/staged-runtime.sha256"
cp -R "$SRC/runtime-rey/." "$COMPILER_RUNTIME_STAGE/"
cp "$SRC/tests/v04/compiler-scratch/legacy-managed.rey" "$COMPILER_RUNTIME_STAGE/managed/main.rey"
sed '/^import regions;$/d' "$SRC/runtime-rey/main.rey" >"$COMPILER_RUNTIME_STAGE/main.rey"
find "$COMPILER_RUNTIME_STAGE" -type f -print0 | sort -z | xargs -0 sha256sum >"$OUT/staged-compiler-runtime.sha256"
mkdir -p "$OUT/runtime-fixtures/original" "$OUT/runtime-fixtures/staged"
cp "$RUNTIME_DRIVER_SOURCE" "$OUT/runtime-fixtures/original/region-basic.rey"
cp "$RUNTIME_DRIVER_SOURCE" "$OUT/runtime-fixtures/staged/region-basic.rey"

run_build() {
    rss=$2
    shift 2
    /usr/bin/time -v -o "$rss" \
        timeout --kill-after=30s 7200 "$@"
}

ANALYSIS_DRIVER="$OUT/bin/compiler-scratch-analysis"
REGION_LOWERING_DRIVER="$OUT/bin/compiler-scratch-region-lowering"
run_build P02-build-scratch-analysis "$OUT/rss/build-analysis.time" \
    "$SEED" build "$STAGE/main.rey" -o "$ANALYSIS_DRIVER" --target linux-x64 \
    --no-regions --runtime-root "$COMPILER_RUNTIME_STAGE" >"$OUT/logs/build-driver.stdout" 2>"$OUT/logs/build-driver.stderr"
test -x "$ANALYSIS_DRIVER"
run_build P02-build-scratch-region-lowering "$OUT/rss/build-region-lowering.time" \
    "$SEED" build "$REGION_STAGE/main.rey" -o "$REGION_LOWERING_DRIVER" --target linux-x64 \
    --no-regions --runtime-root "$COMPILER_RUNTIME_STAGE" >"$OUT/logs/build-region-lowering.stdout" 2>"$OUT/logs/build-region-lowering.stderr"
test -x "$REGION_LOWERING_DRIVER"
find "$SRC/rey-compiler/src" -type f -print0 | sort -z | xargs -0 sha256sum >"$OUT/compiler-source.sha256"
find "$SRC/runtime-rey" -type f -print0 | sort -z | xargs -0 sha256sum >"$OUT/runtime-source.sha256"
sha256sum "$SEED" "$DRIVER_SOURCE" "$REGION_DRIVER_SOURCE" "$RUNTIME_DRIVER_SOURCE" \
    "$SRC/tests/v04/compiler-scratch/stage-runtime.py" "$SRC/tests/v04/compiler-scratch/legacy-managed.rey" \
    "$ANALYSIS_DRIVER" "$REGION_LOWERING_DRIVER" \
    >"$OUT/producer-input-hashes.sha256"
if [ -d "$BASE" ]; then
    find "$BASE/rey-compiler/src/ir/managed" -type f -print0 | sort -z | xargs -0 sha256sum >"$OUT/base-managed-source.sha256"
else
    sha256sum "$BASE" >"$OUT/base-managed-source.sha256"
fi

timeout --kill-after=30s 25200 python3 -B \
    "$SRC/tests/v04/compiler-scratch/r01-uncapped.py" "$SEED" "$SRC" "$OUT/r01"
R01_REGION_DRIVER="$OUT/r01/bin/region-driver"
test -x "$R01_REGION_DRIVER"

run_runtime_probe_build() {
    report=$1
    shift
    /usr/bin/time -v -o "$report" \
        timeout --kill-after=30s 1800 "$@"
}

repeat=1
while [ "$repeat" -le "$REPEATS" ]; do
    stdout="$OUT/results/run-$repeat.stdout"
    stderr="$OUT/results/run-$repeat.stderr"
    nice -n 19 timeout --kill-after=30s 900 "$ANALYSIS_DRIVER" >"$OUT/results/analysis-$repeat.stdout" 2>"$OUT/results/analysis-$repeat.stderr"
    nice -n 19 timeout --kill-after=30s 900 "$REGION_LOWERING_DRIVER" >"$OUT/results/region-$repeat.stdout" 2>"$OUT/results/region-$repeat.stderr"
    runtime_output="$OUT/results/runtime-probe-$repeat"
    mkdir -p "$runtime_output"
    run_runtime_probe_build "$OUT/rss/build-runtime-probe-$repeat.time" \
        "$R01_REGION_DRIVER" region "$OUT/runtime-fixtures/original" "$OUT/runtime-fixtures/staged" \
        "$RUNTIME_STAGE" "$runtime_output" region-basic \
        >"$OUT/results/runtime-driver-$repeat.stdout" 2>"$OUT/results/runtime-driver-$repeat.stderr"
    grep -qx 'region-runtime-case region-basic 0' "$OUT/results/runtime-driver-$repeat.stdout"
    runtime_image="$runtime_output/linux-x64-region-basic"
    test -f "$runtime_image"
    chmod +x "$runtime_image"
    nice -n 19 timeout --kill-after=30s 900 "$runtime_image" \
        >"$OUT/results/runtime-$repeat.stdout" 2>"$OUT/results/runtime-$repeat.stderr"
    cat "$OUT/results/analysis-$repeat.stdout" "$OUT/results/region-$repeat.stdout" "$OUT/results/runtime-$repeat.stdout" >"$stdout"
    cat "$OUT/results/analysis-$repeat.stderr" "$OUT/results/region-$repeat.stderr" "$OUT/results/runtime-$repeat.stderr" >"$stderr"
    repeat=$((repeat + 1))
done

cmp -s "$OUT/results/run-1.stdout" "$OUT/results/run-2.stdout"
cmp -s "$OUT/results/run-1.stdout" "$OUT/results/run-3.stdout"
cmp -s "$OUT/results/run-1.stderr" "$OUT/results/run-2.stderr"
cmp -s "$OUT/results/run-1.stderr" "$OUT/results/run-3.stderr"
for expected in \
    'RESULT cutoffs=ir:511 managed:1 bounded=ok' \
    'RESULT cutoffs=ir:512 managed:1 bounded=ok' \
    'RESULT cutoffs=ir:513 managed:1 bounded=ok' \
    'RESULT cutoffs=ir:517 managed:511 bounded=ok' \
    'RESULT cutoffs=ir:517 managed:512 bounded=ok' \
    'RESULT cutoffs=ir:517 managed:513 bounded=ok'; do
    grep -Fqx "$expected" "$OUT/results/run-1.stdout"
done
test "$(grep -c '^RESULT cutoffs=' "$OUT/results/run-1.stdout")" -eq 6
test "$(grep -c '^RESULT semantic=bounded-scratch outcome=preserved$' "$OUT/results/run-1.stdout")" -eq 6
test "$(grep -c '^RESULT conservative=unknown-effect placement=program$' "$OUT/results/run-1.stdout")" -eq 6
test "$(grep -c '^RESULT conservative=uncertain-alias placement=program$' "$OUT/results/run-1.stdout")" -eq 6
test "$(grep '^RESULT semantic=bounded-scratch outcome=preserved$' "$OUT/results/run-1.stdout" | sort -u | wc -l)" -eq 1
test "$(grep '^RESULT conservative=unknown-effect placement=program$' "$OUT/results/run-1.stdout" | sort -u | wc -l)" -eq 1
test "$(grep '^RESULT conservative=uncertain-alias placement=program$' "$OUT/results/run-1.stdout" | sort -u | wc -l)" -eq 1
test "$(grep -c '^RESULT construction=returned destination=checked$' "$OUT/results/run-1.stdout")" -eq 5
grep -q '^RESULT scratch_pattern=boundedScratch warmed_1x=[1-9][0-9]* iterations_10x=[1-9][0-9]*$' "$OUT/results/run-1.stdout"
grep -qx 'RESULT region-lowering=bounded-cohort end=verified' "$OUT/results/run-1.stdout"
printf '%s\n' 'PASS scratch lifetime' >>"$OUT/results/run-1.stdout"

for report in "$OUT"/rss/*.time "$OUT"/r01/logs/*.time; do
    label=$(basename "$report" .time)
    printf 'compiler build peak RSS: %s ' "$label"
    awk '/Maximum resident set size \(kbytes\):/ { printf "%s KiB\n", $NF; found=1 } END { if (!found) exit 1 }' "$report"
done
cat "$OUT/results/run-1.stdout"
