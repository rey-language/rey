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
- Builtins: println(), len(), push(), pop(), input()
- Arrays: literals, indexing, typed arrays ([int])
- Array methods: length(), push()
- Dictionaries: literals, indexing, typed dicts ({String:int})
- String methods: length/upper/lower/contains/split
- Property access: obj.prop (dictionary key lookup)
- Compile-time type enforcement for annotated vars/functions + common builtins
- Entry point: calls main() if present

## Test files
compiler/v1/src/tests/ — .rey files for each feature
Run any of them with cargo run -- src/tests/.rey

## Current status
`rey v0.0.4-pre` is implemented and staged on `codex`:
- all files in `compiler/v1/src/tests/` run successfully
- `cargo build --release` succeeds
- release binaries + notes are staged in `releases/0.0.4-pre/`

## For next session
- Consider tightening the language spec (what is int vs float at runtime, truthiness rules, dictionary key restrictions).
- Add negative tests for type errors once there's a harness for expected-failure cases.
