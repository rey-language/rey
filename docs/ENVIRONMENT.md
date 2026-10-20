# Build environment

This checkout is on the mounted SSD at `/Volumes/SSD/Developer/rey-language/rey-lang` (macOS arm64). Use this path as the source of truth. The sibling standard library is available at `/Volumes/SSD/Developer/rey-language/std`; the repo's compiler and runtime work must not modify that out-of-scope checkout.

There are stale references to the former home-directory checkout under `/Users/misbahkhursheed/Developer/rey-language/`. In particular, `git rev-parse --show-toplevel` can resolve this SSD checkout while `git status` fails because linked-worktree metadata still names `/Users/misbahkhursheed/Developer/rey-language/rey-lang/.git/worktrees/...`. Treat the mounted SSD path as current; do not try to repair Git metadata manually while the designated recovery work is in progress. `scripts/doctor.sh` reports the live result on each run.

## Rust oracle (`rey-v0`)

The Rust engine is the v1 interpreter in `compiler/v1/`. With Rust installed and on `PATH`, build it from a clean checkout with:

```sh
cd /Volumes/SSD/Developer/rey-language/rey-lang/compiler/v1
cargo build --release
```

The executable is `target/release/rey-v0`. The current machine reports `rustc 1.98.1` and `cargo 1.98.1`. If Rust is installed but not on `PATH`, load the user's Rust environment (commonly `source "$HOME/.cargo/env"`) before building.

## Native engine

The native compiler source is `rey-compiler/main.rey`. The v0.3 sprint plans to provide `scripts/bootstrap.sh`; once that script exists, run it from the repository root to build the native engine from source:

```sh
cd /Volumes/SSD/Developer/rey-language/rey-lang
bash scripts/bootstrap.sh
```

At the time this guide was written, `scripts/bootstrap.sh` is not present, so this command is a future entry point, not a verified build command. The legacy manual bootstrap path is described in `CONTRIBUTING.md`; avoid running it blindly because it writes compiler build outputs into the source tree. Native compilation must use `/usr/bin/clang` (see below). The committed `rey-compiler/main` is a Mach-O arm64 executable, but its presence is not evidence that a fresh native bootstrap succeeds.

## Clang path issue

The charter pins the compiler toolchain to the real Apple clang at `/usr/bin/clang`. On this machine `command -v clang` resolves to `/Users/misbahkhursheed/.swiftly/bin/clang`, a swiftly shim. Running that shim currently fails because Swift 6.3.1 is not installed at the toolchain path it expects. In contrast, `/usr/bin/clang --version` works and identifies Apple clang 21.0.0 for arm64.

Use `/usr/bin/clang` explicitly for builds and in compiler configuration; do not trust the first `clang` found on `PATH`. An owner may either install the Swift toolchain expected by swiftly or adjust their own `PATH` so the shim is not selected. This repository's agents must not modify shell profiles or repair the user's toolchain.

## Standard-library link

`~/.reyc/std` is currently a dangling symlink to `/Users/misbahkhursheed/Developer/rey-language/std`, a path that is not present on this machine. The active SSD checkout does have the standard library at `/Volumes/SSD/Developer/rey-language/std`. If the owner approves a change outside this repository, the recommended repair is:

```sh
ln -sfn /Volumes/SSD/Developer/rey-language/std ~/.reyc/std
```

This command is guidance only; `scripts/doctor.sh` is report-only and never changes symlinks or machine state. Do not run the command without owner approval.

## Disk and memory

The latest check showed the mounted `/Volumes/SSD` volume at 223 GiB total, 115 GiB used, and 108 GiB available (52% capacity), with 8 GiB RAM and 8 logical CPUs. Parallel compiler builds can compete for limited memory; if builds are killed or the machine is under pressure, limit Cargo parallelism, for example `cargo build --release -j 4`. Re-run `bash scripts/doctor.sh` for current values rather than relying on these recorded measurements.

## Bootstrap chain

The Rust `rey-v0` interpreter is the bootstrap engine: it interprets the Rey compiler source in `rey-compiler/main.rey`, which emits LLVM IR; the toolchain then uses `llc` when available and clang as the object-generation fallback, and links the Rey C runtime to produce a native compiler executable. That native executable runs Rey programs directly; rebuilding the compiler itself starts again from the Rust oracle, while the v0.3 goal is to have the native engine rebuild itself to stage 2. `scripts/bootstrap.sh` is intended to make this sequence repeatable once it lands.
