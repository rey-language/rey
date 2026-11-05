# Rey v0.4 matched baseline

This harness is an initial M0 baseline slice. It builds one source file and one executable per language/workload pair, so each competitor build contains the same single workload as the Rey build. The fixtures are independent programs with the same runtime input, signed 64-bit arithmetic, deterministic checksum, and processed-item count.

## Run

From the `rey-lang` repository root:

```sh
python3 benchmarks/v04/run.py
python3 benchmarks/v04/run.py --languages c,rey --workloads numeric,churn --size 20 --repeats 1 --warmups 0
python3 benchmarks/v04/run.py --languages rust,go --workloads numeric --size 100000
```

The default run uses 1,000,000 numeric iterations and 256 collection rounds. `--size` overrides the runtime input for every selected workload. Numeric is bounded at 10,000,000 iterations; churn is bounded at 4,096 rounds because the current Rey runtime does not generally reclaim allocations before process exit. Each churn round constructs an empty vector, appends 32 signed 64-bit values, traverses it for the checksum, then ends its logical lifetime. C uses a small dynamic-vector implementation; Python uses `array('q')` so its elements are signed 64-bit values.

Select language and workload subsets with comma-separated lists. `--rey` or `REY_BIN` selects the Rey compiler seed; `--runtime-dir` or `REY_RUNTIME_DIR` selects the directory containing `librey_rt.a`. An explicit `--rey` takes precedence over automatic defaults. Without an explicit selection, the runner prefers the packaged v0.3.1 seed when present and then falls back to `build/rey`. For a seed built outside this checkout, pass its matching runtime directory explicitly.

Results, executables, and raw process output are written under `build/v04/benchmarks/` by default. Use `--output-dir` to choose another artifact directory. The JSON report records host and source revision information, compiler versions and hashes, source/binary/runtime hashes, exact commands, build and run samples, statuses, normalized output, and median plus median absolute deviation. Failed or unavailable toolchains remain explicit records; no timings are invented for them. Python's build is `not_applicable`, not a zero-second native build.

## Measurement boundaries

- Run times include a fresh process start and output. Warmups are recorded separately from measured runs.
- Each measured build writes a separate executable, but compiler caches are left in their normal state. Build numbers describe the observed cache state; they are not cold, no-op, or incremental-build measurements.
- The runner does not measure compiler startup separately, steady-state in-process throughput, cache invalidation, allocation counts, copied bytes, or managed live/reserved bytes.
- Peak RSS is captured through `/usr/bin/time` on macOS (bytes) and Linux (KiB). Other systems and malformed/missing time output are recorded as unavailable.
- C/C++/Rust/Zig use optimized native builds, Go uses its default optimizing build, and Python uses the selected interpreter. Rey uses the selected seed's defaults. These options and the selected seed are recorded; they are not a claim that every optimizer mode is semantically identical.
- The expected checksum and processed count are computed independently by the runner and checked for each warmup and measured run. This catches output/input-consumption mismatches but does not prove that two implementations have identical machine-level work.
- These two synthetic workloads are baseline tooling checks, not a universal language ranking or evidence about garbage collection. Results are specific to the recorded host, toolchain, seed, and runtime.

The fixtures deliberately avoid external packages. A missing compiler produces an `unavailable` record, while build failures, timeouts, nonzero exits, and output mismatches retain their raw samples and status.

The Zig fixtures use the Zig 0.16 `std.process.Init` entrypoint and argument API. Older Zig versions may require fixture adaptation; failures are retained in the report.
