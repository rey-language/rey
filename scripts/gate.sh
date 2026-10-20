#!/usr/bin/env bash
# Tiered Rey v0.3 gates. Exit 0 pass, 1 fail, 2 incomplete.
#   scripts/gate.sh quick
#   scripts/gate.sh native-conformance
#   scripts/gate.sh selfcheck
#   scripts/gate.sh full
#   scripts/gate.sh bootstrap
#   scripts/gate.sh watch [--interval SECS]
set -u

usage() {
    cat <<'EOF'
Usage: scripts/gate.sh <quick|native-conformance|selfcheck|full|bootstrap|watch>
       scripts/gate.sh watch [--interval SECS]

  quick               oracle preflight, Rust-host check of the compiler source,
                      runtime quick test, native-codegen sidecars
  native-conformance  conformance corpus through the interpreted native pipeline
  selfcheck           Rey-written compiler self-check under the heavy lock
  full                quick, native-conformance, selfcheck, cargo test -j 2,
                      oracle conformance, runtime normal+sanitized, differential,
                      then scripts/ci.sh
  bootstrap           stage1 bootstrap under the heavy lock, at most once per
                      hour, and only after selfcheck passed on this source
  watch               loop: when sources or the oracle change, run quick then
                      native-conformance. Default interval 300s. Does not run
                      selfcheck or bootstrap. Parallelism is 1.

Exit 0 pass, 1 fail, 2 incomplete. watch runs until interrupted.
EOF
}

script_dir=$(cd "$(dirname "$0")" && pwd) || exit 2
script_path=$script_dir/$(basename "$0")
repo=$(cd "$script_dir/.." && pwd) || exit 2

ORACLE=${ORACLE:-/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/sprint2/bin/rey-v0}
SCRATCH=${REY_HARNESS_SCRATCH:-/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/sprint2/harness}
HEAVY_LOCK=${REY_HEAVY_LOCK:-/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/sprint2/heavy.lock}
HISTORY=$SCRATCH/GATE-HISTORY.md
MATRIX=$SCRATCH/NATIVE-MATRIX.md
CI_STATUS=${CI_STATUS:-$SCRATCH/CI-STATUS.md}
CI_PREVIOUS=${CI_PREVIOUS:-$SCRATCH/ci-previous.tsv}
COMPILER=$repo/rey-compiler/main.rey
NATIVE_ROOT=$repo/rey-compiler/tests/native-codegen
FIXTURE_ROOT=$repo/tests/conformance

jobs=${GATE_JOBS:-2}
case "$jobs" in
    1|2|3) ;;
    *) jobs=2 ;;
esac
case_timeout=${GATE_CASE_TIMEOUT:-90}
case "$case_timeout" in
    ''|*[!0-9]*|0) echo 'GATE_CASE_TIMEOUT must be a positive integer' >&2; exit 2 ;;
esac
quick_budget=${GATE_QUICK_BUDGET_SECONDS:-300}
case "$quick_budget" in
    ''|*[!0-9]*|0) echo 'GATE_QUICK_BUDGET_SECONDS must be a positive integer' >&2; exit 2 ;;
esac

if [ -x /usr/bin/clang ]; then
    PATH="/usr/bin:$PATH"
    export PATH
    CC=/usr/bin/clang
    export CC
fi

mkdir -p "$SCRATCH" || exit 2

run_bounded() {
    bound_seconds=$1
    bound_marker=$2
    shift 2
    rm -f "$bound_marker"
    perl - "$bound_seconds" "$bound_marker" "$@" <<'PERL'
use POSIX qw(setpgid WNOHANG);
use Time::HiRes qw(time sleep);
my ($timeout, $marker, @command) = @ARGV;
my $pid = fork();
defined $pid or die "fork failed: $!\n";
if ($pid == 0) {
    setpgid(0, 0) == 0 or exit 125;
    exec @command or die "exec failed: $!\n";
}
my $deadline = time() + $timeout;
while (1) {
    my $waited = waitpid($pid, WNOHANG);
    if ($waited == $pid) {
        my $status = $?;
        exit(($status & 127) ? 128 + ($status & 127) : ($status >> 8));
    }
    if (time() >= $deadline) {
        kill 'TERM', -$pid;
        my $grace = time() + 1;
        while (time() < $grace) {
            last if waitpid($pid, WNOHANG) == $pid;
            sleep(0.05);
        }
        kill 'KILL', -$pid;
        waitpid($pid, 0);
        open my $timeout_file, '>', $marker;
        close $timeout_file if $timeout_file;
        exit 124;
    }
    sleep(0.05);
}
PERL
}

sanitize_text() {
    printf '%s' "$1" | tr '\n\t' '  ' | cut -c 1-240
}

# Group identical root causes. Drop work-dir paths, node ids, and quoted names
# before the 240-character cut, or "checked lowering failed: <long path>: ..."
# becomes a different bucket for every case.
normalize_cause() {
    printf '%s' "$1" | perl -ne '
        chomp;
        s/.*\berror:\s*//;
        s/^(?:[A-Za-z]+Error|Error):\s*//;
        s/checked lowering failed:\s*//g;
        s{/[^\s:]+\.rey}{}g;
        s{\b[\w./-]+\.rey:\d+:\d+:}{}g;
        s/\bnode \d+://g;
        s/\b[A-Za-z_][A-Za-z0-9_]*\d+:\d+:\d+:[A-Za-z0-9_]*:\d+//g;
        s/\b[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)+//g;
        s/\x27[^\x27]*\x27//g;
        s/"[^"]*"//g;
        s/:\s*[A-Za-z_][A-Za-z0-9_]*\s*$//;
        s/\bfor\s*:\s*/for /;
        s/\s+/ /g;
        s/^[: ]+//;
        s/[: ]+$//;
        print;
    '
}

first_error_message() {
    error_file=$1
    error_fallback=${2:-}
    [ -f "$error_file" ] || error_file=$error_fallback
    [ -n "$error_file" ] && [ -f "$error_file" ] || return 0
    error_line=$(grep -m 1 -F 'error:' "$error_file" 2>/dev/null || true)
    if [ -z "$error_line" ] && [ -n "$error_fallback" ] && [ -f "$error_fallback" ]; then
        error_line=$(grep -m 1 -F 'error:' "$error_fallback" 2>/dev/null || true)
    fi
    if [ -z "$error_line" ]; then
        error_line=$(grep -m 1 -v '^$' "$error_file" 2>/dev/null || true)
    fi
    if [ -n "$error_line" ]; then
        case "$error_line" in
            *error:*) printf '%s' "${error_line#*error: }" ;;
            *) printf '%s' "$error_line" ;;
        esac
    fi
}

source_fingerprint() {
    manifest=$(mktemp "$SCRATCH/manifest.XXXXXX") || return 1
    : > "$manifest"
    find "$repo/rey-compiler" -name '*.rey' -type f | LC_ALL=C sort | while IFS= read -r source_file; do
        /usr/bin/shasum -a 256 "$source_file"
    done >> "$manifest"
    find "$repo/rey-compiler/runtime" \( -name '*.c' -o -name '*.h' \) -type f | LC_ALL=C sort | while IFS= read -r source_file; do
        /usr/bin/shasum -a 256 "$source_file"
    done >> "$manifest"
    /usr/bin/shasum -a 256 "$manifest" | awk '{print $1}'
    rm -f "$manifest"
}

oracle_hash() {
    if [ -f "$ORACLE" ]; then
        /usr/bin/shasum -a 256 "$ORACLE" | awk '{print $1}'
    else
        printf 'missing'
    fi
}

append_history() {
    history_mode=$1
    history_overall=$2
    history_summary=$3
    history_extra=$4
    fragment=$(mktemp "$SCRATCH/history.XXXXXX") || return 0
    {
        printf '## %s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$history_mode"
        printf -- '- oracle: `%s`\n' "$(oracle_hash)"
        printf -- '- sources: `%s`\n' "$(source_fingerprint)"
        printf -- '- overall: %s\n' "$history_overall"
        printf -- '- %s\n' "$history_summary"
        if [ -n "$history_extra" ]; then
            printf '%s\n' "$history_extra"
        fi
        printf '\n'
    } > "$fragment"
    /usr/bin/lockf -k "$SCRATCH/history.lock" /bin/sh -c 'cat "$1" >> "$2"' _ "$fragment" "$HISTORY"
    rm -f "$fragment"
}

write_row() {
    row_file=$1
    row_id=$2
    row_status=$3
    row_bucket=$4
    row_detail=$5
    printf '%s\t%s\t%s\t%s\n' "$row_id" "$row_status" "$(sanitize_text "$row_bucket")" "$(sanitize_text "$row_detail")" > "$row_file"
}

internal_worker() {
    job_file=$1
    # Generated assignment file. Paths are absolute and space-free.
    # shellcheck disable=SC1090
    . "$job_file"
    result_status=FAIL
    result_bucket=worker
    result_detail=no-result
    binary=$job_work/program.bin
    stdout_file=$job_work/stdout
    stderr_file=$job_work/stderr
    marker=$job_work/timed-out
    deadline=$((SECONDS + job_timeout))
    remaining_seconds() {
        left=$((deadline - SECONDS))
        if [ "$left" -lt 1 ]; then
            left=1
        fi
        printf '%s' "$left"
    }
    timed_out() {
        [ -f "$marker" ]
    }
    run_compiler() {
        compiler_mode=$1
        shift
        rm -f "$stdout_file" "$stderr_file" "$marker"
        (
            cd "$job_work" || exit 125
            run_bounded "$(remaining_seconds)" "$marker" "$ORACLE" run "$COMPILER" -- "$compiler_mode" "$@"
        ) > "$stdout_file" 2> "$stderr_file"
        compiler_status=$?
    }
    message_of() {
        found=$(first_error_message "$stderr_file" "$stdout_file")
        if [ -n "$found" ]; then
            normalize_cause "$found"
        fi
    }
    if [ "$job_mode" = "build-fail" ]; then
        run_compiler build "$job_entry" -o "$binary" --runtime-dir "$REY_RUNTIME_DIR"
        if timed_out; then
            result_bucket=timeout
            result_detail="build exceeded ${job_timeout}s"
        elif [ "$compiler_status" -eq 1 ]; then
            result_status=PASS
            result_bucket=-
            result_detail=build-exit-1
        elif [ "$compiler_status" -gt 127 ]; then
            result_bucket=crash
            result_detail="build signal status $compiler_status"
        else
            diag=$(message_of)
            result_bucket=${diag:-build-exit-$compiler_status}
            result_detail="expected build exit 1, got $compiler_status"
        fi
        write_row "$job_result" "$job_id" "$result_status" "$result_bucket" "$result_detail"
        exit 0
    fi
    if [ "$job_mode" = "check-fail" ]; then
        run_compiler check "$job_entry"
        marker_text=$(cat "$job_marker_file")
        combined=$job_work/combined
        if [ -z "$marker_text" ]; then
            write_row "$job_result" "$job_id" FAIL 'missing fixture' 'expect_check_fail is empty'
            exit 0
        fi
        cat "$stdout_file" "$stderr_file" > "$combined"
        if timed_out; then
            result_bucket=timeout
            result_detail="check exceeded ${job_timeout}s"
        elif [ "$compiler_status" -gt 127 ]; then
            result_bucket=crash
            result_detail="check signal status $compiler_status"
        elif [ "$compiler_status" -eq 0 ]; then
            result_bucket=negative-accepted
            result_detail='check exited 0'
        elif [ "$compiler_status" -ne 1 ]; then
            result_bucket=negative-exit
            result_detail="check exit $compiler_status"
        elif ! grep -F -q -e "$marker_text" "$combined"; then
            diag=$(message_of)
            result_bucket=${diag:-marker-missing}
            result_detail="diagnostics missed marker"
        else
            result_status=PASS
            result_bucket=-
            result_detail=check-exit-1
        fi
        write_row "$job_result" "$job_id" "$result_status" "$result_bucket" "$result_detail"
        exit 0
    fi
    run_compiler check "$job_entry"
    if timed_out; then
        result_bucket=timeout
        result_detail="check exceeded ${job_timeout}s"
        write_row "$job_result" "$job_id" "$result_status" "$result_bucket" "$result_detail"
        exit 0
    fi
    if [ "$compiler_status" -gt 127 ]; then
        result_bucket=crash
        result_detail="check signal status $compiler_status"
        write_row "$job_result" "$job_id" "$result_status" "$result_bucket" "$result_detail"
        exit 0
    fi
    if [ "$compiler_status" -ne 0 ]; then
        diag=$(message_of)
        result_bucket=${diag:-check-exit-$compiler_status}
        result_detail="check exit $compiler_status"
        write_row "$job_result" "$job_id" "$result_status" "$result_bucket" "$result_detail"
        exit 0
    fi
    if [ -s "$stdout_file" ] || [ -s "$stderr_file" ]; then
        diag=$(message_of)
        result_bucket=${diag:-check-not-silent}
        result_detail='successful check must be silent'
        write_row "$job_result" "$job_id" "$result_status" "$result_bucket" "$result_detail"
        exit 0
    fi
    run_compiler build "$job_entry" -o "$binary" --runtime-dir "$REY_RUNTIME_DIR"
    if timed_out; then
        result_bucket=timeout
        result_detail="build exceeded ${job_timeout}s"
        write_row "$job_result" "$job_id" "$result_status" "$result_bucket" "$result_detail"
        exit 0
    fi
    if [ "$compiler_status" -ne 0 ] || [ ! -f "$binary" ]; then
        diag=$(message_of)
        result_bucket=${diag:-build-exit-$compiler_status}
        result_detail="build exit $compiler_status"
        write_row "$job_result" "$job_id" "$result_status" "$result_bucket" "$result_detail"
        exit 0
    fi
    if [ ! -x "$binary" ]; then
        chmod +x "$binary" 2>/dev/null || true
    fi
    rm -f "$stdout_file" "$stderr_file" "$marker"
    (
        cd "$job_work" || exit 125
        run_bounded "$(remaining_seconds)" "$marker" "$binary"
    ) > "$stdout_file" 2> "$stderr_file"
    run_status=$?
    if timed_out; then
        result_bucket=timeout
        result_detail="program exceeded ${job_timeout}s"
    elif [ "$run_status" -gt 127 ]; then
        result_bucket=crash
        result_detail="program signal status $run_status"
    elif [ "$run_status" -ne "$job_exit" ] && ! cmp -s "$job_stdout" "$stdout_file"; then
        result_bucket=wrong-output
        result_detail="exit $run_status expected $job_exit and stdout differs"
    elif ! cmp -s "$job_stdout" "$stdout_file"; then
        result_bucket=wrong-output
        result_detail='stdout differs'
    elif [ "$run_status" -ne "$job_exit" ]; then
        result_bucket=wrong-exit
        result_detail="exit $run_status expected $job_exit"
    else
        result_status=PASS
        result_bucket=-
        result_detail="exit $run_status"
    fi
    write_row "$job_result" "$job_id" "$result_status" "$result_bucket" "$result_detail"
    exit 0
}

if [ "${1:-}" = "--internal-worker" ]; then
    if [ "$#" -ne 2 ] || [ ! -f "$2" ]; then
        echo 'internal worker requires a job file' >&2
        exit 2
    fi
    internal_worker "$2"
fi

mode=${1:-}
watch_interval=300
case "$mode" in
    quick|native-conformance|selfcheck|full|bootstrap)
        if [ "$#" -gt 1 ]; then
            usage >&2
            exit 2
        fi
        ;;
    watch)
        shift
        while [ "$#" -gt 0 ]; do
            case "$1" in
                --interval)
                    if [ "$#" -lt 2 ]; then
                        echo 'watch --interval requires a number of seconds' >&2
                        exit 2
                    fi
                    shift
                    watch_interval=$1
                    ;;
                *)
                    usage >&2
                    exit 2
                    ;;
            esac
            shift
        done
        case "$watch_interval" in
            ''|*[!0-9]*|0)
                echo 'watch --interval must be a positive integer' >&2
                exit 2
                ;;
        esac
        ;;
    -h|--help|help|'') usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
esac

print_table() {
    table_file=$1
    printf '%-12s %-40s %s\n' STATUS CASE DETAIL
    sort -t "$(printf '\t')" -k 1,1 "$table_file" | while IFS="$(printf '\t')" read -r row_id row_status row_bucket row_detail; do
        [ -n "$row_id" ] || continue
        printf '%-12s %-40s %s\n' "$row_status" "$row_id" "$row_detail"
    done
}

summarize_file() {
    summary_file=$1
    passed=$(awk -F '\t' '$2=="PASS"{n++} END{print n+0}' "$summary_file")
    failed=$(awk -F '\t' '$2=="FAIL"{n++} END{print n+0}' "$summary_file")
    incomplete=$(awk -F '\t' '$2=="INCOMPLETE"{n++} END{print n+0}' "$summary_file")
    total=$(awk -F '\t' 'NF>=2{n++} END{print n+0}' "$summary_file")
    summary_text=$(printf 'summary: %s passed, %s failed, %s incomplete, %s total' "$passed" "$failed" "$incomplete" "$total")
    printf '%s\n' "$summary_text"
    if [ "$failed" -gt 0 ]; then
        echo 'failure causes:'
        awk -F '\t' '$2=="FAIL"{print $3}' "$summary_file" | sed '/^$/d;/^-$/d' | sort | uniq -c | sort -nr
    fi
    if [ "$failed" -gt 0 ]; then
        finish_overall=FAIL
        finish_code=1
    elif [ "$incomplete" -gt 0 ]; then
        finish_overall=INCOMPLETE
        finish_code=2
    else
        finish_overall=PASS
        finish_code=0
    fi
    printf 'overall: %s\n' "$finish_overall"
}

finish_run() {
    finish_mode=$1
    finish_table=$2
    finish_extra=$3
    print_table "$finish_table"
    summarize_file "$finish_table"
    groups=
    if [ "$failed" -gt 0 ]; then
        groups=$(awk -F '\t' '$2=="FAIL"{print $3}' "$finish_table" | sed '/^$/d;/^-$/d' | sort | uniq -c | sort -nr | head -n 20)
    fi
    append_history "$finish_mode" "$finish_overall" "$summary_text" "$groups"
    if [ -n "$finish_extra" ]; then
        printf '%s\n' "$finish_extra"
    fi
    exit "$finish_code"
}

write_matrix() {
    matrix_tsv=$1
    matrix_note=$2
    python3 - "$matrix_tsv" "$MATRIX" "$matrix_note" "$(oracle_hash)" "$(source_fingerprint)" <<'PY'
import collections, pathlib, re, sys
tsv, dest, note, oracle_hash, sources = sys.argv[1:]
rows = []
for line in pathlib.Path(tsv).read_text(errors="replace").splitlines():
    parts = line.split("\t")
    if len(parts) < 4:
        continue
    rows.append(parts[:4])

def normalize_cause(text):
    value = text or ""
    value = re.sub(r".*\berror:\s*", "", value)
    value = re.sub(r"^(?:[A-Za-z]+Error|Error):\s*", "", value)
    value = re.sub(r"checked lowering failed:\s*", "", value)
    value = re.sub(r"/[^\s:]+\.rey", "", value)
    value = re.sub(r"\b[\w./-]+\.rey:\d+:\d+:", "", value)
    value = re.sub(r"\bnode \d+:", "", value)
    value = re.sub(r"\b[A-Za-z_][A-Za-z0-9_]*\d+:\d+:\d+:[A-Za-z0-9_]*:\d+", "", value)
    value = re.sub(r"\b[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)+", "", value)
    value = re.sub(r"'[^']*'", "", value)
    value = re.sub(r'"[^"]*"', "", value)
    value = re.sub(r":\s*[A-Za-z_][A-Za-z0-9_]*\s*$", "", value)
    value = re.sub(r"\bfor\s*:\s*", "for ", value)
    value = re.sub(r"\s+", " ", value).strip(" :")
    return value or "empty-diagnostic"

def feature_of(bucket):
    text = " " + bucket.lower() + " "
    rules = [
        ("top-level entry", ("requires a main function", "top-level initialization")),
        ("float collections", ("float vec", "float hashmap", "float collection")),
        ("vec iteration", ("vec iteration", "for iterable", "unsupported for")),
        ("indexed writes", ("assignment target", "indexed update", "indexed write")),
        ("increments", ("update target is not a local int",)),
        ("struct patterns", ("struct match", "struct pattern", "match scrutinee")),
        ("defaults and variadics", ("default or variadic", "variadic")),
        ("defaults", ("default",)),
        ("qualified calls", ("unresolved qualified call", "typed call symbol", "unresolved call target", "unsupported checked direct call")),
        ("tuples", ("tuple",)),
        ("dicts", ("dictionary", " dict", "hashmap")),
        ("lambdas", ("lambda", "closure")),
        ("instanceof", ("instanceof",)),
        ("intrinsics", ("statement intrinsic", "intrinsic is unresolved")),
        ("codegen representation", ("no codegen representation",)),
        ("nullable and unions", ("nullable", " union", "null", " | ")),
        ("interpolation", ("interpolat",)),
        ("conversions", ("conversion", "toint", "tofloat", "unknown method on float", "ascii decimal")),
        ("methods", ("unknown method", "receiver abi", " method")),
        ("builtins", ("unknown function",)),
        ("unsupported expression", ("unsupported expression", "unsupported expr")),
        ("exhaustiveness", ("non-exhaustive", "exhaustive")),
        ("visibility", ("private field", "private method", "is private")),
        ("modules", ("file not found", "circular import", "duplicate declaration", "imported name", "module has no", "module namespace")),
        ("byte strings", (" nul", "utf-8", "byte")),
        ("same-named types", ("nominal", "owner-matched", "same-named")),
        ("operators", ("invalid binary", "operator requires", "binary operation")),
        ("type mismatches", ("type mismatch", "initializer type")),
        ("arity", ("arity", "function expects")),
        ("returns", ("must return", "return type")),
        ("parse errors", ("type expected", ") expected", "expected pattern")),
        ("any", ("type any", "escape hatch")),
        ("primitive spelling", ("capitalized primitive",)),
        ("fixture layout", ("missing fixture", "missing main", "missing expected")),
        ("negative accepted", ("negative-accepted",)),
        ("wrong output", ("wrong-output", "wrong-exit")),
        ("crash", ("timeout", "crash", "signal")),
    ]
    for name, needles in rules:
        if any(needle in text for needle in needles):
            return name
    return "other"

fails = [row for row in rows if row[1] == "FAIL"]
passes = [row for row in rows if row[1] == "PASS"]
incomplete = [row for row in rows if row[1] == "INCOMPLETE"]
normalized_fails = [(row[0], row[1], normalize_cause(row[2]), row[3]) for row in fails]
by_feature = collections.Counter(feature_of(row[2]) for row in normalized_fails)
by_bucket = collections.Counter(row[2] for row in normalized_fails)
lines = []
lines.append("# Native feature-gap matrix")
lines.append("")
lines.append(f"Generated: {__import__('datetime').datetime.utcnow().strftime('%Y-%m-%dT%H:%M:%SZ')}")
lines.append(f"Oracle: `{oracle_hash}`")
lines.append(f"Sources: `{sources}`")
lines.append("")
lines.append("Pipeline: `$ORACLE run rey-compiler/main.rey -- check|build`, then the linked binary.")
lines.append("Re-run when `rey-compiler/**/*.rey` or the runtime changes:")
lines.append("")
lines.append("```sh")
lines.append("scripts/gate.sh native-conformance")
lines.append("```")
lines.append("")
lines.append(f"Cases: {len(passes)} passed, {len(fails)} failed, {len(incomplete)} incomplete, {len(rows)} total.")
if note:
    lines.append("")
    lines.append(note)
lines.append("")
lines.append("## Ranked by cases unblocked")
lines.append("")
lines.append("| rank | cases | feature | diagnostic |")
lines.append("|---:|---:|---|---|")
rank = 1
for bucket, count in by_bucket.most_common():
    lines.append(f"| {rank} | {count} | {feature_of(bucket)} | `{bucket}` |")
    rank += 1
if not by_bucket:
    lines.append("| 1 | 0 | none | no failing cases |")
lines.append("")
lines.append("## Feature rollup")
lines.append("")
lines.append("| cases | feature |")
lines.append("|---:|---|")
for name, count in by_feature.most_common():
    lines.append(f"| {count} | {name} |")
lines.append("")
lines.append("## Failures")
lines.append("")
lines.append("| case | feature | cause | detail |")
lines.append("|---|---|---|---|")
for case_id, status, bucket, detail in sorted(normalized_fails):
    lines.append(f"| `{case_id}` | {feature_of(bucket)} | `{bucket}` | {detail} |")
lines.append("")
lines.append("## Passes")
lines.append("")
for case_id, status, bucket, detail in sorted(passes):
    lines.append(f"- `{case_id}`")
if incomplete:
    lines.append("")
    lines.append("## Incomplete")
    lines.append("")
    for case_id, status, bucket, detail in sorted(incomplete):
        lines.append(f"- `{case_id}`: {detail}")
lines.append("")
lines.append("`tests/conformance/rey-lang/tests` stays counted. It is a malformed discovery case; do not delete or hide it.")
lines.append("")
pathlib.Path(dest).write_text("\n".join(lines))
print(dest)
PY
}

prepare_run_dir() {
    run_stamp=$(date -u +%Y%m%dT%H%M%SZ)
    run_dir=$SCRATCH/$1-$run_stamp
    mkdir -p "$run_dir/rows" "$run_dir/jobs" "$run_dir/work"
    table_file=$run_dir/results.tsv
    : > "$table_file"
    if [ -n "${GATE_RUN_POINTER:-}" ]; then
        printf '%s\n' "$run_dir" > "$GATE_RUN_POINTER"
    fi
}

collect_rows() {
    : > "$table_file"
    find "$run_dir/rows" -name '*.tsv' | LC_ALL=C sort | while IFS= read -r row_path; do
        cat "$row_path"
    done > "$table_file"
}

launch_jobs() {
    launch_bound=${1:-0}
    job_list=$run_dir/job-list
    launch_marker=$run_dir/launch.timeout
    : > "$job_list"
    find "$run_dir/jobs" -name '*.job' | LC_ALL=C sort > "$job_list"
    if [ -s "$job_list" ]; then
        if [ "$launch_bound" -gt 0 ]; then
            run_bounded "$launch_bound" "$launch_marker" /bin/bash -c \
                'xargs -n 1 -P "$1" /bin/bash "$2" --internal-worker < "$3"' \
                _ "$jobs" "$script_path" "$job_list"
        else
            xargs -n 1 -P "$jobs" /bin/bash "$script_path" --internal-worker < "$job_list"
        fi
    fi
    find "$run_dir/jobs" -name '*.job' | LC_ALL=C sort | while IFS= read -r job_path; do
        # shellcheck disable=SC1090
        . "$job_path"
        if [ ! -f "$job_result" ]; then
            if [ -f "$launch_marker" ]; then
                write_row "$job_result" "$job_id" INCOMPLETE budget "not finished within ${launch_bound}s"
            else
                write_row "$job_result" "$job_id" FAIL crash 'worker produced no result'
            fi
        fi
    done
}

require_runtime() {
    runtime_dir=$run_dir/runtime
    runtime_log=$run_dir/runtime-build.log
    runtime_marker=$run_dir/runtime-build.timeout
    if run_bounded 90 "$runtime_marker" /bin/sh "$repo/scripts/build-runtime.sh" --output-dir "$runtime_dir" > "$runtime_log" 2>&1 \
        && [ -f "$runtime_dir/librey_rt.a" ]; then
        write_row "$run_dir/rows/02-runtime-build.tsv" 'runtime-build' PASS - "$runtime_dir/librey_rt.a"
        REY_RUNTIME_DIR=$runtime_dir
        export REY_RUNTIME_DIR
        return 0
    fi
    if [ -f "$runtime_marker" ]; then
        write_row "$run_dir/rows/02-runtime-build.tsv" 'runtime-build' FAIL timeout 'runtime build exceeded 90s'
    else
        write_row "$run_dir/rows/02-runtime-build.tsv" 'runtime-build' FAIL 'runtime-build' 'librey_rt.a was not produced'
    fi
    return 1
}

host_check_row() {
    host_stdout=$run_dir/host.stdout
    host_stderr=$run_dir/host.stderr
    host_marker=$run_dir/host.timeout
    if [ ! -x "$ORACLE" ]; then
        write_row "$run_dir/rows/01-host-check.tsv" 'host-check' INCOMPLETE 'oracle-missing' "$ORACLE"
        return 1
    fi
    run_bounded 120 "$host_marker" "$ORACLE" check "$COMPILER" > "$host_stdout" 2> "$host_stderr"
    host_status=$?
    if [ -f "$host_marker" ]; then
        write_row "$run_dir/rows/01-host-check.tsv" 'host-check' FAIL timeout 'host check exceeded 120s'
        return 1
    fi
    if [ "$host_status" -ne 0 ] || [ -s "$host_stdout" ] || [ -s "$host_stderr" ]; then
        host_detail=$(first_error_message "$host_stderr" "$host_stdout")
        write_row "$run_dir/rows/01-host-check.tsv" 'host-check' FAIL "${host_detail:-host-check-exit-$host_status}" "exit $host_status"
        return 1
    fi
    write_row "$run_dir/rows/01-host-check.tsv" 'host-check' PASS - 'silent exit 0'
    return 0
}

oracle_preflight_row() {
    if [ ! -x "$ORACLE" ]; then
        write_row "$run_dir/rows/00-oracle-preflight.tsv" 'oracle-preflight' INCOMPLETE 'oracle-missing' "$ORACLE"
        return 1
    fi
    preflight_dir=$run_dir/preflight
    mkdir -p "$preflight_dir"
    printf 'var preflightValue: int = 1;\n' > "$preflight_dir/check.rey"
    printf 'func main(): void {\n    println(1);\n}\n' > "$preflight_dir/run.rey"
    check_marker=$preflight_dir/check.timeout
    run_bounded 30 "$check_marker" "$ORACLE" check "$preflight_dir/check.rey" > "$preflight_dir/check.stdout" 2> "$preflight_dir/check.stderr"
    check_status=$?
    run_marker=$preflight_dir/run.timeout
    run_bounded 30 "$run_marker" "$ORACLE" run "$preflight_dir/run.rey" > "$preflight_dir/run.stdout" 2> "$preflight_dir/run.stderr"
    run_status=$?
    printf '1\n' > "$preflight_dir/expected.stdout"
    if [ -f "$check_marker" ] || [ -f "$run_marker" ]; then
        write_row "$run_dir/rows/00-oracle-preflight.tsv" 'oracle-preflight' FAIL timeout 'preflight exceeded 30s'
        return 1
    fi
    if [ "$check_status" -ne 0 ] || [ -s "$preflight_dir/check.stdout" ] || [ -s "$preflight_dir/check.stderr" ]; then
        write_row "$run_dir/rows/00-oracle-preflight.tsv" 'oracle-preflight' FAIL 'preflight-check' "check exit $check_status"
        return 1
    fi
    if [ "$run_status" -ne 0 ] || ! cmp -s "$preflight_dir/expected.stdout" "$preflight_dir/run.stdout"; then
        write_row "$run_dir/rows/00-oracle-preflight.tsv" 'oracle-preflight' FAIL 'preflight-run' "run exit $run_status"
        return 1
    fi
    write_row "$run_dir/rows/00-oracle-preflight.tsv" 'oracle-preflight' PASS - 'check silent, run prints 1'
    return 0
}

queue_codegen_jobs() {
    codegen_started=$SECONDS
    find "$NATIVE_ROOT" -mindepth 1 -maxdepth 1 \( -name '*.rey' -o -type d \) | LC_ALL=C sort | while IFS= read -r entry_path; do
        if [ -d "$entry_path" ]; then
            [ -f "$entry_path/main.rey" ] || continue
            job_id=$(basename "$entry_path")
            entry=$entry_path/main.rey
            stem=$entry_path/main
        else
            case "$entry_path" in
                *.rey) ;;
                *) continue ;;
            esac
            job_id=$(basename "$entry_path" .rey)
            entry=$entry_path
            stem=$NATIVE_ROOT/$job_id
        fi
        row_path=$run_dir/rows/10-$job_id.tsv
        if [ -f "$stem.expected.build-fail" ]; then
            job_mode=build-fail
        elif [ -f "$stem.expected.stdout" ] && [ -f "$stem.expected.exit" ]; then
            job_mode=execute
        else
            write_row "$row_path" "native-codegen/$job_id" INCOMPLETE 'missing-sidecar' 'no spec-backed sidecar'
            continue
        fi
        elapsed=$((SECONDS - codegen_started))
        if [ "$elapsed" -ge "$quick_budget" ]; then
            write_row "$row_path" "native-codegen/$job_id" INCOMPLETE 'budget' "quick budget ${quick_budget}s exhausted"
            continue
        fi
        job_work=$run_dir/work/$job_id
        mkdir -p "$job_work"
        job_path=$run_dir/jobs/$job_id.job
        job_exit_value=0
        if [ "$job_mode" = "execute" ]; then
            job_exit_value=$(tr -d '[:space:]' < "$stem.expected.exit")
        fi
        cat > "$job_path" <<EOF
job_kind=codegen
job_id=native-codegen/$job_id
job_entry=$entry
job_work=$job_work
job_mode=$job_mode
job_stdout=$stem.expected.stdout
job_exit=$job_exit_value
job_marker_file=/dev/null
job_result=$row_path
job_timeout=$case_timeout
EOF
    done
}

queue_conformance_jobs() {
    find "$FIXTURE_ROOT" -mindepth 2 -maxdepth 2 -type d | LC_ALL=C sort | while IFS= read -r case_dir; do
        case_id=${case_dir#"$FIXTURE_ROOT"/}
        area=${case_id%%/*}
        case_name=${case_id#*/}
        if [ "$area" != "_smoke" ] && [ "${area#_}" != "$area" ]; then
            continue
        fi
        if [ "${case_name#_}" != "$case_name" ]; then
            continue
        fi
        safe_id=$(printf '%s' "$case_id" | tr '/' '_')
        row_path=$run_dir/rows/20-$safe_id.tsv
        if [ ! -f "$case_dir/main.rey" ] || [ ! -f "$case_dir/expected.stdout" ]; then
            write_row "$row_path" "$case_id" FAIL 'missing fixture' 'main.rey or expected.stdout absent'
            continue
        fi
        job_work=$run_dir/work/$safe_id
        mkdir -p "$job_work"
        if ! cp -R "$case_dir/." "$job_work/"; then
            write_row "$row_path" "$case_id" FAIL 'missing fixture' 'could not copy fixture'
            continue
        fi
        if [ -f "$case_dir/expect_check_fail" ]; then
            job_mode=check-fail
            job_exit_value=1
        else
            job_mode=execute
            job_exit_value=0
            if [ -f "$case_dir/expected.exit" ]; then
                job_exit_value=$(tr -d '[:space:]' < "$case_dir/expected.exit")
            fi
        fi
        job_path=$run_dir/jobs/$safe_id.job
        cat > "$job_path" <<EOF
job_kind=conformance
job_id=$case_id
job_entry=$job_work/main.rey
job_work=$job_work
job_mode=$job_mode
job_stdout=$case_dir/expected.stdout
job_exit=$job_exit_value
job_marker_file=$case_dir/expect_check_fail
job_result=$row_path
job_timeout=$case_timeout
EOF
    done
}

run_quick() {
    prepare_run_dir quick
    host_ok=0
    runtime_ok=0
    oracle_preflight_row || true
    if host_check_row; then
        host_ok=1
    fi
    if require_runtime; then
        runtime_ok=1
    fi
    runtime_log=$run_dir/runtime-quick.log
    runtime_marker=$run_dir/runtime-quick.timeout
    if [ -f "$repo/scripts/test-runtime.sh" ]; then
        run_bounded 120 "$runtime_marker" /bin/sh "$repo/scripts/test-runtime.sh" --normal > "$runtime_log" 2>&1
        runtime_status=$?
        if [ -f "$runtime_marker" ]; then
            write_row "$run_dir/rows/03-runtime-quick.tsv" 'runtime-quick' FAIL timeout 'runtime quick exceeded 120s'
        elif [ "$runtime_status" -eq 0 ]; then
            write_row "$run_dir/rows/03-runtime-quick.tsv" 'runtime-quick' PASS - 'test-runtime.sh --normal'
        elif grep -F -q 'overall: INCOMPLETE' "$runtime_log"; then
            write_row "$run_dir/rows/03-runtime-quick.tsv" 'runtime-quick' INCOMPLETE 'runtime-incomplete' 'test-runtime reported INCOMPLETE'
        else
            write_row "$run_dir/rows/03-runtime-quick.tsv" 'runtime-quick' FAIL 'runtime-quick' "exit $runtime_status"
        fi
    else
        write_row "$run_dir/rows/03-runtime-quick.tsv" 'runtime-quick' INCOMPLETE 'runtime-missing' 'scripts/test-runtime.sh missing'
    fi
    if [ "$host_ok" -eq 1 ] && [ "$runtime_ok" -eq 1 ]; then
        queue_codegen_jobs
        launch_jobs "$quick_budget"
    else
        write_row "$run_dir/rows/10-native-codegen.tsv" 'native-codegen' INCOMPLETE 'blocked' 'host check or runtime archive is not ready'
    fi
    collect_rows
    finish_run quick "$table_file" "results: $table_file"
}

run_native_conformance() {
    prepare_run_dir native
    if ! host_check_row; then
        collect_rows
        write_matrix "$table_file" 'Native conformance did not run: the Rust-host check of rey-compiler/main.rey is not silent exit 0.'
        finish_run native-conformance "$table_file" "matrix: $MATRIX"
    fi
    if ! require_runtime; then
        collect_rows
        write_matrix "$table_file" 'Native conformance did not run: the runtime archive did not build.'
        finish_run native-conformance "$table_file" "matrix: $MATRIX"
    fi
    queue_conformance_jobs
    if ! find "$run_dir/jobs" -name '*.job' | grep -q . && ! find "$run_dir/rows" -name '20-*.tsv' | grep -q .; then
        write_row "$run_dir/rows/20-empty.tsv" 'conformance' FAIL 'missing fixture' 'no conformance cases discovered'
    fi
    native_budget=${GATE_NATIVE_BUDGET_SECONDS:-1500}
    launch_jobs "$native_budget"
    collect_rows
    cp "$table_file" "$SCRATCH/native-conformance-latest.tsv"
    write_matrix "$table_file" ''
    finish_run native-conformance "$table_file" "matrix: $MATRIX"
}

run_grouped_diagnostics() {
    diag_file=$1
    if [ ! -s "$diag_file" ]; then
        echo 'diagnostics: none'
        return 0
    fi
    echo 'diagnostics grouped by message:'
    grep -F 'error:' "$diag_file" | sed 's/.*error: //' | sort | uniq -c | sort -nr | head -n 40
}

run_selfcheck() {
    prepare_run_dir selfcheck
    if ! host_check_row; then
        collect_rows
        finish_run selfcheck "$table_file" ''
    fi
    self_stdout=$run_dir/self.stdout
    self_stderr=$run_dir/self.stderr
    self_time=$run_dir/self.time
    self_timeout=$run_dir/self.timeout
    : > "$self_timeout"
    if command -v timeout >/dev/null 2>&1; then
        /usr/bin/lockf -k "$HEAVY_LOCK" timeout 1200 /usr/bin/time -l -o "$self_time" \
            "$ORACLE" run "$COMPILER" -- check "$COMPILER" > "$self_stdout" 2> "$self_stderr"
        self_status=$?
    else
        /usr/bin/lockf -k "$HEAVY_LOCK" perl - "$self_time" "$ORACLE" "$COMPILER" <<'PERL' > "$self_stdout" 2> "$self_stderr"
use POSIX qw(setpgid WNOHANG);
use Time::HiRes qw(time sleep);
my ($time_file, $oracle, $compiler) = @ARGV;
my $pid = fork();
defined $pid or die "fork failed\n";
if ($pid == 0) {
    setpgid(0, 0) == 0 or exit 125;
    exec '/usr/bin/time', '-l', '-o', $time_file, $oracle, 'run', $compiler, '--', 'check', $compiler;
    exit 127;
}
my $deadline = time() + 1200;
while (1) {
    my $waited = waitpid($pid, WNOHANG);
    if ($waited == $pid) {
        my $status = $?;
        exit(($status & 127) ? 128 + ($status & 127) : ($status >> 8));
    }
    if (time() >= $deadline) {
        kill 'TERM', -$pid;
        sleep 1;
        kill 'KILL', -$pid;
        waitpid($pid, 0);
        exit 124;
    }
    sleep(0.05);
}
PERL
        self_status=$?
    fi
    if [ "$self_status" -eq 124 ]; then
        printf 'timeout\n' > "$self_timeout"
    fi
    rss=$(awk '/maximum resident set size/ {print $1; exit}' "$self_time" 2>/dev/null || true)
    real_time=$(awk '/ real / {print $1; exit}' "$self_time" 2>/dev/null || true)
    diag_text=$(run_grouped_diagnostics "$self_stderr")
    printf '%s\n' "$diag_text"
    if [ -f "$self_timeout" ] && [ -s "$self_timeout" ]; then
        write_row "$run_dir/rows/04-selfcheck.tsv" 'selfcheck' FAIL timeout 'exceeded 1200s'
    elif [ "$self_status" -gt 127 ]; then
        write_row "$run_dir/rows/04-selfcheck.tsv" 'selfcheck' FAIL crash "status $self_status rss=${rss:-unknown}"
    elif [ "$self_status" -ne 0 ]; then
        first_diag=$(first_error_message "$self_stderr" "$self_stdout")
        write_row "$run_dir/rows/04-selfcheck.tsv" 'selfcheck' FAIL "${first_diag:-selfcheck-exit-$self_status}" "exit $self_status time=${real_time:-unknown}s rss=${rss:-unknown}"
    elif [ -s "$self_stdout" ] || [ -s "$self_stderr" ]; then
        write_row "$run_dir/rows/04-selfcheck.tsv" 'selfcheck' FAIL 'check-not-silent' "time=${real_time:-unknown}s rss=${rss:-unknown}"
    else
        write_row "$run_dir/rows/04-selfcheck.tsv" 'selfcheck' PASS - "time=${real_time:-unknown}s rss=${rss:-unknown}"
        printf 'fingerprint %s\nepoch %s\ntime %s\nrss %s\n' "$(source_fingerprint)" "$(date +%s)" "${real_time:-unknown}" "${rss:-unknown}" > "$SCRATCH/selfcheck-pass"
    fi
    collect_rows
    finish_run selfcheck "$table_file" "logs: $self_stdout $self_stderr"
}

run_command_row() {
    row_name=$1
    row_bound=$2
    shift 2
    row_log=$run_dir/$row_name.log
    row_marker=$run_dir/$row_name.timeout
    run_bounded "$row_bound" "$row_marker" "$@" > "$row_log" 2>&1
    row_status=$?
    if [ -f "$row_marker" ]; then
        write_row "$run_dir/rows/30-$row_name.tsv" "$row_name" FAIL timeout "exceeded ${row_bound}s"
    elif [ "$row_status" -eq 0 ]; then
        write_row "$run_dir/rows/30-$row_name.tsv" "$row_name" PASS - "exit 0"
    elif [ "$row_status" -eq 2 ]; then
        write_row "$run_dir/rows/30-$row_name.tsv" "$row_name" INCOMPLETE 'exit-2' 'command reported incomplete'
    else
        write_row "$run_dir/rows/30-$row_name.tsv" "$row_name" FAIL "exit-$row_status" "exit $row_status"
    fi
    tail -n 20 "$row_log" || true
}

run_full() {
    prepare_run_dir full
    quick_log=$run_dir/quick.log
    /bin/bash "$script_path" quick > "$quick_log" 2>&1
    quick_status=$?
    tail -n 30 "$quick_log" || true
    if [ "$quick_status" -eq 0 ]; then
        write_row "$run_dir/rows/40-quick.tsv" 'gate-quick' PASS - 'exit 0'
    elif [ "$quick_status" -eq 2 ]; then
        write_row "$run_dir/rows/40-quick.tsv" 'gate-quick' INCOMPLETE 'exit-2' 'quick gate incomplete'
    else
        write_row "$run_dir/rows/40-quick.tsv" 'gate-quick' FAIL "exit-$quick_status" "quick gate exit $quick_status"
    fi
    native_log=$run_dir/native.log
    /bin/bash "$script_path" native-conformance > "$native_log" 2>&1
    native_status=$?
    tail -n 40 "$native_log" || true
    if [ "$native_status" -eq 0 ]; then
        write_row "$run_dir/rows/41-native.tsv" 'gate-native-conformance' PASS - 'exit 0'
    elif [ "$native_status" -eq 2 ]; then
        write_row "$run_dir/rows/41-native.tsv" 'gate-native-conformance' INCOMPLETE 'exit-2' 'native conformance incomplete'
    else
        write_row "$run_dir/rows/41-native.tsv" 'gate-native-conformance' FAIL "exit-$native_status" "native conformance exit $native_status"
    fi
    self_log=$run_dir/selfcheck.log
    /bin/bash "$script_path" selfcheck > "$self_log" 2>&1
    self_status=$?
    tail -n 40 "$self_log" || true
    if [ "$self_status" -eq 0 ]; then
        write_row "$run_dir/rows/42-selfcheck.tsv" 'gate-selfcheck' PASS - 'exit 0'
    elif [ "$self_status" -eq 2 ]; then
        write_row "$run_dir/rows/42-selfcheck.tsv" 'gate-selfcheck' INCOMPLETE 'exit-2' 'selfcheck incomplete'
    else
        write_row "$run_dir/rows/42-selfcheck.tsv" 'gate-selfcheck' FAIL "exit-$self_status" "selfcheck exit $self_status"
    fi
    cargo_dir=$run_dir/cargo-target
    echo "--- cargo test --release -j 2 ---"
    run_command_row cargo-test 900 env CARGO_TARGET_DIR="$cargo_dir" cargo test --manifest-path "$repo/compiler/v1/Cargo.toml" --release -j 2
    built_oracle=$cargo_dir/release/rey-v0
    if [ -x "$built_oracle" ]; then
        echo '--- oracle conformance ---'
        run_command_row oracle-conformance 300 /bin/bash "$repo/scripts/conformance.sh" --engine "$built_oracle"
    else
        write_row "$run_dir/rows/30-oracle-conformance.tsv" 'oracle-conformance' INCOMPLETE 'no-oracle' 'cargo test did not leave rey-v0'
    fi
    echo '--- runtime normal+sanitize ---'
    run_command_row runtime-full 900 /bin/sh "$repo/scripts/test-runtime.sh"
    native_bin=${REY_BIN:-$repo/rey-compiler/main}
    case "$native_bin" in
        /*) ;;
        *) native_bin=$repo/$native_bin ;;
    esac
    if [ -x "$native_bin" ] && [ -x "$built_oracle" ]; then
        echo '--- differential ---'
        run_command_row differential 300 /bin/sh "$repo/scripts/differential.sh" --a "$built_oracle" --b "$native_bin"
    else
        write_row "$run_dir/rows/30-differential.tsv" 'differential' INCOMPLETE 'no-native-binary' "$native_bin"
    fi
    echo '--- scripts/ci.sh ---'
    run_command_row ci 2400 /bin/sh "$repo/scripts/ci.sh"
    collect_rows
    finish_run full "$table_file" "logs: $run_dir"
}

run_bootstrap() {
    prepare_run_dir bootstrap
    current_fp=$(source_fingerprint)
    if [ ! -f "$SCRATCH/selfcheck-pass" ]; then
        write_row "$run_dir/rows/50-bootstrap.tsv" 'bootstrap' INCOMPLETE 'selfcheck-required' 'selfcheck has not passed'
        collect_rows
        finish_run bootstrap "$table_file" ''
    fi
    stamp_fp=$(awk '$1=="fingerprint"{print $2}' "$SCRATCH/selfcheck-pass")
    if [ "$stamp_fp" != "$current_fp" ]; then
        write_row "$run_dir/rows/50-bootstrap.tsv" 'bootstrap' INCOMPLETE 'stale-selfcheck' 'selfcheck passed on a different source fingerprint'
        collect_rows
        finish_run bootstrap "$table_file" ''
    fi
    now_epoch=$(date +%s)
    if [ -f "$SCRATCH/bootstrap-last" ]; then
        last_epoch=$(awk '$1=="epoch"{print $2}' "$SCRATCH/bootstrap-last")
        case "$last_epoch" in
            ''|*[!0-9]*) last_epoch=0 ;;
        esac
        age=$((now_epoch - last_epoch))
        if [ "$age" -lt 3600 ]; then
            write_row "$run_dir/rows/50-bootstrap.tsv" 'bootstrap' INCOMPLETE 'hourly-limit' "last bootstrap ${age}s ago"
            collect_rows
            finish_run bootstrap "$table_file" ''
        fi
    fi
    printf 'epoch %s\nfingerprint %s\n' "$now_epoch" "$current_fp" > "$SCRATCH/bootstrap-last"
    boot_parent=$SCRATCH/bootstrap-runs
    mkdir -p "$boot_parent"
    boot_log=$run_dir/bootstrap.log
    if command -v timeout >/dev/null 2>&1; then
        /usr/bin/lockf -k "$HEAVY_LOCK" timeout 1200 \
            env BOOTSTRAP_BUILD_PARENT="$boot_parent" \
                BOOTSTRAP_TIMEOUT_SECONDS=600 \
                BOOTSTRAP_TOTAL_TIMEOUT_SECONDS=1100 \
                BOOTSTRAP_ORACLE="$ORACLE" \
            /bin/bash "$repo/scripts/bootstrap.sh" > "$boot_log" 2>&1
        boot_status=$?
    else
        write_row "$run_dir/rows/50-bootstrap.tsv" 'bootstrap' INCOMPLETE 'no-timeout' 'timeout(1) is required for the heavy-lock bound'
        collect_rows
        finish_run bootstrap "$table_file" ''
    fi
    tail -n 40 "$boot_log" || true
    if [ "$boot_status" -eq 0 ]; then
        write_row "$run_dir/rows/50-bootstrap.tsv" 'bootstrap' PASS - 'SELFHOST stages completed'
    elif [ "$boot_status" -eq 124 ]; then
        write_row "$run_dir/rows/50-bootstrap.tsv" 'bootstrap' FAIL timeout 'heavy-lock timeout 1200s'
    else
        write_row "$run_dir/rows/50-bootstrap.tsv" 'bootstrap' FAIL "exit-$boot_status" "exit $boot_status"
    fi
    collect_rows
    finish_run bootstrap "$table_file" "log: $boot_log"
}

watch_fingerprint() {
    manifest=$(mktemp "$SCRATCH/manifest.XXXXXX") || return 1
    : > "$manifest"
    printf 'oracle %s\n' "$(oracle_hash)" >> "$manifest"
    find "$repo/rey-compiler" -name '*.rey' -type f | LC_ALL=C sort | while IFS= read -r source_file; do
        /usr/bin/shasum -a 256 "$source_file"
    done >> "$manifest"
    find "$repo/rey-compiler/runtime" \( -name '*.c' -o -name '*.h' \) -type f | LC_ALL=C sort | while IFS= read -r source_file; do
        /usr/bin/shasum -a 256 "$source_file"
    done >> "$manifest"
    if [ -d "$repo/compiler/v1/src" ]; then
        find "$repo/compiler/v1/src" -type f | LC_ALL=C sort | while IFS= read -r source_file; do
            /usr/bin/shasum -a 256 "$source_file"
        done >> "$manifest"
    fi
    /usr/bin/shasum -a 256 "$manifest" | awk '{print $1}'
    rm -f "$manifest"
}

prune_run_dirs() {
    keep=${GATE_KEEP_RUN_DIRS:-10}
    case "$keep" in
        ''|*[!0-9]*|0) keep=10 ;;
    esac
    listing=$(mktemp "$SCRATCH/prune.XXXXXX") || return 0
    find "$SCRATCH" -mindepth 1 -maxdepth 1 -type d \( -name 'quick-*' -o -name 'native-*' \) -print > "$listing" || true
    ranked=$(mktemp "$SCRATCH/prune.XXXXXX") || { rm -f "$listing"; return 0; }
    : > "$ranked"
    while IFS= read -r old_dir; do
        [ -n "$old_dir" ] || continue
        old_mtime=$(stat -f '%m' "$old_dir" 2>/dev/null || echo 0)
        printf '%s\t%s\n' "$old_mtime" "$old_dir" >> "$ranked"
    done < "$listing"
    sort -nr "$ranked" -o "$ranked"
    kept=0
    while IFS="$(printf '\t')" read -r old_mtime old_dir; do
        [ -n "$old_dir" ] || continue
        kept=$((kept + 1))
        if [ "$kept" -gt "$keep" ]; then
            rm -rf "$old_dir"
        fi
    done < "$ranked"
    rm -f "$listing" "$ranked"
}

write_ci_status() {
    ci_fp=$1
    ci_quick_dir=$2
    ci_quick_exit=$3
    ci_native_dir=$4
    ci_native_exit=$5
    ci_quick_tsv=
    ci_native_tsv=
    if [ -n "$ci_quick_dir" ] && [ -f "$ci_quick_dir/results.tsv" ]; then
        ci_quick_tsv=$ci_quick_dir/results.tsv
    fi
    if [ -n "$ci_native_dir" ] && [ -f "$ci_native_dir/results.tsv" ]; then
        ci_native_tsv=$ci_native_dir/results.tsv
    fi
    python3 - "$CI_STATUS" "$CI_PREVIOUS" "$ci_fp" "$(oracle_hash)" \
        "${ci_quick_tsv:-}" "$ci_quick_exit" "${ci_native_tsv:-}" "$ci_native_exit" <<'PY'
import pathlib, sys
dest, previous_path, fingerprint, oracle_hash, quick_tsv, quick_exit, native_tsv, native_exit = sys.argv[1:]

def load(path):
    rows = {}
    if not path:
        return rows
    file = pathlib.Path(path)
    if not file.is_file():
        return rows
    for line in file.read_text(errors="replace").splitlines():
        parts = line.split("\t")
        if len(parts) >= 2 and parts[0]:
            rows[parts[0]] = parts[1]
    return rows

def counts(rows):
    passed = sum(1 for status in rows.values() if status == "PASS")
    failed = sum(1 for status in rows.values() if status == "FAIL")
    incomplete = sum(1 for status in rows.values() if status == "INCOMPLETE")
    total = len(rows)
    if failed:
        overall = "FAIL"
    elif incomplete:
        overall = "INCOMPLETE"
    elif total:
        overall = "PASS"
    else:
        overall = "FAIL"
    summary = f"summary: {passed} passed, {failed} failed, {incomplete} incomplete, {total} total"
    return summary, overall

def load_previous(path):
    saved = {"quick": {}, "native": {}}
    file = pathlib.Path(path)
    if not file.is_file():
        return saved, False
    for line in file.read_text(errors="replace").splitlines():
        parts = line.split("\t")
        if len(parts) >= 3 and parts[0] in saved and parts[1]:
            saved[parts[0]][parts[1]] = parts[2]
    return saved, True

quick_rows = load(quick_tsv)
native_rows = load(native_tsv)
previous, had_previous = load_previous(previous_path)
current = {"quick": quick_rows, "native": native_rows}
newly_passing = []
regressions = []
if had_previous:
    for suite in ("quick", "native"):
        before = previous[suite]
        after = current[suite]
        for case_id, status in sorted(after.items()):
            old = before.get(case_id)
            if old is None:
                continue
            if old != "PASS" and status == "PASS":
                newly_passing.append(f"{suite}/{case_id}")
            if old == "PASS" and status != "PASS":
                regressions.append((f"{suite}/{case_id}", old, status))

lines = ["# CI status", ""]
if regressions:
    lines.append(f"**REGRESSIONS: {len(regressions)}**")
    lines.append("")
lines.append(f"Updated: {__import__('datetime').datetime.utcnow().strftime('%Y-%m-%dT%H:%M:%SZ')}")
lines.append(f"Fingerprint: `{fingerprint}`")
lines.append(f"Oracle: `{oracle_hash}`")
lines.append("")
lines.append("## REGRESSIONS")
lines.append("")
if not had_previous:
    lines.append("No previous run to compare.")
elif not regressions:
    lines.append("none")
else:
    for case_id, old, status in regressions:
        lines.append(f"- REGRESSION `{case_id}` was {old}, now {status}")
lines.append("")
lines.append("## Quick")
lines.append("")
lines.append(f"- exit: {quick_exit}")
if quick_rows:
    summary, overall = counts(quick_rows)
    lines.append(f"- {summary}")
    lines.append(f"- overall: {overall}")
    if quick_tsv:
        lines.append(f"- results: `{quick_tsv}`")
else:
    lines.append("- overall: FAIL")
    lines.append("- no results table (peer build or gate did not finish)")
lines.append("")
lines.append("## Native conformance")
lines.append("")
lines.append(f"- exit: {native_exit}")
if native_rows:
    summary, overall = counts(native_rows)
    lines.append(f"- {summary}")
    lines.append(f"- overall: {overall}")
    if native_tsv:
        lines.append(f"- results: `{native_tsv}`")
else:
    lines.append("- overall: FAIL")
    lines.append("- no results table (peer build or gate did not finish)")
lines.append("")
lines.append("## Newly passing")
lines.append("")
if not had_previous:
    lines.append("No previous run to compare.")
elif not newly_passing:
    lines.append("none")
else:
    for case_id in newly_passing:
        lines.append(f"- `{case_id}`")
lines.append("")
pathlib.Path(dest).write_text("\n".join(lines) + "\n")
merged = {"quick": dict(previous["quick"]), "native": dict(previous["native"])}
for suite in ("quick", "native"):
    if current[suite]:
        merged[suite] = current[suite]
previous_lines = []
for suite in ("quick", "native"):
    for case_id, status in sorted(merged[suite].items()):
        previous_lines.append(f"{suite}\t{case_id}\t{status}")
pathlib.Path(previous_path).write_text("\n".join(previous_lines) + ("\n" if previous_lines else ""))
print(dest)
PY
}

run_watch_gate() {
    gate_bound=$1
    gate_mode=$2
    gate_pointer=$3
    rm -f "$gate_pointer"
    gate_log=$SCRATCH/watch-$gate_mode.log
    : > "$gate_log"
    echo "watch: starting $gate_mode (jobs=1, bound ${gate_bound}s)"
    if command -v timeout >/dev/null 2>&1; then
        GATE_JOBS=1 GATE_RUN_POINTER="$gate_pointer" timeout -k 20 "$gate_bound" \
            /bin/bash "$script_path" "$gate_mode" > "$gate_log" 2>&1
        watch_gate_status=$?
    else
        GATE_JOBS=1 GATE_RUN_POINTER="$gate_pointer" \
            /bin/bash "$script_path" "$gate_mode" > "$gate_log" 2>&1
        watch_gate_status=$?
    fi
    tail -n 8 "$gate_log" || true
    echo "watch: $gate_mode exit $watch_gate_status"
}

run_watch() {
    if [ "${GATE_WATCH_SELFTEST:-}" = 1 ]; then
        write_ci_status "${GATE_WATCH_SELFTEST_FP:-selftest}" \
            "${GATE_WATCH_SELFTEST_QUICK_DIR:-}" "${GATE_WATCH_SELFTEST_QUICK_EXIT:-1}" \
            "${GATE_WATCH_SELFTEST_NATIVE_DIR:-}" "${GATE_WATCH_SELFTEST_NATIVE_EXIT:-1}"
        exit 0
    fi
    watch_pid_file=$SCRATCH/watch.pid
    if [ -f "$watch_pid_file" ]; then
        old_watch_pid=$(tr -cd '0-9' < "$watch_pid_file" || true)
        if [ -n "$old_watch_pid" ] && kill -0 "$old_watch_pid" 2>/dev/null; then
            echo "watch already running as pid $old_watch_pid" >&2
            exit 2
        fi
    fi
    printf '%s\n' "$$" > "$watch_pid_file"
    trap 'rm -f "$watch_pid_file"; exit 130' INT TERM
    trap 'rm -f "$watch_pid_file"' EXIT
    echo "watch: interval ${watch_interval}s; quick then native-conformance when the fingerprint changes"
    echo "watch: selfcheck and bootstrap stay manual under the heavy lock"
    watch_iteration=0
    while true; do
        watch_iteration=$((watch_iteration + 1))
        if ! watch_fp=$(watch_fingerprint); then
            echo "watch: iteration $watch_iteration fingerprint failed; not launching gates"
            sleep "$watch_interval"
            continue
        fi
        watch_last=
        if [ -f "$SCRATCH/watch-fingerprint" ]; then
            watch_last=$(tr -cd '0-9a-f' < "$SCRATCH/watch-fingerprint" || true)
        fi
        echo "watch: iteration $watch_iteration fingerprint $watch_fp"
        if [ -n "$watch_last" ] && [ "$watch_fp" = "$watch_last" ]; then
            echo "watch: iteration $watch_iteration fingerprint unchanged, skipping gates"
            sleep "$watch_interval"
            continue
        fi
        quick_pointer=$SCRATCH/watch-pointer-quick
        native_pointer=$SCRATCH/watch-pointer-native
        run_watch_gate 700 quick "$quick_pointer" || true
        quick_exit=$watch_gate_status
        quick_dir=
        if [ -f "$quick_pointer" ]; then
            quick_dir=$(head -n 1 "$quick_pointer" || true)
        fi
        run_watch_gate 1700 native-conformance "$native_pointer" || true
        native_exit=$watch_gate_status
        native_dir=
        if [ -f "$native_pointer" ]; then
            native_dir=$(head -n 1 "$native_pointer" || true)
        fi
        if ! write_ci_status "$watch_fp" "$quick_dir" "$quick_exit" "$native_dir" "$native_exit"; then
            echo "watch: CI-STATUS write failed; will retry this fingerprint next interval" >&2
            sleep "$watch_interval"
            continue
        fi
        printf '%s\n' "$watch_fp" > "$SCRATCH/watch-fingerprint"
        prune_run_dirs || true
        echo "watch: iteration $watch_iteration complete, sleeping ${watch_interval}s"
        sleep "$watch_interval"
    done
}

case "$mode" in
    quick) run_quick ;;
    native-conformance) run_native_conformance ;;
    selfcheck) run_selfcheck ;;
    full) run_full ;;
    bootstrap) run_bootstrap ;;
    watch) run_watch ;;
esac
