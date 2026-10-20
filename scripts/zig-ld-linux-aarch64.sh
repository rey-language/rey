#!/usr/bin/env bash
args=()
for arg in "$@"; do
    if [[ "$arg" == *"--fix-cortex-a53-843419"* ]]; then
        continue
    fi
    args+=("$arg")
done
exec zig cc -target aarch64-linux-musl "${args[@]}"
