#!/bin/sh
# Gate a v0.4 release directory. This script does not compile, link, or publish.
# scripts/build-release-target.sh remains the legacy 0.3.1 path.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
EVIDENCE=${1:-"$ROOT/build/v04/release-evidence"}
if [ ! -d "$EVIDENCE" ]; then
    echo "error: evidence directory does not exist: $EVIDENCE" >&2
    echo "expected one JSON file per platform plus release-notes.md" >&2
    exit 2
fi
python3 "$ROOT/scripts/v04-release-readiness.py" "$EVIDENCE"
echo "v0.4 package contract: rey-package-1"
echo "no archive was created; publication stays blocked until the command above exits 0 and a later publish step is requested"
