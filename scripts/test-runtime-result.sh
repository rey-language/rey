#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd -P)
runtime_dir=${REY_RUNTIME_DIR:-$repo_root/rey-compiler/runtime}
test_source=$runtime_dir/tests/result_error_test.c
compiler=${CC:-/usr/bin/clang}
timeout_seconds=${REY_RUNTIME_TEST_TIMEOUT_SECONDS:-30}
scratch_root=/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03

case "$timeout_seconds" in
    ''|*[!0-9]*) echo "timeout must be a positive integer" >&2; exit 2 ;;
esac
if [ "$timeout_seconds" -eq 0 ]; then
    echo "timeout must be a positive integer" >&2
    exit 2
fi
if ! command -v "$compiler" >/dev/null 2>&1; then
    echo "C compiler not found: $compiler" >&2
    exit 2
fi
if [ ! -f "$test_source" ]; then
    echo "test source not found: $test_source" >&2
    exit 2
fi

mkdir -p "$scratch_root"
scratch_root=$(CDPATH= cd -- "$scratch_root" && pwd -P)
case "$scratch_root" in
    "$repo_root"|"$repo_root"/*) echo "refusing temp output root inside source tree: $scratch_root" >&2; exit 2 ;;
esac
work_dir=$(mktemp -d "$scratch_root/runtime-result.XXXXXX")
case "$work_dir" in
    "$scratch_root"/runtime-result.*) ;;
    *) echo "unexpected temp output path: $work_dir" >&2; exit 2 ;;
esac
cleanup() {
    case "$work_dir" in
        "$scratch_root"/runtime-result.*) rm -rf "$work_dir" ;;
        *) echo "refusing to remove unexpected path: $work_dir" >&2; return 1 ;;
    esac
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM

run_bounded() {
    /usr/bin/perl -e '
        use POSIX qw(:sys_wait_h);
        my $seconds = shift @ARGV;
        my $pid = fork();
        die "timeout wrapper fork failed: $!\n" unless defined $pid;
        if ($pid == 0) {
            setpgrp(0, 0) or die "timeout wrapper setpgrp failed: $!\n";
            exec { $ARGV[0] } @ARGV or die "timeout wrapper exec failed: $!\n";
        }
        $SIG{ALRM} = sub {
            kill "TERM", -$pid;
            kill "KILL", -$pid;
            waitpid($pid, 0);
            print STDERR "timed out after $seconds seconds\n";
            exit 124;
        };
        alarm($seconds);
        waitpid($pid, 0);
        alarm(0);
        my $status = $?;
        if (WIFEXITED($status)) { exit WEXITSTATUS($status); }
        if (WIFSIGNALED($status)) { exit 128 + WTERMSIG($status); }
        exit 1;
    ' "$timeout_seconds" "$@"
}

run_mode() {
    mode=$1
    shift
    binary=$work_dir/result-error-$mode
    stdout_file=$work_dir/$mode.stdout
    stderr_file=$work_dir/$mode.stderr
    sanitizer_flags=
    if [ "$mode" = sanitize ]; then
        sanitizer_flags="-fsanitize=address,undefined -fno-omit-frame-pointer"
    fi

    echo "[$mode] compiler: $compiler"
    # sanitizer_flags is intentionally word-split into compiler options.
    if ! run_bounded "$compiler" -Wall -Wextra -Werror -std=c11 -D_POSIX_C_SOURCE=200809L \
        -I "$runtime_dir" $sanitizer_flags \
        "$test_source" \
        "$runtime_dir/mem.c" "$runtime_dir/string.c" "$runtime_dir/vec.c" \
        "$runtime_dir/hashmap.c" "$runtime_dir/result.c" "$runtime_dir/enum.c" \
        "$runtime_dir/io.c" "$runtime_dir/value.c" \
        -o "$binary" >"$work_dir/$mode.compile.stdout" 2>"$work_dir/$mode.compile.stderr"; then
        cat "$work_dir/$mode.compile.stdout" "$work_dir/$mode.compile.stderr" >&2
        return 1
    fi

    if [ "$mode" = sanitize ]; then
        # Runtime strings/results have process lifetime and no public free API.
        if ! run_bounded /usr/bin/env ASAN_OPTIONS=detect_leaks=0 UBSAN_OPTIONS=halt_on_error=1 "$binary" \
            >"$stdout_file" 2>"$stderr_file"; then
            cat "$stdout_file" "$stderr_file" >&2
            return 1
        fi
    else
        if ! run_bounded "$binary" >"$stdout_file" 2>"$stderr_file"; then
            cat "$stdout_file" "$stderr_file" >&2
            return 1
        fi
    fi
    cat "$stdout_file"
    if [ -s "$stderr_file" ]; then cat "$stderr_file" >&2; fi
}

mode=${1:-all}
case "$mode" in
    normal) run_mode normal ;;
    sanitize) run_mode sanitize ;;
    all) run_mode normal; run_mode sanitize ;;
    *) echo "usage: $0 [normal|sanitize|all]" >&2; exit 2 ;;
esac
