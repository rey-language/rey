# rey-compiler (bootstrap, v5.0)

A compiler for Rey, written in Rey — the long-term "Rey-in-Rey" bootstrap target.

## What this is

`rey-compiler/` is the start of a self-hosted Rey compiler. It will eventually
replace the current Rust interpreter (`compiler/v1/`) as the canonical
implementation. The goal is a compiler that can compile itself.

## How it fits the roadmap

| Track | Status | Role |
|-------|--------|------|
| `compiler/v1/` | Shipping | Rust interpreter + typechecker + import resolver (v0.x) |
| `rey-compiler/` | API skeleton | Self-hosted bootstrap compiler (v5.0 target) |

The v1 Rust runtime must be stable enough to host and execute the bootstrap
compiler before implementation work begins. Until then, `rey-compiler/` holds
the agreed-upon architecture so we can stabilize types and interfaces early.

## Current state

**API surface only.** The project defines core types and public function
signatures across five layers. No implementations yet.

### File layout

```
rey-compiler/
├── main.rey                  # Top-level entrypoint types (CompileOptions, etc.)
└── src/
    ├── lexer/
    │   ├── main.rey          # Lexer struct, newLexer(), nextToken(), lexAll()
    │   └── token.rey         # TokenKind enum, Token struct, LexError
    ├── parser/
    │   ├── main.rey          # Parser struct, newParser(), parseProgram()
    │   └── ast.rey           # Node, Literal, Program types
    ├── typecheck/
    │   └── main.rey          # TypeKind, TypeEnv, typecheck()
    ├── codegen/
    │   └── main.rey          # IrKind, Backend, lowerToIr(), emit()
    └── diagnostics/
        └── main.rey          # Diagnostic, DiagnosticBag helpers
```

## What's next

Implementation order (each step depends on the previous):

1. **Lexer** — tokenize Rey source into `[Token]` using the TokenKind enum
2. **Parser** — recursive descent, produces `[Node]` AST from tokens
3. **Typecheck** — walk the typed AST, resolve names, check assignability
4. **Codegen** — lower to IR, then emit bytecode / C / LLVM / JS per backend
5. **Self-host** — compile this compiler with itself, verify round-trip

Each layer will be tested against the v1 Rust interpreter as a reference.

## Building and testing

The bootstrap compiler is written in Rey and runs on `compiler/v1/`:

```bash
cd compiler/v1
cargo build --release
./target/release/rey-v0 ../../rey-compiler/main.rey
```

(Will only produce output once implementations are added.)

## Contributing

- All work happens on the `codex` branch. Open PRs against `main` when a
  layer is fully implemented and verified against v1.
- Match existing Rey style: camelCase identifiers, compact formatting, minimal
  comments.
- Every public function signature in the API skeleton is a contract — don't
  change signatures without updating all dependents and documenting the change
  in `CHANGELOG.md`.
- Run v1 tests (`cargo test` in `compiler/v1/`) after any change that touches
  shared language behavior.
