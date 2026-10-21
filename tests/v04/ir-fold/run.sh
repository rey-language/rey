#!/bin/sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)"
exec python3 "$ROOT/tests/v04/ir-fold/run.py" "$@"
