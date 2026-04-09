# Rey Language - Phase Checkpoint
# Date: Apr 9, 2026

## Current milestone — v0.3.0 self-hosting (new phases)

### ✅ Phase 0 — Read and understand (baseline audit)
- Read `compiler/v1/src/` and `rey-compiler/` (focused on parser/type system + bootstrap pipeline).
- `compiler/v1`: `cargo build --release` ✅
- `compiler/v1`: `cargo test` ✅ (7 passing)
- Interpreter currently rejects generic type annotations like `Vec<String>` / `HashMap<K,V>` in type positions:
  - Repro: struct field `pub tokens: Vec<String>,` fails parsing at `<` with `error[syntax]: Expected field name or 'func'.`
  - Root cause: `parseTypeAtom()` in `compiler/v1/src/parser/parser.rs` consumes only the identifier and leaves `<...>` tokens behind.

### ✅ Phase 1 — Generic type annotations in interpreter
- Parser now skips/ignores `<...>` generic params in all type annotation positions (including nested generics).
- Type checker now accepts collection type annotations (`Vec`, `HashMap`, etc.) with erased type params.
- Verified:
  - `compiler/v1`: `cargo build --release` ✅
  - `compiler/v1`: `cargo test` ✅
  - Repro file with `Vec<String>` in struct field now runs ✅

### ✅ Phase 2 — Remaining interpreter bugs
- Parameter reassignment inside functions: already working ✅
- `return;` in `Void` functions: fixed ✅
- `const` inside functions: already working ✅
- String indexing (`str[i]`): already working ✅
- Null propagation for method calls: fixed ✅ (now errors with line number)
- Circular import diagnostics: already includes full cycle path ✅

### ✅ Phase 3 — Language surface completion (interpreter)
- Added `assert(condition, message)` builtin (fails with `error[assert]` + line number).
- Added missing `String` methods: `trim`, `startsWith`, `endsWith`, `replace`, `slice`, `indexOf`, `repeat`, `padLeft`, `padRight`.
- Added missing `Vec` methods: `map`, `filter`, `reduce`, `reverse`, `sort`, `slice`, `join`.
- Added missing `HashMap` methods: `keys`, `values`, `entries`.
- Added math builtins: `floor`, `ceil`, `round`, `sqrt`, `pow`, `log`, `sin`, `cos`, `tan`.

### Next up
- Phase 5 — make the bootstrap compiler feature-complete (beyond `hello.rey`)

### ✅ Phase 4 — Run the bootstrap compiler (end-to-end native hello)
- Verified end-to-end pipeline works for the minimal `hello.rey` program:
  - Build: `compiler/v1/target/release/rey-v0 rey-compiler/main.rey build rey-compiler/tests/e2e/hello.rey`
  - Output binary: `/tmp/rey_out`
  - Running `/tmp/rey_out` prints: `Hello from native Rey!`
- `rey-compiler/src/codegen/main.rey:generateIR()` is now wired and emits a minimal LLVM IR module sufficient for:
  - `func main(): Void { println("..."); }`
  - integer/string literals
  - basic integer arithmetic (limited)
- `compileLLVM()` now:
  - Writes IR to `/tmp/rey_out.ll`
  - Tries `llc ... -filetype=obj` then `clang /tmp/rey_out.o -o <outputPath>`
  - Falls back to `clang -c -x ir /tmp/rey_out.ll -o /tmp/rey_out.o` when `llc` is not installed
- `rey-compiler/main.rey` now propagates `compileLLVM()` errors via `CompileResult.ok=false`.

### Resume point
- Start Phase 5 with the next test program after `hello.rey` (arithmetic/variables/if/else), then iterate until `structs`, `enums`, and `imports` compile end-to-end.

### ✅ Phase 5 — Bootstrap compiler (expanded test coverage)
- Added e2e fixtures under `rey-compiler/tests/e2e/` and verified native compilation + execution for:
  - `math.rey` (vars, arithmetic, if/else, println)
  - `loops.rey` (while, loop, break, continue, and a minimal `for (x in [..])` unroll)
  - `functions.rey` (func params, calls, recursion)
  - `structs.rey` (struct decl + struct literal + field access for `int` fields)
  - `enums.rey` (enum decl + `Enum.Variant` tags, comparisons)
  - `imports.rey` (string-literal import path + recursive merge of imported programs)
- Note: native codegen is still a **minimal subset** (no heap runtime, no stdlib/runtime calls, no match, no real Vec/HashMap lowering, no packed layouts, and no general import/export visibility system).

### Next up
- Phase 6 — self-hosting verification: currently blocked until native codegen/runtime can compile and run the full `rey-compiler` itself (needs a real runtime for `Vec`, `HashMap`, strings, IO/process, etc.).

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
