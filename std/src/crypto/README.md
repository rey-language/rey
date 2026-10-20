# `std::crypto`

Small hash helpers for checksums and identifiers, plus host-backed cryptographic
hashing and random bytes.

```rey
import std::crypto;

println(crypto.sha256("hello"));
println(crypto.toHex(crypto.randomBytes(16)));
```

## Functions

- `fnv1a(data: string): int` and `djb2(data: string): int` return 32-bit
  non-cryptographic hashes. They hash the UTF-8 encoding of the input and are
  not suitable for security-sensitive use.
- `md5(data: string): string` and `sha256(data: string): string` return
  lowercase hexadecimal digests, or an empty string if no supported command
  succeeds. Data is passed through a shell-quoted `printf` pipeline.
- `randomBytes(n: int): Vec<int>` returns `n` cryptographically random bytes in
  the range 0–255. Non-positive lengths return an empty vector. It tries
  `openssl rand` first, then the POSIX `/dev/urandom` and `od` tools; if neither
  source is available or produces enough data, it returns an empty vector.
- `toHex(bytes: Vec<int>): string` encodes each value's low eight bits as two
  lowercase hexadecimal digits.

The MD5 and SHA-256 helpers try the standard macOS/Linux command variants
(`md5`/`md5sum` and `shasum`/`sha256sum`). These APIs depend on the host shell and
installed commands, and their output should be checked for an empty string when
the command may be unavailable. MD5 is included for compatibility and checksums,
not for security-sensitive applications.
