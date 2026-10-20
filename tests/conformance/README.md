# Conformance fixtures

Each case lives at `tests/conformance/<area>/<case>/`:

```text
main.rey               required — the program
expected.stdout        required — exact stdout bytes (may be an empty file)
expected.exit          optional — integer, default 0
expect_check_fail      optional — require `check` to fail; its contents must
                                 occur as a substring of combined diagnostics
```

The runner copies the complete fixture into a fresh working directory, so a
case can use relative imports or write temporary files without affecting the
source fixture or another case. Before counting cases, the runner preflights
the engine using a valid program: `check` must exit 0 and produce no output.
Positive cases must also pass `check` silently before they are run; their
stdout is compared byte-for-byte and their runtime exit status is compared with
`expected.exit`. A case with `expect_check_fail` is compile-negative: only
`check` is run, and it must exit exactly 1 with the non-empty marker text in
its diagnostics. Usage errors, timeouts, signals, and missing executables do
not count as compile errors. Its `expected.stdout` is still required by the
common fixture format, but is not used.

Run the suite from any directory:

```sh
scripts/conformance.sh --engine "compiler/v1/target/release/rey-v0"
scripts/conformance.sh --engine "/absolute/path/to/rey2" --filter funcs
```

The `--engine` value may be a path (including spaces) or a shell command string
with arguments. For an exact executable path containing spaces, quote the
whole CLI value:

```sh
scripts/conformance.sh --engine "/path/to/builds/rey v0"
```

When combining a spaced executable path with arguments, quote that path inside
the command string, for example `--engine '"/path/to/builds/rey v0" --some-option'`.

Each case has a 30-second wall-clock budget shared by its `check` and `run`
commands. The test timeout can be shortened with `CONFORMANCE_TIMEOUT_SECONDS`.

To add a case, create a new directory under one of the areas below, add
`main.rey` and `expected.stdout`, and optionally add `expected.exit` or
`expect_check_fail`. Keep expected stdout bytes—including the final newline—
exactly as the engine should emit them. Directories beginning with `_` are
skipped, except `_smoke`, which is included in unfiltered runs. `--filter`
selects cases by a substring of their area/case path. An empty corpus or a
filter matching no cases is an error, not a successful empty run.

Current corpus areas: `types/`, `exprs/`, `control/`, `data/`, `funcs/`,
`modules/`, `errors/`, and `robustness/`, plus the default-on `_smoke/` cases.

Engine targets:

- Rust reference/oracle: `compiler/v1/target/release/rey-v0`
- Native compiler: the executable produced by `rey-compiler` (commonly
  `rey-compiler/build/rey`)
- Stage-2 native compiler: `rey2`, once built

Pass the executable path or another shell command that accepts the `check`
and `run` subcommands. The v1 oracle may temporarily have a different CLI
while its migration is in progress.
