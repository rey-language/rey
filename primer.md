# Primer — rey-lang
Last updated: Mar 30, 2026 (bootstrap unblock)

## Session objective
Unblock bootstrap: remove generic type params (`Vec<T>`, `HashMap<K,V>`) from `rey-compiler/` so it can compile without generics support.

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
- Vec: push, pop, len, get, set, contains, indexOf
- HashMap: set, get, delete, has, len  
- Stack: push, pop, peek, isEmpty, len
- Queue: enqueue, dequeue, peek, isEmpty, len
- Option: Some, None, unwrap, unwrapOr, isSome, isNone
- Result: Ok, Err, unwrap, unwrapOr, isOk, isErr

## Next steps - Phase 3
Add native file system builtins:
- readFile, writeFile, appendFile
- fileExists, deleteFile, mkdir, listDir
- getEnv, args, exit, exec

Then Phase 4: std library in Rey

## Blockers
- Generics in `rey-compiler/` are removed (no more `Vec<T>` / `HashMap<K,V>` in token/AST).
- Bootstrap pipeline is still incomplete (typecheck/codegen are stubbed; CLI arg indexing is probably off).

## Rey-compiler bootstrap
- `rey-compiler/src` now avoids generics and uses concrete-ish shapes (arrays + string token kinds) to keep parsing simple.
- Next: wire `typecheck` + `codegen` back in once module/value scoping for enums/types is sorted.

(End file)
