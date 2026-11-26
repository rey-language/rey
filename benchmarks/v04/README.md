# Rey v0.4 matched native benchmarks

The runner builds and directly executes one fixture per language/workload cell.
The default matrix is C, C++, Rust, Zig, Go, and Rey across numeric, churn,
parse, and stream workloads. Every output is checked against an independent
Python oracle. Python fixtures remain available as an optional comparison with
`--languages python,rey`.

## Run

```sh
python3 -B benchmarks/v04/run.py \
  --rey "$SEED" \
  --languages c,cpp,rust,zig,go,rey \
  --workloads numeric,churn,parse,stream \
  --repeats 3 --warmups 1 --timeout 600 \
  --output-dir "$W/bench"
```

Supply `--baseline-rey "$BASE"` to compile and run a paired baseline for each
Rey workload. The candidate and baseline compile the same Rey fixture from the
same source tree with the same native flags. Their build time, image hash/size,
startup estimate, run RSS, full-run median, workload estimate, and relative
timing are retained in `results.json` and `table.md`.

`--rey` selects the direct native Rey CLI. Its fixture build is equivalent to:

```sh
rey build SOURCE -o OUTPUT --target linux-x64 --regions --managed
```

It uses the managed runtime embedded in the compiler. No legacy C runtime
archive, helper `chmod`, fast exit, packed-output trust, or relaxed region flag
is used. Every benchmark compiler invocation goes through B01's
`scripts/v04-linux-x64-budget.sh run` wrapper, which takes the shared build lock,
checks available memory, logs the producer output, and applies `nice -n 19` and
GNU `timeout`. Program invocations also use `nice -n 19` and GNU `timeout`; on
Linux `/usr/bin/time -v` records peak RSS.

## Workloads and oracles

| Workload | Input size | Work measured | Independent result |
|---|---|---|---|
| `numeric` | Iterations | Signed 64-bit linear congruential state and checksum | Python recomputation of every state |
| `churn` | Vector rounds | Construct, fill, traverse, and discard a 32-value vector | Python recomputation of each vector value |
| `parse` | Decimal tokens | Repeatedly parse and fold eight signed decimal argument strings | Python integer parsing and token-cycle sum |
| `stream` | Records | Constant-space generated-record filter and rolling checksum fold | Python execution of the record state machine |

All programs print the same `checksum=<integer> processed=<integer>` line and
must exit 0. `parse` processes the fixed signed tokens `11,-23,37,-41,53,-67,79,-89`.
`stream` generates and consumes one record at a time without materializing a
collection. It measures a streaming transform, not file I/O.

The default sizes are 1,000,000 numeric iterations, 256 churn rounds, 250,000
decimal tokens, and 1,000,000 stream records. `--size` overrides all selected
workloads. Limits are recorded in the JSON report and enforced by the runner.

## Measurement and artifacts

Each native cell compiles a fresh executable `repeats` times. It then uses the
last image for a zero-size startup probe, the requested warmup count, and the
recorded workload samples. The table reports compile median, startup median,
full run samples (in JSON), estimated workload median, run RSS median, and
image size. The startup estimate is the median wall time for the same executable
and arguments at size zero; workload time is `max(0, full-run median - startup
median)`. This is a process-level estimate, not an in-process timer.

The report records source and executable SHA-256 values, exact wrapped commands,
compiler versions and options, sizes, host CPU/OS, compile time, startup and
workload timings, peak RSS, and image size. Build caches are left in their
normal state and are not described as cold. `table.md` is the compact report;
`results-<run-id>.json` preserves all samples and raw stdout/stderr. Failed,
missing, timed-out, skipped, or output-mismatching cells are not counted as
valid and make the command exit nonzero.

The workloads are synthetic matched checks, not a universal language ranking
or a claim about file parsing, garbage collection, or real application speed.
