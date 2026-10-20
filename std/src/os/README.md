# `std::os`

Host operating-system and machine details for programs that need to adapt to
their current environment.

```rey
import std::os;

println(os.name());
println(os.arch());
println(os.tempDir());
```

## Functions

- `name(): string` returns `macos`, `linux`, `windows`, or `unknown`.
- `arch(): string` returns a normalized architecture where recognized
  (`arm64`, `x86_64`, or `x86`) and otherwise returns the host's reported value.
- `eol(): string` returns CRLF on Windows and LF on other hosts.
- `hostname(): string` returns the system hostname, or an empty string when the
  host command and environment fallbacks provide no value.
- `homedir(): string` returns `HOME`, `USERPROFILE`, or the Windows home-drive
  and path combination, or an empty string when none is available.
- `tempDir(): string` returns the runtime's host temporary directory.

OS name and architecture detection use `uname` on POSIX hosts and standard
environment variables on Windows. Hostname detection invokes `hostname` first.
These command-backed functions require the corresponding command to be
available through the host shell.
