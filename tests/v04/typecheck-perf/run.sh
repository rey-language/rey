#!/bin/sh
set -eu
# Typecheck the compiler import graph and the lambda-capture fixtures.
#
# The pinned v0.3.1 seed builds the frontend driver from this tree. That
# driver runs checkSourceFileGraph, which is the typecheck step native
# driver H performs before lowering. /usr/bin/time -l prints the typecheck
# wall time and peak RSS. A successful check is silent.
#
# H's darwin-arm64 --regions --managed self-compile is not launched here.
# After typechecking, that command enters ir/lower (findExprInfo and
# managedTypeKey) and keeps allocating. Run it only under /tmp/memcap.sh.
PATH=/usr/bin:/bin:/usr/sbin:/sbin
CC=/usr/bin/clang
CXX=/usr/bin/clang++
export PATH CC CXX
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
ENGINE="$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME="$(dirname -- "$ENGINE")/runtime"
OUT="$ROOT/build/v04/typecheck-perf"
mkdir -p "$OUT"
STAGE=$(mktemp -d "$OUT/stage.XXXXXX")
mkdir -p "$STAGE/compiler"
cp -R "$ROOT/rey-compiler/src/." "$STAGE/compiler/src/"
cp "$ROOT/rey-compiler/frontend-main.rey" "$STAGE/compiler/frontend-main.rey"
cp "$ROOT/rey-compiler/native-main.rey" "$STAGE/compiler/native-main.rey"
test -x "$ENGINE"
test -d "$RUNTIME"
"$ENGINE" build "$STAGE/compiler/frontend-main.rey" -o "$STAGE/frontend" --runtime-dir "$RUNTIME"
test -x "$STAGE/frontend"

: > "$STAGE/check.stdout"
set +e
/usr/bin/time -l "$STAGE/frontend" check "$STAGE/compiler/native-main.rey" >"$STAGE/check.stdout" 2>"$STAGE/check.time"
check_status=$?
set -e
awk '
  / real / && wall == "" { wall = $1 }
  /maximum resident set size/ && rss == "" { rss = $1 }
  END {
    if (wall == "") wall = "unknown"
    if (rss == "") rss = "unknown"
    printf "wall_seconds=%s\npeak_rss_bytes=%s\n", wall, rss
  }
' "$STAGE/check.time"
printf 'typecheck_exit=%s\n' "$check_status"
if [ -s "$STAGE/check.stdout" ]; then
  printf '---- typecheck stdout ----\n'
  cat "$STAGE/check.stdout"
fi

check_fixture() {
  name=$1
  path=$2
  expect=$3
  set +e
  "$STAGE/frontend" check "$path" >"$STAGE/$name.stdout" 2>"$STAGE/$name.stderr"
  fixture_status=$?
  set -e
  printf 'fixture=%s exit=%s expected=%s\n' "$name" "$fixture_status" "$expect"
  if [ "$fixture_status" != "$expect" ]; then
    cat "$STAGE/$name.stdout" "$STAGE/$name.stderr"
    return 1
  fi
  return 0
}

check_fixture lambda-return-snapshot "$ROOT/tests/conformance/funcs/lambda-return-snapshot/main.rey" 0
check_fixture capture-scalar-snapshot "$ROOT/tests/conformance/robustness/capture-scalar-snapshot/main.rey" 0
check_fixture capture-aggregate-shared "$ROOT/tests/conformance/robustness/capture-aggregate-shared/main.rey" 0
check_fixture negative-mutable-capture "$ROOT/tests/conformance/funcs/negative-mutable-capture/main.rey" 1
if ! grep -q "mutable capture" "$STAGE/negative-mutable-capture.stderr"; then
  printf 'negative-mutable-capture did not report mutable capture\n'
  cat "$STAGE/negative-mutable-capture.stderr"
  exit 1
fi
printf 'stage=%s\n' "$STAGE"
exit "$check_status"
