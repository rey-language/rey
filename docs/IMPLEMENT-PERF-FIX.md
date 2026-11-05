# Performance / self-compile fix — 2026-11-03

## One thing breaking
The optimizer passes (fold, propagate, copy, dead, range, inline, bounds, loops, loads, zero, fuse) exist in `rey-compiler/src/ir/optimize/` but the production `native-main.rey` / `ir-main.rey` drivers only call `foldConstants`. Region/lifetime metadata is not updated by passes, so they are excluded from the region runtime path entirely. The self-hosted x64 emitter (S1→S2→S3) therefore emits un-optimized code, producing ~21 ms numeric / ~23 ms churn vs v3 LLVM's ~6 ms / ~2.5 ms, and the full bootstrap teardown is unverified because fast-exit skips integrity checks. Binary writer also omits `chmod +x`.

## Plan
1. Wire `--passes` / `--fold-constants` into `native-main.rey` without altering region verification.
2. Recompute region facts when a pass touches allocation/escape (only inline/specialize need this; safe passes like fold/copy/dead do not).
3. Add executable-bit write in x64 native image output; verify `Path` alias guard.
4. Run matched benchmark (same CPU pin, same workload) comparing optimized vs baseline S3.
5. Complete clean S3 teardown verification (not fast-exit) and record SHA-256 fixed point.

## Transition decision — 2026-11-04

The existing LLVM route is not a drop-in backend for the v0.4 managed path. `codegen.compileLLVM` consumes LLVM text generated from a checked source program, invokes `llc` or `clang`, then links a prebuilt C runtime archive. `native-main.rey` instead lowers through `buildCompletedRegionProgram` and emits a native image directly. Routing v0.4 through the old LLVM driver would bypass the current region and promotion lowering unless a managed-IR-to-LLVM backend and matching runtime ABI are implemented.

Use the LLVM compiler only as a temporary reference/bootstrap tool while optimizing and validating the direct native path. Keep it out of the v0.4 release and ordinary build path. The work should proceed in these gates:

1. Freeze a clean direct-native baseline: successful normal-teardown self-build, reproducible S2/S3 outputs, matched compile-time/RSS/runtime measurements, and current region/promotion checks.
2. Improve the direct path first with existing optimization passes that preserve allocation and lifetime facts. Keep allocation-changing transforms disabled until their metadata and region-verification rules are explicit.
3. If LLVM is used for differential performance work, label its output as legacy and compare identical source/runtime semantics. Do not count that result as a v4 self-host or region-path result.
4. Optimize direct code generation and compiler memory use until its measured regressions are understood and reduced; retain region/promotion lowering on that path.
5. Gate completion on a clean rebuild with compiler tools absent, subprocess and dynamic-library inspection, region/promotion correctness, stable memory behavior, and matched benchmark results.

This is a large compiler project, not a one-switch fix. The LLVM route can provide a useful baseline, but it cannot by itself deliver the combined v4 requirements. No tests or benchmarks were run for this planning update.

## Agents
- A: optimizer wiring (`native-main.rey`, `ir-main.rey`)
- B: region recomputation / verification guard (`ir/managed/lifetime/`, `ir/verify/`)
- C: writer executable bit + output safety (`native/image/x64/`)

Scope per agent is exclusive; only coordinator commits.
