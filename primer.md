# Primer — rey-lang
Last updated: Mar 27, 2026 (long autonomous session)

## Session objective
Build Rey into a complete, self-hosted, natively compiled language (Phase 0-10).

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
- None currently

## Rey-compiler bootstrap
- Skeleton exists in rey-compiler/
- Need to implement Phases 5-9 (lexer, parser, typecheck, codegen, wire up)

(End file)