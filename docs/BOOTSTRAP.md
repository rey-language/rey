# Native compiler bootstrap

Run the complete gate from any working directory:

```sh
bash /path/to/rey-lang/scripts/bootstrap.sh
```

The default invocation is always a full stage-2 verification. It does not offer
a stage-stop or partial-success mode. `SELFHOST VERIFIED` is printed only after
the native stage-2 compiler has passed the full current conformance corpus and
successfully rebuilt the compiler source into a stage-3 executable.

## Pipeline and output isolation

Each invocation creates a new `build/rey-bootstrap.XXXXXX/` directory (or a
unique child of `BOOTSTRAP_BUILD_PARENT`). It rejects a build parent inside
`rey-compiler/` and verifies the canonical build/source paths are disjoint
before copying compiler source. It never reuses or removes an existing build
directory. The directory contains:

- `rust-target/`: Cargo's private `CARGO_TARGET_DIR`; Cargo builds release mode
  with one job.
- `runtime/`: the runtime archive built by the current
  `scripts/build-runtime.sh --output-dir <directory>` interface, using
  `/usr/bin/clang` and `/usr/bin/ar`.
- `compiler-source/`: an unmodified copy of `rey-compiler/main.rey` and `src/`.
- `stage1/rey`, `stage2/rey`, and `stage3/rey`: compiler outputs.
- `logs/`: captured stdout/stderr for every bounded stage.

Compiler source is copied so an unsupported or ignored `-o` option cannot
overwrite the checkout's `rey-compiler/main`, IR, object, or runtime archive.
All compiler invocations pass absolute, separately quoted argv elements. The
stage-1 invocation is:

```text
rey-v0 run <build>/compiler-source/main.rey -- build <build>/compiler-source/main.rey -o <build>/stage1/rey --runtime-dir <build>/runtime
```

Stage 1 builds stage 2 with `stage1/rey build ... -o stage2/rey
--runtime-dir <build>/runtime`. A temporary stage-2 engine wrapper appends the
same explicit `--runtime-dir` when the conformance runner invokes `check` or
`run`. Finally, stage 2 builds `stage3/rey` from the same copied compiler
source. The wrapper and all paths are quoted, including paths with spaces.

Every compiler, test, or build-tool process has a process-tree timeout. The
default is 300 seconds per stage; set `BOOTSTRAP_TIMEOUT_SECONDS` to a positive
integer to change it. Timeouts are reported as status 124. Failure output is
printed and retained in the unique build directory; the script propagates the
failing command's status. It never cleans the build directory, including after
failure. `--help` only prints usage and performs no build.

Environment overrides intended for diagnostics/testing:

- `BOOTSTRAP_BUILD_PARENT`: directory under which a fresh unique build
  directory is created. Defaults to the repository's `build/`; it must not
  contain `..` components or resolve inside `rey-compiler/`.
- `BOOTSTRAP_TIMEOUT_SECONDS`: per-stage timeout, default 300.
- `BOOTSTRAP_CARGO`: Cargo executable path, default `cargo`.

## Runtime gates and source interfaces

Before asking the Rust interpreter to compile compiler source, bootstrap builds
and runs an argv probe. The contract is interpreted `args() == [script,
...userArgs]`: the probe passes an argument containing spaces after `--` and
requires the normalized values. It prevents wrapper argv (`rey-v0`, `run`,
`SCRIPT`, `--`) or an empty argument vector from being mistaken for a usable
bootstrap engine.

Source inspection shows `run_file(path, programArgs)` constructing
`Interpreter::new_with_args(programArgs)` and `run_cli` passing
`[file, ...programArgs]`. This is source-wired argv support, not proof that the
interpreted argv probe succeeds at runtime. The bootstrap executes that probe
and reports its actual status; it does not infer failure or readiness from
source inspection. Native `main.rey` contains parser/code paths for
`-o`/`--output` and `--runtime-dir`, but those options are not end-to-end
verified until the native build reaches them.

`scripts/build-runtime.sh` has the observed `--output-dir DIR` interface. It
uses the runtime Makefile with `/usr/bin/clang` and `/usr/bin/ar`, emitting all
objects and `librey_rt.a` in the requested output directory. Bootstrap invokes
this helper with a unique per-run runtime directory; it does not build into the
source runtime directory. Each stage is gated on actual command status and
required artifacts; there is no checked-in binary fallback or stub success.

## Script behavior tests (fake tools; not bootstrap evidence)

These validate command construction, status handling, path quoting, and timeout
cleanup. Except for the separately labeled runtime-helper invocation, they do
not prove that Rust built the native compiler or that stage 1/2 works.

Commands below were run from `/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03`, not the repository root. `$ROOT` was `/Volumes/SSD/Developer/rey-language/rey-lang`; `$SCR` was `/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/bootstrap-checks.PSJXVC`.

Syntax/help checks:

```sh
bash -n "$ROOT/scripts/bootstrap.sh"
"$ROOT/scripts/bootstrap.sh" --help
```

Observed: syntax passed; help printed usage and performed no build.

The runtime helper was also invoked directly from the same non-repository
working directory:

```sh
"$ROOT/scripts/build-runtime.sh" --output-dir "$SCR/runtime helper standalone with spaces"
```

Observed: the final stdout line was
`$SCR/runtime helper standalone with spaces/librey_rt.a`; that output directory
contained the six objects and archive, with no source-tree build output used.

Fake Cargo status propagation:

```sh
BOOTSTRAP_CARGO="$SCR/fake tools/fail cargo" \
BOOTSTRAP_BUILD_PARENT="$SCR/final fail parent" \
BOOTSTRAP_TIMEOUT_SECONDS=10 "$ROOT/scripts/bootstrap.sh"
```

Observed: injected Cargo diagnostic was retained, the failure was reported at
`Rust oracle`, and process exit 23 propagated:

```text
injected cargo diagnostic
bootstrap: FAIL stage=Rust oracle exited 23.
bootstrap: NOT SELFHOST VERIFIED; build outputs retained at .../final fail parent/rey-bootstrap.fcJ1mP
fake_cargo_exit=23
```

Fake oracle and actual runtime-builder path/argv test:

```sh
BOOTSTRAP_CARGO="$SCR/fake tools/fake cargo creates failing oracle" \
BOOTSTRAP_BUILD_PARENT="$SCR/direct helper with spaces" \
BOOTSTRAP_TIMEOUT_SECONDS=30 "$ROOT/scripts/bootstrap.sh"
```

The fake Cargo generated only a fake `rey-v0` that prints its received argv and
exits 91. Before the explicit argv-contract failure, the real
`scripts/build-runtime.sh --output-dir <build>/runtime` succeeded using
`/usr/bin/clang` and `/usr/bin/ar`; it produced
`$SCR/direct helper with spaces/rey-bootstrap.n0yZm8/runtime/librey_rt.a` and
all six objects. The bootstrap preserved the spaced paths and each argv
boundary (`<run> <.../argv-probe.rey> <--> <bootstrap-argv-probe> <argument
with spaces>`), stopped at `Rust interpreter argv contract`, and propagated
exit 91. It did not accept the fake executable as stage 1 or stage 2.

A second fake oracle emitted the argv-probe success output and then failed the
stage-1 build with status 72:

```sh
BOOTSTRAP_CARGO="$SCR/fake tools/fake cargo to stage1 failure" \
BOOTSTRAP_BUILD_PARENT="$SCR/stage1 copy verified with spaces" \
BOOTSTRAP_TIMEOUT_SECONDS=30 "$ROOT/scripts/bootstrap.sh"
```

Observed: the probe passed, an isolated compiler-source copy was used, and the
byte comparison of copied `main.rey` and recursive source tree passed. The
printed stage-1 command preserved the separate `--`, `-o <stage1/rey>`, and
`--runtime-dir <runtime archive dir>` arguments despite spaces in the paths.
The injected diagnostic was retained and exit 72 propagated; no stage-1 binary
was accepted.

Timeout/process cleanup test:

```sh
FAKE_CARGO_CHILD_PID="$SCR/final-timeout-child.pid" \
BOOTSTRAP_CARGO="$SCR/fake tools/sleeping cargo" \
BOOTSTRAP_BUILD_PARENT="$SCR/final-final-timeout parent" \
BOOTSTRAP_TIMEOUT_SECONDS=1 "$ROOT/scripts/bootstrap.sh"
```

Observed: status 124 at `Rust oracle`; the recorded child PID was absent after
the run and `pgrep -fl 'sleep 30'` returned no processes.

Source/destination guard:

```sh
BOOTSTRAP_BUILD_PARENT="$ROOT/rey-compiler/bootstrap-disjoint-guard-test" \
  "$ROOT/scripts/bootstrap.sh"
```

Observed: status 2, `build parent overlaps compiler source`, and the candidate
directory was not created.

## Bounded real pipeline attempt (2026-10-17)

One real run was made from the non-repository cwd
`/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03`.
No `BOOTSTRAP_CARGO` or `CARGO_TARGET_DIR` override was set. The exact command
was:

```sh
ROOT=/Volumes/SSD/Developer/rey-language/rey-lang
RUN_DIR=/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/bootstrap-real-20261017.qWtbJn
BOOTSTRAP_BUILD_PARENT="$RUN_DIR/build parent" \
BOOTSTRAP_TIMEOUT_SECONDS=180 "$ROOT/scripts/bootstrap.sh"
```

The local command clock reported start `2026-10-16T18:34:03Z`, end
`2026-10-16T18:34:07Z`, and process status 101. Cargo used the script's private
target at `$RUN_DIR/build parent/rey-bootstrap.UqHU2x/rust-target`. The first
stage (`Rust oracle`) failed; no runtime build, argv probe, native build,
conformance run, or self-rebuild followed. Full output is retained at
`$RUN_DIR/bootstrap.log`; stage logs are in
`$RUN_DIR/build parent/rey-bootstrap.UqHU2x/logs/`.

The real `cargo build --release -j 1` reported eight peer-source errors in
`compiler/v1/src/typecheck.rs`:

- `E0027`: patterns omit `is_pub` at lines 665, 675, 689, 705, 1306, and 1307.
- `E0277`: `String: Borrow<&String>` is not satisfied at line 1386.
- `E0382`: moved `paramTy` is used again at line 1907.
- Warning: unused `body` at `compiler/v1/src/imports.rs:895`.

No peer source was edited and the run was not retried. Source inspection shows
the Rust argv wiring is present, but its runtime probe was not reached; the
native driver and stage-2 gates remain unverified. No checked-in release binary
was used.

## Input fingerprints and snapshot coherence

Immediately before and after the real run, the source manifests were captured
at `$RUN_DIR/source-fingerprints.start` and
`$RUN_DIR/source-fingerprints.end`. Each has 104 lines: 100 hashes and four
section labels (29 Rust/manifest inputs, 47 compiler `.rey` inputs, 14 compiler
`.ll` inputs, and 10 runtime source/header/Makefile/helper inputs). Both files
have the same aggregate SHA-256:

```text
e41c9ddd950c54839aae3d40300da7edcd357e379889f34361b6c2a7c58c556c
```

The start/end lists match at those two boundaries. This is not proof of an
immutable snapshot or of no transient edit between samples; the shared source
tree was active. A single post-run process snapshot also showed a separate
`rustc --test` targeting the shared scratch `.../rey-v03/target/release/deps`,
not this run's private target. It was left untouched; no further polling was
done. The standalone runtime-helper and fake-tool checks above are script/runtime
behavior evidence only, not native bootstrap proof. Stage 1, stage 2,
full-corpus conformance, and stage-2 rebuild remain unverified.
