# v0.3.1 compiler source recovery

The tracked release driver imports `SourceGraph`, `CheckedProgram`, and
`typecheckGraph`, then calls `codegen.generateIRChecked`. At `4a1c5f1`, the
tracked typechecker was a 133-line bootstrap stub and codegen was the older
2,100-line implementation. Neither provided that interface. This is a source
tree inconsistency, rather than evidence that v0.3.0 cannot parse v0.3.1.

## Recovery basis

No complete implementation was found in the path histories of the local
branches, stashes, or unreachable commits. Two retained bootstrap directories
contained isolated compiler-source copies:

- `build/rey-bootstrap.gpeD8l/compiler-source`
- `build/rey-bootstrap.3atk9E/compiler-source`

`scripts/bootstrap.sh` copies the compiler source and checks the copy against
the source before building. These copies match the tracked driver except for
the v0.3.1 version string. Their lexer and parser AST differ from the tracked
files only in whitespace, and their parser implementation is identical. Both
copies have the same 4,295-line checker, including all checked-program and
nominal-layout metadata consumed by the driver and backend. Their codegen
implementations differ only by the later Vec loop and release LTO changes.

The source pair from `3atk9E` was recovered together, rather than adding dummy
exports or bypassing typechecking. The snapshot SHA-256 values before changes
are:

| Module | SHA-256 |
|---|---|
| typecheck/main.rey | `f699662c2812ef482fb29b77031c92aa6919d2cb5a5d82b4bba7d7f9e4095b42` |
| codegen/main.rey | `0fe69449b4fb97de561cf8612961397dc69170c3b5ddba991202133749af824e` |

The recovered checker is unchanged. Codegen retains the recovered LTO and loop
optimizations, replaces the fixed Apple target with host detection, and resolves
`clang` through PATH instead of hardcoding `/usr/bin/clang`. Host detection uses
supported `exec` calls; `getEnv` could be typechecked but could not be lowered
by the v0.3.0 native seed. Literal LLVM braces are escaped in the recovered
implementation. No other compiler modules or the existing Vec realloc edit
were replaced.

This is a recovery of locally retained source, not a restoration from an
authoritative published source commit. The large diff reflects two missing
implementation modules. Build evidence establishes compatibility of the source
pair; it does not establish that every language behavior is correct.

## Release bootstrap

The native v0.3.0 seed contains absolute `/usr/bin/clang` command strings, so
PATH shims cannot reliably route it through the Linux and Windows linker
wrappers. The release builder instead:

1. Builds the reference interpreter from tracked `compiler/v1` with Cargo.
2. Runs the current compiler source through that interpreter to build stage 1.
3. Uses stage 1 to rebuild the current source as the packaged native compiler.
4. Requires the packaged compiler to report exactly `rey 0.3.1`.

Every target rebuilds its runtime archive from tracked C sources with a
matching platform toolchain. Windows selects `io_windows.c`, which provides
Win32 file, directory, and process APIs; Unix uses the POSIX runtime source.
Linux uses Zig for musl compilation and archive indexing, macOS uses its
native toolchain, and Windows uses matching MSYS2 Clang/LLVM tools. This avoids
the LLVM-version mismatch in the v0.3.0 bitcode archives. Only the native
compiler, rebuilt runtime archive, and current standard library enter the
package. Build intermediates and the Rust bootstrap remain outside it.

The five release targets are unchanged. The non-macOS-arm64 targets require
their native CI runners; retaining their workflow legs does not establish that
they build successfully. The Windows runtime source and process-capture path
must pass the Windows release matrix before publication. The builder does not
run the complete runtime or language conformance suites.

No test suites are run by this release builder. The normal `bootstrap.sh`
workflow includes conformance tests; historical snapshot logs are not new
verification.
