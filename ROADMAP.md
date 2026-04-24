# Rey Language Roadmap

---

## Where we are

**v0.2.0 (current)** — Self-hosted native compiler. The Rey compiler is written in Rey, compiles to LLVM IR, and links against a C runtime. All e2e fixtures pass with identical output to the Rust reference interpreter.

Bootstrap chain:
```
Rust interpreter (rey-v0) → compiles main.rey → native binary → compiles Rey programs
```

---

## Known Limitations (near-term blockers)

### 1. Self-compile instability

The native compiler passes all e2e tests but panics when compiling its own source (a complex program). Root causes:

- Remaining untyped struct field accesses triggering wrong heuristic matches
- Complex recursive codegen functions with many in-scope variables sharing alloca slots

**Fix:** Annotate all remaining untyped variables in `codegen/main.rey` with explicit type declarations. This is tedious but mechanical.

### 2. Type checker is a stub

All programs "pass" type checking. Type annotations are parsed and stored but not enforced.

**Fix:** Implement bidirectional type inference. The AST already carries type tags on all declarations; the checker just needs to validate them.

### 3. Dual-format enum checks

The codegen contains checks like:
```rey
if kStr == "LiteralKind::String" || kStr == "0" {
```
because the Rust interpreter represents enums as strings and native code represents them as integers.

**Fix:** Once the type checker enforces enum types, the codegen can emit proper `icmp eq i64` without string fallbacks.

### 4. String ABI is `i8*`, everything else is `i64`

Strings have a special ABI (`i8*` pointing to the data field) while all other heap values use `i64` (pointer cast to integer). This creates awkward coercions in the codegen and runtime.

**Fix (long term):** Unify all heap values as `i64`; implement `rey_str_as_i64` / `rey_i64_as_str` helpers that the codegen emits. Or: change String to `i64` ABI and fix all call sites.

---

## v0.3.0 — Full type checking

**Goal:** Static type errors at compile time.

- Bidirectional type inference for all expressions
- Function signature checking at call sites
- Struct field type validation
- Enum exhaustiveness in `match`
- Meaningful error messages with source location

---

## v0.4.0 — Closures and captures

**Goal:** First-class closures with proper capture semantics.

Currently: lambdas are lifted to top-level functions. Captured variables must be immutable.

- Closure environment struct generation
- Mutable capture via reference
- Recursion within lambdas

---

## v0.5.0 — Generics (type-erased)

**Goal:** Parameterized types without monomorphization.

```rey
func map<T, U>(v: Vec<T>, f: (T) -> U): Vec<U> { ... }
```

Strategy: type-erase to `i64` at the IR level (already the case), add generic syntax to the parser and type checker, validate instantiation sites.

---

## v0.6.0 — Standard library

**Goal:** A useful standard library written in Rey.

- `std/string` — split, parse, format
- `std/io` — buffered read/write, stdin
- `std/math` — numeric utilities
- `std/collections` — sorted map, set
- `std/json` — JSON encode/decode
- `std/process` — subprocess with stdin/stdout control

---

## v0.7.0 — Optimization

**Goal:** Emit optimized IR.

- Pass `-O2` or `-O3` to `clang` in release mode
- Inline small functions at the IR level
- Constant folding for `const` expressions

---

## v1.0.0 — Stable self-hosting

**Goal:** The native compiler compiles itself (stage-3 fixed point).

Requirements:
- Full type checker
- Native compiler survives compiling `main.rey` without crashes
- Stage-2 and stage-3 binaries produce identical IR

---

## Research directions

- **WASM backend** — emit WASM instead of x86/arm64 for portable deployment
- **Incremental compilation** — cache per-file IR, only recompile changed files
- **Language server protocol (LSP)** — IDE integration (hover, go-to-definition, diagnostics)
- **Package manager** — `rey add <package>`, module registry

---

## Not planned (by design)

- Borrow checker / ownership — Rey uses GC-friendly arena allocation
- Async/await — out of scope for a learning compiler
- `unsafe` blocks — the runtime handles low-level operations
