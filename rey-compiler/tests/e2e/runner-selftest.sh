#!/usr/bin/env bash
set -euo pipefail

test_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
runner="$test_dir/run.sh"
[[ -n "${TMPDIR:-}" && -d "$TMPDIR" ]] || { echo "selftest: TMPDIR must be set" >&2; exit 2; }
repo_root="$(cd "$test_dir/../../.." && pwd)"
temp_root="$(cd "$TMPDIR" && pwd)"
case "$temp_root/" in
  "$repo_root/"*) echo "selftest: TMPDIR must be outside the repository" >&2; exit 2 ;;
esac
scratch="$(mktemp -d "${temp_root%/}/rey runner selftest.XXXXXX")"
cleanup() {
  rm -rf "$scratch"
}
trap cleanup EXIT

runtime_dir="$scratch/runtime archive path"
temp_dir="$scratch/runner temp path"
caller_dir="$scratch/unrelated caller cwd"
engine="$scratch/fake engine"
mkdir -p "$runtime_dir" "$temp_dir" "$caller_dir"
: > "$runtime_dir/librey_rt.a"

cat > "$engine" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail

hang_if_requested() {
  local command_name=$1
  [[ "${FAKE_HANG_COMMAND:-}" == "$command_name" ]] || return 0
  (
    trap '' TERM
    sleep 60 &
    grandchild_pid=$!
    if [[ -n "${FAKE_CHILD_PID_FILE:-}" ]]; then
      printf '%s\n' "$grandchild_pid" > "$FAKE_CHILD_PID_FILE.grandchild"
    fi
    wait "$grandchild_pid" || true
  ) &
  child_pid=$!
  if [[ -n "${FAKE_CHILD_PID_FILE:-}" ]]; then
    printf '%s\n' "$child_pid" > "$FAKE_CHILD_PID_FILE"
  fi
  trap '' TERM
  while :; do sleep 60; done
}

if [[ "${1:-}" == "--version" ]]; then
  hang_if_requested version
  printf 'rey 0.3.1\n'
  exit 0
fi

command_name=${1:-}
source_file=${2:-}
shift 2 || true
output_path=""
runtime_dir=""
while (($#)); do
  case "$1" in
    -o|--output)
      (($# >= 2)) || exit 2
      output_path=$2
      shift 2
      ;;
    --runtime-dir)
      (($# >= 2)) || exit 2
      runtime_dir=$2
      shift 2
      ;;
    *)
      echo "fake engine: unknown arg $1" >&2
      exit 2
      ;;
    esac
done
hang_if_requested "$command_name"

if [[ "${FAKE_FAIL_COMMAND:-}" == "$command_name" ]]; then
  echo "fake engine failure for $command_name" >&2
  exit "${FAKE_FAIL_STATUS:-17}"
fi

if [[ "$command_name" == "check" ]]; then
  exit 0
fi
if [[ "$command_name" != "build" && "$command_name" != "run" ]]; then
  echo "fake engine: unsupported command $command_name" >&2
  exit 2
fi
[[ -n "$source_file" && -n "$output_path" ]] || exit 2
[[ -f "$runtime_dir/librey_rt.a" ]] || exit 2
printf '#!/bin/sh\nexit 0\n' > "$output_path"
chmod +x "$output_path"
if [[ "$command_name" == "build" ]]; then
  printf '%s\n' "$output_path"
else
  golden="${source_file%.rey}.out"
  [[ -f "$golden" ]] || exit 2
  cat "$golden"
fi
FAKE
chmod +x "$engine"

set +e
(cd "$caller_dir" && TMPDIR="$temp_dir" bash "$runner" --engine "$engine" --runtime-dir "$runtime_dir") > "$scratch/success.stdout" 2> "$scratch/success.stderr"
success_status=$?
set -e
if ((success_status != 0)); then
  echo "selftest: fake engine success run exited $success_status" >&2
  cat "$scratch/success.stdout" "$scratch/success.stderr" >&2
  exit 1
fi
grep -Fx 'e2e passed: 11 fixtures' "$scratch/success.stdout" >/dev/null
[[ ! -s "$scratch/success.stderr" ]] || { cat "$scratch/success.stderr" >&2; exit 1; }

set +e
(cd "$caller_dir" && TMPDIR="$temp_dir" FAKE_FAIL_COMMAND=run FAKE_FAIL_STATUS=17 bash "$runner" --engine "$engine" --runtime-dir "$runtime_dir") > "$scratch/failure.stdout" 2> "$scratch/failure.stderr"
failure_status=$?
set -e
if ((failure_status == 0)); then
  echo "selftest: runner hid a fake engine failure" >&2
  exit 1
fi
grep -F 'hello-run exited 17' "$scratch/failure.stderr" >/dev/null

assert_timeout_cleanup() {
  local command_name=$1
  local label=$2
  local expected_status=$3
  local child_pid_file="$scratch/$command_name.child.pid"
  local stdout_file="$scratch/$command_name-timeout.stdout"
  local stderr_file="$scratch/$command_name-timeout.stderr"
  local status child_pid child_state pid_file

  set +e
  (cd "$caller_dir" && TMPDIR="$temp_dir" REY_E2E_TIMEOUT_SECONDS=1 REY_E2E_KILL_GRACE_SECONDS=1 \
    FAKE_HANG_COMMAND="$command_name" FAKE_CHILD_PID_FILE="$child_pid_file" \
    bash "$runner" --engine "$engine" --runtime-dir "$runtime_dir") > "$stdout_file" 2> "$stderr_file"
  status=$?
  set -e
  if ((status != expected_status)); then
    echo "selftest: $command_name hang returned $status (expected $expected_status)" >&2
    cat "$stdout_file" "$stderr_file" >&2
    exit 1
  fi
  grep -F -- "$label timed out after 1s" "$stderr_file" >/dev/null
  [[ -s "$child_pid_file" ]] || { echo "selftest: $command_name spawned no recorded child" >&2; exit 1; }
  [[ -s "$child_pid_file.grandchild" ]] || { echo "selftest: $command_name spawned no recorded grandchild" >&2; exit 1; }
  for pid_file in "$child_pid_file" "$child_pid_file.grandchild"; do
    child_pid=$(cat "$pid_file")
    child_state="$(ps -p "$child_pid" -o stat= 2>/dev/null | tr -d '[:space:]' || true)"
    if [[ -n "$child_state" && "$child_state" != Z* ]]; then
      echo "selftest: live descendant survived $command_name timeout (pid $child_pid, state $child_state)" >&2
      exit 1
    fi
  done
}

assert_timeout_cleanup version '--version' 2
assert_timeout_cleanup check hello-check 1
assert_timeout_cleanup run hello-run 1

echo 'runner selftest passed (fake engine; no language behavior asserted)'
