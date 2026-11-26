#!/bin/sh
set -eu
[ "$#" -eq 1 ] || exit 2
printf 'producer wrote an invalid image\n' >"$1"
exit 7
