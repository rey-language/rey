# Changelog

## [feature] — 2026-03-23
- Implemented full import system for Rey with compile-time resolution.
- Added `export pub` function modifier and import visibility enforcement:
  - `export pub func` => importable
  - `pub func` => not importable
  - `func` => private
- Added parser/AST support for:
  - `import file.symbol`
  - `import file.{a,b}`
  - `import module`
  - `import module::file`
  - `import module::{a,b}`
- Added import resolver pipeline (`compiler/v1/src/imports.rs`) and integrated it into compiler entry flow.
- Implemented resolver lookup order:
  1. current file directory
  2. project root (entry file directory)
  3. `~/.reyc/std/src` for `std` prefix
  4. `~/.reyc/packages`
- Added module semantics:
  - `module/main.rey` required for `import module`
  - module namespace auto-collects all `export pub` functions from `.rey` files in folder
  - `module::file` resolves direct file namespace
- Added import diagnostics for:
  - file not found
  - missing module `main.rey`
  - function not found
  - function is `pub` but not `export pub`
  - circular imports
  - duplicate imports
- Added runtime/typecheck namespace dispatch support for `namespace.func()`.
- Added import fixtures under `tests/imports/` for success and all required error cases.

## [release] — 2026-03-19
- Shipped `rey v0.0.6-pre`
- **Implemented full Struct System**: 
  - Data structures with named fields.
  - Instance methods with direct field scoping (no more `self.x` inside methods for fields!).
  - Static-style methods for construction and utilities.
  - Visibility control with `pub` (private by default).
  - Method overloading support.
  - Struct literals for easy instantiation.
- Added `structs.rey` comprehensive test file.
- Improved field access error messages with "did you mean?" suggestions using Levenshtein distance.

## [release] — 2026-03-19
- Shipped `rey v0.0.5-pre`
- Added string interpolation syntax `"{var}"` and mixed type string conversions (`"a" + 1`)
- Built-in type conversions: `.toString()`, `.toInt()`, `.toFloat()`
- Added multiple arguments to `println` and new `print()` equivalent
- Built-in functions: `abs()`, `max()`, `min()`, `random()`
- Added `const` values. Unannotated `var`s are now treated as dynamic types!
- Comprehensive rewrite of Error diagnostics to a visual snippet pointing format.

## [tooling] — 2026-03-17
- Added Linguist submission prep files: `languages/Rey.yaml`, `languages/samples/Rey.rey`, `LINGUIST.md`

## [fix] — 2026-03-17
- Added array methods: `.length()` and `.push()`
- Allowed typed empty array assignment like `var xs: [int] = []`
- Added `compiler/v1/src/tests/array_methods.rey` regression test

## [init] — 2026-03-17
- Claude initialized as contributor
- Created CLAUDE.md and primer.md
- Branch `claude` created off `main`

## [release] — 2026-03-17
- Shipped `rey v0.0.3-pre` on `codex`
- Added lexer support for skipping `//` comments
- Removed compiler warnings and cleaned unused parser/interpreter/lexer surfaces
- Hardened parser flow to prevent panic after lexer failure
- Updated/fixed test fixtures so every `.rey` file in `compiler/v1/src/tests/` runs cleanly
- Built and staged release binaries:
  - `releases/0.0.3-pre/rey-v0-macos-arm64`
  - `releases/0.0.3-pre/rey-v0-windows-x86_64.exe`
- Added `releases/0.0.3-pre/RELEASE.md`
