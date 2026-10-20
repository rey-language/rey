# Primer — rey-lang

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
