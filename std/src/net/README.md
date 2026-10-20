# `std::net`

Network address helpers and a host reachability check. Import with
`import std::net;`.

## API

- `SocketAddr { host: string, port: int }` and `socketAddr(host, port)` create
  an address value. `formatAddr(addr)` formats `host:port`, adding brackets
  around colon-containing hosts such as IPv6 literals.
- `isLoopback(addr)` recognizes `localhost`, `::1`, `[::1]`, and IPv4 addresses
  in `127.0.0.0/8`.
- `isIpv4(host)` validates dotted decimal IPv4 syntax. `parseIpv4(host)` returns
  its four integer octets, or an empty vector for invalid input.
- `pingHost(host)` invokes the host `ping` command once (`-c 1` on Unix and
  `-n 1` on Windows) and returns whether the command exits successfully. It
  accepts only alphanumeric host labels plus `.`, `-`, `_`, and `:` to keep
  shell metacharacters out of the command.

`pingHost` requires the system `ping` executable and uses the platform shell
through `std::process`. It can be affected by DNS, host firewall policy, and
network availability. This module currently provides address utilities and
ping only; it does not provide TCP or UDP sockets.
