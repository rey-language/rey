# Contributing to Rey

The Rey compiler is a research project and learning environment. Contributions are welcome — whether fixes, new language features, tests, documentation, or runtime improvements.

---

## Getting Started

```sh
# Clone and build the bootstrap interpreter
git clone https://github.com/rey-language/rey-lang
cd rey-lang
cd compiler/v1 && cargo build --release && cd ../..

# Build the native compiler
./compiler/v1/target/release/rey-v0 rey-compiler/main.rey build rey-compiler/main.rey
codesign -s - rey-compiler/main.bin

# Run the test suite
make -C rey-compiler/runtime
bash rey-compiler/tests/e2e/run.sh
```

---

## Code Style

### Rey source code

- **camelCase** for all identifiers (variables, functions, struct fields, enum variants)
- **PascalCase** for struct and enum type names
- Compact formatting — no excessive blank lines
- Comments only when the *why* is non-obvious (not the *what*)
- Avoid comments that just restate the code name

```rey
// good: explains non-obvious constraint
var slen = info.len - 1;  // ReyStr.len excludes null terminator

// bad: restates the obvious
var result = add(a, b);   // add a and b
```

### C runtime code

- Follow C99 style
- `snake_case` for all identifiers
- Functions named `rey_<module>_<operation>`: `rey_str_concat`, `rey_vec_push`
- Every function documented in `rey_rt.h` with input/output types

---

## Repository Layout

Before adding files, understand the layout:

```
rey-compiler/src/lexer/main.rey      — tokenizer
rey-compiler/src/parser/main.rey     — parser and AST definition
rey-compiler/src/parser/ast/main.rey — AST node struct declarations
rey-compiler/src/typecheck/main.rey  — type checker (stub)
rey-compiler/src/codegen/main.rey    — LLVM IR emitter
rey-compiler/runtime/rey_rt.h        — C runtime ABI
rey-compiler/runtime/*.c             — C runtime implementation
rey-compiler/tests/e2e/              — end-to-end test fixtures
```

---

## Adding a New Language Feature

### 1. Parser change

Add the new AST node to `src/parser/ast/main.rey`:

```rey
struct NewExprType {
    pub field1: String,
    pub field2: int,
    pub span: Span,
}
```

Add parsing in `src/parser/main.rey`. Follow the existing recursive-descent style. Always record spans.

### 2. Codegen support

Add IR emission in `src/codegen/main.rey` inside `genExprValue` or `genStmt2`.

**Critical rules for codegen:**

- **Always use typed variables** when accessing struct fields:
  ```rey
  // WRONG — heuristic may pick wrong struct
  var name = expr.name;

  // CORRECT — explicit type forces correct field offset
  var identE: IdentExpr = expr;
  var name = identE.name;
  ```

- **All allocas must be in the entry block** — add new variables to `ctx.funcAllocas`, never emit `alloca` inline in conditional branches.

- **New struct types** used as return values or stored in collections must be declared as named `struct` in the codegen source (not as anonymous `{ field: value }` literals, which compile to null in native code).

- **Use `GenExprResult { ty, val, code }`** as the standard return type from expression emitter functions.

### 3. Runtime function (if needed)

Add the C function to the appropriate `runtime/*.c` file, declare it in `rey_rt.h`, and add a `declare` line in `codegenProgram2` in `codegen/main.rey`.

### 4. Test

Add a fixture to `rey-compiler/tests/e2e/`:
- `newfeature.rey` — the test program
- `newfeature.out` — expected stdout

Run the suite: `bash rey-compiler/tests/e2e/run.sh`

### 5. Rebuild and verify

```sh
./compiler/v1/target/release/rey-v0 rey-compiler/main.rey build rey-compiler/main.rey
codesign -s - rey-compiler/main.bin
mv rey-compiler/main.bin rey-compiler/main
bash rey-compiler/tests/e2e/run.sh
```

---

## Testing Philosophy

- **e2e tests are the source of truth** — if the output matches, the feature works
- Tests should be deterministic — same input, same output every time
- Test fixtures cover: basic syntax, control flow, collections, structs, enums, strings, I/O, imports
- Each fixture has a `.rey` source and a `.out` expected-output file

---

## Commits

Write commits like a professional engineer:

```
fix: resolve struct field heuristic for untyped FieldAccess in codegen
feat(lexer): add character literal support
docs: expand ARCHITECTURE.md with memory model details
chore: remove build artifacts from e2e test directories
test: add edge-case fixture for nested struct access
```

- One logical change per commit
- Present tense, imperative mood
- Reference what changed and why in the body if non-obvious

---

## Architecture Constraints

Do not violate these invariants:

1. **The Rust interpreter is the semantic reference** — native compiler must produce identical results for all e2e fixtures
2. **No GC** — all allocations use `rey_alloc` (malloc); no freeing until process exit
3. **i64 ABI** — all Rey values passed between functions as `i64` (scalars direct, heap values as pointer-as-integer)
4. **Struct header at ptr-16** — `instanceof` reads the tag byte 16 bytes before the data pointer
5. **Allocas in entry block** — LLVM requirement; use `ctx.funcAllocas` accumulator

---

## Questions

Open an issue or look at the existing source — the codegen is the most complex part, but it's all plain Rey code.
