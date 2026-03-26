# Primer — rey-lang
Last updated: Mar 27, 2026 (session start)

## Session objective
Docs/examples audit: syntax.md audit, examples directory creation, rey-compiler README.

## What was done
- SYNTAX.md audit (JOB 1):
  - Added `pub` field modifier documentation to structs section.
  - Added `self` keyword note — reserved but not required for field access in methods.
  - Added `String.contains()` and `String.split()` to string methods list.
  - Added dot-notation method syntax for arrays (`xs.push(4)`, `xs.length()`).
  - Added `range(start, end)` to built-ins list.
  - Added string concatenation section (including auto-conversion with `+`).
  - Removed incorrect `Dictionary.length()` method (use `len()` built-in instead).
- Examples (JOB 2):
  - Created `examples/` directory from scratch (was empty).
  - `examples/basic/hello.rey` — hello world.
  - `examples/basic/variables.rey` — variables, types, interpolation, division.
  - `examples/basic/functions.rey` — defaults, variadic, lambdas.
  - `examples/advanced/self_cure_stub.rey` — struct + enum + match working together.
- rey-compiler README (JOB 3):
  - Expanded `rey-compiler/README.md` with full structure, roadmap, file layout, implementation order, and contributing section.

## Limitations discovered during audit
- Enum variant names (`Severity::Critical`) are only valid in match patterns, not in general expression positions. Must use unqualified names (`Critical`) in expressions.
- Custom type names (structs, enums) can't be used in type annotations — the type checker rejects them via `Ty::fromAnnotation`. Struct declarations skip type checking, but function return types and variable annotations fail.
- Trailing commas not supported in array/dict literals or struct definitions.
- `pub` on struct fields is parsed but not widely used in test files.
- `self` keyword is lexed but struct methods inject fields directly (no `self.field` syntax).

## Current state
- All three jobs complete, ready for commits.
- `cargo test` passes (7/7).
- All 4 example files verified running clean.

## Next steps
- Commit the three jobs separately as requested.
- Open PR if Misbah approves.

(End of file)
