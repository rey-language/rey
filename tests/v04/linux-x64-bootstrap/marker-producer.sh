#!/bin/sh
set -eu
[ "$#" -eq 1 ] || exit 2
: >"$1"
