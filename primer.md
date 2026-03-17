# Primer — rey-lang
Last updated: Mar 17, 2026 (session end)

## What this project is
Rey is a custom programming language built by Misbah. Currently on v0 — a tree-walking interpreter written in Rust. The language has C-like syntax with type inference, functions, control flow, and basic builtins. v0 is the working prototype; future versions will likely move toward compilation.

## Key architecture
```
compiler/v1/src/
├── lexer/        — tokenizer (cursor, token, span, error)
├── parser/       — produces AST (parser.rs, error.rs)
├── ast/          — AST node types (expr, stmt, literal, ty)
└── interpreter/  — tree-walker (evaluator, executor, environment, value, function, std, control_flow)
```
Pipeline: source → lexer → tokens → parser → AST → interpreter → output

## Build & run
```bash
cd compiler/v1
cargo build --release
./target/release/rey-v0 .rey

cargo run -- src/tests/variables.rey
```

## What's implemented in v0
- Variables with optional type annotations (var x = 10, var x: int = 10)
- Types: int, float, String, bool, null, Void
- Arithmetic, comparison, logical, assignment operators
- if/else, while, for x in range(start, end)
- break, continue
- Functions with optional typed params and return types
- println() builtin
- Entry point: func main(): Void

## What's NOT implemented yet
- Arrays ([1, 2, 3])
- Dictionaries ({key: value})
- Index access (arr[i], dict["key"])
- input() builtin
- String methods (.length())
- Property access (obj.prop)
- Type enforcement at compile time (parsed, not enforced)

## Test files
compiler/v1/src/tests/ — .rey files for each feature
Run any of them with cargo run -- src/tests/.rey

## Current status
`rey v0.0.3-pre` release work is complete on `codex`:
- lexer now skips `//` comments
- compiler builds clean with zero warnings
- parser no longer panics on lexer failures
- all files in `compiler/v1/src/tests/` run without lexer/parser/runtime errors
- release binaries and release notes are in `releases/0.0.3-pre/`

## For next session
- Start from this primer + CLAUDE.md
- Pick one limitation and implement it end-to-end (arrays or dictionaries are highest impact)
- Keep test fixtures aligned with supported syntax as parser evolves
