#!/usr/bin/env bash
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
rey="$root/compiler/v1/target/release/rey-v0"
compiler="$root/rey-compiler/main.rey"
dir="$root/rey-compiler/tests/e2e"

tests=(
  hello
  math
  loops
  functions
  structs
  enums
  imports
  strings
  vec
  collections
  io
)

pushd "$dir" >/dev/null

for t in "${tests[@]}"; do
  src="$dir/$t.rey"
  expected="$dir/$t.out"
  out="$dir/$t"
  if [[ -e "$out" ]]; then
    out="$out.bin"
  fi

  "$rey" "$compiler" build "$src" >/dev/null
  if [[ ! -x "$out" ]]; then
    echo "missing output binary: $out" >&2
    exit 1
  fi

  got="$("$out")"
  if ! diff -u <(printf "%s\n" "$got") "$expected" >/dev/null; then
    echo "FAIL: $t" >&2
    diff -u "$expected" <(printf "%s\n" "$got") >&2 || true
    exit 1
  fi

  rm -f "$out"
done

echo "OK"

popd >/dev/null
