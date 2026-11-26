#!/bin/sh
# Pinned-seed bootstrap entry point for native Linux x64.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
tests="$root/tests/v04/linux-x64-bootstrap"
budget="$root/scripts/v04-linux-x64-budget.sh"
service="$tests/run-service.sh"
service_helper="$tests/service-helper.sh"
seed_manifest="$root/docs/v04-linux-x64-seed.sha256"
seed_sha=$(awk 'NF && $1 !~ /^#/ { print $1; exit }' "$seed_manifest")
[ -n "$seed_sha" ] || { echo 'seed hash manifest is empty' >&2; exit 2; }

fail() { printf 'bootstrap: %s\n' "$*" >&2; exit 1; }

usage() {
    cat >&2 <<'EOF'
Usage:
  scripts/v04-selfhost-linux-x64.sh --self-test --seed PATH --source DIR --work DIR
  scripts/v04-selfhost-linux-x64.sh --candidate --seed PATH --source DIR --revision SHA --work DIR
  scripts/v04-selfhost-linux-x64.sh --plan

The candidate mode performs one normally exiting seed-to-S1 build. A full
multi-stage verification chain is held until its separate memory admission.
EOF
}

emit_plan() {
    printf '%s\n' \
        'BUILD /seed/rey build /src/compiler/main.rey -o /out/S1 --target linux-x64 --regions --managed' \
        'BUILD /out/S1 build /src/compiler/main.rey -o /out/S2 --target linux-x64 --regions --managed' \
        'BUILD /out/S2 build /src/compiler/main.rey -o /out/S3 --target linux-x64 --regions --managed' \
        'ALLOW_EXEC /usr/sbin/chroot' \
        "ALLOW_EXEC /seed/rey sha256=$seed_sha" \
        'ALLOW_EXEC /out/S1 sha256=recorded-at-S1-build' \
        'ALLOW_EXEC /out/S2 sha256=recorded-at-S2-build'
}

check_hash() {
    path=$1
    expected=$2
    [ -f "$path" ] || { echo "seed file is missing: $path" >&2; return 1; }
    actual=$(sha256sum "$path" | awk '{ print $1 }') || return 1
    [ "$actual" = "$expected" ] || {
        echo "seed hash mismatch: expected $expected, got $actual" >&2
        return 1
    }
}

check_static_elf() {
    path=$1
    header=$(readelf -h "$path" 2>&1) || { echo "not a readable ELF: $path" >&2; return 1; }
    printf '%s\n' "$header" | grep -q 'Class:.*ELF64' || { echo 'seed is not ELF64' >&2; return 1; }
    printf '%s\n' "$header" | grep -q 'Type:.*EXEC' || { echo 'seed is not ET_EXEC' >&2; return 1; }
    printf '%s\n' "$header" | grep -q 'Machine:.*X86-64' || { echo 'seed is not AMD x86-64' >&2; return 1; }
    program_headers=$(readelf -lW "$path" 2>&1) || { echo "cannot read ELF program headers: $path" >&2; return 1; }
    if printf '%s\n' "$program_headers" | grep -Eq '^[[:space:]]*(INTERP|DYNAMIC)([[:space:]]|$)'; then
        echo 'seed has PT_INTERP or PT_DYNAMIC' >&2
        return 1
    fi
    dynamic=$(readelf -d "$path" 2>&1) || { echo "cannot inspect ELF dynamic entries: $path" >&2; return 1; }
    case "$dynamic" in
        *'There is no dynamic section'*) ;;
        *'Dynamic section'*) echo 'seed has dynamic entries' >&2; return 1 ;;
        *) echo 'ELF dynamic-section inspection was inconclusive' >&2; return 1 ;;
    esac
}

check_seed() {
    path=$1
    [ -x "$path" ] || { echo "seed is not executable: $path" >&2; return 1; }
    check_hash "$path" "$seed_sha" || return 1
    check_static_elf "$path"
}

source_revision() {
    source_dir=$1
    revision=$2
    [ -d "$source_dir/.git" ] || { echo "source is not a Git checkout: $source_dir" >&2; return 1; }
    resolved=$(git -C "$source_dir" rev-parse --verify "$revision^{commit}" 2>/dev/null) || {
        echo "revision is not a commit: $revision" >&2
        return 1
    }
    [ "$resolved" = "$revision" ] || { echo 'revision must be the full commit SHA' >&2; return 1; }
}

new_work_dir() {
    requested=$1
    source_dir=$2
    case "$requested" in /*) ;; *) echo 'work directory must be an absolute path' >&2; return 1 ;; esac
    [ ! -e "$requested" ] || { echo "work directory already exists; refusing to reuse it: $requested" >&2; return 1; }
    parent=$(dirname -- "$requested")
    name=$(basename -- "$requested")
    [ -d "$parent" ] || { echo "work parent directory is absent: $parent" >&2; return 1; }
    parent=$(CDPATH= cd -- "$parent" && pwd -P) || return 1
    requested="$parent/$name"
    case "$requested" in *[[:space:]]*) echo 'work directory must not contain whitespace' >&2; return 1 ;; esac
    case "$requested" in
        "$source_dir"|"$source_dir"/*) echo 'work directory must be outside the source tree' >&2; return 1 ;;
    esac
    mkdir "$requested"
    printf '%s\n' "$requested"
}

stage_inputs() {
    source_dir=$1
    revision=$2
    work_dir=$3
    seed=$4
    input_mode=$5
    minimal_root="$work_dir/root"
    fixture_archive=
    mkdir -p "$minimal_root/seed" "$minimal_root/src/compiler" "$minimal_root/out" \
        "$work_dir/logs" "$work_dir/evidence" "$work_dir/archive"
    cp -p "$seed" "$minimal_root/seed/rey"
    check_hash "$minimal_root/seed/rey" "$seed_sha" || return 1

    archive="$work_dir/source.tar"
    git -C "$source_dir" archive --format=tar "$revision" \
        rey-compiler/src rey-compiler/native-main.rey >"$archive"
    tar -xf "$archive" -C "$work_dir/archive"
    cp -R "$work_dir/archive/rey-compiler/src/." "$minimal_root/src/compiler/"
    cp -p "$work_dir/archive/rey-compiler/native-main.rey" "$minimal_root/src/compiler/main.rey"

    if [ "$input_mode" = smoke ]; then
        fixture_archive="$work_dir/smoke-fixture.tar"
        git -C "$source_dir" archive --format=tar "$revision" \
            tests/v04/linux-x64-bootstrap/smoke.rey >"$fixture_archive"
        tar -xf "$fixture_archive" -C "$work_dir/archive"
        cp -p "$work_dir/archive/tests/v04/linux-x64-bootstrap/smoke.rey" \
            "$minimal_root/src/smoke.rey"
    fi

    if find "$minimal_root" -type l -print | grep -q .; then
        echo 'staged minimal root contains a symlink' >&2
        return 1
    fi
    find "$minimal_root/src/compiler" -type f -print | LC_ALL=C sort | while IFS= read -r path; do
        sha256sum "$path"
    done >"$work_dir/evidence/source-files.sha256"
    sha256sum "$archive" "$work_dir/evidence/source-files.sha256" \
        >"$work_dir/evidence/source-inputs.sha256"
    if [ -n "$fixture_archive" ]; then
        sha256sum "$fixture_archive" >>"$work_dir/evidence/source-inputs.sha256"
    fi
    printf '%s\n' "$revision" >"$work_dir/evidence/revision.txt"
    printf '%s\n' "$seed_sha" >"$work_dir/evidence/seed.sha256"
    printf '%s\n' "$minimal_root"
}

run_network_probe() {
    work_dir=$1
    trace="$work_dir/logs/network-socket.trace"
    helper="$tests/network-probe.sh"
    if ! "$budget" run-held --seconds 20 --label network-probe -- \
        "$service" network 15 "$work_dir" "$trace" "$helper" >/dev/null; then
        return 1
    fi
    [ -s "$trace" ] || { echo 'network probe did not write a socket trace' >&2; return 1; }
}

run_failed_isolation_probe() {
    work_dir=$1
    trace="$work_dir/logs/failed-isolation.trace"
    missing_root="$work_dir/root-that-must-not-exist"
    helper="$tests/failed-isolation-probe.sh"
    [ ! -e "$missing_root" ] || { echo 'failed-isolation test path already exists' >&2; return 1; }
    if ! "$budget" run-held --seconds 20 --label failed-isolation -- \
        "$service" failed-isolation 15 "$work_dir" "$trace" "$missing_root" "$helper" >/dev/null; then
        return 1
    fi
    [ -s "$trace" ] || { echo 'failed-isolation trace is missing' >&2; return 1; }
    ! grep -Fq '"/seed/rey"' "$trace" || {
        echo 'producer was invoked after a failed chroot' >&2
        return 1
    }
}

run_stage() {
    work_dir=$1
    minimal_root=$2
    input=$3
    timeout_sec=$4
    build_mode=$5
    expected_stdout=$6
    trace="$work_dir/logs/producer.trace"
    stdout_file="$work_dir/evidence/smoke.stdout"
    stderr_file="$work_dir/evidence/smoke.stderr"
    if ! "$budget" run-held --seconds "$timeout_sec" --label "${build_mode}-producer" -- \
        "$service" build "$timeout_sec" "$work_dir" "$service_helper" \
        "$minimal_root" "$work_dir" "$seed_sha" "$input" /out/S1 "$trace" \
        "$stdout_file" "$stderr_file" "$expected_stdout" "$build_mode" >/dev/null; then
        return 1
    fi
    [ -s "$trace" ] || { echo 'producer trace is missing' >&2; return 1; }
    [ -s "$work_dir/evidence/producer.time.txt" ] || { echo 'producer time report is missing' >&2; return 1; }
    [ -s "$work_dir/evidence/cgroup-before.txt" ] && [ -s "$work_dir/evidence/cgroup-after.txt" ] || {
        echo 'producer cgroup evidence is missing' >&2
        return 1
    }
}

check_trace_checker() {
    work_dir=$1
    good="$work_dir/evidence/trace-good.txt"
    bad="$work_dir/evidence/trace-injected-exec.txt"
    printf '%s\n' \
        '123 execve("/usr/sbin/chroot", ["/usr/sbin/chroot"], 0x0) = 0' \
        '123 execve("/seed/rey", ["/seed/rey"], 0x0) = 0' >"$good"
    sh "$tests/check-exec-trace.sh" "$good" /usr/sbin/chroot /seed/rey || return 1
    cp -p "$good" "$bad"
    printf '%s\n' '123 execve("/unexpected-tool", ["/unexpected-tool"], 0x0) = -1 ENOENT' >>"$bad"
    if sh "$tests/check-exec-trace.sh" "$bad" /usr/sbin/chroot /seed/rey \
        >"$work_dir/evidence/injected-exec-check.log" 2>&1; then
        echo 'trace checker accepted an unexpected failed exec attempt' >&2
        return 1
    fi
}

check_compiler_failure_modes() {
    work_dir=$1
    scratch_lock="$work_dir/contention.lock"
    ready="$work_dir/contention.ready"
    marker="$work_dir/contention-invoked"
    holder_log="$work_dir/evidence/contention-holder.log"
    flock -n "$scratch_lock" sh -c 'printf ready >"$1"; sleep 2' sh "$ready" >"$holder_log" 2>&1 &
    holder_pid=$!
    tries=0
    while [ ! -f "$ready" ]; do
        tries=$((tries + 1))
        [ "$tries" -le 30 ] || { wait "$holder_pid" || :; echo 'lock holder did not start' >&2; return 1; }
        sleep 0.1
    done
    set +e
    REY_BUDGET_SELF_TEST=1 "$budget" test-run --seconds 5 --lock "$scratch_lock" -- \
        "$tests/marker-producer.sh" "$marker" >"$work_dir/evidence/contention-check.log" 2>&1
    status=$?
    set -e
    wait "$holder_pid"
    [ "$status" -eq 73 ] && [ ! -e "$marker" ] || {
        echo 'lock contention did not prevent the test producer' >&2
        return 1
    }

    marker="$work_dir/low-memory-invoked"
    set +e
    REY_BUDGET_SELF_TEST=1 "$budget" test-run --seconds 5 --lock "$work_dir/low-memory.lock" \
        --available-gb 4 -- "$tests/marker-producer.sh" "$marker" \
        >"$work_dir/evidence/low-memory-check.log" 2>&1
    status=$?
    set -e
    [ "$status" -eq 75 ] && [ ! -e "$marker" ] || {
        echo 'simulated low memory did not prevent the test producer' >&2
        return 1
    }

    image="$work_dir/evidence/fake-image"
    set +e
    REY_BUDGET_SELF_TEST=1 "$budget" test-run --seconds 5 --lock "$work_dir/fake-producer.lock" -- \
        "$tests/fake-producer.sh" "$image" >"$work_dir/evidence/fake-producer-check.log" 2>&1
    status=$?
    set -e
    [ "$status" -eq 7 ] && [ -s "$image" ] || {
        echo 'fake producer output was accepted despite its exit status 7' >&2
        return 1
    }
}

self_test() {
    seed=$1
    source_dir=$2
    requested_work=$3
    source_dir=$(CDPATH= cd -- "$source_dir" && pwd -P) || fail "source directory is absent: $source_dir"
    revision=$(git -C "$source_dir" rev-parse HEAD) || fail 'source HEAD is unavailable'
    source_revision "$source_dir" "$revision" || fail 'source revision is invalid'
    seed=$(readlink -f -- "$seed") || fail 'seed path is absent'
    check_seed "$seed" || fail 'pinned seed check failed'
    work_dir=$(new_work_dir "$requested_work" "$source_dir") || fail 'work directory is not fresh and external'
    mkdir -p "$work_dir/evidence" "$work_dir/logs"

    cp -p "$seed" "$work_dir/tampered-seed"
    printf x >>"$work_dir/tampered-seed"
    if check_hash "$work_dir/tampered-seed" "$seed_sha" >"$work_dir/evidence/tampered-seed.log" 2>&1; then
        fail 'tampered seed was accepted'
    fi
    if check_static_elf /bin/true >"$work_dir/evidence/dynamic-seed.log" 2>&1; then
        fail 'dynamic seed was accepted'
    fi

    emit_plan >"$work_dir/evidence/dry-plan.txt"
    cmp -s "$work_dir/evidence/dry-plan.txt" "$tests/expected-plan.txt" || fail 'dry plan differs from the seed-to-S3 plan'
    check_trace_checker "$work_dir" || fail 'trace checker did not reject an injected exec'
    check_compiler_failure_modes "$work_dir" || fail 'budget wrapper self-tests failed'
    run_failed_isolation_probe "$work_dir" || fail 'failed isolation did not stop producer execution'
    run_network_probe "$work_dir" || fail 'network socket restriction was not effective'

    minimal_root=$(stage_inputs "$source_dir" "$revision" "$work_dir" "$seed" smoke) || fail 'deterministic source staging failed'
    printf 'rey bootstrap smoke\n' >"$work_dir/evidence/expected.stdout"
    run_stage "$work_dir" "$minimal_root" /src/smoke.rey 480 smoke \
        "$work_dir/evidence/expected.stdout" || fail 'isolated native smoke failed'
    cp -p "$minimal_root/out/S1" "$work_dir/S1"
    check_static_elf "$work_dir/S1" || fail 'smoke output is not a static Linux x64 ELF'
    [ -x "$work_dir/S1" ] || fail 'smoke output is not directly executable'
    sha256sum "$work_dir/S1" >"$work_dir/evidence/S1.sha256"
}

candidate() {
    seed=$1
    source_dir=$2
    revision=$3
    requested_work=$4
    source_dir=$(CDPATH= cd -- "$source_dir" && pwd -P) || fail "source directory is absent: $source_dir"
    source_revision "$source_dir" "$revision" || fail 'source revision is invalid'
    seed=$(readlink -f -- "$seed") || fail 'seed path is absent'
    check_seed "$seed" || fail 'pinned seed check failed'
    work_dir=$(new_work_dir "$requested_work" "$source_dir") || fail 'work directory is not fresh and external'

    minimal_root=$(stage_inputs "$source_dir" "$revision" "$work_dir" "$seed" candidate) || fail 'deterministic source staging failed'
    run_failed_isolation_probe "$work_dir" || fail 'failed isolation did not stop producer execution'
    run_network_probe "$work_dir" || fail 'network socket restriction was not effective'
    run_stage "$work_dir" "$minimal_root" /src/compiler/main.rey 2400 candidate \
        "$work_dir/evidence/unused-expected.stdout" || fail 'normal seed-to-S1 build failed'
    cp -p "$minimal_root/out/S1" "$work_dir/S1"
    check_static_elf "$work_dir/S1" || fail 'candidate output is not a static Linux x64 ELF'
    [ -x "$work_dir/S1" ] || fail 'candidate output is not directly executable'
    sha256sum "$work_dir/S1" >"$work_dir/evidence/S1.sha256"
    printf '%s\n' 'PASS candidate S1'
}

if [ "${1:-}" = --plan ]; then
    [ "$#" -eq 1 ] || { usage; exit 2; }
    emit_plan
    exit 0
fi

mode=${1:-}
case "$mode" in
    --self-test)
        shift
        exec "$budget" job --seconds 600 --label bootstrap-self-test -- "$root/scripts/v04-selfhost-linux-x64.sh" --locked-self-test "$@"
        ;;
    --candidate)
        shift
        exec "$budget" job --seconds 2400 --label bootstrap-candidate -- "$root/scripts/v04-selfhost-linux-x64.sh" --locked-candidate "$@"
        ;;
    --locked-self-test) mode=--self-test; shift ;;
    --locked-candidate) mode=--candidate; shift ;;
    *)
    if [ "$mode" = --verify ]; then
        echo 'full chain is unavailable until the §4 memory admission is cleared' >&2
        exit 2
    fi
    usage
    exit 2
    ;;
esac

seed=
source_dir=
revision=
work_dir=
while [ "$#" -gt 0 ]; do
    case "$1" in
        --seed) [ "$#" -ge 2 ] || fail 'missing --seed value'; seed=$2; shift 2 ;;
        --source) [ "$#" -ge 2 ] || fail 'missing --source value'; source_dir=$2; shift 2 ;;
        --revision) [ "$#" -ge 2 ] || fail 'missing --revision value'; revision=$2; shift 2 ;;
        --work) [ "$#" -ge 2 ] || fail 'missing --work value'; work_dir=$2; shift 2 ;;
        *) fail "unknown option: $1" ;;
    esac
done
[ -n "$seed" ] && [ -n "$source_dir" ] && [ -n "$work_dir" ] || { usage; exit 2; }
if [ "$mode" = --candidate ]; then
    [ -n "$revision" ] || fail '--revision is required in candidate mode'
    candidate "$seed" "$source_dir" "$revision" "$work_dir"
else
    [ -z "$revision" ] || fail '--revision is not accepted by self-test mode'
    self_test "$seed" "$source_dir" "$work_dir"
    printf '%s\n' 'PASS bootstrap harness'
fi
