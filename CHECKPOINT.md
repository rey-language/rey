# Rey Language - Phase Checkpoint
# Date: Mar 27, 2026

## Current Phase: PHASE 2 COMPLETE - New Data Types

### What's Working (v0.2.0+):
- ✅ All 7 cargo tests pass
- ✅ Phase 0 - Baseline audit complete
- ✅ Phase 1 - Compiler v0.2.0 hardening complete  
- ✅ Phase 2 - New data types implemented:
  - Vec: push, pop, len, get, set, contains, indexOf
  - HashMap: set, get, delete, has, len
  - Stack: push, pop, peek, isEmpty, len
  - Queue: enqueue, dequeue, peek, isEmpty, len
  - Option: Some, None, unwrap, unwrapOr, isSome, isNone
  - Result: Ok, Err, unwrap, unwrapOr, isOk, isErr

### Phase 3 - File System Builtins - NOT YET STARTED
Need to add: readFile, writeFile, appendFile, fileExists, deleteFile, mkdir, listDir, getEnv, args, exit, exec

### Phase 4 - Std Library in Rey - NOT YET STARTED
Need to create: std/src/fs/main.rey, std/src/process/main.rey, std/src/io/main.rey, std/src/string/main.rey, std/src/collections/main.rey

### Phases 5-10 - Rey Compiler Bootstrap - NOT YET STARTED
Lexing/parsing/typechecking/codegen in Rey

### What Still Needs Work (Phase 2 continuation):
- Vec: add map, filter, reduce, reverse, sort, slice
- LinkedList: add head(), tail(), append(), toArray() 
- HashMap: add keys(), values()
- Named tuples / tuple destructuring

### Syntax.md - Needs update
New data types need to be documented