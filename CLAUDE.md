# Project — Rey Language

**Rey** is an experimental programming language focused on language design, interpreters, and execution models. It's designed to be simple and easy to learn.

## Core Rules

1. Variables are declared using `var`
2. Type annotations are optional; unannotated values are inferred
3. Type annotations are optional
4. If a type is specified, it is enforced at compile time
5. If no type is specified, the compiler infers it from the initializer
6. Once a type is specified, it cannot change
7. Rey is designed to be simple and easy to learn

## Structure

```
rey-lang/
├── compiler/           # Reference interpreter implementation
│   └── v1/
│       ├── src/
│       │   ├── ast/        # AST node definitions (expr, stmt, lit, ty)
│       │   ├── lexer/      # Lexical analysis (cursor, token, span)
│       │   ├── parser/     # Recursive descent parser
│       │   ├── interpreter/# Tree-walk interpreter (evaluator, executor, environment)
│       │   ├── tests/      # .rey test files
│       │   └── main.rs     # CLI entry point
│       ├── Cargo.toml
│       └── target/         # Build artifacts
├── spec/               # Language specification (functions.md, variables.md, types.md)
├── releases/           # Pre-built binaries (macOS arm64, Windows x86_64)
├── README.md           # Language overview
├── CONTRIBUTING.md     # Contribution guidelines
└── AUTHORS.md          # Author info
```

## Build & Run

The compiler is written in Rust. Build and run using Cargo:

```bash
cd compiler/v1
cargo build --release
./target/release/rey-v0 <path-to-.rey-file>
```

Example:
```bash
cd compiler/v1
cargo run -- simple.rey
```

## Conventions

From the codebase and CONTRIBUTING.md:

- **Rust style**: Standard Rust conventions (mod.rs for modules, separate files per concern)
- **Naming**: snake_case for Rust code, camelCase for Rey language identifiers
- **Comments**: Explain *why*, not *what* — only for non-obvious logic
- **Simplicity**: Prefer explicit over clever, small functions, no unnecessary abstractions
- **Spec-first**: The `spec/` directory defines what Rey is; implementation must follow spec

## My role as contributor

- This file describes repo structure and conventions for assistants/contributors.
- The Rust reference interpreter lives at `compiler/v1`.
- The experimental bootstrap compiler (Rey → LLVM IR → native) lives at `rey-compiler/`.

## Forward v0.4 architecture — 2026-10-21

- The implementation proposal is [docs/V0.4-EXECUTION-PLAN.md](docs/V0.4-EXECUTION-PLAN.md). It supersedes the old v0.4 LLVM/C architecture; it does not claim implementation or amend SPEC.md by itself.
- The goal is a compiler, optimizer, native backend, executable writer/linker, and runtime implemented in Rey. Clang/LLVM and the historical C runtime belong only to recorded bootstrap history, not the production dependency closure.
- Implement in gated slices: baseline, typed IR, direct native backend/runtime, correct reclamation, native self-hosting, inferred memory optimizations, pipeline optimization, incremental builds, platform ports, and release validation.
- Preserve shared identity, aliases, specified errors, and arithmetic semantics. Performance goals require workload-specific measurements; allocator nanoseconds do not establish compilation or application runtime.

## v0.1.0 snapshot

- Full import system is implemented:
  - `export pub` for importable function exports
  - file and module import syntax
  - compile-time import resolver with visibility checks
- Language/runtime includes enums, match, structs, tuples, lambdas, `instanceof`, nullable and union type annotations.
