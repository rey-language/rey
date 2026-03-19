# Primer — rey-lang
Last updated: Mar 19, 2026 (session start)

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
- Immutable variables with const (const pi: float = 3.14)
- Unannotated variables use dynamic typing (Ty::Any)
- Types: int, float, String, bool, null, Void
- Arithmetic, comparison, logical, assignment operators
- if/else, while, for x in range(start, end)
- break, continue
- Functions with optional typed params and return types
- Builtins: print(), println(), len(), push(), pop(), input(), abs(), max(), min(), random()
- Arrays: literals, indexing, typed arrays ([int])
- Array methods: length(), push()
- Dictionaries: literals, indexing, typed dicts ({String:int})
- String interpolation ("{var}"), mixed typings ("HP: " + 10)
- String methods: length/upper/lower/contains/split/toString/toInt/toFloat
- Property access: obj.prop (dictionary key lookup)
- Structs: definitions, literals, static/instance methods, field scoping, and pub/private visibility.
- Compile-time type enforcement for annotated vars/functions + common builtins
- Entry point: calls main() if present
- Rust/Miette-like visual Error Diagnostics.

## Test files
compiler/v1/src/tests/ — .rey files for each feature
Run any of them with cargo run -- src/tests/.rey

## Current status
`rey v0.0.7-pre` is in progress on `codex`:
- `v0.0.6-pre` shipped with full struct implementation.
- `fake-cli` project implemented in `projects/fake-cli/cli.rey`.
- Conflict resolved in `typecheck.rs` following rebase onto `master`.
- Preparing release artifacts for `v0.0.7-pre` (Mac & Windows).

## Next up (v0.0.7-pre batch)
- Build and package binaries for macOS (arm64) and Windows (x86_64).
- Finalize `v0.0.7-pre` release notes and changelog.
- Implement missing operators: `++`, `--`, `+=`, `-=`, `*=`, `/=`, `%=`, and `%` modulo.
- Add additional variable types: `char`, `uint`, `double`, `byte`.
- Add null safety: nullable types (`int?`), `null` comparisons, and clean error on `null` access.
- Add `try`/`catch` error handling.
- Update `syntax.md` and ensure all tests pass.
