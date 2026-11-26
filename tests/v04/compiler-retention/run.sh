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
[ "$STAGED" -eq 1 ] || { echo "compiler retention acceptance requires --staged" >&2; exit 2; }
[ "$REPEATS" = 3 ] || { echo "compiler retention acceptance requires --repeats 3" >&2; exit 2; }

CALLER=$(pwd -P)
case "$OUT" in /*) ;; *) OUT="$CALLER/$OUT" ;; esac
SRC=$(CDPATH= cd -- "$SRC" && pwd -P)
case "$BASE" in
    /*) ;;
    *) BASE="$CALLER/$BASE" ;;
esac
SEED=$(CDPATH= cd -- "$(dirname -- "$SEED")" && pwd -P)/$(basename -- "$SEED")
BUDGET="$SRC/scripts/v04-linux-x64-budget.sh"
CONTROL="$SRC/tests/v04/compiler-retention/control/managed/link/main.rey"
CONTROL_SHA=af3b2dc9614d1954f9ff62433e14a2086830aead2ac25ab2878938768c495975

if [ -e "$OUT" ]; then
    # Keep the incomplete or prior attempt intact and isolate this retry.
    mkdir -p "$OUT"
    OUT="$OUT/run-$(date -u +%Y%m%dT%H%M%SZ)-$$"
fi
if [ -d "$BASE" ]; then
    test -f "$BASE/rey-compiler/src/ir/managed/link/main.rey"
elif [ ! -x "$BASE" ]; then
    echo "FAIL P01: BASE must be a source tree or executable: $BASE" >&2
    exit 1
fi
test -x "$SEED"
test -f "$SRC/rey-compiler/native-main.rey"
test -f "$SRC/rey-compiler/src/ir/managed/link/main.rey"
test -f "$CONTROL"
test -f "$BUDGET"
actual_control_sha=$(sha256sum "$CONTROL" | awk '{print $1}')
if [ "$actual_control_sha" != "$CONTROL_SHA" ]; then
    echo "FAIL P01: frozen pre-P01 linker control changed" >&2
    exit 1
fi

mkdir -p "$OUT/stage/control/src" "$OUT/stage/current/src" "$OUT/compiler" "$OUT/fixtures" "$OUT/results" "$OUT/profiles" "$OUT/logs"
OUT=$(CDPATH= cd -- "$OUT" && pwd -P)

run_budget() {
    label=$1
    shift
    if ! sh "$BUDGET" run --seconds 1800 --label "$label" -- \
        sh "$SRC/tests/v04/compiler-retention/cgroup-exec.sh" "$@" \
        >"$OUT/logs/$label.wrapper.log" 2>&1; then
        cat "$OUT/logs/$label.wrapper.log" >&2
        return 1
    fi
}

stage_driver() {
    variant=$1
    stage="$OUT/stage/$variant/src"
    cp -R "$SRC/rey-compiler/src/." "$stage/"
    cp "$SRC/rey-compiler/native-main.rey" "$stage/main.rey"
    if [ "$variant" = control ]; then
        cp "$CONTROL" "$stage/ir/managed/link/main.rey"
    fi
    run_budget "P01-build-$variant-driver" "$SEED" build "$stage/main.rey" \
        -o "$OUT/compiler/$variant" --target linux-x64 --managed --regions \
        --runtime-root "$SRC/runtime-rey"
    test -x "$OUT/compiler/$variant"
}

generate_case() {
    name=$1
    functions=$2
    operations=$3
    source="$OUT/fixtures/$name.rey"
    : >"$source"
    function_index=0
    helper_count=$((functions - 1))
    while [ "$function_index" -lt "$helper_count" ]; do
        function_name=$(printf 'retained_%04d' "$function_index")
        printf 'func %s(): int {\n' "$function_name" >>"$source"
        printf '    var v000: int = 0;\n' >>"$source"
        operation_index=1
        while [ "$operation_index" -le "$operations" ]; do
            previous=$(printf 'v%03d' $((operation_index - 1)))
            current=$(printf 'v%03d' "$operation_index")
            printf '    var %s: int = %s + 1;\n' "$current" "$previous" >>"$source"
            operation_index=$((operation_index + 1))
        done
        final_value=$(printf 'v%03d' "$operations")
        printf '    return %s;\n}\n\n' "$final_value" >>"$source"
        function_index=$((function_index + 1))
    done
    cat >>"$source" <<'EOF'
func main(): Void {
}
EOF
}

run_compile() {
    variant=$1
    case_name=$2
    repetition=$3
    profile=$4
    compiler="$OUT/compiler/$variant"
    source="$OUT/fixtures/$case_name.rey"
    run_dir="$OUT/results/$variant/$case_name/run-$repetition"
    mkdir -p "$run_dir"
    label="P01-$variant-$case_name-r$repetition"
    if [ "$profile" = 1 ]; then
        run_budget "$label" env REY_COMPILER_RETENTION_PROFILE=1 python3 -B \
            "$SRC/tests/v04/compiler-retention/capture-profile.py" \
            --events "$run_dir/events.jsonl" --stdout "$run_dir/stdout" \
            --stderr "$run_dir/stderr" --status "$run_dir/status.json" -- \
            "$compiler" build "$source" -o "$run_dir/output" \
            --target linux-x64 --managed --regions --runtime-root "$SRC/runtime-rey"
        test -f "$run_dir/output"
        return
    fi
    run_budget "$label" sh -c '
        set -eu
        profile=$1
        timefile=$2
        compiler=$3
        source=$4
        image=$5
        runtime=$6
        stdout=$7
        stderr=$8
        if [ "$profile" = 1 ]; then export REY_COMPILER_RETENTION_PROFILE=1; fi
        exec /usr/bin/time -v -o "$timefile" "$compiler" build "$source" -o "$image" \
            --target linux-x64 --managed --regions --runtime-root "$runtime" >"$stdout" 2>"$stderr"
    ' p01 "$profile" "$run_dir/time.txt" "$compiler" "$source" "$run_dir/output" \
        "$SRC/runtime-rey" "$run_dir/stdout" "$run_dir/stderr"
    test -f "$run_dir/output"
}

stage_driver control
stage_driver current

generate_case small 16 2
generate_case functions-511 511 2
generate_case functions-512 512 2
generate_case functions-513 513 2
generate_case scaled 2049 24

for case_name in small functions-511 functions-512 functions-513 scaled; do
    repetition=1
    while [ "$repetition" -le "$REPEATS" ]; do
        run_compile control "$case_name" "$repetition" 0
        run_compile current "$case_name" "$repetition" 0
        repetition=$((repetition + 1))
    done
done

for case_name in small functions-511 functions-512 functions-513 scaled; do
    repetition=1
    while [ "$repetition" -le "$REPEATS" ]; do
        control_run="$OUT/results/control/$case_name/run-$repetition"
        current_run="$OUT/results/current/$case_name/run-$repetition"
        if ! cmp -s "$control_run/stdout" "$current_run/stdout" || ! cmp -s "$control_run/stderr" "$current_run/stderr"; then
            echo "FAIL P01: $case_name stdout or diagnostics differ at repeat $repetition" >&2
            diff -u "$control_run/stderr" "$current_run/stderr" >&2 || true
            exit 1
        fi
        if ! cmp -s "$control_run/output" "$current_run/output"; then
            echo "FAIL P01: $case_name image differs at repeat $repetition" >&2
            exit 1
        fi
        repetition=$((repetition + 1))
    done
done

for case_name in small functions-511 functions-512 functions-513 scaled; do
    run_compile current "$case_name" profile 1
    profile_run="$OUT/results/current/$case_name/run-profile"
    sed '/^REY_RETENTION event=/d' "$profile_run/stderr" >"$OUT/profiles/$case_name.diagnostics"
    if ! cmp -s "$OUT/results/control/$case_name/run-1/stderr" "$OUT/profiles/$case_name.diagnostics"; then
        echo "FAIL P01: profiling changed $case_name compiler diagnostics" >&2
        exit 1
    fi
    if ! cmp -s "$OUT/results/current/$case_name/run-1/output" "$profile_run/output"; then
        echo "FAIL P01: profiling changed $case_name output" >&2
        exit 1
    fi
    grep '^REY_RETENTION event=' "$profile_run/stderr" >"$OUT/profiles/$case_name.events"
    test -s "$OUT/profiles/$case_name.events"
    grep -q 'event=runtime_reload_sources ' "$OUT/profiles/$case_name.events"
    grep -q 'event=runtime_reload_lowered ' "$OUT/profiles/$case_name.events"
    grep -q 'event=user_lowered ' "$OUT/profiles/$case_name.events"
    grep -q 'event=user_lowering_start ' "$OUT/profiles/$case_name.events"
    grep -q 'event=runtime_reload_start ' "$OUT/profiles/$case_name.events"
    grep -q 'event=verification_start ' "$OUT/profiles/$case_name.events"
    grep -q 'event=link_copy_start ' "$OUT/profiles/$case_name.events"
    grep -q 'event=scalar_replacement_start ' "$OUT/profiles/$case_name.events"
    grep -q 'event=region_lowering_start ' "$OUT/profiles/$case_name.events"
    grep -q 'event=copy ' "$OUT/profiles/$case_name.events"
    grep -q 'event=region_lowered ' "$OUT/profiles/$case_name.events"
    python3 -B - "$profile_run/events.jsonl" "$profile_run/status.json" "$OUT/profiles/$case_name.phases.csv" <<'PY'
import json
import sys
from pathlib import Path

events_path, status_path, output_path = map(Path, sys.argv[1:])
events = [json.loads(line) for line in events_path.read_text().splitlines()]
by_name = {}
for item in events:
    fields = dict(part.split("=", 1) for part in item["line"].split()[1:])
    name = item["line"].split(" event=", 1)[1].split()[0]
    by_name[name] = (item["timestamp_ns"], fields)
status = json.loads(status_path.read_text())
phases = [
    ("user_lowering", "user_lowering_start", "user_lowered"),
    ("runtime_reload_and_lower", "runtime_reload_start", "runtime_reload_lowered"),
    ("link_verification", "verification_start", "runtime_verification" if "runtime_verification" in by_name else "link_verification"),
    ("link_copy", "link_copy_start", "copy"),
    ("scalar_replacement", "scalar_replacement_start", "scalar_replacement"),
    ("region_lowering_and_final_verification", "region_lowering_start", "region_lowered"),
]
rows = ["phase,wall_ns"]
for label, start, end in phases:
    if start not in by_name or end not in by_name:
        continue
    duration = by_name[end][0] - by_name[start][0]
    if duration < 0:
        raise SystemExit("phase event timestamps are out of order")
    rows.append(f"{label},{duration}")
rows.append(f"whole_compile,{status['ended_ns'] - status['started_ns']}")
output_path.write_text("\n".join(rows) + "\n")
PY
done

for expected in 16 511 512 513; do
    case_name=functions-$expected
    if [ "$expected" = 16 ]; then case_name=small; fi
    event_line=$(grep -E 'event=user_verification |event=link_verification ' "$OUT/profiles/$case_name.events" | head -n 1 || true)
    actual=$(printf '%s\n' "$event_line" | sed -n 's/.*functions=\([0-9][0-9]*\).*/\1/p')
    if [ "$actual" != "$expected" ]; then
        echo "FAIL P01: fixture $case_name lowered to $actual functions; expected $expected" >&2
        exit 1
    fi
done
grep -q 'event=user_verification ' "$OUT/profiles/small.events"
grep -q 'event=runtime_verification ' "$OUT/profiles/small.events"
grep -q 'event=user_verification functions=511 ' "$OUT/profiles/functions-511.events"
grep -q 'event=user_verification functions=512 ' "$OUT/profiles/functions-512.events"
grep -q 'event=link_verification functions=513 skipped=512_cutoff' "$OUT/profiles/functions-513.events"

copy_line=$(grep 'event=copy ' "$OUT/profiles/scaled.events" | tail -n 1)
saved_bytes=$(printf '%s\n' "$copy_line" | sed -n 's/.*saved_payload_bytes=\([0-9][0-9]*\).*/\1/p')
baseline_bytes=$(printf '%s\n' "$copy_line" | sed -n 's/.*baseline_link_copy_bytes=\([0-9][0-9]*\).*/\1/p')
retained_bytes=$(printf '%s\n' "$copy_line" | sed -n 's/.*retained_link_copy_bytes=\([0-9][0-9]*\).*/\1/p')
test -n "$saved_bytes" && test -n "$baseline_bytes" && test -n "$retained_bytes"
saved_percent=$(awk -v saved="$saved_bytes" -v baseline="$baseline_bytes" 'BEGIN { if (baseline == 0) print 0; else printf "%.2f", 100 * saved / baseline }')

max_rss_kb() {
    variant=$1
    case_name=$2
    awk '/Maximum resident set size \(kbytes\):/ { if ($NF > maximum) maximum = $NF } END { if (maximum == "") exit 1; print maximum }' \
        "$OUT/results/$variant/$case_name/run-1/time.txt" \
        "$OUT/results/$variant/$case_name/run-2/time.txt" \
        "$OUT/results/$variant/$case_name/run-3/time.txt"
}
baseline_peak=$(max_rss_kb control scaled)
retained_peak=$(max_rss_kb current scaled)
rss_gate=$(awk -v control="$baseline_peak" -v current="$retained_peak" 'BEGIN { if (current <= control * 1.05) print "pass"; else print "fail" }')

cat >"$OUT/summary.md" <<EOF
# P01 compiler retention experiment

- Source: $SRC
- BASE argument: $BASE
- Seed: $SEED
- Repeats: $REPEATS
- Attempt output: $OUT
- Control linker SHA-256: $CONTROL_SHA
- Current linker SHA-256: $(sha256sum "$SRC/rey-compiler/src/ir/managed/link/main.rey" | awk '{print $1}')
- Runtime tree SHA-256 list: $OUT/runtime-hashes.sha256
- Staged module comparison: diagnostics and output images match for small, 511, 512, 513, and scaled inputs.
- Scaled fixture: 2049 source functions, 24 integer operations per helper.
- Link nested payload copied in control model: $baseline_bytes bytes.
- Link nested payload copied with retained user IR: $retained_bytes bytes.
- Link nested payload avoided: $saved_bytes bytes ($saved_percent%).
- Scaled maximum RSS: control $baseline_peak KiB; retained $retained_peak KiB; 5% gate $rss_gate.
- Phase event details: profiles/*.events.
- R01 staged-driver report: r01/.
EOF

find "$SRC/runtime-rey" -type f -print0 | sort -z | xargs -0 sha256sum >"$OUT/runtime-hashes.sha256"
find "$SRC/rey-compiler/src" -type f -print0 | sort -z | xargs -0 sha256sum >"$OUT/compiler-source-hashes.sha256"
sha256sum "$SEED" "$SRC/rey-compiler/src/ir/managed/link/main.rey" "$CONTROL" \
    "$OUT/compiler/control" "$OUT/compiler/current" >"$OUT/input-hashes.sha256"

sh "$SRC/scripts/v04-linux-x64-region-gate.sh" --engine "$OUT/compiler/current" \
    --source "$SRC" --out "$OUT/r01" --suite regions >"$OUT/r01.stdout" 2>"$OUT/r01.stderr" || {
        cat "$OUT/r01.stdout" "$OUT/r01.stderr" >&2
        exit 1
    }

if [ "$rss_gate" != pass ]; then
    echo "FAIL P01: scaled peak RSS regression exceeds 5% ($baseline_peak -> $retained_peak KiB)" >&2
    exit 1
fi
if [ "$(awk -v percent="$saved_percent" 'BEGIN { print (percent >= 10.00) ? "pass" : "fail" }')" != pass ]; then
    echo "FAIL P01: scaled link copy reduction is below 10% ($saved_percent%)" >&2
    exit 1
fi

printf 'scaled_link_copy_bytes control=%s retained=%s saved=%s (%s%%)\n' \
    "$baseline_bytes" "$retained_bytes" "$saved_bytes" "$saved_percent"
printf 'scaled_peak_rss_kib control=%s retained=%s gate=%s\n' "$baseline_peak" "$retained_peak" "$rss_gate"
cat "$OUT/r01.stdout"
cat "$OUT/profiles/small.phases.csv" "$OUT/profiles/scaled.phases.csv"
printf '%s\n' 'PASS compiler retention'
