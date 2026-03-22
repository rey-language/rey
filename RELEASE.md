# Rey v0.1.0 Release Notes

## v0.0.1-pre
- Initial pre-release binary drop for Rey (`rey-v0`).
- Established baseline language pipeline (lexer -> parser -> interpreter).
- Early CLI workflow for running `.rey` files.

## v0.0.2-pre
- Stabilized core interpreter internals (environment/value/execution wiring).
- Improved support for functions and control-flow execution.
- Continued parser/executor iteration toward wider syntax coverage.

## v0.0.3-pre
- Added lexer support for `//` comments.
- Hardened parser behavior and removed panic paths after lexer failures.
- Synced and normalized `compiler/v1/src/tests/` fixtures to current behavior.
- Cleaned compiler warnings for that release milestone.

## v0.0.4-pre
- Added arrays: literals, indexing, typed arrays, and push/pop/len built-ins.
- Added dictionaries: literals, indexing, typed dictionaries.
- Added property access (`obj.prop`) for dictionary keys.
- Added `input()` builtin and expanded string methods.
- Added compile-time type enforcement for annotated values/calls.

## v0.0.5-pre
- Added string interpolation (`"HP: {hp}"`) and mixed-type string concatenation.
- Added `print()` and variadic-style `println(...)` behavior.
- Added conversion methods: `.toString()`, `.toInt()`, `.toFloat()`.
- Added math built-ins: `abs`, `max`, `min`, `random`.
- Added `const` declarations and upgraded diagnostic output style.

## v0.0.6-pre
- Shipped full struct system:
  - Struct declarations and literals
  - Instance/static-style methods
  - `pub` visibility metadata
  - Method overloading behavior in runtime dispatch
- Improved struct field diagnostics with suggestion support.

## v0.0.7-pre
- Fixed `else if` chaining behavior.
- Fixed struct field assignment behavior.
- Fixed array index assignment behavior.
- Fixed integer division behavior.
- Added/solidified `loop`, `for ... in array`, enums, and match support in the v0.0.7 cycle.

## v0.1.0
- Implemented full import system with compile-time resolution.
- Added `export pub` visibility for importable functions.
- Added file-level imports:
  - `import file.symbol`
  - `import file.{a, b}`
- Added module-level imports:
  - `import module`
  - `import module::item`
  - `import module::{itemA, itemB}`
- Added deterministic resolver order:
  1. current file directory
  2. project root
  3. `~/.reyc/std/src`
  4. `~/.reyc/packages`
- Added scope injection semantics for symbol and namespace imports.
- Added import diagnostics for missing files/modules/symbols, non-exported symbols, circular imports, and duplicates.
- Added import fixtures under `tests/imports/` for success and failure scenarios.

## What's Next (v0.2.0)
- Enums are done.
- Match is done.
- Planned focus:
  - Generics
  - Better closure ergonomics and runtime semantics
  - Continued standard library and module ecosystem maturity
