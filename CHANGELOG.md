# Changelog

## [unreleased] — v0.4 implementation
- Added a pure Rey fixed-layout heap and explicit-root iterative tracing collector. The 44 allocator and 10 collector cases pass 402 coordinator checks per mode, with 54 executions per ARM64 OS in default and slot-reuse modes (216 total), including native 128-node collect-before-allocation stress. Added opt-in `--reuse-slots`: 396 source checks and 69 direct-IR checks pass, with a measured 336-to-16-byte chain frame reduction. The existing 240-check scalar/native regression passes. Balanced binary encoding fixes quadratic copying in the historical seed harness. Compiler roots, source aggregates, regions, native self-hosting and broad performance results remain pending; x64 execution is limited to unstable QEMU emulation.
- Added immutable fixed tracing-layout descriptors, semantic SSA liveness and conservative per-block constant folding behind `--fold-constants`. Runtime integration passes 444 checks with 77 Darwin and 77 Linux ARM64 executions. Folding integration passes 367 checks, including 26 Darwin and 24 executions per Linux target (x64 emulated); 150 exact bigint arithmetic cases and the existing 240-check regression pass separately. Fixed empty/duplicate/missing CLI option handling. These are groundwork for collection and storage placement, not automatic memory or performance results.
- Added the pure Rey owned raw byte-buffer API with geometric growth and exact mapping release, plus explicit trusted runtime failures. The expanded gate passes 261 checks and 47 executions per ARM64 OS. A separate frontend-only comparison retains identical 62/70 expected results with eight baseline mismatches and no regressions.
- Added authorized raw-address checking, precise raw IR operations/effects, bounded evaluator mappings with address-cell provenance, and direct ARM64 OS memory mapping/access. The source/native gate passes 145 checks and 32 executions each on Darwin and Linux arm64. These are manual trusted primitives, not an automatic collector or inferred regions.
- Added the direct Linux x86-64 scalar emitter, SysV ABI and pure Rey ELF64 writer. Execution caught and fixed a division-guard relocation bug. The x64 gate passes 64 source and two direct-IR executables under emulation in the ARM64 Linux VM; the same 64 sources pass on matching Linux arm64 (196 checks per backend). Native x64 hardware and other x64 platforms remain unverified.
- Extended checked scalar arithmetic, unary/boolean operators, short-circuit CFGs and module-graph function lowering. The expanded native gate passes 238 checks and 59 macOS arm64 executions; an independent 55-program arithmetic oracle passes 227 process checks. Nine emitted ELF programs pass 31 checks in a Linux arm64 VM. These are correctness results, not performance or release claims.
- Added explicit-seed capability/baseline runners, matched seven-language numeric/vector benchmark fixtures, and a scalar typed IR model with deterministic printing.
- Added the checked scalar frontend/IR pipeline with conservative effects, mutable local/while CFG lowering, precise memory/runtime ABI contract, and parallel implementation ledger. Scalar integration passes 26 checks. Pure Rey byte encoding and SHA-256 pass standard/binary/padding-boundary cases.
- Added a direct scalar ARM64 emitter and pure Rey Mach-O/ELF image writer, including embedded Mach-O signing and OS loader metadata. The first native gate passes 114 checks and 24 real macOS arm64 executions, including source builds with compiler tools absent from PATH. ELF AArch64 is structurally checked only. Full language/native runtime support, self-hosting, collector, regions, tasks and release ports remain pending.
- The pinned seed passes 154 conformance and 11 native E2E cases. The focused frontend baseline retains six failures; details and artifact paths are recorded in `docs/V0.4-PARALLEL-PHASES.md`.

## [unreleased] — v0.4 execution plan
- Added `docs/V0.4-EXECUTION-PLAN.md` for a pure Rey compiler/backend/runtime, explicit seed-to-native bootstrap, correct reclamation, inferred regions and promotion, incremental Capy builds, and all five platform gates.
- Marked the earlier LLVM/C-based v0.4 proposal historical and updated contributor handoff/context links. This is documentation only; performance targets and milestones are not implementation results.

## [0.3.1] — patch release
- Vec capacity growth now uses `realloc` instead of allocating and zeroing a second full buffer; only live kind metadata is initialized. Large Vec workloads used 36–48% less peak RSS in the current macOS arm64 comparison.
- Recovered the native typechecker and checked-program backend required by the existing driver; the tracked modules had been an old checker stub and backend. Source provenance and limits are recorded in `docs/BOOTSTRAP-SOURCE-RECOVERY.md`.
- The five-target release workflow rebuilds the tracked Rust source bootstrap, builds stage 1, and uses stage 1 to rebuild the packaged native compiler. Every target rebuilds the runtime from tracked sources; Windows uses Win32-native file and process I/O with matching MSYS2 LLVM tools. The compiler requires `rey 0.3.1` and emits the host triple. Manual dispatch builds packages without publishing; tag publication requires all five builds.
- All five hosted package-build preflight targets pass: macOS arm64/x86-64, Linux x86-64/AArch64, and Windows x86-64. The tag-triggered publication workflow repeats the builds and requires all five assets. No conformance test suites were run for this patch.

## [unreleased] — performance
- Capy skips the per-command `rey --version` subprocess during normal toolchain resolution; `capy doctor` still checks the actual compiler version.

## [release] v0.3.0 — 2026-10-20
- **Multi-Stage Self-Hosting**: Verified native self-hosting pipeline (`scripts/bootstrap.sh`). Stage 1 builds Stage 2; Stage 2 passes 100% of the conformance suite and reproduces Stage 3.
- **Full Language Conformance**: 154 / 154 conformance tests pass natively (100% test pass rate).
- **Native E2E Suite**: 11 / 11 end-to-end fixtures passing natively with LLVM IR backend.
- **Zero Vector Storage Overhead**: Eliminated secondary metadata allocation overhead for unboxed vectors (`Vec<int>`, `Vec<bool>`, `Vec<string>`); `kinds` array is lazily allocated only for polymorphic/tagged slots.
- **Link-Time Optimization (LTO)**: Release builds emit LLVM bitcode and invoke Clang/lld with `-O3 -flto`, enabling cross-module function inlining across the runtime ABI.
- **SIMD Vectorization & Restrict Pointers**: Added `restrict` pointers to `ReyVec` data buffers and accessor helpers, enabling LLVM loop vectorization (NEON/AVX SIMD loops).
- **Scalar Evolution & SSA Register Promotion**: Emitted `add nsw i64` on loop counters and iterators; hoisted local variable allocas into function entry blocks for LLVM `mem2reg` optimization.


## [pause] — 2026-10-17
- Stopped active v0.3 frontend/backend jobs and the orphaned self-check at the owner's request; source and artifacts were preserved, with no landing or cleanup.
- Recorded completed reference/native/runtime work, verification limits, the interrupted nominal-layout integration, exact stop fingerprints, and remaining gates in `docs/HANDOFF-2026-10-17.md`.
- Refreshed the primer's current-state pointer without removing its historical content. v0.3 remains incomplete; Stage 1/Stage 2 self-hosting is not verified.

## [fix] — 2026-04-10
- Parser semicolon errors are now standardized to `error[syntax]: expected ';'` with existing file/line/column reporting.
- Enforced explicit comma separators between `match` arms (`Expected ',' between match arms.`).
- Enforced explicit comma separators between enum variants (`Expected ',' between enum variants.`).
- Enforced explicit comma separators for struct field declarations in `struct` bodies.
- Runtime binary/unary ops now report null use as `null dereference at line <n>`.
- Added array/string bounds diagnostics: `index out of bounds (i=..., len=...)`.
- Added runtime equality support for arrays and structs with reference-equality semantics.
- Match runtime now reports non-exhaustive branches as `error[match]: non-exhaustive patterns`.
- Added parser regression tests:
  - `compiler/v1/src/tests/syntax_missing_semicolon.rey`
  - `compiler/v1/src/tests/match_missing_comma.rey`
  - `compiler/v1/src/tests/enum_missing_comma.rey`
  - `compiler/v1/src/tests/runtime_ref_equality.rey`
  - `compiler/v1/src/tests/runtime_index_oob.rey`
  - `compiler/v1/src/tests/match_non_exhaustive.rey`
  - unit assertions in `compiler/v1/src/main.rs`
- Updated syntax reference to explicitly state semicolon-required statements and comma separator rules for structs/enums/match.

## [release] — 2026-03-27
- Bumped compiler crate version to `0.2.0` in `compiler/v1/Cargo.toml`.
- Fixed string indexing: `source[i]` now returns single character as String.
- Added type check support for string index type validation.
- Type errors now include span information for better diagnostics.

## [release] — 2026-03-26
- Bumped compiler crate version to `0.1.1` in `compiler/v1/Cargo.toml`.
- Fixed `match` on struct instances via struct field patterns.
- Fixed enum variant matching for both qualified (`Enum::Variant`) and unqualified (`Variant`) patterns.
- Improved grouped import diagnostics so missing items highlight the specific missing name.
- Verified nested module resolution where `module/main.rey` imports other local files.
- Fixed numeric semantics by distinguishing `int` and `float` values at runtime:
  - `10 / 3` performs integer division.
  - `10.0 / 3.0` and `10 / 3.0` perform float division.
- Allowed external mutation of `pub` struct fields (`obj.field = ...`, `obj.field += ...`) and emit a clear error for nested field assignment (`obj.inner.field = ...`) for now.
- Added `rey-compiler/` v5 bootstrap skeleton (API surface only).
- Added `compiler/README.md` to document the v1 Rust interpreter vs bootstrap target.

## [release] — 2026-03-23
- Prepared `v0.1.0` release candidate assets and docs.
- Audited syntax documentation against current parser/runtime behavior.
- Added root `RELEASE.md` covering `v0.0.1-pre` through `v0.1.0`.
- Added packaged release folder:
  - `releases/0.1.0/rey-v0-macos-arm64`
  - `releases/0.1.0/RELEASE.md`
- Bumped compiler crate version to `0.1.0` in `compiler/v1/Cargo.toml`.
- Updated README current version and import-system feature status.
- Cleaned compiler warnings (unused imports/variables/dead methods) and restored import parsing regression for `module::item`.
- Fixed static call parser bug so `StructName.create(...)` resolves method names correctly.
- Updated stale test fixture `compiler/v1/src/tests/test_rand.rey` to reflect current type checking behavior.

## [feature] — 2026-03-23
- Implemented full import system for Rey with compile-time resolution.
- Added `export pub` function modifier and import visibility enforcement:
  - `export pub func` => importable
  - `pub func` => not importable
  - `func` => private
- Added parser/AST support for:
  - `import file.symbol`
  - `import file.{a,b}`
  - `import module`
  - `import module::file`
  - `import module::{a,b}`
- Added import resolver pipeline (`compiler/v1/src/imports.rs`) and integrated it into compiler entry flow.
- Implemented resolver lookup order:
  1. current file directory
  2. project root (entry file directory)
  3. `~/.reyc/std/src` for `std` prefix
  4. `~/.reyc/packages`
- Added module semantics:
  - `module/main.rey` required for `import module`
  - module namespace auto-collects all `export pub` functions from `.rey` files in folder
  - `module::file` resolves direct file namespace
- Added import diagnostics for:
  - file not found
  - missing module `main.rey`
  - function not found
  - function is `pub` but not `export pub`
  - circular imports
  - duplicate imports
- Added runtime/typecheck namespace dispatch support for `namespace.func()`.
- Added import fixtures under `tests/imports/` for success and all required error cases.

## [release] — 2026-03-19
- Shipped `rey v0.0.7-pre`
- Added `projects/fake-cli/cli.rey` - a full interactive TUI implementation in Rey.
- Fixed struct field/method resolution bugs and optimized typechecking for complex expressions.
- Enhanced property access and indexing consistency.

## [release] — 2026-03-19
- Shipped `rey v0.0.6-pre`
- **Implemented full Struct System**: 
  - Data structures with named fields.
  - Instance methods with direct field scoping (no more `self.x` inside methods for fields!).
  - Static-style methods for construction and utilities.
  - Visibility control with `pub` (private by default).
  - Method overloading support.
  - Struct literals for easy instantiation.
- Added `structs.rey` comprehensive test file.
- Improved field access error messages with "did you mean?" suggestions using Levenshtein distance.

## [release] — 2026-03-19
- Shipped `rey v0.0.5-pre`
- Added string interpolation syntax `"{var}"` and mixed type string conversions (`"a" + 1`)
- Built-in type conversions: `.toString()`, `.toInt()`, `.toFloat()`
- Added multiple arguments to `println` and new `print()` equivalent
- Built-in functions: `abs()`, `max()`, `min()`, `random()`
- Added `const` values. Unannotated `var`s are now treated as dynamic types!
- Comprehensive rewrite of Error diagnostics to a visual snippet pointing format.

## [tooling] — 2026-03-17
- Added Linguist submission prep files: `languages/Rey.yaml`, `languages/samples/Rey.rey`, `LINGUIST.md`

## [fix] — 2026-03-17
- Added array methods: `.length()` and `.push()`
- Allowed typed empty array assignment like `var xs: [int] = []`
- Added `compiler/v1/src/tests/array_methods.rey` regression test

## [fix] — 2026-03-19
- Fixed struct method resolution with typed parameters - methods now properly resolve when parameters have struct type annotations
- Fixed string interpolation with nested quotes - parser now handles `{scores["alice"]}` syntax using escaped quotes
- Fixed array type parsing in struct fields - `[int]`, `[String]`, etc. now work as field types
- Added comprehensive test file `compiler/v1/src/tests/struct_advanced.rey` covering all three fixes
- Claude initialized as contributor
- Created CLAUDE.md and primer.md
- Branch `claude` created off `main`

## [release] — 2026-03-17
- Shipped `rey v0.0.3-pre` on `codex`
- Added lexer support for skipping `//` comments
- Removed compiler warnings and cleaned unused parser/interpreter/lexer surfaces
- Hardened parser flow to prevent panic after lexer failure
- Updated/fixed test fixtures so every `.rey` file in `compiler/v1/src/tests/` runs cleanly
- Built and staged release binaries:
  - `releases/0.0.3-pre/rey-v0-macos-arm64`
  - `releases/0.0.3-pre/rey-v0-windows-x86_64.exe`
- Added `releases/0.0.3-pre/RELEASE.md`
