# rey v0.0.4-pre
Date: March 17, 2026

## What's new
- Arrays: literals (`[1, 2, 3]`), indexing (`xs[0]`), `push/pop/len`, typed arrays (`[int]`).
- Dictionaries: literals (`{key: value}`), indexing (`d["key"]`), typed dictionaries (`{String:int}`).
- String methods: `.length()`, `.upper()`, `.lower()`, `.contains()`, `.split()`.
- `input()` builtin (optional prompt string).
- Property access: `obj.prop` (dictionary key lookup).
- Compile-time type enforcement for annotated variables, functions, and common builtins.

## Notes
- Entry point: programs execute by calling `main()` if it exists.
- Dictionary keys are string-keyed (identifier keys like `{x: 1}` become `"x"`).

## Known limitations
- Modulo operator (`%`) is tokenized but not parsed.
- Compound assignment (`+=`, `-=`, etc) is not implemented.
- Increment/decrement (`++`, `--`) is not implemented.

## How to install and run
1. Use the binaries in this folder:
- `rey-v0-macos-arm64`
- `rey-v0-windows-x86_64.exe`

2. Run a Rey file:
- macOS arm64:
  `./rey-v0-macos-arm64 ../../compiler/v1/src/tests/full_demo.rey`
- Windows x86_64:
  `rey-v0-windows-x86_64.exe ..\\..\\compiler\\v1\\src\\tests\\full_demo.rey`

