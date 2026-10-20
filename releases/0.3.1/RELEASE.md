# Rey v0.3.1

Rey v0.3.1 is a patch release focused on lower peak memory use, restored native typechecking/code generation, and reproducible compiler/runtime packages across supported platforms.

## Highlights

- Vec capacity growth now uses `realloc` instead of allocating and zeroing a second full buffer. Only live kind metadata is initialized. In the current macOS arm64 comparison, large Vec workloads used 36–48% less peak RSS.
- Restored the native typechecker and checked-program backend required by the tracked compiler driver. See [the source recovery notes](../../docs/BOOTSTRAP-SOURCE-RECOVERY.md) for provenance and limitations.
- Release packages are rebuilt from tracked compiler and runtime sources. Every target rebuilds the runtime with matching platform tools; Windows uses Win32 file, directory, and process I/O rather than reusing incompatible LLVM bitcode from v0.3.0.
- Packages include the native compiler, `librey_rt.a`, and the current standard library source tree.

## Downloads

| Platform | Asset |
|---|---|
| macOS arm64 | `rey-v0.3.1-macos-arm64.tar.gz` |
| macOS x86-64 | `rey-v0.3.1-macos-x86_64.tar.gz` |
| Linux x86-64 | `rey-v0.3.1-linux-x86_64.tar.gz` |
| Linux aarch64 | `rey-v0.3.1-linux-aarch64.tar.gz` |
| Windows x86-64 | `rey-v0.3.1-windows-x86_64.zip` |

## Validation notes

All five hosted release package-build preflight targets passed. The tag-triggered release workflow repeats the builds and checks for all five assets before publishing. The release build does not run the full runtime or language conformance suites, and the memory comparison is limited to the noted macOS arm64 workloads.
