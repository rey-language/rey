# Primer — rey-lang

## Active v0.4 implementation — 2026-10-21

- Restart recovery completed: the fixed layout registry, SSA liveness and optional scalar folding are verified. The coordinator's runtime gate passes 444 checks with 77 executions per ARM64 OS; folding integration passes 367 checks, including 26 Darwin executions and 24 executions per Linux target (x64 emulated). The independent arithmetic gate passes 150 exact cases and four Darwin executions; existing scalar/native regression passes 240 checks. See the parallel ledger for frozen evidence. No collector, slot reuse or performance claim follows from this checkpoint. Next: fixed-layout heap/explicit-root collector and conservative stack-slot planning.
- User authorized phased parallel implementation using GPT-6 Luna at medium effort, with each worker testing after its assigned code and fixtures are written. The coordinator reviews diffs and runs integration gates before advancing automatically.
- Integration branch: `codex-v0.4`, starting at plan commit `b236b22`. Worker scopes are exclusive; only the coordinator changes shared contracts/driver wiring and performs Git operations.
- The expanded arithmetic/module-graph native gate passes 238 checks including 59 macOS arm64 executions. An independent source oracle passes 55 programs (227 process checks), and nine emitted ELF programs pass 31 checks in a Linux arm64 VM. Source builds run with compiler tools absent from PATH. The pinned seed baseline passes 154 conformance and 11 E2E fixtures and retains six focused frontend failures. See `docs/V0.4-PARALLEL-PHASES.md` for scope and artifacts.
- ARM64/x86-64 instructions, Mach-O/ELF headers and signatures are emitted in Rey. macOS requires OS dyld/libSystem startup; no Rey C runtime or external build linker is used in generated programs. The Linux x64 target passes 196 checks across 64 source programs under emulation, plus two direct-IR ABI executables; native x64 hardware remains unverified. Trusted raw-memory primitives are integrated as recorded below. Full source coverage, runtime reclamation/self-hosting, regions/promotion, structured tasks, caching/Capy, and remaining platform/release gates are pending.
- No v0.4 performance or native-independence claim is established by this handoff. Record observed gates as implementation proceeds.
- Trusted raw-memory checking, IR semantics and ARM64 primitives pass 145 coordinator checks including 32 Darwin and 32 Linux arm64 executions. The source pipeline exercises all bytes and signed extrema; unauthorized raw use is rejected. This is manual raw ownership, not collection/regions. The next workers implement owned byte buffers and an independent old/new frontend comparison. See `build/v04/integration-raw-1/result.json` and the parallel ledger.
- Owned raw byte buffers and direct runtime failure now pass the expanded 261-check gate, including 47 executions per ARM64 OS and 300 repeated ownership cycles in the evaluator. The standalone frontend comparison retains identical 62/70 expected results before/after raw checking, no regressions, distinct from the released-seed baseline. Next workers implement tracing-layout metadata, liveness analysis and conservative scalar folding. See `build/v04/integration-raw-buffer-1/result.json` and `build/v04/frontend-regression/parent-status-gate-1/results.json`.

## Current v0.4 planning handoff — 2026-10-21

- This session delivers a written plan, not compiler/runtime implementation. The contributor branch is `codex-v0.4-plan`, based on the released source at `0003e19`; the existing divergent `codex` branch and performance branch history are preserved.
- [docs/V0.4-EXECUTION-PLAN.md](docs/V0.4-EXECUTION-PLAN.md) is the forward proposal: a compiler, optimizer, native backend, executable writer/linker, and runtime in Rey, with no Clang/LLVM/C compiler/external linker in production builds or native self-rebuilds.
- Order: reproducible baseline; typed IR; direct ARM64/Mach-O and minimal Rey runtime; correct tracing reclamation; native self-hosting; escape analysis/stack placement/regions/destination construction/promotion; data-movement optimization; incremental compiler and Capy; all five platform ports and concurrency gates; packaging/release.
- The old v0.4 document is marked historical. Source semantics remain in SPEC.md. Low-level trusted primitives, bootstrap boundaries, alias preservation, precise roots, and measurable exit conditions are specified in the plan. Numerical performance goals are targets, not observations.
- Next implementation work is M0 baseline/seed-capability validation followed by M1's smallest verified IR slice. This planning session is documentation only. Validation covers five documents, nine local links, ordered M0-M9 milestones, ten exit conditions, and the staged Git diff; compiler/runtime suites were not run for this documentation change. Git publication is reported in the session result.
- Historical notes below predate completed v0.3.1 publication and must not be treated as current release status. Rey v0.3.1 and Capy v0.1.1 publication completed in the parent session; the current release workflow history is the source of build evidence.

## Current release repair — 2026-10-20, v0.3.1

- Preserved the shared `perf-build-memory` branch, all unrelated local edits, and the existing Vec realloc change. No tests, commits, pushes, or tags were run during this repair.
- Recovered the missing native checker and checked-program codegen as a matching pair from the retained `rey-bootstrap.3atk9E` source copy after comparing both bootstrap copies, tracked driver/parser APIs, and available Git history. This is a large source recovery, not a small API stub patch; see `docs/BOOTSTRAP-SOURCE-RECOVERY.md` for provenance and limits.
- The recovered source builds on macOS arm64 and reports exactly `rey 0.3.1`. The release builder rebuilds the tracked Rust interpreter, builds stage 1, then uses stage 1 to rebuild the packaged native compiler. It retains the matching v0.3.0 runtime archive with only `vec.o` replaced.
- All five release targets remain in the workflow. macOS x86_64, Linux x86_64/aarch64, and Windows x86_64 still need their native CI runs. Manual workflow dispatch builds without publishing; tag publication is gated on all five package builds. No renewed conformance or cross-platform readiness claim is made.
- Parent handles review, commits, tags, and publishing. Local build outputs and original edited-file backups are retained under ignored `build/` directories.

## Current handoff — 2026-10-20, performance iteration

- Active compiler branch: `perf-build-memory`; active Capy branch: `codex/perf-build-memory`.
- Rey Vec growth now uses `realloc` and only initializes live kind metadata. This cut peak RSS by 36–48% on the large sieve/vector/stream/transpose workloads in the current native benchmark run.
- Capy toolchain discovery now uses the pinned bundled version on the normal path; `capy doctor` checks the live compiler version when requested.
- Current macOS arm64 comparison: matching checksums on all 10 workloads. Rey is faster than C/Rust/C++/Zig on 6/10 and still trails on sieve, stream, transpose, and pointer chase. Capy build median is 0.394s versus 0.143s for direct `rey build`.
- Benchmark artifacts: `comparison/.build/perf-all-languages.csv`, `comparison/.build/perf-rebenchmark.csv`, and the associated metadata files. The comparison runner now honors `REY_RUNTIME_DIR` and includes Zig.
- Verification: runtime `make test` reports `ALL OK`; Vec regression has 6,687 assertions; typed-value normal and sanitizer runs pass; 70/70 language/workload checksum rows pass.
- Remaining work: reduce Capy command/build overhead, narrow the remaining Vec-heavy runtime gaps, and address process-lifetime allocations. No commit or push was made.

## Historical handoff — 2026-10-17, superseded by v0.3 release

- Read `docs/HANDOFF-2026-10-17.md` for the full last-day work report, exact stop fingerprints, evidence paths, and prioritized remaining work. The historical notes below are preserved, not current release claims.
- Both active agents and the orphaned compiler self-check were stopped. Do not resume development until the owner asks.
- Latest completed Rust checkpoint: 154 unit tests passed; all 124 complete reference conformance cases passed. Strict conformance still fails on the unapproved malformed discovery directory.
- Real native smoke, enum/match, and multi-unit programs ran. No Stage 1/Stage 2 compiler or full native conformance pass is verified.
- Current codegen is interrupted mid-integration: it references `CheckedNominalLayout` / `CheckedProgram.nominalLayouts`, but the checker has not added that metadata. Finish producer/consumer consistency before treating current source as buildable.
- No commits, pushes, tags, publishing, resets, or cleanup are authorized. Preserve existing WIP/stashes/artifacts. The sprint charter documents the in-place workflow override.

## Preserved historical primer

Last updated: Apr 10, 2026 (interpreter stabilization + bootstrap alignment)

## Session objective
Stabilize and align the Rust interpreter (`compiler/v1`) with syntax spec expectations and bootstrap behavior, starting with parser consistency and core runtime semantics.

## What was done this session

### Phase 1 - Syntax consistency (DONE)
- Standardized missing semicolon parser error to `error[syntax]: expected ';'`.
- Enforced comma separators between:
  - struct fields
  - enum variants
  - match arms
- Added parser regression coverage:
  - `compiler/v1/src/tests/syntax_missing_semicolon.rey`
  - `compiler/v1/src/tests/match_missing_comma.rey`
  - `compiler/v1/src/tests/enum_missing_comma.rey`
- Verified diagnostic output includes file + line + column via CLI path.

### Phase 2/3 - Runtime and match stabilization (PARTIAL DONE)
- Added array/string bounds diagnostics:
  - `index out of bounds (i=..., len=...)`
- Added runtime null guard in expression operations:
  - `null dereference at line <n>`
- Implemented equality semantics in runtime binary evaluation:
  - primitives and strings: value equality
  - arrays: reference equality
  - structs: reference equality
- Updated match fallback failure message to:
  - `error[match]: non-exhaustive patterns`
- Added runtime regression coverage:
  - `compiler/v1/src/tests/runtime_ref_equality.rey`
  - `compiler/v1/src/tests/runtime_index_oob.rey`
  - `compiler/v1/src/tests/match_non_exhaustive.rey`
  - direct executor-level null dereference unit assertion in `compiler/v1/src/main.rs`

## Verification run
- `cargo build --release` (pass)
- `cargo test` (pass, 14 tests)

## What's working now (v0.2.0+)
- Integer division, struct mutation, enum match, imports
- String indexing, parameter reassignment, return from nested blocks
- Vec: push, pop, len, get, set, contains, indexOf, map/filter/reduce, reverse/sort/slice/join
- HashMap: set, get, delete, has, len, keys/values/entries
- Stack: push, pop, peek, isEmpty, len
- Queue: enqueue, dequeue, peek, isEmpty, len
- Option: Some, None, unwrap, unwrapOr, isSome, isNone
- Result: Ok, Err, unwrap, unwrapOr, isOk, isErr
- `assert(condition, message)` builtin
- Math builtins: `floor`, `ceil`, `round`, `sqrt`, `pow`, `log`, `sin`, `cos`, `tan`

## Bootstrap compiler status
The bootstrap compiler is still **experimental**. It can compile the e2e fixtures under `rey-compiler/tests/e2e/` to native binaries using LLVM IR.

Run:
```bash
rey-compiler/tests/e2e/run.sh
```

## Blockers
- Self-hosting is not reached yet: native codegen/runtime needs a real lowering/runtime for `Vec`, `HashMap`, strings, IO/process, and more of the language surface.

(End file)
