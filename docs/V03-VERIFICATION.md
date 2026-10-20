# v0.3 snapshot verification ledger

## Result

- A serial release build of the verifier snapshot succeeded.
- The Rust test suite compiled and ran: **69 passed, 9 failed**. The failures are legacy test fixtures rejected by the current lowercase-type rules.
- The conformance runner's valid-program preflight passed. Against its goldens, **51 passed, 17 failed, 68 total**. This is not a green conformance result.
- Those results are historical for the fingerprinted snapshot. The struct fixture correction, private-field negative, and CLI args API wiring described below were made afterward.
- The latest direct-source run built successfully; tests report **104 passed, 9 failed**, and conformance reports **102 passed, 23 failed, 125 total**. These are not green gates; current evidence is below.
- The earlier accidental `tests/imports/imports` duplicate remains as recorded below. No further snapshot copy was made for the current-source run.
- During the initial verification pass, an accidental copy-helper typo invoked `cp -R` with `tests/imports` as both source and destination, creating a nested `tests/imports/imports` duplicate in the source checkout. It was not deleted. The first snapshot was discarded; the later snapshot fingerprint matches the checkout as it stood after that incident, but is not a pristine pre-incident baseline. The subsequent fixture and CLI edits are explicitly authorized and listed below. Parent review/cleanup is required before relying on the checkout as clean.

## Snapshot identity

- Source root: `$REPO_ROOT`
- Snapshot: `/tmp/rey-v03/verifier-20261016T180754Z/verified-snapshot`
- Corrected snapshot file timestamps: approximately `2026-10-16T18:09:04Z`.
- Included: `compiler/v1/src`, `compiler/v1/Cargo.toml`, `compiler/v1/Cargo.lock`, `tests/imports`, `tests/conformance`, and `scripts`; no source-repo `target/` or `.git` was copied.
- Fingerprint manifest: 281 files. SHA-256 of each manifest file (`verified-before.sha256`, `verified-after.sha256`, `verified-snapshot.sha256`):
  `a3e6b278fe2429994906775b23547f23bf3dcd1fb6cff53b283ff70ab7e9c7ef`
- Before, after, and snapshot manifests compared equal on the corrected copy. The equality describes the checkout after the duplicate-directory incident, not a clean checkout before it.
- Snapshot and failed logs are preserved under `/tmp/rey-v03/verifier-20261016T180754Z/`.

## Build and unit-test evidence

Exact commands, run in the snapshot's `compiler/v1` directory:

```sh
CARGO_TARGET_DIR=/tmp/rey-v03/verifier-20261016T180754Z/build-target cargo build --release -j1
CARGO_TARGET_DIR=/tmp/rey-v03/verifier-20261016T180754Z/test-target cargo test --release -j1
```

Build tail:

```text
warning: `rey-v0` (bin "rey-v0") generated 5 warnings
Finished `release` profile [optimized] target(s) in 16.82s
exit=0
```

Test summary:

```text
running 78 tests
test result: FAILED. 69 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out
```

All nine failures are old `compiler/v1/src/tests` fixtures rejected before their runtime assertions:

| Test | Observed diagnostic | Category |
|---|---|---|
| `tests::fsAndProcessBuiltinsMore` | `use lowercase 'string' instead of 'String'` | Fixture migration / typecheck |
| `tests::integerDivisionSemantics` | `use lowercase 'void' instead of 'Void'` | Fixture migration / typecheck |
| `tests::matchReportsNonExhaustivePatterns` | `use lowercase 'void' instead of 'Void'` | Fixture migration / typecheck |
| `tests::matchEnumVariantsQualifiedAndUnqualified` | `use lowercase 'void' instead of 'Void'` | Fixture migration / typecheck |
| `tests::matchStructPatterns` | `use lowercase 'void' instead of 'Void'` | Fixture migration / typecheck |
| `tests::runtimeReportsIndexBoundsWithIndexAndLength` | `use lowercase 'void' instead of 'Void'` | Fixture migration / typecheck |
| `tests::runtimeUsesReferenceEqualityForArrayAndStruct` | `use lowercase 'void' instead of 'Void'` | Fixture migration / typecheck |
| `tests::structFieldMutationPubWorks` | `use lowercase 'void' instead of 'Void'` | Fixture migration / typecheck |
| `tests::structFieldMutationNestedErrorsClearly` | `use lowercase 'void' instead of 'Void'` | Fixture migration / typecheck |

The follow-up CLI tests for argv parsing, strict `check`/`build` args, silent/non-executing `check`, source-coordinate mapping, usage, and missing-source exit status passed in this suite.

## Conformance evidence

Exact command (run from the snapshot root):

```sh
REY_CONFORMANCE_SCRATCH=/tmp/rey-v03/verifier-20261016T180754Z/runner-scratch /tmp/rey-v03/verifier-20261016T180754Z/verified-snapshot/scripts/conformance.sh --engine /tmp/rey-v03/verifier-20261016T180754Z/build-target/release/rey-v0
```

The runner's valid-program check preflight passed before fixtures ran. Negative fixtures were counted as passing only when `check` exited exactly 1 and emitted the marker; usage errors and false-success checks failed. Summary:

```text
Engine preflight PASS
Summary: 51 passed, 17 failed, 68 total
```

Concise failures, grouped by observed phase/category (not individual-agent blame):

**Syntax/parser (2)**

- `control/enum-match-variants-payload` — valid fixture rejected: `Expected ',' between enum variants.`
- `types/enum-payloads` — valid fixture rejected: `Expected ',' between enum variants.`

**Typecheck/validation (8)**

- `control/enum-match-qualified-bare` — enum field access rejected (`field access is not supported on Enum("Direction")`).
- `control/negative-break-outside-loop` — expected check exit 1, got 0.
- `control/negative-continue-outside-loop` — expected check exit 1, got 0.
- `control/negative-nonexhaustive-enum` — expected non-exhaustive diagnostic; got enum field-access error instead.
- `funcs/immutable-capture-higher-order` — valid function type rejected as unknown annotation `(int) -> int`.
- `funcs/negative-mutable-capture` — expected check exit 1, got 0.
- `types/conversions` — valid conversion rejected (`undefined variable 'int'`).
- `types/erased-generics` — `Vec(Any)` versus `Vec(String)` assignment rejected.

**Observed interpreter/golden mismatches (6; root cause not independently adjudicated)**

- `control/lexical-shadowing` — expected `20\n10\n`, got `20\n20\n`.
- `control/range-inclusive-endpoints-fallback` — expected two `range` lines then `fallback`; got two `fallback` lines.
- `exprs/short-circuit` — expected one `boom` then `true`; got three `boom` lines then `true`.
- `exprs/string-concat-interpolation` — run exited 1 after `hello rey`; stderr: `Invalid binary operation`.
- `funcs/parameter-shadowing` — expected `99\n7\n7\n`, got `99\n7\n99\n`.
- `types/union-instanceof` — expected `rey`, got `not string`.

**Invalid positive fixture and checker gap (1)**

- `types/struct-fields` originally declared every `Profile` field private, then attempted to mutate `age` and read all five fields externally. Its `check` passed, but `run` correctly rejected the mutation before the reads (`cannot mutate private field 'age' on struct 'Profile'`). This is a fixture defect, not evidence of a runtime privacy bug; the successful check also exposes a typechecker privacy-enforcement gap.

Post-snapshot fixture correction: all five fields in `types/struct-fields/main.rey` are now `pub`, preserving their types, program operations, and `expected.stdout`; the positive fixture passed in the latest conformance run. Added `types/private-field-mutation/`, which uses the same grammar for a private `age` mutation, requires the diagnostic substring `private`, and has a zero-byte `expected.stdout`. The latest run observed check exit 0 for that negative, confirming the checker privacy gap; it is not recorded as passing.

No conformance-runner failure was observed; its preflight and golden comparisons ran. The full historical output is preserved in `conformance.log`.

## Binary and log fingerprints

- Binary: `/tmp/rey-v03/verifier-20261016T180754Z/build-target/release/rey-v0`
- Binary mtime UTC: `2026-10-16T18:09:46Z`
- Binary SHA-256: `4fae4ee7f2ea9bfe25083decf38d803c27a8105ee8325b31bf36aa7ce938fdee`
- Logs: `build.log`, `test.log`, and `conformance.log` in the snapshot-work directory above.

## argv handoff

The agreed convention is interpreted `args()` = `[script, ...userArgs]` and native `args()` = `[executable, ...userArgs]`; user arguments follow the CLI `--` separator. The CLI constructs `[file, ...parsedArgsAfterSeparator]` and passes that vector to `Interpreter::new_with_args`. The normalized-argv CLI regression passed in the latest test run. Native `main.rey` still uses historical fixed indexes (`cliArgs[1]` compiler path, `[2]` command, `[3]` input) and must be reconciled with the agreed native convention. No end-to-end bootstrap claim is made.

## Metadata-aware check integration

`run_file` and `check_file` share `resolveAndCheck`: it resolves once, calls `TypeChecker::checkResolvedProgram(&program)`, and reports either resolver diagnostics or `LocatedTypeError` using that diagnostic's own `file`, `source`, `error.span`, and `error.message`. Successful `check` still returns without constructing an interpreter or printing. `run` performs this module-aware precheck and then calls `Interpreter::interpret_resolved(&program, path)`.

Added `checkRejectsPrivateImportsAndLocatesImportedErrors`: it exercises a private grouped import (check exit 1, root import-site file/source) and an imported module's return-type error (check exit 1, imported `math/main.rey` file/source and line). Its scratch directories are unique `std::env::temp_dir()` children and assert lexical disjointness from `CARGO_MANIFEST_DIR`. This test passed in the latest test run.

Runtime errors from `interpret_resolved` return only `String` with no assured origin/span. The CLI reports them as `<runtime>:?:?: error: ...`; it does not reinterpret byte-offset hints using entry-source text. Runtime module-initialization order and empty-binding imports remain a handoff risk and were not independently covered; no claim is made for those cases.

### Earlier compile blocker (before the metadata API landed)

Exact command from `compiler/v1` (isolated target; no snapshot copy):

```sh
CARGO_TARGET_DIR=/tmp/rey-v03/argv-integration-target cargo test --release -j1
```

Compilation stopped before tests ran because the peer-owned typechecker does not handle newly added AST variants:

```text
error[E0004]: non-exhaustive patterns: `&stmt::Stmt::EnumDeclWithPayload { .. }` not covered
  --> src/typecheck.rs:660
error[E0004]: non-exhaustive patterns: `&stmt::Pattern::EnumVariantPayload { .. }` not covered
  --> src/typecheck.rs:879
error[E0004]: non-exhaustive patterns: `&expr::Expr::EnumVariant { .. }` not covered
  --> src/typecheck.rs:996
error: could not compile `rey-v0` (bin "rey-v0" test) due to 3 previous errors
exit=101
```

Full output is preserved at `/tmp/rey-v03/argv-integration-test.log`.

### Earlier runtime-representation compile blocker (before the runtime API landed)

One bounded direct-source attempt, after the metadata-aware `main.rs` patch, used:

```sh
CARGO_TARGET_DIR=/tmp/rey-v03/metadata-check-target cargo test --release -j1
```

It stopped before tests ran with seven peer-owned `E0308` errors after runtime representation changes:

```text
src/interpreter/executor.rs:278 — range pattern compares &char with &u8 (two errors)
src/interpreter/std.rs:959 — HashMap<String, Value> where Dict expects HashMap<Vec<u8>, Value>
src/interpreter/std.rs:1232 — HashMap<String, Value> where Dict expects HashMap<Vec<u8>, Value>
src/interpreter/tests.rs:195 — HashMap<String, Value> where Dict expects HashMap<Vec<u8>, Value>
src/interpreter/tests.rs:457 — Value::Char('Z') passes char where u8 is required
src/interpreter/tests.rs:492 — Value::Char('B') passes char where u8 is required
error: could not compile `rey-v0` (bin "rey-v0" test) due to 7 previous errors
exit=101
```

Full output is preserved at `/tmp/rey-v03/metadata-check-test.log`. That blocker was superseded by the successful build in the latest direct-source run below.

## Latest direct-source verification (2026-10-16T19:13:30Z)

No snapshot copy was made. Rust inputs and runtime fixtures were fingerprinted before and after the run: 442 files each; before and after manifest SHA-256 both equal `e220a166e45c85c166503dc3914ba503a6399e29f1707ce36d88c1f9f0e80b7a`. No concurrent input changes were detected.

Exact commands from `compiler/v1`, using the same fresh isolated target:

```sh
CARGO_TARGET_DIR=/tmp/rey-v03/runtime-consumer-20261016T191330Z/target cargo build --release -j1
CARGO_TARGET_DIR=/tmp/rey-v03/runtime-consumer-20261016T191330Z/target cargo test --release -j1
```

Build tail:

```text
warning: `rey-v0` (bin "rey-v0") generated 9 warnings
Finished `release` profile [optimized] target(s) in 15.38s
exit=0
```

Test result:

```text
running 113 tests
test result: FAILED. 104 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out
```

The new CLI tests passed, including imported-private rejection/imported-file location, normalized argv injection, and silent non-executing `check`. The nine remaining unit failures are fixture/typecheck migration failures:

- `tests::fsAndProcessBuiltinsMore`: fixture uses capitalized `String`.
- `tests::integerDivisionSemantics`, `matchReportsNonExhaustivePatterns`, `matchEnumVariantsQualifiedAndUnqualified`, `matchStructPatterns`, `runtimeReportsIndexBoundsWithIndexAndLength`, `runtimeUsesReferenceEqualityForArrayAndStruct`, `structFieldMutationNestedErrorsClearly`, `structFieldMutationPubWorks`: fixtures use capitalized `Void`.

Because the release build succeeded, the conformance runner was run on that fresh binary, including after the unit-test failures:

```sh
REY_CONFORMANCE_SCRATCH=/tmp/rey-v03/runtime-consumer-20261016T191330Z/conformance-scratch $REPO_ROOT/scripts/conformance.sh --engine /tmp/rey-v03/runtime-consumer-20261016T191330Z/target/release/rey-v0
```

The runner's valid silent-check preflight passed. It enforced exact check exit/diagnostic markers and byte-exact stdout goldens:

```text
Engine preflight PASS
Summary: 102 passed, 23 failed, 125 total
```

Failures grouped by observed phase (not individual-agent blame):

**Typecheck/check diagnostics (18)**

- `control/enum-match-qualified-bare` — enum field access rejected: `field access is not supported on Enum("Direction")`.
- `control/negative-break-outside-loop`, `control/negative-continue-outside-loop` — both checks exited 1, but the expected `outside of loop` marker did not match `only valid inside a loop`.
- `control/negative-nonexhaustive-enum` — enum field-access error appeared instead of its expected non-exhaustive marker.
- `data/hashmap-delete-collision-chain`, `data/hashmap-status-delete` — `HashMap.new` unknown static method.
- `data/option-lifecycle` — unknown `Option::Some`; `data/result-lifecycle` and `data/result-unwrap-err-on-ok` — unknown `Result::Ok`; `data/result-unwrap-err-int` and `data/result-unwrap-err-string` — unknown `Result::Err`.
- `data/vec-literal-for-reverse-join` — `reverse` not found on `Vec(Any)`; `data/vec-new-mutations` and `robustness/capture-aggregate-shared` — `Vec.new` unknown static method.
- `modules/nested-diamond` — undefined `shared` in imported `right/main.rey`.
- `robustness/unsupported-conversion-source` — expected `toInt` marker but got a `Vec.new` static-method error.
- `types/erased-generics` — rejected `HashMap(Any)` versus `HashMap(Int)` assignment.
- `types/private-field-mutation` — expected private-field check failure, got exit 0. This confirms the outstanding checker privacy gap; it is not a pass.

The two out-of-loop negative markers above were fixture wording mismatches, not checker false-successes: both checks exited 1 with `break is only valid inside a loop` / `continue is only valid inside a loop`. Their marker files now use the meaningful shared substring `only valid inside a loop`. These two cases were rerun individually against the same fresh binary and passed; this is a fixture-only correction, not a new full-suite result.

**Parser (1)**

- `data/tuple-return` — rejected with `Expected '->' after function parameter types.`

**Observed runtime/golden mismatches (3; cause not independently adjudicated)**

- `data/aggregate-alias-identity` — produced `true`, `2`, `8`, then `<runtime>:?:?: error: Invalid binary operation`; final expected `true` was absent.
- `data/dict-read-write-len` — produced `1`, `2`, `4`, then `<runtime>:?:?: error: Namespace function 'len' not found`; final expected `2` was absent.
- `robustness/float-to-int-out-of-range` — expected exit 1, got 0, and printed `9223372036854775807` beyond the expected output.

**Fixture/runner input (1)**

- `rey-lang/tests` — directory lacked required `main.rey` and `expected.stdout`.

This malformed `tests/conformance/rey-lang` directory remains unresolved. Cleanup was not authorized, so it was neither deleted nor skipped; the recorded 125-case full run still includes its failure.

The corrected positive `types/struct-fields` passed; its private-mutation negative failed as described above. Runtime module initialization ordering and empty-binding imports were not separately verified.

Fresh binary:

- Path: `/tmp/rey-v03/runtime-consumer-20261016T191330Z/target/release/rey-v0`
- mtime UTC: `2026-10-16T19:13:56Z`
- SHA-256: `cf9e85894d863e76f8bc38ac6b26a6eca76c01d318021460419556b52fdd6f33`
- Logs: `build.log`, `test.log`, `conformance.log`; before/after input manifests are in the same `runtime-consumer-20261016T191330Z` scratch directory.

### Targeted marker follow-up

The SPEC requires `break` and `continue` outside a loop to be errors; the actual checker diagnostics are `break is only valid inside a loop` and `continue is only valid inside a loop`. Only the two corresponding `expect_check_fail` marker files changed. Each case was then run separately through the runner against the existing binary at `/tmp/rey-v03/runtime-consumer-20261016T191330Z/target/release/rey-v0`; both targeted runs passed. No build, full-suite rerun, or count update was performed. The unrelated malformed `rey-lang/tests` directory remains a recorded full-run failure.

Exact commands and outputs:

```sh
REY_CONFORMANCE_SCRATCH=/tmp/rey-v03/targeted-break $REPO_ROOT/scripts/conformance.sh --engine /tmp/rey-v03/runtime-consumer-20261016T191330Z/target/release/rey-v0 --filter negative-break-outside-loop
Engine preflight PASS
PASS control/negative-break-outside-loop
Summary: 1 passed, 0 failed, 1 total

REY_CONFORMANCE_SCRATCH=/tmp/rey-v03/targeted-continue $REPO_ROOT/scripts/conformance.sh --engine /tmp/rey-v03/runtime-consumer-20261016T191330Z/target/release/rey-v0 --filter negative-continue-outside-loop
Engine preflight PASS
PASS control/negative-continue-outside-loop
Summary: 1 passed, 0 failed, 1 total
```

## Snapshot-copy guard correction

Before any future snapshot copy:

1. Require absolute source and destination paths. Inspect every existing path component with `lstat` before canonicalization; reject symlink components rather than following them. Reject symlinks anywhere inside the source tree and refuse an existing destination.
2. Resolve the checked paths and reject equality or either path being an ancestor of the other. Keep build/output directories outside the source tree.
3. Hash sorted relative file paths and contents before copying. After copying, re-hash the source and snapshot; accept only if both manifests match. If source changes during the copy, retry once to a fresh disjoint destination; if the retry also changes, mark the snapshot unreliable.

The guard was exercised only under external scratch at `/tmp/rey-v03/verifier-20261016T180754Z/copy-guard-selftest.py`; it did not read or write the source checkout. A tiny scratch-only tree was copied after validation and its manifests compared. Output:

```text
PASS absolute disjoint source/destination accepted
PASS copied scratch manifest matches
PASS same path rejected
PASS destination ancestor rejected
PASS destination descendant rejected
PASS symlink path rejected without following
PASS symlink inside source tree rejected
```

## Fresh full direct-source verification (2026-10-17)

This is the latest unfiltered verification, run directly against the working
tree without a source snapshot. The previous **119/125** result is superseded;
this run still has one strict conformance failure and is not an all-pass gate.

Fresh isolated target:
`/tmp/opencode/recovery/full-verification-20261017/target`

Exact commands, run from `compiler/v1` except the conformance runner, which ran
from the repository root:

```sh
CARGO_TARGET_DIR=/tmp/opencode/recovery/full-verification-20261017/target cargo build --release -j1
CARGO_TARGET_DIR=/tmp/opencode/recovery/full-verification-20261017/target cargo test --release -j1
REY_CONFORMANCE_SCRATCH=/tmp/opencode/recovery/full-verification-20261017/conformance-scratch $REPO_ROOT/scripts/conformance.sh --engine /tmp/opencode/recovery/full-verification-20261017/target/release/rey-v0
```

Build completed successfully (`cargo build` exit 0; 7 existing warnings).
The full, unfiltered Rust test run completed:

```text
running 134 tests
test result: ok. 134 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

The full conformance runner preflight passed and ran all **125** depth-2 case
directories without filters or skips:

```text
Engine preflight PASS
Summary: 124 passed, 1 failed, 125 total
```

All 124 case directories containing both required files (`main.rey` and
`expected.stdout`) passed: **124/124 valid complete cases**. The sole strict
failure is the known, malformed `tests/conformance/rey-lang/tests` directory;
it was counted and not deleted, skipped, or hidden:

```text
rey-lang/tests
  missing required file: main.rey
  missing required file: expected.stdout
```

The inventory captured **154 recursive directories** under
`tests/conformance` and **125** runner case directories. The before/after
directory and case inventories are identical (inventory SHA-256
`7d97b026445eda225346b2e59e0085be629961a38cb179b0cca077e418960073`). The
malformed directory is explicitly listed with both required-file flags false.

Input fingerprints were taken before and after the build/test/conformance
window (2026-10-16 21:16:29Z–21:18:56Z). Each category digest is over sorted
relative-path records, including directory entries and file size/content
SHA-256 records:

| Inputs | Before | After | Result |
|---|---|---|---|
| Rust source (`compiler/v1/src`, `.rs` files) — 6 dirs, 27 files | `d127343dee7e5649819d76a96367f2f758f08999ba61d0d2aad61fb0794e94a5` | same | unchanged |
| Cargo inputs (`compiler/v1/Cargo.toml`, `Cargo.lock`) — 2 files | `53c06e3403881cfbd82461167230178711ea78fab2d2b65c0fde943834c84eb5` | same | unchanged |
| Fixtures (`compiler/v1/src/tests`, `tests/imports`, `tests/conformance`) — 180 dirs, 412 files | `e31060918483e94e141e1e81b3c9482f64a70fa623728deb5ff87c52967608b1` | same | unchanged |
| Runner inputs (`scripts`) — 2 dirs, 16 files | `bbe182ac31c2a22a1541dfeced8d357eb82af38926bd79ae96c33a79b0cec990` | same | unchanged |
| Native `.rey` inputs (`rey-compiler`, separate from Rust inputs) — 18 dirs, 74 `.rey` files | `e08b4d9fd9dbd70009df9f769ac935c2ab31baae9f35ea9de9922e468e84aff6` | `986126010f37fc374f7802b1db2148fc99344b86d0484a2125e9bda3b57b9053` | changed during window |

The native `.rey` change is separate from Rust/Cargo/fixture/runner coherency:
one file, `rey-compiler/src/diagnostics/main.rey`, had an mtime inside the
fingerprint window. The four Rust-verification input categories remained
identical across the run. This verification exercised the Rust interpreter
only; it provides no native compiler or native `.rey` execution proof.

Fresh binary:

- Path: `/tmp/opencode/recovery/full-verification-20261017/target/release/rey-v0`
- SHA-256: `68375435d32fe32daebb631b8b2314fc3ea0a595ca12ff112a8ffb6100989740`
- The pre-existing `/tmp/opencode/recovery/interpreter-runtime-target/release/rey-v0` was not used as the build target; its hash after verification is identical.
- Logs and complete input/case inventories: `/tmp/opencode/recovery/full-verification-20261017/logs/` (`build.log`, `test.log`, `conformance.log`, `before-fingerprint.log`, `after-fingerprint.log`, `coherency-comparison.log`).

## Full gate after nominal-owner integration (2026-10-17)

The supplied nominal-owner release binary matched its expected SHA-256 before
and after verification, so it was used directly without rebuilding. This is a
new full gate; the earlier 134/134 unit result is historical and superseded.

Exact commands:

```sh
CARGO_TARGET_DIR=/tmp/opencode/recovery/nominal-owner-typecheck-target cargo test --release -j1
REY_CONFORMANCE_SCRATCH=/tmp/opencode/recovery/full-verification-nominal-owner-20261017/conformance-scratch $REPO_ROOT/scripts/conformance.sh --engine /tmp/opencode/recovery/nominal-owner-typecheck-target/release/rey-v0
```

The full unfiltered Rust test run had one remaining failure:

```text
running 141 tests
test result: FAILED. 140 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
tests::matchReportsNonExhaustivePatterns
got: $REPO_ROOT/compiler/v1/src/tests/match_non_exhaustive.rey:8:11: error: TypeError: non-exhaustive match for enum '$REPO_ROOT/compiler/v1/src/tests/match_non_exhaustive.rey::Direction'
```

Full conformance preflight passed. All 124 complete cases passed; the strict
result is **124 passed, 1 failed, 125 total**. The only failure is still the
unapproved malformed `tests/conformance/rey-lang/tests` directory, counted in
the run with both `main.rey` and `expected.stdout` absent. It was not changed,
skipped, or hidden. The inventory had 154 recursive directories and 125
depth-2 cases; its before/after SHA-256 is
`7d97b026445eda225346b2e59e0085be629961a38cb179b0cca077e418960073`.

Input fingerprints (sorted relative-path records include directories and
file size/content hashes) were taken before and after the 2026-10-17
09:17:20Z–09:18:13Z verification window:

| Inputs | Before | After | Result |
|---|---|---|---|
| Rust source (`compiler/v1/src`, `.rs` files) — 6 dirs, 27 files | `468053f60401e7e25c577b4acc10c0e65d2a47cf736ea4066ef3a7e2066e0a63` | same | unchanged |
| Cargo inputs — 2 files | `53c06e3403881cfbd82461167230178711ea78fab2d2b65c0fde943834c84eb5` | same | unchanged |
| Fixtures (`compiler/v1/src/tests`, `tests/imports`, `tests/conformance`) — 180 dirs, 412 files | `e31060918483e94e141e1e81b3c9482f64a70fa623728deb5ff87c52967608b1` | same | unchanged |
| Runner inputs (`scripts`) — 2 dirs, 16 files | `bbe182ac31c2a22a1541dfeced8d357eb82af38926bd79ae96c33a79b0cec990` | same | unchanged |
| Native `.rey` inputs (`rey-compiler`) — 18 dirs, 74 files | `be01f78ff26b1b3d6ae078fb8b607a2a6efdcd260beb3f843063423a74b12ebc` | `4fe18acb046e29ef3c24dd98ada0a56b514932c4146391a4cc6b5e89d5d28fd2` | changed separately |

Two native production sources had mtimes in the verification window:
`rey-compiler/src/typecheck/main.rey` and `rey-compiler/src/parser/main.rey`.
They are outside the Rust/Cargo/fixture/runner inputs and do not establish
native compiler behavior. This gate exercised only the Rust interpreter.

Binary used:

- Path: `/tmp/opencode/recovery/nominal-owner-typecheck-target/release/rey-v0`
- SHA-256 before and after: `42e104af8b14b760aed42b5f8afa06372f8110676339366454dc47d35c6fd746`
- Logs and complete inventories: `/tmp/opencode/recovery/full-verification-nominal-owner-20261017/logs/` (`test.log`, `conformance.log`, `before-fingerprint.log`, `after-fingerprint.log`, `coherency-comparison.log`).

## Conformance after built-in static generic arguments (2026-10-17)

The preceding full release unit run for the built-in static-argument checker
passed **147/147**; its test and build logs are under
`/tmp/opencode/recovery/builtin-generic-checker-target/logs/`.
This verification did not rerun unit tests or rebuild. The supplied oracle hash
matched before and after the full conformance run.

Exact unfiltered command:

```sh
REY_CONFORMANCE_SCRATCH=/tmp/opencode/recovery/conformance-builtin-static-20261017/scratch $REPO_ROOT/scripts/conformance.sh --engine /tmp/opencode/recovery/builtin-generic-checker-target/release/rey-v0
```

The runner preflight passed. It ran all **125** depth-2 cases, without filters
or skips:

```text
Engine preflight PASS
Summary: 124 passed, 1 failed, 125 total
```

All 124 complete cases passed. The sole strict failure remains the unapproved
malformed `tests/conformance/rey-lang/tests` directory; it was counted and not
deleted, skipped, or hidden:

```text
rey-lang/tests
  missing required file: main.rey
  missing required file: expected.stdout
```

The case inventory recorded 154 recursive directories and 125 runner cases;
the before/after directory and case inventories are identical (SHA-256
`7d97b026445eda225346b2e59e0085be629961a38cb179b0cca077e418960073`). The
malformed directory is present with both required-file flags false.

Input fingerprints were captured before and after the 2026-10-17
09:42:27Z–09:43:21Z run. Digests are over sorted relative paths, directory
entries, and file size/content hashes:

| Inputs | Before | After | Result |
|---|---|---|---|
| Rust source (`compiler/v1/src`, `.rs` files) — 6 dirs, 27 files | `c1d5aa05cc9a9d4286f2ea9dd2dca08567f8b0818d8cd058a3ba9209fc370b8a` | same | unchanged |
| Cargo inputs — 2 files | `53c06e3403881cfbd82461167230178711ea78fab2d2b65c0fde943834c84eb5` | same | unchanged |
| Fixtures (`compiler/v1/src/tests`, `tests/imports`, `tests/conformance`) — 180 dirs, 412 files | `e31060918483e94e141e1e81b3c9482f64a70fa623728deb5ff87c52967608b1` | same | unchanged |
| Runner inputs (`scripts`) — 2 dirs, 16 files | `bbe182ac31c2a22a1541dfeced8d357eb82af38926bd79ae96c33a79b0cec990` | same | unchanged |
| Native production `.rey` files (`rey-compiler`) — 18 dirs, 74 files | `8306a9601cfd8c98770fe13ea158861d5fe09308862acf14fe1d339f255ef7a6` | `d9316e750d05283d16a92a837e1c8a7cae5be6e3d2aacbc5be5b243fd455a380` | changed separately |

Two native production files had mtimes in the window:
`rey-compiler/src/typecheck/main.rey` and `rey-compiler/src/codegen/main.rey`.
These are separate from the Rust reference-interpreter inputs. This was a Rust
interpreter conformance run, not proof that the in-progress native AST/tag,
parser, checker, or codegen pipeline works.

Oracle used:

- Path: `/tmp/opencode/recovery/builtin-generic-checker-target/release/rey-v0`
- SHA-256 before and after: `da15cbef37c2005edee3dd4cbed48347154534d461cd3a190980c0e7998014cc`
- Logs and inventories: `/tmp/opencode/recovery/conformance-builtin-static-20261017/logs/` (`conformance.log`, `before-fingerprint.log`, `after-fingerprint.log`, `coherency-comparison.log`).
