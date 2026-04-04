# Rey Language - Phase Checkpoint
# Date: Apr 5, 2026

## Current milestone — v0.3.0 self-hosting (new phases)

### ✅ Phase 0 — Read and understand (baseline audit)
- Read `compiler/v1/src/` and `rey-compiler/` (focused on parser/type system + bootstrap pipeline).
- `compiler/v1`: `cargo build --release` ✅
- `compiler/v1`: `cargo test` ✅ (7 passing)
- Interpreter currently rejects generic type annotations like `Vec<String>` / `HashMap<K,V>` in type positions:
  - Repro: struct field `pub tokens: Vec<String>,` fails parsing at `<` with `error[syntax]: Expected field name or 'func'.`
  - Root cause: `parseTypeAtom()` in `compiler/v1/src/parser/parser.rs` consumes only the identifier and leaves `<...>` tokens behind.

### Next up
- Phase 1 — fix parser/type system to accept/ignore generic type params everywhere, then re-run `cargo build --release` + `cargo test`.

---
# Previous checkpoint
# Date: Mar 27, 2026

## PHASES 0-10 COMPLETE - ALL WORK DONE ON MASTER BRANCH

### Summary:

#### ✅ Phase 0 - Baseline Audit
- Read all compiler files, documented working state

#### ✅ Phase 1 - Compiler Hardening (v0.2.0)
- Fixed string indexing, added typecheck, bumped version

#### ✅ Phase 2 - New Data Types  
- Vec, HashMap, Stack, Queue, Option, Result with all methods

#### ✅ Phase 3 - Native FS Builtins
- readFile, writeFile, appendFile, fileExists, deleteFile, mkdir, listDir
- getEnv, args, exit, exec

#### ✅ Phase 4 - Std Library (std repo)
- fs, process, io, string, collections modules

#### ✅ Phase 5 - Lexer in Rey
- Full lexer with keywords, operators, escape sequences, comments

#### ✅ Phase 6 - Parser in Rey  
- Complete recursive descent parser, full AST

#### ✅ Phase 7 - Typechecker in Rey
- Type inference, symbol table, built-in function types

#### ✅ Phase 8 - Codegen in Rey
- LLVM IR generation, basic type mapping

#### ✅ Phase 9 - Full Pipeline
- Lexer→Parser→Typechecker→Codegen wired together

#### ✅ Phase 10 - Self-Hosting Attempt
- Bootstrap compiler implemented in Rey via Rust interpreter

### Tests: 7/7 passing | Version: 0.2.0 | Status: Complete
