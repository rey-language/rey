#!/usr/bin/env bash

set -u

usage() {
  cat <<'USAGE'
Usage: scripts/bootstrap.sh [--help]

Builds the Rust oracle and isolated runtime, bootstraps native stage 1, builds
stage 2, runs the full conformance suite on stage 2, then verifies stage 2 can
rebuild the compiler. Success is reported only after every gate passes.

Environment:
  BOOTSTRAP_BUILD_PARENT            parent for a new unique build directory
  BOOTSTRAP_TIMEOUT_SECONDS         per-stage process-tree timeout (default: 300)
  BOOTSTRAP_TOTAL_TIMEOUT_SECONDS   optional wall-clock budget for the whole run
  BOOTSTRAP_CARGO                   cargo executable override (testing/diagnostics)
  BOOTSTRAP_ORACLE                  use this rey-v0 and skip the cargo rebuild

Bounded heavy-lock run, supplying the pinned oracle so stage 1 can use the budget:
  lockf -k <heavy.lock> timeout 1200 env \
    BOOTSTRAP_BUILD_PARENT=<scratch> \
    BOOTSTRAP_TIMEOUT_SECONDS=600 \
    BOOTSTRAP_TOTAL_TIMEOUT_SECONDS=1100 \
    BOOTSTRAP_ORACLE=<rey-v0> \
    scripts/bootstrap.sh
USAGE
}

case "$#" in
  0) ;;
  1)
    if [[ "$1" == --help || "$1" == -h ]]; then
      usage
      exit 0
    fi
    printf 'Unknown option: %s\n' "$1" >&2
    usage >&2
    exit 2
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P) || exit 2
repo_root=$(cd "$script_dir/.." && pwd -P) || exit 2
initial_cwd=$PWD
compiler_source_root=$(cd "$repo_root/rey-compiler" && pwd -P) || {
  printf 'bootstrap: FAIL stage=compiler source; directory is absent: %s\n' "$repo_root/rey-compiler" >&2
  exit 2
}
timeout_seconds=${BOOTSTRAP_TIMEOUT_SECONDS:-300}
if [[ ! "$timeout_seconds" =~ ^[1-9][0-9]*$ ]]; then
  printf 'BOOTSTRAP_TIMEOUT_SECONDS must be a positive integer.\n' >&2
  exit 2
fi
if [[ -n "${BOOTSTRAP_TOTAL_TIMEOUT_SECONDS:-}" ]] && [[ ! "${BOOTSTRAP_TOTAL_TIMEOUT_SECONDS}" =~ ^[1-9][0-9]*$ ]]; then
  printf 'BOOTSTRAP_TOTAL_TIMEOUT_SECONDS must be a positive integer.\n' >&2
  exit 2
fi
if [[ -x /usr/bin/clang ]]; then
  PATH="/usr/bin:${PATH}"
  export PATH
  CC=/usr/bin/clang
  export CC
fi

build_parent=${BOOTSTRAP_BUILD_PARENT:-$repo_root/build}
case "/$build_parent/" in
  */../*)
    printf 'bootstrap: FAIL stage=build directory; BOOTSTRAP_BUILD_PARENT must not contain .. components.\n' >&2
    exit 2
    ;;
esac

canonicalize_parent() {
  local path=$1
  local absolute probe tail component physical
  case "$path" in
    /*) absolute=$path ;;
    *) absolute="$initial_cwd/$path" ;;
  esac
  while [[ "$absolute" != / && "$absolute" == */ ]]; do
    absolute=${absolute%/}
  done
  probe=$absolute
  tail=
  while [[ ! -e "$probe" ]]; do
    if [[ "$probe" == / ]]; then break; fi
    component=${probe##*/}
    tail="$component${tail:+/$tail}"
    probe=${probe%/*}
    if [[ -z "$probe" ]]; then probe=/; fi
  done
  physical=$(cd "$probe" && pwd -P) || return 1
  if [[ -n "$tail" ]]; then
    printf '%s/%s' "$physical" "$tail"
  else
    printf '%s' "$physical"
  fi
}

path_is_inside() {
  local path=$1 parent=$2
  case "$path/" in
    "$parent/"*) return 0 ;;
    *) return 1 ;;
  esac
}

canonical_build_parent=$(canonicalize_parent "$build_parent") || {
  printf 'bootstrap: FAIL stage=build directory; cannot resolve %s\n' "$build_parent" >&2
  exit 1
}
if path_is_inside "$canonical_build_parent" "$compiler_source_root"; then
  printf 'bootstrap: FAIL stage=build directory; build parent overlaps compiler source: %s\n' "$canonical_build_parent" >&2
  exit 2
fi
if ! mkdir -p "$build_parent"; then
  printf 'bootstrap: FAIL stage=build directory; cannot create %s\n' "$build_parent" >&2
  exit 1
fi
build_parent=$(cd "$build_parent" && pwd -P) || exit 1
build_dir=$(mktemp -d "$build_parent/rey-bootstrap.XXXXXX") || {
  printf 'bootstrap: FAIL stage=build directory; mktemp failed under %s\n' "$build_parent" >&2
  exit 1
}
build_dir=$(cd "$build_dir" && pwd -P) || exit 1
if path_is_inside "$build_dir" "$compiler_source_root" || path_is_inside "$compiler_source_root" "$build_dir"; then
  printf 'bootstrap: FAIL stage=build directory; generated build directory overlaps compiler source: %s\n' "$build_dir" >&2
  exit 2
fi

logs_dir="$build_dir/logs"
cargo_target="$build_dir/rust-target"
runtime_dir="$build_dir/runtime"
compiler_source="$build_dir/compiler-source"
stage1_dir="$build_dir/stage1"
stage2_dir="$build_dir/stage2"
stage3_dir="$build_dir/stage3"
if ! mkdir -p "$logs_dir" "$stage1_dir" "$stage2_dir" "$stage3_dir"; then
  printf 'bootstrap: FAIL stage=build layout; cannot create directories under %s\n' "$build_dir" >&2
  exit 1
fi

cargo_bin=${BOOTSTRAP_CARGO:-cargo}
case "$cargo_bin" in
  */*)
    case "$cargo_bin" in
      /*) ;;
      *) cargo_bin="$initial_cwd/$cargo_bin" ;;
    esac
    ;;
esac
runtime_builder="$repo_root/scripts/build-runtime.sh"
compiler_main="$compiler_source/main.rey"

print_command() {
  local arg
  printf 'command:'
  for arg in "$@"; do
    printf ' %q' "$arg"
  done
  printf '\n'
}

terminate_tree() {
  local pid=$1
  local child
  for child in $(pgrep -P "$pid" 2>/dev/null); do
    terminate_tree "$child"
  done
  kill -KILL "$pid" 2>/dev/null || true
}

active_pid=
active_watchdog=
on_interrupt() {
  if [[ -n "${active_watchdog}" ]]; then
    kill -KILL "$active_watchdog" 2>/dev/null || true
  fi
  if [[ -n "${active_pid}" ]]; then
    terminate_tree "$active_pid"
  fi
  printf 'bootstrap: interrupted; NOT SELFHOST VERIFIED; build outputs retained at %s\n' "${build_dir:-unknown}" >&2
  exit 130
}
trap on_interrupt HUP INT TERM

run_stage() {
  local stage=$1
  local cwd=$2
  local key=$3
  shift 3
  local stdout_log="$logs_dir/$key.stdout.log"
  local stderr_log="$logs_dir/$key.stderr.log"
  local timeout_marker="$logs_dir/$key.timed-out"
  local pid watchdog status state arg result
  local stage_timeout=$timeout_seconds
  if [[ -n "${BOOTSTRAP_TOTAL_TIMEOUT_SECONDS:-}" ]]; then
    local remaining=$((BOOTSTRAP_TOTAL_TIMEOUT_SECONDS - SECONDS))
    if ((remaining < 1)); then
      printf 'bootstrap: FAIL stage=%s; total timeout of %s seconds is exhausted.\n' "$stage" "$BOOTSTRAP_TOTAL_TIMEOUT_SECONDS" >&2
      printf 'bootstrap: NOT SELFHOST VERIFIED; build outputs retained at %s\n' "$build_dir" >&2
      return 124
    fi
    if ((remaining < stage_timeout)); then
      stage_timeout=$remaining
    fi
  fi

  printf 'bootstrap: stage=%s cwd=%s\n' "$stage" "$cwd"
  print_command "$@"
  : > "$timeout_marker"
  (
    cd "$cwd" || exit 125
    "$@"
  ) > "$stdout_log" 2> "$stderr_log" &
  pid=$!
  active_pid=$pid
  (
    sleep "$stage_timeout"
    if kill -0 "$pid" 2>/dev/null; then
      state=$(ps -o stat= -p "$pid" 2>/dev/null | tr -d '[:space:]')
      case "$state" in
        Z*) exit 0 ;;
      esac
      printf 'timeout\n' > "$timeout_marker"
      terminate_tree "$pid"
    fi
  ) &
  watchdog=$!
  active_watchdog=$watchdog
  wait "$pid" 2>/dev/null
  status=$?
  active_pid=
  terminate_tree "$watchdog"
  active_watchdog=
  wait "$watchdog" 2>/dev/null || true

  if [[ -s "$stdout_log" ]]; then cat "$stdout_log"; fi
  if [[ -s "$stderr_log" ]]; then cat "$stderr_log" >&2; fi
  if [[ -s "$timeout_marker" ]]; then
    result=124
    printf 'bootstrap: FAIL stage=%s timed out after %s seconds (status 124).\n' "$stage" "$stage_timeout" >&2
  elif ((status != 0)); then
    result=$status
    printf 'bootstrap: FAIL stage=%s exited %s.\n' "$stage" "$status" >&2
  else
    return 0
  fi
  print_command "$@" >&2
  printf 'bootstrap: logs: %s and %s\n' "$stdout_log" "$stderr_log" >&2
  printf 'bootstrap: NOT SELFHOST VERIFIED; build outputs retained at %s\n' "$build_dir" >&2
  return "$result"
}

fail_artifact() {
  local stage=$1
  local artifact=$2
  local status=${3:-1}
  printf 'bootstrap: FAIL stage=%s; expected executable artifact is absent: %s\n' "$stage" "$artifact" >&2
  printf 'bootstrap: NOT SELFHOST VERIFIED; build outputs retained at %s\n' "$build_dir" >&2
  exit "$status"
}

if [[ ! -x "$runtime_builder" ]]; then
  printf 'bootstrap: FAIL stage=runtime archive; required helper is absent or not executable.\n' >&2
  print_command bash "$runtime_builder" --output-dir "$runtime_dir" >&2
  printf 'bootstrap: NOT SELFHOST VERIFIED; no fixtures were run.\n' >&2
  exit 127
fi

oracle_rebuilt=1
if [[ -n "${BOOTSTRAP_ORACLE:-}" ]]; then
  case "$BOOTSTRAP_ORACLE" in
    /*) oracle=$BOOTSTRAP_ORACLE ;;
    *) oracle="$initial_cwd/$BOOTSTRAP_ORACLE" ;;
  esac
  if [[ ! -x "$oracle" ]]; then
    fail_artifact 'Rust oracle' "$oracle" 127
  fi
  oracle_rebuilt=0
  printf 'bootstrap: stage=Rust oracle skipped; using supplied executable (not rebuilt from this tree): %s\n' "$oracle"
else
  run_stage 'Rust oracle' "$repo_root" rust-oracle \
    env "CARGO_TARGET_DIR=$cargo_target" "$cargo_bin" build \
    --manifest-path "$repo_root/compiler/v1/Cargo.toml" --release -j 1
  stage_status=$?
  if ((stage_status != 0)); then exit "$stage_status"; fi
  oracle="$cargo_target/release/rey-v0"
  if [[ ! -x "$oracle" ]]; then
    fail_artifact 'Rust oracle' "$oracle"
  fi
fi

run_stage 'Runtime archive' "$repo_root" runtime-archive \
  env CC=/usr/bin/clang AR=/usr/bin/ar MAKE=/usr/bin/make \
  "$runtime_builder" --output-dir "$runtime_dir"
stage_status=$?
if ((stage_status != 0)); then exit "$stage_status"; fi
if [[ ! -f "$runtime_dir/librey_rt.a" ]]; then
  fail_artifact 'Runtime archive' "$runtime_dir/librey_rt.a"
fi

# Verify the interpreter's agreed args() layout before invoking compiler source.
argv_probe="$build_dir/argv-probe.rey"
if ! cat > "$argv_probe" <<'REY'
func main() {
    var observed = args();
    if observed.len() != 3 { exit(81); }
    if observed[0].len() == 0 { exit(84); }
    if observed[1] != "bootstrap-argv-probe" { exit(82); }
    if observed[2] != "argument with spaces" { exit(83); }
    println("argv-contract-ok");
}
REY
then
  printf 'bootstrap: FAIL stage=Rust interpreter argv probe source creation.\n' >&2
  exit 1
fi
run_stage 'Rust interpreter argv contract' "$repo_root" argv-contract \
  "$oracle" run "$argv_probe" -- bootstrap-argv-probe 'argument with spaces'
stage_status=$?
if ((stage_status != 0)); then exit "$stage_status"; fi
if ! printf 'argv-contract-ok\n' > "$build_dir/argv-probe.expected"; then
  printf 'bootstrap: FAIL stage=Rust interpreter argv probe expectation creation.\n' >&2
  exit 1
fi
if ! cmp -s "$build_dir/argv-probe.expected" "$logs_dir/argv-contract.stdout.log" || [[ -s "$logs_dir/argv-contract.stderr.log" ]]; then
  printf 'bootstrap: FAIL stage=Rust interpreter argv contract; expected exactly argv-contract-ok plus LF on stdout and empty stderr.\n' >&2
  printf 'bootstrap: NOT SELFHOST VERIFIED; build outputs retained at %s\n' "$build_dir" >&2
  exit 1
fi

if ! mkdir -p "$compiler_source/src" || \
   ! cp "$compiler_source_root/main.rey" "$compiler_main" || \
   ! cp -R "$compiler_source_root/src/." "$compiler_source/src/"; then
  printf 'bootstrap: FAIL stage=isolated compiler source copy.\n' >&2
  printf 'bootstrap: NOT SELFHOST VERIFIED; build outputs retained at %s\n' "$build_dir" >&2
  exit 1
fi
if ! cmp -s "$compiler_source_root/main.rey" "$compiler_main" || \
   ! diff -qr "$compiler_source_root/src" "$compiler_source/src" >/dev/null; then
  printf 'bootstrap: FAIL stage=isolated compiler source copy; copy differs from source.\n' >&2
  printf 'bootstrap: NOT SELFHOST VERIFIED; build outputs retained at %s\n' "$build_dir" >&2
  exit 1
fi

stage1_bin="$stage1_dir/rey"
run_stage 'Stage 1 compiler build (Rust interpreter)' "$compiler_source" stage1-build \
  "$oracle" run "$compiler_main" -- build "$compiler_main" \
  -o "$stage1_bin" --runtime-dir "$runtime_dir"
stage_status=$?
if ((stage_status != 0)); then exit "$stage_status"; fi
if [[ ! -x "$stage1_bin" ]]; then
  fail_artifact 'Stage 1 compiler build' "$stage1_bin"
fi

stage2_bin="$stage2_dir/rey"
run_stage 'Stage 2 compiler build (stage 1)' "$compiler_source" stage2-build \
  "$stage1_bin" build "$compiler_main" -o "$stage2_bin" --runtime-dir "$runtime_dir"
stage_status=$?
if ((stage_status != 0)); then exit "$stage_status"; fi
if [[ ! -x "$stage2_bin" ]]; then
  fail_artifact 'Stage 2 compiler build' "$stage2_bin"
fi

conformance_runner="$repo_root/scripts/conformance.sh"
conformance_wrapper="$build_dir/stage2/rey-with-runtime"
if [[ ! -x "$conformance_runner" ]]; then
  printf 'bootstrap: FAIL stage=stage 2 conformance; runner is absent or not executable: %s\n' "$conformance_runner" >&2
  print_command "$conformance_runner" --engine "$conformance_wrapper" >&2
  exit 127
fi
if ! printf '#!/usr/bin/env bash\nexec %q "$@" --runtime-dir %q\n' "$stage2_bin" "$runtime_dir" > "$conformance_wrapper" || \
   ! chmod +x "$conformance_wrapper"; then
  printf 'bootstrap: FAIL stage=stage 2 conformance wrapper creation.\n' >&2
  exit 1
fi
if ! bash -n "$conformance_wrapper"; then
  printf 'bootstrap: FAIL stage=stage 2 conformance wrapper syntax.\n' >&2
  exit 1
fi
run_stage 'Stage 2 full conformance suite' "$repo_root" stage2-conformance \
  "$conformance_runner" --engine "$conformance_wrapper"
stage_status=$?
if ((stage_status != 0)); then exit "$stage_status"; fi

stage3_bin="$stage3_dir/rey"
run_stage 'Stage 2 rebuild capability' "$compiler_source" stage2-rebuild \
  "$stage2_bin" build "$compiler_main" -o "$stage3_bin" --runtime-dir "$runtime_dir"
stage_status=$?
if ((stage_status != 0)); then exit "$stage_status"; fi
if [[ ! -x "$stage3_bin" ]]; then
  fail_artifact 'Stage 2 rebuild capability' "$stage3_bin"
fi

if ((oracle_rebuilt)); then
  printf 'SELFHOST VERIFIED: stage 2 passed the full conformance suite and rebuilt the compiler. The Rust oracle was rebuilt from this tree.\n'
else
  printf 'SELFHOST VERIFIED: stage 2 passed the full conformance suite and rebuilt the compiler. The Rust oracle was supplied via BOOTSTRAP_ORACLE and was not rebuilt from this tree.\n'
fi
printf 'build directory: %s\nstage 1: %s\nstage 2: %s\nstage 3 rebuild: %s\nruntime archive: %s\n' \
  "$build_dir" "$stage1_bin" "$stage2_bin" "$stage3_bin" "$runtime_dir/librey_rt.a"
