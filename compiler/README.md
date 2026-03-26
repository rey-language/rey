# Compiler Layout

This repo currently contains two compiler tracks:

## `compiler/v1/`
The shipping Rust implementation for Rey v0.

It is a reference interpreter + typechecker + import resolver used for day-to-day
language development and releases.

## `rey-compiler/`
The long-term bootstrap target: a compiler written in Rey ("Rey-in-Rey").

At the moment this is API-only (types + public function signatures) so we can
stabilize the architecture and start iterating once the v1 Rust runtime is
stable enough to host it.

