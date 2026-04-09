# Changelog

## [checkpoint] — 2026-04-05
- Verified `compiler/v1` builds in release mode and all Rust tests pass (7/7).
- Confirmed interpreter parse failure for generic type annotations like `Vec<String>` (fails at `<` in type positions); tracked to `parseTypeAtom()` not consuming generic parameter tokens.

## [fix] — 2026-04-05
- Parser now accepts generic type annotations like `Vec<Token>` / `HashMap<String, Vec<int>>` by skipping the `<...>` portion during type parsing.
- Type checker recognizes collection type annotations (`Vec`, `HashMap`, `LinkedList`, `Stack`, `Queue`, `Option`, `Result`) as dynamically-dispatched at runtime (type params erased).
- Added support for bare `return;` (no expression), enabling early returns from `Void` functions.
- Method calls on `null` now error explicitly and include a line number.

## [feat] — 2026-04-05
- Added `assert(condition, message)` builtin (exits with `error[assert]`).
- Added missing `String` methods: `trim`, `startsWith`, `endsWith`, `replace`, `slice`, `indexOf`, `repeat`, `padLeft`, `padRight`.
- Added missing `Vec` methods: `map`, `filter`, `reduce`, `reverse`, `sort`, `slice`, `join`.
- Added missing `HashMap` methods: `keys`, `values`, `entries`.
- Added math builtins: `floor`, `ceil`, `round`, `sqrt`, `pow`, `log`, `sin`, `cos`, `tan`.
- Import resolver now supports module/file lookup under a project `src/` directory, and module imports include struct/enum declarations.
- Enums now expose a namespace dict under the enum name (enables `Enum.Variant` access) and typecheck defines enum names in scope.
- `exec()` now returns a `Result` value instead of a raw string.
- Added `String.len()` alias and lexicographic string comparisons (`<`, `<=`, `>`, `>=`) for bootstrap tooling.

## [feat] — 2026-04-09
- Bootstrap compiler now generates a minimal LLVM IR module and produces a working native binary end-to-end for `rey-compiler/tests/e2e/hello.rey`.
- `compileLLVM()` writes `<outputPath>.ll`, compiles via `llc` + `clang`, and falls back to `clang -c -x ir` when `llc` is unavailable.
- Extended bootstrap compiler parser/codegen to compile additional e2e programs end-to-end: `math.rey`, `loops.rey`, `functions.rey`, `structs.rey`, `enums.rey`, `imports.rey`.
- Added minimal struct literals (`Point { x: 1, y: 2 }`) and field access codegen for `int` fields.
- Added minimal enum variant codegen via `Enum.Variant` as integer tags.
- Added minimal import loader in `rey-compiler/main.rey` that recursively merges imported programs for compilation.
- Added `rey-compiler/tests/e2e/run.sh` plus `*.out` expected outputs, and moved native outputs to be written next to sources (no `/tmp` outputs).
- Added repo-local `reyc` launcher script as a convenience wrapper for invoking the bootstrap compiler via the Rust interpreter.

## [release] — 2026-03-27
- Bumped compiler crate version to `0.2.0` in `compiler/v1/Cargo.toml`.
- Fixed string indexing: `source[i]` now returns single character as String.
- Added type check support for string index type validation.
- Type errors now include span information for better diagnostics.

## [release] — 2026-03-26
- Bumped compiler crate version to `0.1.1` in `compiler/v1/Cargo.toml`.
- Fixed `match` on struct instances via struct field patterns.
- Fixed enum variant matching for both qualified (`Enum::Variant`) and unqualified (`Variant`) patterns.
- Improved grouped import diagnostics so missing items highlight the specific missing name.
- Verified nested module resolution where `module/main.rey` imports other local files.
- Fixed numeric semantics by distinguishing `int` and `float` values at runtime:
  - `10 / 3` performs integer division.
  - `10.0 / 3.0` and `10 / 3.0` perform float division.
- Allowed external mutation of `pub` struct fields (`obj.field = ...`, `obj.field += ...`) and emit a clear error for nested field assignment (`obj.inner.field = ...`) for now.
- Added `rey-compiler/` v5 bootstrap skeleton (API surface only).
- Added `compiler/README.md` to document the v1 Rust interpreter vs bootstrap target.

## [release] — 2026-03-23
- Prepared `v0.1.0` release candidate assets and docs.
- Audited syntax documentation against current parser/runtime behavior.
- Added root `RELEASE.md` covering `v0.0.1-pre` through `v0.1.0`.
- Added packaged release folder:
  - `releases/0.1.0/rey-v0-macos-arm64`
  - `releases/0.1.0/RELEASE.md`
- Bumped compiler crate version to `0.1.0` in `compiler/v1/Cargo.toml`.
- Updated README current version and import-system feature status.
- Cleaned compiler warnings (unused imports/variables/dead methods) and restored import parsing regression for `module::item`.
- Fixed static call parser bug so `StructName.create(...)` resolves method names correctly.
- Updated stale test fixture `compiler/v1/src/tests/test_rand.rey` to reflect current type checking behavior.

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
- Shipped `rey v0.0.7-pre`
- Added `projects/fake-cli/cli.rey` - a full interactive TUI implementation in Rey.
- Fixed struct field/method resolution bugs and optimized typechecking for complex expressions.
- Enhanced property access and indexing consistency.

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

## [fix] — 2026-03-19
- Fixed struct method resolution with typed parameters - methods now properly resolve when parameters have struct type annotations
- Fixed string interpolation with nested quotes - parser now handles `{scores["alice"]}` syntax using escaped quotes
- Fixed array type parsing in struct fields - `[int]`, `[String]`, etc. now work as field types
- Added comprehensive test file `compiler/v1/src/tests/struct_advanced.rey` covering all three fixes
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
