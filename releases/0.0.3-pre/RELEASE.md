# rey v0.0.3-pre
Date: March 17, 2026

## What's new
- Lexer now supports `//` line comments and skips them cleanly.
- Parser comparison support was hardened for `<=` and `>=`.
- Test fixtures in `compiler/v1/src/tests/` were normalized to current supported syntax/runtime behavior.

## What's fixed
- Removed compiler warnings across the v1 codebase (`cargo build` now runs cleanly).
- Fixed a parser crash path when lexing failed (no more out-of-bounds panic on partial token streams).
- Aligned token handling across parser/executor for `!=`, `!`, `&&`, and `||`.
- Verified every existing `.rey` file in `compiler/v1/src/tests/` executes without lexer/parser/runtime errors.

## Known limitations
- Arrays (`[1, 2, 3]`)
- Dictionaries (`{key: value}`)
- Index access (`arr[i]`, `dict["key"]`)
- `input()` builtin
- String methods (`.length()`)
- Property access (`obj.prop`)
- Type enforcement at compile time (parsed, not enforced)

## How to install and run
1. Use the binaries in this folder:
- `rey-v0-macos-arm64`
- `rey-v0-windows-x86_64.exe`

2. Run a Rey file:
- macOS arm64:
  `./rey-v0-macos-arm64 ../../compiler/v1/src/tests/variables.rey`
- Windows x86_64:
  `rey-v0-windows-x86_64.exe ..\\..\\compiler\\v1\\src\\tests\\variables.rey`
