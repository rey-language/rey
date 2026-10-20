#!/usr/bin/env bash
set -u

real_clang=${REY_RELEASE_REAL_CLANG:-}
if [[ -z "$real_clang" ]]; then
    echo "release clang shim: REY_RELEASE_REAL_CLANG is not set" >&2
    exit 2
fi

args=("$@")
is_ir_input=false
input_index=-1
for ((i = 0; i < ${#args[@]}; i++)); do
    if [[ ${args[$i]} == "-x" && $((i + 1)) -lt ${#args[@]} && ${args[$((i + 1))]} == "ir" ]]; then
        is_ir_input=true
    fi
    if [[ ${args[$i]} == *.ll && -f ${args[$i]} ]]; then
        input_index=$i
    fi
done

if [[ $is_ir_input != true || $input_index -lt 0 ]]; then
    exec "$real_clang" "${args[@]}"
fi

tmp_ir=$(mktemp "${TMPDIR:-/tmp}/rey-release-ir.XXXXXX") || exit 2
if ! sed -E '/^[[:space:]]*target (datalayout|triple) = /d' "${args[$input_index]}" > "$tmp_ir"; then
    rm -f "$tmp_ir"
    exit 2
fi
args[$input_index]=$tmp_ir
"$real_clang" "${args[@]}"
status=$?
rm -f "$tmp_ir"
exit "$status"
