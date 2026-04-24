# Rey v0.2.0 — Native Compiler (Self-Hosted)

**Released:** 2026-04-24  
**Platform:** macOS arm64

---

## What's in this release

`rey-macos-arm64` is the **native Rey compiler** — compiled from Rey source code itself, without the Rust interpreter in the critical path.

This is the first release where the compiler is built **from its own source** (`rey-compiler/main.rey`) using the LLVM backend, producing real native binaries via `clang`.

---

## How to use

```sh
# Build a Rey program
./rey-macos-arm64 <path/to/main.rey> build <your-program.rey>

# Run (compile + execute)
./rey-macos-arm64 <path/to/main.rey> run <your-program.rey>
```

The first argument is the compiler's own source path (used to locate the runtime library). For a clean setup, use the `reyc` launcher from the repo root which handles this automatically.

---

## What changed since v0.1.0

### LLVM backend (new)

A full LLVM IR emitter was added under `rey-compiler/`. Programs now compile to native machine code via:

```
.rey source → Lexer → Parser → LLVM IR emitter → clang → native binary
```

### C runtime library

A C runtime (`rey-compiler/runtime/`) implements all heap operations:

- **Strings** — heap-allocated, length-prefixed UTF-8 (`rey_str_*`)
- **Vec** — dynamic arrays with push/pop/get/set/map/filter/join (`rey_vec_*`)
- **HashMap** — string-keyed hash maps (`rey_hm_*`)
- **Result / Option** — tagged union values (`rey_ok`, `rey_err`, `rey_some`, `rey_none`)
- **IO** — `readFile`, `writeFile`, `fileExists`, `listDir`, `exec`
- **Process** — `args`, `exit`, `exec`

### Self-hosting

The compiler compiles itself. Phase H complete:

- `rey-compiler/main.rey` compiled to native binary by `rey-v0` (Rust bootstrap)
- Native binary passes all e2e tests identically to the Rust-hosted compiler

### Value representation

All Rey values are `i64` in LLVM IR:

- Scalars (`int`, `bool`, `float`) — stored directly
- Heap values (`String`, `Vec`, `HashMap`, `struct`, `Result`, `Option`) — pointer cast to `i64`
- Struct header: 16-byte prefix with a type tag byte for `instanceof` checks

---

## Known limitations

- **macOS arm64 only** — no Windows or Linux binary yet
- **No garbage collector** — arena allocation, freed at process exit (correct for short-lived compiler processes)
- **No closures over mutable state** — lambdas are lifted to top-level functions
- **Typechecker is a stub** — type inference works but full static type checking is not enforced
- **Self-compile crashes** — the native compiler can compile all e2e fixtures but panics on its own complex source (remaining type-annotation gaps in codegen)

---

## Verified tests

All e2e fixtures pass under the native compiler:

| Test | Status |
|------|--------|
| hello | ✅ |
| math | ✅ |
| loops | ✅ |
| functions | ✅ |
| structs | ✅ |
| enums | ✅ |
| strings | ✅ |
| vec | ✅ |
| collections | ✅ |
| io | ✅ |
| imports | ✅ |

---

## Bootstrap chain

```
rey-v0 (Rust)  →  rey-compiler/main.rey  →  rey-macos-arm64
```

The Rust interpreter remains in the repo as a reference implementation and fallback. It is not required to run compiled programs.
