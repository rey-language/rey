# Changelog

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
