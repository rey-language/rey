# Primer — rey-lang
Last updated: Mar 23, 2026 (session end)

## What this project is
Rey is a custom language by Misbah. Current runtime is a Rust tree-walking interpreter (`compiler/v1`) with compile-time parsing/typechecking and runtime execution.

## Key architecture
```
compiler/v1/src/
├── lexer/        # tokenizer + spans
├── parser/       # recursive descent parser + parse errors
├── ast/          # expressions/statements/types
├── typecheck.rs  # static checks
└── interpreter/  # executor/evaluator/environment
```
Pipeline: source -> lexer -> parser -> AST -> typecheck -> interpreter

## Session completed
- Added new function visibility model in AST/parser/lexer:
  - `export pub func` => importable
  - `pub func` => local/module visibility but blocked from imports
  - `func` => private
- Added import AST and parser support:
  - `import file.symbol`
  - `import file.{a,b}`
  - `import module`
  - `import module::file`
  - `import module::{fileA,fileB}`
- Added compile-time import resolver (`compiler/v1/src/imports.rs`) and integrated it into `main.rs`.
- Implemented resolver order:
  1. current file directory
  2. entry project root
  3. `~/.reyc/std/src` for `std` module prefix
  4. `~/.reyc/packages`
- Implemented module rules:
  - `import action` requires `action/main.rey`
  - module namespace auto-collects `export pub` symbols from every `.rey` file in that folder
  - `import action::walk` resolves `action/walk.rey`
- Implemented scope injection:
  - file-symbol imports inject names directly
  - module imports inject namespace dicts (`action.func()`, `walk.func()`)
- Implemented diagnostics for:
  - file not found
  - missing module `main.rey`
  - function not found
  - function exists but only `pub`
  - circular imports (with cycle chain)
  - duplicate imports
- Added namespace method-call dispatch in executor/typechecker for imported namespace calls.

## Tests added
- `tests/imports/success/` full passing integration case with file and module import forms.
- `tests/imports/errors/` covers all required error categories:
  - missing file
  - missing module main
  - missing function
  - `pub` not `export pub`
  - circular import
  - duplicate import

## Verification run this session
- `cargo build` (pass)
- `cargo test` (pass)
- `cargo run -- ../../tests/imports/success/main.rey` (pass)
- `cargo run -- ../../tests/imports/errors/*.rey` (expected compile-time failures, all correct category/messages)

## Current project state
- Import system is fully implemented for the requested spec.
- Branch has five logical commits for parser/visibility, resolver, modules, scope dispatch, and tests.

## Next up
- Add automated Rust integration tests that execute the new import fixtures and assert expected stdout/stderr.
- Add docs update in `syntax.md` describing import grammar and `export pub` rules.
