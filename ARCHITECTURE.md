# Rey Compiler Architecture

This document describes the full compiler pipeline, design decisions, and internal representations used in the Rey self-hosted compiler.

---

## Pipeline Overview

```
Source (.rey)
     │
     ▼
  Lexer  ──────────────────────────────  rey-compiler/src/lexer/
     │   tokens: Vec<Token>
     ▼
  Parser  ─────────────────────────────  rey-compiler/src/parser/
     │   AST: Program { statements: Vec<Stmt> }
     ▼
  Type checker (stub)  ────────────────  rey-compiler/src/typecheck/
     │   TypecheckResult { ok: bool, errors: Vec<TypeError> }
     ▼
  LLVM IR emitter  ────────────────────  rey-compiler/src/codegen/
     │   LLVM IR text (.ll file)
     ▼
  clang  ──────────────────────────────  external tool
     │   native binary
     ▼
  C runtime  ──────────────────────────  rey-compiler/runtime/
```

---

## Stage 1 — Lexer (`src/lexer/`)

**Input:** Raw source string, filename  
**Output:** `LexerResult { tokens: Vec<Token>, errors: Vec<LexError> }`

Each `Token` carries:
- `kind: String` — token type name (e.g. `"Identifier"`, `"IntLiteral"`, `"LeftBrace"`)
- `lexeme: String` — raw source text
- `span: Span` — `{ start, end, line, column }`

The lexer is a single-pass character scanner. String literals are stored with surrounding quotes in `lexeme`; the codegen strips them when emitting globals.

---

## Stage 2 — Parser (`src/parser/`)

**Input:** `Vec<Token>`, filename  
**Output:** `ParserResult { program: Program, errors: Vec<ParseError> }`

A hand-written recursive descent parser. The AST is built from concrete structs defined in `src/parser/ast/main.rey`.

### AST node types

All AST nodes are heap-allocated structs identified by a type tag byte in their 16-byte header (see Value Representation below).

**Statements:**
- `VarDeclStmt` — `var name: Type = value`
- `ConstDeclStmt` — `const name: Type = value`
- `FuncDeclStmt` — `func name(params): ReturnType { body }`
- `StructDeclStmt` — `struct Name { fields }`
- `EnumDeclStmt` — `enum Name { variants }`
- `AssignStmt` — `target = value`, `target += value`, etc.
- `ExprStmt` — expression used as statement
- `IfStmt` — `if cond { then } else { else }`
- `WhileStmt` — `while cond { body }`
- `LoopStmt` — `loop { body }`
- `ForStmt` — `for x in iterable { body }`
- `ReturnStmt` — `return expr`
- `BreakStmt`, `ContinueStmt`
- `ImportStmt` — `import module`
- `BlockStmt` — `{ stmts }`

**Expressions:**
- `LiteralExpr` — `{ kind: LiteralKind, text: String }`
- `IdentExpr` — `{ name: String }`
- `BinaryExpr` — `{ left, op, right }`
- `UnaryExpr` — `{ op, right }`
- `CallExpr` — `{ callee, args }`
- `FieldAccessExpr` — `{ object, name }`
- `IndexExpr` — `{ target, index }`
- `ArrayExpr` — `{ elements }`
- `StructLiteralExpr` — `{ name, fields }`
- `LambdaExpr` — `{ params, body }`

**Literal kinds** (stored as `i64` integers in native code):
```
LiteralKind: String=0, Char=1, Int=2, Float=3, Bool=4, Null=5
```

---

## Stage 3 — Type Checker (`src/typecheck/`)

Currently a stub: always returns `ok: true`. Type annotations are parsed and recorded in AST nodes but not fully enforced. The codegen uses type annotations for stack slot allocation (via `llvmTypeFromTagCtx`).

---

## Stage 4 — LLVM IR Emitter (`src/codegen/`)

**Input:** `Program` (merged AST with all imports inlined)  
**Output:** LLVM IR text string

The emitter is a single-pass tree-walker over the AST. It maintains a `CodegenContext` struct containing:
- `locals: HashMap` — variable name → `LocalVar { ptr, ty }` (alloca address + LLVM type)
- `structDefs: HashMap` — struct name → `StructDef { llvmName, fields: Vec<StructFieldDef> }`
- `enumDefs: HashMap` — enum name → `Vec<String>` (variant names in order)
- `structTags: HashMap` — struct name → integer tag (for `instanceof` checks)
- `funcSigs: HashMap` — function signature → LLVM type (for call-site coercion)
- `loopStack: Vec<LoopInfo>` — break/continue label stack
- `funcAllocas: String` — accumulated alloca instructions hoisted to entry block

### Key design decisions

#### 1. Everything is `i64` in LLVM IR

All Rey values are 64-bit integers at the IR level:

| Rey type | LLVM IR type | Encoding |
|----------|-------------|----------|
| `int` | `i64` | direct |
| `bool` | `i64` | 0 = false, 1 = true |
| `float` | `double` | *(exception — 64-bit float)* |
| `String` | `i8*` | pointer to ReyStr data field |
| `Vec`, `HashMap` | `i64` | heap pointer cast to integer |
| `Result`, `Option` | `i64` | heap pointer cast to integer |
| `struct` | `i64` | heap pointer cast to integer |

When passing between functions, struct/vec/map values are passed as `i64`. The callee casts them back to the appropriate struct pointer type via `inttoptr`.

#### 2. Heap object layout

Every heap-allocated value has a 16-byte header:

```
offset  0:  uint8_t  tag       // type tag for instanceof
offset  1:  uint8_t  [7 bytes padding]
offset  8:  int64_t  reserved  // (used by String for length)
offset 16:  data...            // struct fields / string bytes
```

The data pointer (what Rey code sees) points to offset 16. `instanceof` reads the tag byte at `ptr - 16`.

Well-known tags:
```c
REY_TAG_STR    = 1
REY_TAG_VEC    = 2
REY_TAG_MAP    = 3
REY_TAG_OK     = 4
REY_TAG_ERR    = 5
REY_TAG_SOME   = 6
REY_TAG_NONE   = 7
REY_TAG_STRUCT = 8   // user structs start here, incrementing
```

#### 3. Alloca hoisting

LLVM requires all `alloca` instructions to be in the function entry block. During body code generation, allocas are accumulated in `ctx.funcAllocas` and prepended to the function body before emission. This avoids SIGBUS on ARM64 from uninitialized alloca addresses in branches that were never entered.

#### 4. Struct field disambiguation

When the type of a local variable is not known (untyped `i64`), the emitter searches `ctx.structDefs` for a struct whose field name matches the access. To avoid nondeterminism, typed variable declarations (`var x: TypeName = expr`) are used wherever struct fields are accessed. The heuristic prefers structs with scalar-typed fields (`i8*`, `i64`, `i1`) over struct-pointer fields to reduce false matches.

#### 5. Enum values as integers

Enum variants compile to `i64` indices (0-based, in declaration order). `LiteralKind.String` = 0, `LiteralKind.Int` = 2, etc. Comparisons use `icmp eq i64`.

The codegen source contains dual-format checks for enum kinds to work in both the Rust interpreter (which represents enums as strings like `"LiteralKind::String"`) and native code (which represents them as `i64`):

```rey
if kStr == "LiteralKind::String" || kStr == "0" {
```

#### 6. Named struct types for anonymous object literals

The emitter defines several internal named structs to carry structured return values between codegen functions. Without these, `return { ty: ..., val: ..., code: ... }` would compile to a null pointer (the codegen has no unnamed struct support in native code).

Named internal structs:
- `GenExprResult { ty: String, val: String, code: String }` — expression codegen return value
- `CStringInfo { escaped: String, len: int }` — LLVM string constant metadata
- `LocalVar { ptr: String, ty: String }` — alloca slot descriptor
- `LoopInfo { breakLabel: String, continueLabel: String }` — loop stack entry
- `StructFieldDef { name: String, ty: String }` — struct field type descriptor
- `StructDef { llvmName: String, fields: Vec }` — struct type definition

---

## Stage 5 — C Runtime (`runtime/`)

A static library (`librey_rt.a`) linked into every compiled binary. Implements all built-in operations that would be too verbose in raw LLVM IR.

### Files

| File | Responsibility |
|------|---------------|
| `mem.c` | `rey_alloc` (malloc wrapper), `rey_panic` |
| `string.c` | String heap allocation, concat, slice, search, conversion |
| `vec.c` | Dynamic array: push, pop, get, set, map, filter, join |
| `hashmap.c` | Open-addressing string→i64 hash map |
| `result.c` | Result/Option boxing, unwrap, tag read; `instanceof` |
| `io.c` | File I/O, `exec` (popen subprocess), `args`, `exit` |
| `rey_rt.h` | Public ABI header — the interface the codegen generates `declare` for |

### Memory model

Arena/bump allocator: all allocations come from `malloc`, nothing is freed until process exit. This is acceptable for a compiler process (short-lived, bounded memory). No garbage collector.

---

## Bootstrap Chain

```
Phase A-G: rey-v0 (Rust) interprets main.rey → generates LLVM IR → clang → native binary

Phase H:   native binary compiles Rey programs directly
           rey-v0 only needed to rebuild the compiler itself
```

The Rust interpreter remains authoritative for semantics. When the Rust interpreter and native compiler disagree on a result, the Rust interpreter is correct.

---

## Import Resolution

`loadProgramWithImports` in `main.rey` inlines all imports before codegen runs, producing a single merged `Program`. Import resolution order:

1. Same directory as importing file
2. `<dir>/src/<module>/main.rey`
3. `<dir>/<module>/main.rey`
4. Project root variants of the above

Circular imports are detected via an explicit `stack: Vec<String>` of paths currently being loaded.

---

## Compilation Flow (compileLLVM)

```
1. Write LLVM IR to <output>.ll
2. Try: llc -O0 -filetype=obj <output>.ll -o <output>.o
3. If llc fails: clang -O0 -c -x ir <output>.ll -o <output>.o
4. clang <output>.o -L<rtDir> -lrey_rt -o <output>
```

The `llc` step is attempted first for faster compilation; `clang -c` is the reliable fallback.

---

## Known Gaps and Technical Debt

1. **Type checker is a stub** — all Rey programs pass type checking. Real enforcement is future work.
2. **Self-compile crashes** — the native compiler panics on its own complex source due to remaining untyped struct field accesses in deeply nested codegen loops.
3. **Dual-format enum checks** — the `"LiteralKind::String" || "0"` pattern is fragile; should be replaced once the type checker enforces enum types.
4. **No closures over mutable state** — lambdas are lifted to top-level functions; captured mutable variables are not supported.
5. **No optimization passes** — all code is compiled at `-O0`. LLVM optimization flags could be plumbed through.
6. **String ABI inconsistency** — the `i8*` ABI for strings vs `i64` ABI for everything else requires explicit casts in many places.
