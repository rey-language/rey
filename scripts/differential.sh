#!/bin/sh

set -u

usage() {
    echo 'usage: scripts/differential.sh --a "<cmd>" --b "<cmd>" [--filter s]' >&2
}

resolve_engine() {
    command_text=$1
    if [ -x "$command_text" ] && [ ! -d "$command_text" ]; then
        resolved=$command_text
    else
        resolved=$(sh -c 'set -f; eval "set -- $1"; [ "$#" -gt 0 ] || exit 1; command -v "$1"' _ "$command_text" 2>/dev/null) || return 1
    fi
    case "$resolved" in
        /*) ;;
        */*) resolved="$CALLER_DIR/$resolved" ;;
        *)
            old_ifs=$IFS
            IFS=:
            for path_entry in $PATH; do
                [ -n "$path_entry" ] || path_entry=$CALLER_DIR
                if [ -x "$path_entry/$resolved" ]; then
                    case "$path_entry" in
                        /*) resolved="$path_entry/$resolved" ;;
                        *) resolved="$CALLER_DIR/$path_entry/$resolved" ;;
                    esac
                    break
                fi
            done
            IFS=$old_ifs
            ;;
    esac
    [ -x "$resolved" ] || return 1
    printf '%s\n' "$resolved"
}

check_has_text() {
    text_file=$1
    out_file=$2
    err_file=$3
    [ -s "$text_file" ] || return 1
    expected_text=$(cat "$text_file")
    [ -n "$expected_text" ] || return 1
    combined_file="$out_file.combined"
    cat "$out_file" "$err_file" > "$combined_file"
    grep -F -q -e "$expected_text" "$combined_file" 2>/dev/null
}

run_bounded() {
    run_dir=$1
    run_mode=$2
    run_command=$3
    run_binary=$4
    run_stdout=$5
    run_stderr=$6
    run_timeout_marker=$7

    (cd "$run_dir" && perl - "$run_mode" "$run_command" "$run_binary" "$run_timeout_marker" "$run_dir/main.rey" <<'PERL'
use POSIX qw(setpgid WNOHANG);
use Time::HiRes qw(time sleep);

my ($mode, $command, $binary, $timeout_marker, $source) = @ARGV;
my $pid = fork();
defined $pid or die "fork failed: $!\n";
if ($pid == 0) {
    setpgid(0, 0) == 0 or exit 125;
    my $script = q{
engine=$1
binary=$2
mode=$3
source=$4
set -f
if [ -x "$engine" ] && [ ! -d "$engine" ]; then
    set --
else
    eval "set -- $engine"
    shift
fi
exec "$binary" "$@" "$mode" "$source"
};
    exec 'sh', '-c', $script, 'rey-engine', $command, $binary, $mode, $source;
    exit 127;
}

my $deadline = time() + 30;
while (1) {
    my $waited = waitpid($pid, WNOHANG);
    if ($waited == $pid) {
        my $status = $?;
        exit(($status & 127) ? 128 + ($status & 127) : ($status >> 8));
    }
    if (time() >= $deadline) {
        kill 'TERM', -$pid;
        my $grace_deadline = time() + 1;
        while (time() < $grace_deadline) {
            last if waitpid($pid, WNOHANG) == $pid;
            sleep(0.05);
        }
        kill 'KILL', -$pid;
        waitpid($pid, 0);
        open my $marker, '>', $timeout_marker;
        close $marker if $marker;
        exit 124;
    }
    sleep(0.05);
}
PERL
    ) >"$run_stdout" 2>"$run_stderr"
    run_status=$?
    if [ -f "$run_timeout_marker" ]; then
        run_outcome=timeout
    else
        run_outcome="exit=$run_status"
    fi
}

preflight_engine() {
    preflight_side=$1
    preflight_command=$2
    preflight_binary=$3
    preflight_dir="$workspace/preflight-$preflight_side"
    mkdir -p "$preflight_dir" || {
        echo "Engine preflight FAIL: engine-$preflight_side (cannot create isolated directory)" >&2
        return 1
    }
    printf 'var preflightValue: int = 1;\n' > "$preflight_dir/main.rey"
    run_bounded "$preflight_dir" check "$preflight_command" "$preflight_binary" \
        "$preflight_dir/stdout" "$preflight_dir/stderr" "$preflight_dir/timed-out"
    if [ "$run_outcome" = timeout ]; then
        echo "Engine preflight FAIL: engine-$preflight_side check timed out" >&2
    elif [ "$run_status" -ne 0 ]; then
        echo "Engine preflight FAIL: engine-$preflight_side valid check must exit 0; got $run_status" >&2
    elif [ -s "$preflight_dir/stdout" ] || [ -s "$preflight_dir/stderr" ]; then
        echo "Engine preflight FAIL: engine-$preflight_side successful check must be silent" >&2
    else
        echo "Engine preflight PASS: engine-$preflight_side"
        return 0
    fi
    if [ -s "$preflight_dir/stdout" ]; then
        echo '  check stdout:' >&2
        cat "$preflight_dir/stdout" >&2
    fi
    if [ -s "$preflight_dir/stderr" ]; then
        echo '  check stderr:' >&2
        cat "$preflight_dir/stderr" >&2
    fi
    return 1
}

record_result() {
    result_path=$1
    result_case=$2
    result_a=$3
    result_b=$4
    result_status=$5
    printf '%s\t%s\t%s\t%s\n' "$result_case" "$result_a" "$result_b" "$result_status" > "$result_path"
}

worker() {
    engine_a=$1
    binary_a=$2
    engine_b=$3
    binary_b=$4
    workspace=$5
    corpus_dir=$6
    case_index=$7

    IFS= read -r case_path < "$workspace/inputs/$case_index" || case_path=
    case_dir=${case_path%/main.rey}
    case_name=${case_dir#"$corpus_dir"/}
    result_path="$workspace/results/$case_index"
    case_work="$workspace/cases/$case_index"
    mkdir -p "$case_work" || {
        record_result "$result_path" "$case_name" 'worker setup failed' 'worker setup failed' FAIL
        return 0
    }

    if [ ! -f "$case_dir/expected.stdout" ]; then
        record_result "$result_path" "$case_name" 'missing expected.stdout' 'missing expected.stdout' FAIL
        return 0
    fi
    expected_exit=0
    if [ -f "$case_dir/expected.exit" ]; then
        IFS= read -r expected_exit < "$case_dir/expected.exit" || [ -n "$expected_exit" ]
        case "$expected_exit" in
            ''|*[!0-9]*)
                record_result "$result_path" "$case_name" 'invalid expected.exit' 'invalid expected.exit' FAIL
                return 0
                ;;
        esac
    fi

    if [ -f "$case_dir/expect_check_fail" ]; then
        expect_check_fail=1
        if [ ! -s "$case_dir/expect_check_fail" ]; then
            record_result "$result_path" "$case_name" 'invalid empty expect_check_fail' 'invalid empty expect_check_fail' FAIL
            return 0
        fi
    else
        expect_check_fail=0
    fi

    for side in a b; do
        side_dir="$case_work/engine-$side/check"
        mkdir -p "$side_dir" || {
            record_result "$result_path" "$case_name" 'worker setup failed' 'worker setup failed' FAIL
            return 0
        }
        cp -R "$case_dir/." "$side_dir/" 2>/dev/null || {
            record_result "$result_path" "$case_name" 'fixture copy failed' 'fixture copy failed' FAIL
            return 0
        }
    done

    if [ "$expect_check_fail" -eq 1 ]; then
        for side in a b; do
            if [ "$side" = a ]; then
                side_command=$engine_a
                side_binary=$binary_a
            else
                side_command=$engine_b
                side_binary=$binary_b
            fi
            side_dir="$case_work/engine-$side/check"
            run_bounded "$side_dir" check "$side_command" "$side_binary" "$side_dir/stdout" "$side_dir/stderr" "$side_dir/timed-out"
            if [ "$run_outcome" = timeout ]; then
                side_check='check=timeout'
                side_good=0
            elif [ "$run_status" -eq 0 ]; then
                side_check='check=ok (expected failure)'
                side_good=0
            elif [ "$run_status" -ne 1 ]; then
                side_check="check=exit:$run_status (expected exit 1)"
                side_good=0
            elif check_has_text "$case_dir/expect_check_fail" "$side_dir/stdout" "$side_dir/stderr"; then
                side_check="check=failed:$run_status (expected)"
                side_good=1
            else
                side_check="check=failed:$run_status (diagnostic mismatch)"
                side_good=0
            fi
            if [ "$side" = a ]; then
                a_result=$side_check
                a_good=$side_good
            else
                b_result=$side_check
                b_good=$side_good
            fi
        done
        if [ "$a_good" -eq 1 ] && [ "$b_good" -eq 1 ]; then
            record_result "$result_path" "$case_name" "$a_result" "$b_result" PASS
        else
            record_result "$result_path" "$case_name" "$a_result" "$b_result" FAIL
        fi
        return 0
    fi

    for side in a b; do
        if [ "$side" = a ]; then
            side_command=$engine_a
            side_binary=$binary_a
        else
            side_command=$engine_b
            side_binary=$binary_b
        fi
        side_dir="$case_work/engine-$side/check"
        run_bounded "$side_dir" check "$side_command" "$side_binary" "$side_dir/stdout" "$side_dir/stderr" "$side_dir/timed-out"
        if [ "$run_outcome" = timeout ]; then
            side_check='check=timeout'
            side_good=0
        elif [ "$run_status" -eq 0 ]; then
            if [ -s "$side_dir/stdout" ] || [ -s "$side_dir/stderr" ]; then
                side_check='check=noisy (expected silent success)'
                side_good=0
            else
                side_check='check=ok'
                side_good=1
            fi
        else
            side_check="check=failed:$run_status"
            side_good=0
        fi
        if [ "$side" = a ]; then
            a_check=$side_check
            a_check_good=$side_good
        else
            b_check=$side_check
            b_check_good=$side_good
        fi
    done

    if [ "$a_check_good" -ne 1 ] || [ "$b_check_good" -ne 1 ]; then
        record_result "$result_path" "$case_name" "$a_check;run=not-run" "$b_check;run=not-run" FAIL
        return 0
    fi

    for side in a b; do
        side_dir="$case_work/engine-$side/run"
        mkdir -p "$side_dir" || {
            record_result "$result_path" "$case_name" 'worker setup failed' 'worker setup failed' FAIL
            return 0
        }
        cp -R "$case_dir/." "$side_dir/" 2>/dev/null || {
            record_result "$result_path" "$case_name" 'fixture copy failed' 'fixture copy failed' FAIL
            return 0
        }
    done

    for side in a b; do
        if [ "$side" = a ]; then
            side_command=$engine_a
            side_binary=$binary_a
        else
            side_command=$engine_b
            side_binary=$binary_b
        fi
        side_dir="$case_work/engine-$side/run"
        run_bounded "$side_dir" run "$side_command" "$side_binary" "$side_dir/stdout" "$side_dir/stderr" "$side_dir/timed-out"
        if [ "$run_outcome" = timeout ]; then
            side_run='run=timeout'
            side_good=0
        else
            if cmp -s "$case_dir/expected.stdout" "$side_dir/stdout"; then
                stdout_result=stdout=match
            else
                stdout_result=stdout=mismatch
            fi
            if [ "$run_status" -eq "$expected_exit" ] && [ "$stdout_result" = stdout=match ]; then
                side_good=1
            else
                side_good=0
            fi
            side_run="run=$run_status,$stdout_result"
        fi
        if [ "$side" = a ]; then
            a_run=$side_run
            a_run_status=$run_status
            a_run_good=$side_good
        else
            b_run=$side_run
            b_run_status=$run_status
            b_run_good=$side_good
        fi
    done

    if [ "$a_run" = 'run=timeout' ] || [ "$b_run" = 'run=timeout' ]; then
        runs_match=0
        a_run="$a_run,parity=not-compared"
        b_run="$b_run,parity=not-compared"
    elif [ "$a_run_status" -eq "$b_run_status" ] && cmp -s "$case_work/engine-a/run/stdout" "$case_work/engine-b/run/stdout"; then
        runs_match=1
        a_run="$a_run,parity=match"
        b_run="$b_run,parity=match"
    else
        runs_match=0
        a_run="$a_run,parity=different"
        b_run="$b_run,parity=different"
    fi
    if [ "$a_run_good" -eq 1 ] && [ "$b_run_good" -eq 1 ] && [ "$runs_match" -eq 1 ]; then
        verdict=PASS
    else
        verdict=FAIL
    fi
    record_result "$result_path" "$case_name" "$a_check;$a_run" "$b_check;$b_run" "$verdict"
    return 0
}

SCRIPT_DIR=$(CDPATH= cd "$(dirname "$0")" 2>/dev/null && pwd) || exit 2
SELF="$SCRIPT_DIR/differential.sh"
REPO_ROOT=$(CDPATH= cd "$SCRIPT_DIR/.." 2>/dev/null && pwd) || exit 2
CALLER_DIR=$(pwd -P)

if [ "${1:-}" = --worker ]; then
    shift
    worker "$@"
    exit $?
fi

engine_a=
engine_b=
filter=
filter_set=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        --a)
            [ "$#" -ge 2 ] || { usage; exit 2; }
            engine_a=$2
            shift 2
            ;;
        --b)
            [ "$#" -ge 2 ] || { usage; exit 2; }
            engine_b=$2
            shift 2
            ;;
        --filter)
            [ "$#" -ge 2 ] || { usage; exit 2; }
            filter=$2
            filter_set=1
            shift 2
            ;;
        *)
            usage
            exit 2
            ;;
    esac
done
[ -n "$engine_a" ] && [ -n "$engine_b" ] || { usage; exit 2; }

corpus="$REPO_ROOT/tests/conformance"
if [ ! -d "$corpus" ]; then
    echo "error: conformance corpus directory not found: $corpus" >&2
    exit 2
fi

binary_a=$(resolve_engine "$engine_a") || {
    echo "error: missing engine binary for --a: $engine_a" >&2
    exit 2
}
binary_b=$(resolve_engine "$engine_b") || {
    echo "error: missing engine binary for --b: $engine_b" >&2
    exit 2
}
if [ -x "$engine_a" ] && [ ! -d "$engine_a" ]; then
    engine_a=$binary_a
fi
if [ -x "$engine_b" ] && [ ! -d "$engine_b" ]; then
    engine_b=$binary_b
fi

scratch=/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03
mkdir -p "$scratch" || { echo "error: cannot create scratch directory: $scratch" >&2; exit 2; }
workspace=$(mktemp -d "$scratch/differential.XXXXXX") || { echo 'error: cannot create differential work directory' >&2; exit 2; }
if [ "${REY_KEEP_DIFFERENTIAL_SCRATCH:-0}" = 1 ]; then
    trap 'echo "retained differential workspace: $workspace"' EXIT HUP INT TERM
else
    trap 'rm -rf "$workspace"' EXIT HUP INT TERM
fi
mkdir -p "$workspace/results" "$workspace/cases" "$workspace/inputs"

preflight_failed=0
preflight_engine a "$engine_a" "$binary_a" || preflight_failed=1
preflight_engine b "$engine_b" "$binary_b" || preflight_failed=1
if [ "$preflight_failed" -ne 0 ]; then
    echo 'No conformance cases were counted or run.' >&2
    exit 1
fi

if ! find "$corpus" -type f -name main.rey -print > "$workspace/cases.unsorted"; then
    echo "error: cannot enumerate conformance corpus: $corpus" >&2
    exit 2
fi
LC_ALL=C sort "$workspace/cases.unsorted" > "$workspace/cases.all" || exit 2
: > "$workspace/cases.selected"
while IFS= read -r case_path; do
    [ -n "$case_path" ] || continue
    case_name=${case_path#"$corpus"/}
    include_case=1
    if [ "$filter_set" -eq 1 ]; then
        case "$case_name" in
            *"$filter"*) ;;
            *) include_case=0 ;;
        esac
    fi
    if [ "$include_case" -eq 1 ]; then
        printf '%s\n' "$case_path" >> "$workspace/cases.selected"
    fi
done < "$workspace/cases.all"

case_count=0
while IFS= read -r case_path; do
    [ -n "$case_path" ] || continue
    printf '%s\n' "$case_path" > "$workspace/inputs/$case_count"
    printf '%s\n' "$case_count" >> "$workspace/tasks"
    case_count=$((case_count + 1))
done < "$workspace/cases.selected"
if [ "$case_count" -eq 0 ]; then
    if [ "$filter_set" -eq 1 ]; then
        echo "error: no conformance cases matched filter: $filter" >&2
    else
        echo "error: no conformance cases found under: $corpus" >&2
    fi
    exit 2
fi

if ! xargs -P 4 -I '{}' "$SELF" --worker "$engine_a" "$binary_a" "$engine_b" "$binary_b" "$workspace" "$corpus" '{}' < "$workspace/tasks"; then
    echo 'warning: one or more differential workers exited unexpectedly' >&2
fi

passed=0
failed=0
ordinal=0
divergence_file="$workspace/divergences"
: > "$divergence_file"
while IFS= read -r case_path; do
    [ -n "$case_path" ] || continue
    # Result files are keyed by sorted-case ordinal, not by the number of failures.
    result_path="$workspace/results/$ordinal"
    ordinal=$((ordinal + 1))
    if [ ! -f "$result_path" ]; then
        case_name=${case_path#"$corpus"/}
        case_name=${case_name%/main.rey}
        printf '%s\t%s\t%s\tFAIL\n' "$case_name" 'worker produced no result' 'worker produced no result' >> "$divergence_file"
        failed=$((failed + 1))
        continue
    fi
    IFS="$(printf '\t')" read -r result_case result_a result_b result_status < "$result_path"
    if [ "$result_status" = PASS ]; then
        passed=$((passed + 1))
    else
        printf '%s\t%s\t%s\t%s\n' "$result_case" "$result_a" "$result_b" "$result_status" >> "$divergence_file"
        failed=$((failed + 1))
    fi
done < "$workspace/cases.selected"

if [ "$failed" -gt 0 ]; then
    echo 'Divergences (includes golden-fixture failures):'
    printf '%-36s | %-40s | %s\n' 'case' 'engine-a result' 'engine-b result'
    while IFS="$(printf '\t')" read -r result_case result_a result_b result_status; do
        [ -n "$result_case" ] || continue
        printf '%-36s | %-40s | %s\n' "$result_case" "$result_a" "$result_b"
    done < "$divergence_file"
else
    echo 'Divergences: none'
fi
printf 'Pass count: %s/%s\n' "$passed" "$case_count"
printf 'Fail count: %s/%s\n' "$failed" "$case_count"
[ "$failed" -eq 0 ]
