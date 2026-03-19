# Changelog

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
