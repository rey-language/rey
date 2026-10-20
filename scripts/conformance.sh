#!/usr/bin/env bash

set -u

usage() {
  cat <<'USAGE'
Usage: scripts/conformance.sh --engine "<cmd>" [--filter <substr>]

Runs tests/conformance/<area>/<case> fixtures. Engine commands are evaluated
by bash from each isolated case directory, with `run main.rey` or
`check main.rey` appended.
USAGE
}

engine_cmd=
filter=
while (($#)); do
  case "$1" in
    --engine)
      if (($# < 2)); then
        usage >&2
        exit 2
      fi
      engine_cmd=$2
      shift 2
      ;;
    --filter)
      if (($# < 2)); then
        usage >&2
        exit 2
      fi
      filter=$2
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'Unknown option: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$engine_cmd" ]]; then
  printf 'Missing required --engine command.\n' >&2
  usage >&2
  exit 2
fi

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P) || exit 2
repo_root=$(cd "$script_dir/.." && pwd -P) || exit 2
fixture_root="$repo_root/tests/conformance"
case_timeout=${CONFORMANCE_TIMEOUT_SECONDS:-30}
if [[ ! "$case_timeout" =~ ^[0-9]+$ || ! "$case_timeout" =~ [1-9] ]]; then
  printf 'CONFORMANCE_TIMEOUT_SECONDS must be a positive integer.\n' >&2
  exit 2
fi

# Resolve direct executable paths before changing to fixture directories. The
# shell-escaped form preserves spaces in a path passed as one --engine arg.
engine_path=
case "$engine_cmd" in
  /*)
    if [[ -e "$engine_cmd" ]]; then
      engine_path=$engine_cmd
    fi
    ;;
  *)
    if [[ -e "$repo_root/$engine_cmd" ]]; then
      engine_path="$repo_root/$engine_cmd"
    elif [[ -e "$engine_cmd" ]]; then
      engine_path="$PWD/$engine_cmd"
    fi
    ;;
esac
if [[ -n "$engine_path" ]]; then
  engine_dir=$(cd "$(dirname "$engine_path")" && pwd -P) || exit 2
  engine_cmd=$(printf '%q' "$engine_dir/$(basename "$engine_path")")
fi

scratch_root=${REY_CONFORMANCE_SCRATCH:-/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/conformance}
if ! mkdir -p "$scratch_root"; then
  printf 'Cannot create scratch directory: %s\n' "$scratch_root" >&2
  exit 2
fi

terminate_tree() {
  local pid=$1
  local child
  for child in $(pgrep -P "$pid" 2>/dev/null); do
    terminate_tree "$child"
  done
  kill -KILL "$pid" 2>/dev/null || true
}

command_result=
run_timed() {
  local cwd=$1
  local command_string=$2
  local stdout_file=$3
  local stderr_file=$4
  local marker=$5
  local pid watchdog status state remaining

  remaining=$((case_timeout - (SECONDS - case_started)))
  if ((remaining < 1)); then
    command_result=timeout
    return
  fi

  : > "$marker"
  (
    cd "$cwd" || exit 125
    bash -c "$command_string"
  ) > "$stdout_file" 2> "$stderr_file" &
  pid=$!
  (
    sleep "$remaining"
    if kill -0 "$pid" 2>/dev/null; then
      state=$(ps -o stat= -p "$pid" 2>/dev/null | tr -d '[:space:]')
      case "$state" in
        Z*) exit 0 ;;
      esac
      printf 'timeout\n' > "$marker"
      terminate_tree "$pid"
    fi
  ) &
  watchdog=$!
  wait "$pid" 2>/dev/null
  status=$?
  kill "$watchdog" 2>/dev/null || true
  wait "$watchdog" 2>/dev/null || true
  if [[ -s "$marker" ]]; then
    command_result=timeout
  else
    command_result=$status
  fi
}

# Reject engines that do not implement the v0.3 check contract before fixtures
# are counted. A legacy CLI can otherwise turn a usage error into a false pass.
preflight_dir=$(mktemp -d "$scratch_root/preflight.XXXXXX") || {
  printf 'Engine preflight FAIL: cannot create isolated scratch directory.\n' >&2
  exit 1
}
printf 'var preflightValue: int = 1;\n' > "$preflight_dir/main.rey"
preflight_stdout="$preflight_dir/stdout"
preflight_stderr="$preflight_dir/stderr"
preflight_marker="$preflight_dir/timeout"
case_started=$SECONDS
run_timed "$preflight_dir" "$engine_cmd check main.rey" "$preflight_stdout" "$preflight_stderr" "$preflight_marker"
preflight_result=$command_result
if [[ "$preflight_result" == timeout ]]; then
  printf 'Engine preflight FAIL: check timed out after %s seconds.\n' "$case_timeout" >&2
elif [[ "$preflight_result" != 0 ]]; then
  printf 'Engine preflight FAIL: check must exit 0 on a valid program; got %s.\n' "$preflight_result" >&2
elif [[ -s "$preflight_stdout" || -s "$preflight_stderr" ]]; then
  printf 'Engine preflight FAIL: successful check must be silent.\n' >&2
else
  printf 'Engine preflight PASS\n'
fi
if [[ "$preflight_result" == timeout || "$preflight_result" != 0 || -s "$preflight_stdout" || -s "$preflight_stderr" ]]; then
  if [[ -s "$preflight_stdout" ]]; then
    printf 'check stdout:\n' >&2
    cat "$preflight_stdout" >&2
  fi
  if [[ -s "$preflight_stderr" ]]; then
    printf 'check stderr:\n' >&2
    cat "$preflight_stderr" >&2
  fi
  printf 'No fixture cases were counted or run.\n' >&2
  exit 1
fi

case_count=0
pass_count=0
fail_count=0
matched_count=0
failure_cases=()
failure_details=()

while IFS= read -r case_dir; do
  case_id=${case_dir#"$fixture_root"/}
  area=${case_id%%/*}
  case_name=${case_id#*/}
  if [[ "$area" == _* && "$area" != _smoke ]] || [[ "$case_name" == _* ]]; then
    continue
  fi
  if [[ -n "$filter" && "$case_id" != *"$filter"* ]]; then
    continue
  fi

  matched_count=$((matched_count + 1))
  case_count=$((case_count + 1))
  case_started=$SECONDS
  case_failed=0
  work_dir=$(mktemp -d "$scratch_root/case.XXXXXX") || {
    printf 'FAIL %s (could not create isolated work directory)\n' "$case_id"
    fail_count=$((fail_count + 1))
    failure_cases+=("$case_id")
    failure_details+=("scratch directory creation failed under $scratch_root")
    continue
  }
  detail_file="$work_dir/failure-details"
  : > "$detail_file"

  if [[ ! -f "$case_dir/main.rey" ]]; then
    printf '  missing required file: main.rey\n' >> "$detail_file"
    case_failed=1
  fi
  if [[ ! -f "$case_dir/expected.stdout" ]]; then
    printf '  missing required file: expected.stdout\n' >> "$detail_file"
    case_failed=1
  fi
  if ((case_failed)); then
    printf 'FAIL %s\n' "$case_id"
  else
    if ! cp -R "$case_dir/." "$work_dir/"; then
      printf '  could not copy fixture into isolated work directory\n' >> "$detail_file"
      case_failed=1
      printf 'FAIL %s\n' "$case_id"
    else
      stdout_file="$work_dir/.conformance.stdout"
      stderr_file="$work_dir/.conformance.stderr"
      marker="$work_dir/.conformance.timeout"
      check_command="$engine_cmd check main.rey"

      if [[ -f "$case_dir/expect_check_fail" ]]; then
        expected_diagnostic=$(cat "$case_dir/expect_check_fail")
        if [[ -z "$expected_diagnostic" ]]; then
          printf '  expect_check_fail must contain a non-empty substring\n' >> "$detail_file"
          case_failed=1
        else
          run_timed "$work_dir" "$check_command" "$stdout_file" "$stderr_file" "$marker"
          check_result=$command_result
          if [[ "$check_result" == timeout ]]; then
            printf '  check timed out after %s seconds\n' "$case_timeout" >> "$detail_file"
            case_failed=1
          elif [[ "$check_result" != 1 ]]; then
            printf '  expected compile-error check exit 1, got %s\n' "$check_result" >> "$detail_file"
            case_failed=1
          else
            combined_file="$work_dir/.conformance.combined"
            cat "$stdout_file" "$stderr_file" > "$combined_file"
            combined_output=$(cat "$combined_file")
            case "$combined_output" in
              *"$expected_diagnostic"*) ;;
              *)
                printf '  check failed with exit %s, but diagnostics did not contain expected substring: %s\n' "$check_result" "$expected_diagnostic" >> "$detail_file"
                printf '  combined check output:\n' >> "$detail_file"
                cat "$combined_file" >> "$detail_file"
                case_failed=1
                ;;
            esac
          fi
        fi
      else
        run_timed "$work_dir" "$check_command" "$stdout_file" "$stderr_file" "$marker"
        check_result=$command_result
        if [[ "$check_result" == timeout ]]; then
          printf '  check timed out after %s seconds\n' "$case_timeout" >> "$detail_file"
          case_failed=1
        elif [[ "$check_result" != 0 ]]; then
          printf '  check expected exit 0, got %s\n' "$check_result" >> "$detail_file"
          printf '  check stdout:\n' >> "$detail_file"
          cat "$stdout_file" >> "$detail_file"
          printf '  check stderr:\n' >> "$detail_file"
          cat "$stderr_file" >> "$detail_file"
          case_failed=1
        elif [[ -s "$stdout_file" || -s "$stderr_file" ]]; then
          printf '  successful check must be silent\n' >> "$detail_file"
          printf '  check stdout:\n' >> "$detail_file"
          cat "$stdout_file" >> "$detail_file"
          printf '  check stderr:\n' >> "$detail_file"
          cat "$stderr_file" >> "$detail_file"
          case_failed=1
        else
          run_command="$engine_cmd run main.rey"
          run_timed "$work_dir" "$run_command" "$stdout_file" "$stderr_file" "$marker"
          run_result=$command_result

          expected_exit=0
          if [[ -f "$case_dir/expected.exit" ]]; then
            expected_exit=$(cat "$case_dir/expected.exit")
            expected_exit=${expected_exit//$'\n'/}
          fi
          if [[ ! "$expected_exit" =~ ^[0-9]+$ ]] || ((10#$expected_exit > 255)); then
            printf '  expected.exit must contain an integer from 0 to 255, got: %s\n' "$expected_exit" >> "$detail_file"
            case_failed=1
          elif [[ "$run_result" == timeout ]]; then
            printf '  run timed out after %s seconds (expected exit %s)\n' "$case_timeout" "$expected_exit" >> "$detail_file"
            case_failed=1
          elif [[ "$run_result" != "$((10#$expected_exit))" ]]; then
            printf '  expected run exit %s, got %s\n' "$((10#$expected_exit))" "$run_result" >> "$detail_file"
            case_failed=1
          fi

          if [[ "$run_result" != timeout ]] && ! cmp -s "$case_dir/expected.stdout" "$stdout_file"; then
            expected_bytes=$(wc -c < "$case_dir/expected.stdout" | tr -d '[:space:]')
            got_bytes=$(wc -c < "$stdout_file" | tr -d '[:space:]')
            printf '  stdout differs byte-for-byte (expected %s bytes, got %s bytes)\n' "$expected_bytes" "$got_bytes" >> "$detail_file"
            printf '  stdout diff:\n' >> "$detail_file"
            diff -u "$case_dir/expected.stdout" "$stdout_file" >> "$detail_file" 2>&1 || true
            case_failed=1
          fi
          if [[ "$run_result" =~ ^[0-9]+$ ]] && ((run_result != 0)); then
            printf '  run stderr:\n' >> "$detail_file"
            cat "$stderr_file" >> "$detail_file"
          fi
        fi
      fi
      if ((case_failed)); then
        printf 'FAIL %s\n' "$case_id"
      else
        printf 'PASS %s\n' "$case_id"
      fi
    fi
  fi

  if ((case_failed)); then
    fail_count=$((fail_count + 1))
    failure_cases+=("$case_id")
    failure_details+=("$detail_file")
  else
    pass_count=$((pass_count + 1))
  fi
done < <(find "$fixture_root" -mindepth 2 -maxdepth 2 -type d -print 2>/dev/null | sort)

if ((matched_count == 0)); then
  printf 'No conformance cases matched under %s%s; treating this as failure.\n' "$fixture_root" "$([[ -n "$filter" ]] && printf ' (filter: %s)' "$filter")" >&2
  printf 'Summary: no cases run (empty selection is a failure).\n'
  exit 1
fi
printf 'Summary: %s passed, %s failed, %s total\n' "$pass_count" "$fail_count" "$case_count"
if ((fail_count)); then
  printf 'Failure details:\n'
  for index in "${!failure_cases[@]}"; do
    printf '%s\n' "${failure_cases[$index]}"
    if [[ -f "${failure_details[$index]}" ]]; then
      cat "${failure_details[$index]}"
    else
      printf '  %s\n' "${failure_details[$index]}"
    fi
  done
  exit 1
fi
exit 0
