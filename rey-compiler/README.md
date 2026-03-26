# rey-compiler (bootstrap, v5.0)

This folder is the start of a compiler written in Rey, intended to eventually
compile Rey itself (a "Rey-in-Rey" bootstrap compiler).

Right now this is **API surface only**: it defines the core structs/enums and
the public function signatures for the lexer, parser, typechecker, codegen, and
diagnostics layers. Implementations will come in later milestones once the v1
Rust interpreter is stable enough to host and iterate on the bootstrap.

How this fits the roadmap:
- `compiler/v1/` is the current Rust interpreter (the shipping compiler/runtime today).
- `rey-compiler/` is the long-term self-hosted compiler target (v5.0 bootstrap).

Status:
- Types: defined
- Public APIs: defined
- Implementations: not started

