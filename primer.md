# Primer — rey-lang
Last updated: Mar 26, 2026 (session end)

## Session objective
Ship v0.1.1 patches (match/imports/eval) and add `rey-compiler/` bootstrap skeleton.

## What was done
- Match fixes:
  - Added struct patterns (`StructName { field: pattern }`) and fixed matching on struct instances.
  - Fixed enum matching for both qualified (`Enum::Variant`) and unqualified (`Variant`) patterns.
  - Added `.rey` regression programs plus a small `cargo test` harness to execute them.
- Import fixes:
  - Grouped imports now track per-item spans so missing names highlight the specific item.
  - Added fixtures and a `cargo test` check for grouped-missing-symbol.
  - Added fixture and test for module `main.rey` importing a sibling file (nested resolution).
- Evaluator/runtime fixes:
  - Numbers now preserve `int` vs `float` at runtime (lexer -> AST literal -> `Value`), fixing integer division and mixed int/float division semantics.
  - External mutation of `pub` struct fields works (`obj.field = ...`, `obj.field += ...`).
  - Nested field assignment (e.g. `obj.inner.field = ...`) now errors clearly.
  - Added `.rey` regression programs for division and field mutation.
- `rey-compiler/`:
  - Added API-only bootstrap skeleton with the requested file layout and public signatures.
- Docs:
  - Added `compiler/README.md` to document v1 vs bootstrap.
  - Updated `syntax.md` for v0.1.1 behavior deltas.
- Versioning:
  - Bumped `compiler/v1/Cargo.toml` to `0.1.1`.
  - Added a `CHANGELOG.md` entry for 2026-03-26.

## Current state
- v0.1.1 patch series is complete on `codex` and validated via `cargo test`.
- `rey-compiler/` skeleton exists (API only).

## Next steps after this session
- Merge PR for v0.1.1.
- Decide next v0.2.0 compiler/runtime milestone list (match-as-expression, type system, better stdlib).
