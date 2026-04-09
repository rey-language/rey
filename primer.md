# Primer — rey-lang
Last updated: Apr 9, 2026 (bootstrap compiler progress)

## Session objective
Get the bootstrap compiler (`rey-compiler/`) compiling more of the language to native binaries, and keep docs/tests in sync with what actually works.

## What was done - Session Complete

### Phase 0 - Baseline Audit (COMPLETE):
- Read all compiler/v1/src/ files
- Read rey-compiler/ and syntax.md
- Documented all known bugs

### Phase 1 - Compiler v1 Hardening (COMPLETE):
- Fixed string indexing: source[i] returns single character
- Added typecheck for string indexing
- Version bumped to 0.2.0

### Phase 2 - New Data Types (COMPLETE):
- Implemented Vec, HashMap, Stack, Queue, Option, Result
- All container methods working
- Parser updated for Option.Some syntax
- Tests pass

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
