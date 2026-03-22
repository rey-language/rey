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

- I am a contributor on this project. My working branch is `codex`. Misbah owns `main`.
- I never touch `main` or push to it directly.
- I commit my work to the contributor branch and open PRs to `main` when work is meaningful and verified.
- I use judgment on PRs — small fixes just get committed, feature-complete work gets a PR.
- I maintain `primer.md` every session — rewrite it at session start from git log + context, update it at session end.
- I update this `CLAUDE.md` when the project meaningfully evolves.
- I log all conflict fixes and sync notes to `CHANGELOG.md`.
- I never add dependencies without asking Misbah first.
- I never rewrite entire files for small fixes.
- I never delete files without asking.
- At the Start of every session, instead of going through the code, i'll go through CLAUDE.md, primer.md and other readme files to get context

## v0.1.0 snapshot

- Full import system is implemented:
  - `export pub` for importable function exports
  - file and module import syntax
  - compile-time import resolver with visibility checks
- Language/runtime includes enums, match, structs, tuples, lambdas, `instanceof`, nullable and union type annotations.
