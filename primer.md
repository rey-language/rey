# Primer — rey-lang
Last updated: Apr 10, 2026 (interpreter stabilization + bootstrap alignment)

## Session objective
Stabilize and align the Rust interpreter (`compiler/v1`) with syntax spec expectations and bootstrap behavior, starting with parser consistency and core runtime semantics.

## What was done this session

### Phase 1 - Syntax consistency (DONE)
- Standardized missing semicolon parser error to `error[syntax]: expected ';'`.
- Enforced comma separators between:
  - struct fields
  - enum variants
  - match arms
- Added parser regression coverage:
  - `compiler/v1/src/tests/syntax_missing_semicolon.rey`
  - `compiler/v1/src/tests/match_missing_comma.rey`
  - `compiler/v1/src/tests/enum_missing_comma.rey`
- Verified diagnostic output includes file + line + column via CLI path.

### Phase 2/3 - Runtime and match stabilization (PARTIAL DONE)
- Added array/string bounds diagnostics:
  - `index out of bounds (i=..., len=...)`
- Added runtime null guard in expression operations:
  - `null dereference at line <n>`
- Implemented equality semantics in runtime binary evaluation:
  - primitives and strings: value equality
  - arrays: reference equality
  - structs: reference equality
- Updated match fallback failure message to:
  - `error[match]: non-exhaustive patterns`
- Added runtime regression coverage:
  - `compiler/v1/src/tests/runtime_ref_equality.rey`
  - `compiler/v1/src/tests/runtime_index_oob.rey`
  - `compiler/v1/src/tests/match_non_exhaustive.rey`
  - direct executor-level null dereference unit assertion in `compiler/v1/src/main.rs`

## Verification run
- `cargo build --release` (pass)
- `cargo test` (pass, 14 tests)

<<<<<<< HEAD
## What's working now (v0.2.0+)
- Integer division, struct mutation, enum match, imports
- String indexing, parameter reassignment, return from nested blocks
- Vec: push, pop, len, get, set, contains, indexOf, map/filter/reduce, reverse/sort/slice/join
- HashMap: set, get, delete, has, len, keys/values/entries
- Stack: push, pop, peek, isEmpty, len
- Queue: enqueue, dequeue, peek, isEmpty, len
- Option: Some, None, unwrap, unwrapOr, isSome, isNone
- Result: Ok, Err, unwrap, unwrapOr, isOk, isErr
- `assert(condition, message)` builtin
- Math builtins: `floor`, `ceil`, `round`, `sqrt`, `pow`, `log`, `sin`, `cos`, `tan`

## Bootstrap compiler status
The bootstrap compiler is still **experimental**. It can compile the e2e fixtures under `rey-compiler/tests/e2e/` to native binaries using LLVM IR.

Run:
```bash
rey-compiler/tests/e2e/run.sh
```

## Blockers
- Self-hosting is not reached yet: native codegen/runtime needs a real lowering/runtime for `Vec`, `HashMap`, strings, IO/process, and more of the language surface.

(End file)
