#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
    echo 'usage: stage-l01-runtime.sh NEW_STAGE_DIR' >&2
    exit 2
fi

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
RUNTIME_REVISION=3ee37f973a8b65e4b0b75500d9d0e64c2e3c8cec
git -C "$ROOT" cat-file -e "$RUNTIME_REVISION:runtime-rey/main.rey"
mkdir -- "$1"
git -C "$ROOT" archive "$RUNTIME_REVISION" runtime-rey | tar -xf - -C "$1"

# Use this path with candidate7 --runtime-root when building native-main.rey.
# The runtime supports checked single-image publication and predates the float unit.
printf '%s\n' "$(CDPATH= cd -- "$1/runtime-rey" && pwd -P)"
