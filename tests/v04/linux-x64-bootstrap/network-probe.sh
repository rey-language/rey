#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
    echo 'usage: network-probe.sh TRACE' >&2
    exit 2
fi
trace=$1

# AF_INET/AF_INET6 socket creation must be denied by the systemd seccomp rule.
# 192.0.2.1 is a documentation-only address; no packet can reach a real host.
set +e
/usr/bin/strace -f -qq -e trace=socket -o "$trace" -- \
    /usr/bin/curl --noproxy '*' --connect-timeout 1 --max-time 2 http://192.0.2.1:9/
curl_status=$?
set -e
[ "$curl_status" -ne 0 ] || { echo 'network probe unexpectedly connected' >&2; exit 1; }
grep -Eq 'socket\(AF_INET(6)?[^=]*= -1 (EPERM|EAFNOSUPPORT)' "$trace" || {
    echo 'network socket was not rejected by the active systemd filter' >&2
    exit 1
}
printf 'network socket blocked by address-family filter\n'
