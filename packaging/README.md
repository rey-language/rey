# Rey v0.4 package layout

`rey-package-1` is the compiler/runtime package contract for v0.4.

```text
<toolchain>/
├── package.toml
├── bin/rey
├── bin/runtime/main.rey
│   or bin/runtime/runtime-rey/main.rey
└── std/src/
```

`package.toml` must contain `contract = "rey-package-1"`. The runtime entry is
Rey source. A C archive named `librey_rt.a` is the legacy 0.3 contract and is
not a substitute for this layout.

Capy accepts either contract. The published Capy 0.1.1 pin remains the legacy
layout until a release-readiness gate says otherwise. Clang is a legacy
requirement. v0.4 still depends on the platform loader: dyld and libSystem on
macOS, the Linux kernel for the direct exit syscall, and kernel32.dll for the
Windows `ExitProcess` import.

The five release platforms are `darwin-arm64`, `darwin-x86_64`,
`linux-aarch64`, `linux-x86_64`, and `windows-x86_64`. Cross-compiled images
are not execution evidence. `scripts/v04-release-readiness.py` refuses a
platform whose evidence file does not record passing build, run, bootstrap,
and package gates.
