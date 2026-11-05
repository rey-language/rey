# Changelog

## [unreleased] — performance work in progress
- Region handles now combine the owning registry's live mapping identity with a registry-wide serial, including a unique serial for each program region. This prevents stale handles from resolving after context destruction, context address reuse, or across distinct live registries while the registry remains live. Added native Linux x64 regressions for region and program handles. Handles are scoped to the registry lifetime.
- Changed region slab free lists to retain released blocks beyond the old 128-block cap, added slab/cache byte counters, and reduced promotion scratch arrays. Added bounded local lifetime proofs for some large-program allocations, managed call-graph pruning, indexed function lookups, an x64 adjacent-result reload optimization, and a trusted output path without image readback. See `docs/V0.4-PERFORMANCE-WORK.md` for focused verification and remaining work.
- Follow-on work adds occupancy-aware slab release, indexed promotion worklists, bounded large-program region proofs, an experimental escape summary and leaf-hook proof, opt-in Linux x64 and ARM64 register plans, fewer repeated vector checks, cached byte-character strings, embedded runtime and standard-library sources, and a trusted self-host packed-output hook. Placement now conservatively handles managed addresses converted to integers, and cohort proofs reject untracked derived addresses. Linux and Darwin managed host sources are selected by target; Windows remains fail-closed. Raw pointers remain caller-trusted; region handles are scoped to a live registry.
- Added requested-byte and block-count metrics for direct region mappings alongside slab/cache counters; these do not represent RSS or page-rounded bytes.
- Packed eight string-literal bytes per store, added managed assertions and scalar printing support, and made rejected destination proofs fall back to the program region. Focused Linux x64 runtime, compiler, embedded-library and native output checks pass; a single 8192-byte literal build dropped from 4.17 s/1,531,488 KiB peak RSS to 2.15 s/772,076 KiB. Full regression and bootstrap verification remain open.
- A packed-literal Linux x64 build produced S1 in 1:21.37 with 9,457,912 KiB peak RSS. S1 compiled and ran scalar, managed and isolated embedded-stdlib fixtures; fixed-point self-hosting remains unverified. All 16 region-source programs passed native Linux x64 in normal and promotion-stress modes.
- Updated direct region fixtures for smaller initial chunks and extended the Linux self-host script's default timeout to two hours; its dry run retains the S2/S3 comparison.
- Linux self-host runs now stage sources in a fresh directory without deleting older stages, avoiding stale source contamination, and display GNU time metrics; a fake-seed staging and smoke check passed.
- Added a per-lexer byte-string cache with a lazy table for longer varied inputs. Seed and managed-native differential fixtures agree on token/error output and spans, including NUL and high bytes. One concurrent repeated-tokenization run used 62.04 s/348,160 KiB peak RSS versus 82.75 s/520,692 KiB before; tiny-input runs were slightly slower.
- Final context destruction now validates region/chunk accounting, releases live direct mappings, and unmaps slabs in bulk. Focused native teardown, reclamation, handle, promotion and accounting checks pass; full bootstrap and broad runtime verification remain open.
- Restored full output readback for normal builds and added an explicit trusted `--selfhost-fast-exit` option that skips final region teardown. Focused flag, output-failure and image-equality checks pass. An S1-to-S2 build with the one-entry lexer cache returned status 0 in 27:15.31 with 9,468,928 KiB peak RSS; S1/S2 are byte-identical and S2 passes scalar smoke. A later adaptive-cache S1-to-S2 build completed with fast exit in 26:35.95 at 9,511,636 KiB peak RSS; the images are byte-identical and S2 passes a scalar smoke. The seven remaining updated direct runtime accounting fixtures and the strengthened container assertion pass natively. S2 built S3 with fast exit in 26:43.18 at 9,508,904 KiB peak RSS; all three images are byte-identical, and six S3 smoke cases pass.
- Normal context teardown now uses one complete region/group/chunk validation walk to link direct chunk records through an unused metadata word, then releases direct mappings from that list before slab cleanup. This removes the second metadata walk without scratch allocation. Five current-runtime Linux x64 reclamation fixtures pass, including mixed destruction under a 16 MiB VM limit and 512 direct chunks; a temporary `used > capacity` corruption case is rejected. These focused tests do not establish latest-source bootstrap completion.
- Added experimental Linux x64 packed emission and output integration behind `--experimental-packed-x64` and `--trust-packed-output`. The packed writer's EINTR, short-write, and persistent-failure injection checks pass. A frozen packed S1-to-S2 snapshot was byte-identical (SHA-256 `400c7431647b4f2571ff7e6274ffbf314d29c36ccb8bc9f1bb857f13263827e3`), exiting 0 in 27:27.85 at 9,281,216 KiB peak RSS; it used trusted fast exit and therefore skips normal teardown integrity checks. Sparse emitter verification remains in progress, and a full bootstrap from the latest source remains unverified.
- Restored scan-based scalar use counting after reverting the numeric count index because its performance was not measured. The final safety guards preserve allocation metadata while its call remains and limit layout-definition removal to unused `ConstI64` instructions; the focused managed-native scalar fixture passed (`/tmp/rey-v04-scalar-index-review/run9.log`). A read-only independent audit found no concrete valid-IR issue, but did not run builds or tests. The full managed packed-x64 emitter fixture timed out at 15 minutes (`/tmp/rey-v04-packed-x64-managed.4Rx1UX`). An earlier pre-fix candidate H built S1 in 1:33.31 at 9,178,696 KiB peak RSS, with SHA-256 `1635693c5d3650e4f554a40c30061794b84edf55932aef76db11433d554bd373` in `/tmp/rey-v04-current-integration.Di6IKZ`; the seed process that wrote that H was stopped under swap pressure. An earlier pre-fix normal-teardown S1-to-S2 attempt wrote S2 with SHA-256 `320dfc449acbc68e0b3944b80b95dc29a8183b8a5fc8e26571db82f6cb12aa6a`, different from S1; S2 `--help` and scalar smoke pass. Its producer was SIGTERM'd after 20 minutes of swap-heavy teardown (48:30.47 total, 1,876,575 major faults, 9,347,456 KiB peak RSS). The log records “Command terminated by signal 15” despite `/usr/bin/time`'s exit status 0, so clean exit is unverified. Artifacts: `/tmp/rey-v04-current-integration.Di6IKZ`.
- Fixed x64 `RawOffset` to trap on negative bases and offsets before checked addition, matching the evaluator and raw-memory contract while retaining the existing trap path. Seed-built evaluator, native, and normal/verified/reuse/packed differential fixtures pass. Current native exits are valid 103 and 1 for negative base, negative offset, overflow, and null base; a frozen pre-fix control returns 255 for negative base and 50 for negative offset. Current run: `/tmp/rey-v04-rawoffset-contract-20261103d/run.log`; pre-fix control and exits: `/tmp/rey-v04-rawoffset-baseline-20261103a`. Full current-source bootstrap and clean normal teardown remain unverified.
- The compact x64 `RawOffset` check is live, independently reviewed, and passed the Linux seed native fixture (`/tmp/rey-v04-live-raw-offset.0ROICe`). It saves 18 bytes and two fixups per offset versus the initial correctness fix in focused cases. Compact-source H-to-S1 exited 0 in 1:31.41 / 9,391,992 KiB; S1 hash `d82f9253fbcb20d88044b7e93e3076b822e22f923d928cdd4bdf01d261c808c4`. S1 `--help` and scalar smoke pass. Compact S1-to-S2 with trusted fast exit exited 0 in 27:50.78 / 9,271,312 KiB with 39,215 major faults; S2 hash `c05638d77cf4de4528fc7e35d034e7a1f7749efd3179594f3445800e3cd5a0ed` differs from S1. S2 `--help` and scalar smoke pass. Fast exit skips normal context teardown and integrity checks. S2 passed a cyclic-container graph under `--promotion-stress` with normal teardown; ordinary and packed images and symbol maps matched, and both outputs ran. Compact S2-to-S3 used trusted fast exit and wrote an S3 image byte-identical to S2 (SHA-256 `c05638d77cf4de4528fc7e35d034e7a1f7749efd3179594f3445800e3cd5a0ed`). The S3 smoke suite passed with `PATH` empty: `--help` and all six Linux x64 fixtures passed; the overflow case produced its expected exit 1, and variadic managed+regions passed. Results: `/tmp/rey-v04-compact-s3-smoke.cAW6Yi/results.txt`. The monitoring session was interrupted; the process is gone, the original producer exit status and peak metrics are unavailable, and its log reached the image output path. These smoke results verify the produced S3 binary, but successful producer exit, normal full-build teardown, and a fixed point remain unverified. Clean full self-build teardown remains unverified.
- The compact and prior sign-corrected self-build snapshots differ only in `native/x64/main.rey`; emitted code changed from 29,410,615 to 27,416,458 bytes (-1,994,157), and fixups from 949,350 to 727,261 (-222,089). This is not a matched speed comparison. The accidental untracked `--help/readiness.json` artifact is preserved under the no-deletion rule. Artifacts: `/tmp/rey-v04-compact-integration.dJlsyX`.
- IR codec prototype checksum controls used the same isolated candidate H executable and snapshot (859,717 bytes, 301 functions) in sequential runs while S3 ran concurrently. Load-plus-verify took 33.54 s / 683,720 KiB, checksum/read/conversion alone took 25.26 s / 26,056 KiB, and decode-plus-verify took 35.44 s / 156,360 KiB; all exited 0. The 1.90 s difference between load-plus-verify and decode-plus-verify is approximate arithmetic, and contention affected the measurements. The codec remains a prototype, not production; these results support no performance attribution. Artifacts: `/tmp/rey-ir-codec-proto.2xX56W`.
- On the corrected RawOffset source, H-to-S1 exited 0 in 1:32.58 / 9,175,972 KiB with SHA-256 `54db04707ed0366bef2d07930d290b899dc70e8150c4fa1b6ebbbbb5fb0c486c`. S1-to-S2 using trusted fast exit exited 0 in 31:34.02 / 9,299,496 KiB; S2 hash `42768dfa4869997127f3a54fad95916d64de3b9325771f73ce8bfede36ec6733` differs from S1. S2 `--help` and scalar smoke pass; other experiments were concurrent, and fast exit bypasses normal teardown and its integrity checks. The later S2-to-S3 attempt was stopped at 11:48 before output as source improved, so no fixed point is established. Full normal teardown remains unverified. Artifacts: `/tmp/rey-v04-rawoffset-integration.rSOrPT`.

## [unreleased] — v0.4 implementation
- Replaced the production managed path with compiler-inferred invocation/program regions and address-preserving allocation-group promotion. The region runtime has no tracing collector, GC roots/safepoints, reference counting, or public heap allocation; unknown persistent escapes reach the program region, ending a region releases its remaining groups, and the compiler verifies placement, stores, region-end dominance and cycles before lowering. Added return promotion before region end (including nullable returns and a null no-op), container current-region allocation, a fail-closed legacy region builder, and a region-only source harness. Final region matrix passes 967 checks with 124 Darwin and 124 Linux ARM64 images; direct runtime 17/17; compiler static 21; frontend candidate 70/70; scalar/native 240; raw 444; optimization/reuse 396; historical fixed heap/GC 402; x64 structural 8; seed conformance 154/154, E2E 11/11 and probes 2/2. Container methods and unknown callees remain conservative program-region escapes, so caller-region retention and RSS/timing are not claimed. Retired managed-runtime/managed-containers suites require explicit `--historical-managed`; the default memory harness is fixed heap/GC only. See [region validation](docs/V0.4-REGION-VALIDATION.md).
- Historical managed-root batch (superseded by region-only correction): added compiler-generated precise roots, exact tracing layouts for structs/tuples/tagged values/strings/vectors/maps, managed collection safepoints, inferred fixed-layout regions and construction in caller destinations. Main source integration passes 1,429 checks with 370 native images, supplemental generics 534 with 160 images, and managed runtime 191 per slot mode, including actual Darwin/Linux ARM64 execution and native teardown assertions. Added bounded root-cell sharing, per-function liveness/storage budgets, sixteen-machine-argument support and malformed reference-tail rejection. Recursive immutable equality fails explicitly; uncertain lifetimes use the tracing heap. Full v0.4 release, self-hosting and performance validation remain ahead.
- Added a pure Rey fixed-layout heap and explicit-root iterative tracing collector. The 44 allocator and 10 collector cases pass 402 coordinator checks per mode, with 54 executions per ARM64 OS in default and slot-reuse modes (216 total), including native 128-node collect-before-allocation stress. Added opt-in `--reuse-slots`: 396 source checks and 69 direct-IR checks pass, with a measured 336-to-16-byte chain frame reduction. The existing 240-check scalar/native regression passes. Balanced binary encoding fixes quadratic copying in the historical seed harness. At that checkpoint, compiler roots, source aggregates and regions remained pending; their follow-on is recorded above. Native self-hosting and broad performance validation remain ahead; x64 execution is limited to unstable QEMU emulation.
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
## [fix] — v0.4 region handles
- Handle tokens now use the owning registry mapping page for their 36-bit namespace and registry-wide monotonic 27-bit region serials, preventing collisions across live registries and revival after context recreation.
- Added two-registry and destroy/recreate ABA coverage, with generated Linux x64 execution in the region-runtime runner; updated the embedded runtime and region contract.
