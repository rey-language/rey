# Rey Standard Library Architecture

This document describes the current shape of `std`, the boundaries between Rey code and host capabilities, how imports resolve, and a staged path for growth. It targets the **Rey v0.3.0** language and compiler. The roadmap is directional: a listed module or capability is not implemented until it exists under `src/` and is documented as available.

## Purpose and design principles

The standard library should make common Rey programs pleasant to write without turning every small task into a framework choice. Its architecture follows these principles:

### Zero unexpected dependencies

Prefer language features, pure Rey code, and compiler-provided runtime operations. A module should not quietly require a downloaded executable, service, or package. When a host dependency is necessary, state it in the module README and at the call site where practical. Today, `std::http` shells out to `curl` and requires it on `PATH`; this is a deliberate, documented boundary rather than a claim that every module is dependency-free.

### POSIX runtime ergonomics

Make ordinary command-line and filesystem work direct: paths, environment variables, working directories, temporary directories, and subprocesses should be straightforward to use. Keep host behavior explicit, avoid pretending every OS has identical semantics, and document platform-specific details. The current process wrapper runs `sh -c` on Unix and `cmd /C` on Windows; APIs that invoke commands therefore inherit shell behavior and quoting rules.

### Type safety and honest contracts

Public APIs should communicate accepted values and results in their signatures. Prefer specific lowercase primitive and applied types supported by Rey v0.3, and avoid `any` when a useful type is available. Several existing wrappers still use broad or inferred types, reflecting the current language/runtime boundary; new APIs should improve clarity without claiming guarantees the implementation cannot provide. Document sentinels, errors, units, byte behavior, and other observable semantics.

### Contributor and agent accessibility

Keep module entry points easy to scan, tests executable without a custom harness, and dependencies discoverable. Root and module documentation should distinguish what works today from ideas. Changes should be small enough for maintainers and coding agents to understand, verify, and review without guessing at hidden conventions.

## Current source layout

```text
std/
├── src/
│   ├── core/main.rey
│   ├── fmt/main.rey
│   ├── collections/main.rey
│   ├── cli/main.rey
│   ├── fs/main.rey
│   ├── http/main.rey
│   ├── io/main.rey
│   ├── math/main.rey
│   ├── process/main.rey
│   ├── string/main.rey
│   ├── time/main.rey
│   ├── json/main.rey
│   ├── log/main.rey
│   ├── test/main.rey
│   ├── os/main.rey
│   ├── env/main.rey
│   ├── crypto/main.rey
│   ├── sql/main.rey
│   ├── mem/main.rey
│   ├── sync/main.rey
│   ├── net/main.rey
│   ├── regex/main.rey
│   ├── compress/main.rey
│   └── image/main.rey
└── tests/
    ├── core/core_test.rey
    ├── fmt/fmt_test.rey
    ├── collections/collections_test.rey
    ├── cli/cli_test.rey
    ├── fs/fs_test.rey
    ├── http/http_test.rey
    ├── io/io_test.rey
    ├── json/json_test.rey
    ├── log/log_test.rey
    ├── math/math_test.rey
    ├── process/process_test.rey
    ├── string/string_test.rey
    ├── test/test_test.rey
    ├── time/time_test.rey
    ├── os/os_test.rey
    ├── env/env_test.rey
    ├── crypto/crypto_test.rey
    ├── sql/sql_test.rey
    ├── mem/mem_test.rey
    ├── sync/sync_test.rey
    ├── net/net_test.rey
    ├── regex/regex_test.rey
    ├── compress/compress_test.rey
    └── image/image_test.rey
```

Each current module has a corresponding checked-in test program. The `http` test is a manual placeholder and does not make a request. The `json` and `time` modules are implemented with deliberately narrow APIs: JSON serializes values but does not parse them, while time formats durations and sleeps but does not measure clocks.

Each `src/<module>/main.rey` is the importable module entry point. Its `pub` declarations are the API. A module can add private helper files as useful, but keep public names discoverable and intentional. A module README records behavior and prerequisites close to the implementation; root [`README.md`](README.md) provides the overview matrix.

## Module boundaries and host requirements

| Module | Implementation boundary | Host requirement or caveat |
|---|---|---|
| `std::core` | Rey wrappers over built-in Option/Result methods and process diagnostics | `panic`, `todo`, and `unreachable` exit the current process with status 1. |
| `std::fmt` | Pure Rey placeholder substitution, string padding, and integer radix conversion | Placeholders in source string literals must use doubled braces to avoid Rey string interpolation. |
| `std::math` | Rey helpers plus compiler math built-ins for selected operations | Numerical accuracy and input-domain behavior depend on the operation; see source/tests. |
| `std::string` | Pure Rey string manipulation | Current helpers operate on Rey's string/index semantics; do not assume Unicode grapheme-aware indexing. |
| `std::collections` | Rey functions over built-in `Vec` and `HashMap` values | Callback helpers operate on dynamic values because general user-defined generics are not part of v0.3. |
| `std::fs` | Thin Rey wrappers over runtime filesystem built-ins | Needs filesystem access and inherits host path/permission behavior. |
| `std::io` | ANSI terminal output and formatting helpers | Color escape sequences may not be interpreted by every output destination. |
| `std::process` | Thin wrappers over runtime process/environment functions | Command execution uses a platform shell; treat command text and quoting accordingly. |
| `std::http` | Builds HTTP calls on process execution and `curl` | Requires `curl` on `PATH`; response bodies are raw text and JSON decoding is not included. |
| `std::time` | Millisecond sleep and duration formatting | No elapsed-time or wall-clock measurement API is present yet. |
| `std::json` | Escaping and serialization of primitive values, arrays, and objects | No parser; array/object values are inserted as serialized fragments without validation. |
| `std::test` | Assertion and reporting helpers using built-in `assert` and output | It provides no test discovery or runner; execute test files with the compiler. |
| `std::os` | Rey wrappers over environment variables, runtime temp directory, and host commands | OS/architecture and hostname detection may invoke `uname` or `hostname`. |
| `std::env` | Thin wrappers over runtime environment built-ins | Values and mutation behavior inherit host environment rules. |
| `std::crypto` | Rey non-cryptographic hashes and shell-backed system hashing/randomness | Cryptographic operations require host shell commands; hashing or random generation returns an empty value if unavailable. |
| `std::cli` | Pure Rey parsing of argument vectors and help formatting | Without a declaration schema, a long switch followed by a non-switch token is parsed as an option/value pair. |
| `std::log` | Formats and prints leveled messages; stores the threshold in the process environment | Uses `REY_STD_LOG_LEVEL`; the configured value can be inherited by child processes. |
| `std::linalg` | Pure Rey vector operations over public 2D and 3D structs | Normalizing a zero vector returns a zero vector. |
| `std::binary` | Pure Rey byte-vector packing, unpacking, and hexadecimal conversion | Inputs are unsigned byte values; packing accepts only unsigned 16-bit or 32-bit values. |
| `std::sql` | Executes SQL through `sqlite3` using the process shell; falls back to an empty mock | Requires `sqlite3` for database behavior. The mock does not parse or retain SQL data; CLI results use pipe-delimited text. |
| `std::regex` | Pure Rey single-character pattern matching with anchors and quantifiers | Escapes, grouping, alternation, and Unicode grapheme matching are not implemented. |
| `std::compress` | Pure Rey run-length encoding and shell-backed gzip/tar wrappers | System wrappers require a POSIX shell and the corresponding commands on `PATH`. |
| `std::image` | Packed RGB vector storage and P3 PPM serialization in Rey | File output uses runtime filesystem access; bitmap decoding and binary formats are not included. |
| `std::mem` | Primitive size constants and bounded views/copies over Rey vectors | `Span` is a vector-backed view, not a raw pointer; copy validates ranges and supports overlap. |
| `std::sync` | Shared-value wrappers for integers, a nonblocking lock state, and a FIFO vector queue | v0.3 has no threads; the APIs provide no cross-thread atomicity, blocking, or memory ordering. |
| `std::net` | Address formatting and validation plus `ping` through `std::process` | Ping requires the host command and network/host permission; TCP and UDP sockets are not implemented. |

Keep these boundaries visible. For example, code that only trims text should remain pure Rey, while a network request is expected to say what executable or runtime support it needs.

## Module resolution

For an installed standard library at `$HOME/.reyc/std`, the compiler uses its `src/` directory as the standard module root. The mapping is:

| Rey import | Resolved module entry point |
|---|---|
| `import std::core;` | `$HOME/.reyc/std/src/core/main.rey` |
| `import std::fmt;` | `$HOME/.reyc/std/src/fmt/main.rey` |
| `import std::cli;` | `$HOME/.reyc/std/src/cli/main.rey` |
| `import std::log;` | `$HOME/.reyc/std/src/log/main.rey` |
| `import std::math;` | `$HOME/.reyc/std/src/math/main.rey` |
| `import std::string;` | `$HOME/.reyc/std/src/string/main.rey` |
| `import std::collections;` | `$HOME/.reyc/std/src/collections/main.rey` |
| `import std::linalg;` | `$HOME/.reyc/std/src/linalg/main.rey` |
| `import std::binary;` | `$HOME/.reyc/std/src/binary/main.rey` |
| `import std::sql;` | `$HOME/.reyc/std/src/sql/main.rey` |
| `import std::regex;` | `$HOME/.reyc/std/src/regex/main.rey` |
| `import std::compress;` | `$HOME/.reyc/std/src/compress/main.rey` |
| `import std::image;` | `$HOME/.reyc/std/src/image/main.rey` |

In general, the compiler removes the leading `std::`, converts the remaining `::` separators to directory separators, and looks for that module directory's `main.rey`. Thus `std::a::b` maps to `$HOME/.reyc/std/src/a/b/main.rey`. `$HOME/.reyc/std` may be a directory or a symlink to a checkout; it is the package root, while `src/` is the module source root.

The v0.3 resolver also checks importer-relative and project-local candidates before the installed standard-library path. Avoid defining a project-local path that shadows a `std::...` module when you intend to exercise the installed standard library. When an import fails, first verify the resolved source exists at the expected path, then inspect the compiler's import error before changing code.

## API and compatibility rules

- Use `pub func` for public functions. `export` is not the visibility form for v0.3 code.
- Use lowercase primitive types such as `string` and `void`; public signatures should make inputs and outputs obvious.
- Keep helper functions private unless callers have a real use for them.
- Treat published function names and observable edge behavior as API. Explain a breaking change and coordinate it rather than silently changing it.
- Do not promise error-return behavior if a wrapper currently panics, exits, or returns a sentinel. Document the behavior that callers can observe today.
- Add implementation and focused executable tests together. Keep the module README and root matrix current.
- New host interactions or external dependencies need a reason, explicit requirements, and a test strategy that does not make ordinary local development surprising.

Rey v0.3 has built-in vectors, string-keyed hash maps, options, and results, but it does not provide general user-defined generic functions/types. Prefer the strongest supported signature without inventing unsupported abstractions. Revisit those APIs as the language gains expressive type features.

## Test model

Files in `tests/<module>/` are regular Rey programs. The standard library does not currently include a separate test discovery command. A contributor runs one directly from the library root:

```sh
rey-v0 run tests/math/math_test.rey
```

`std::test` supplies convenience assertions and `PASS`/`FAIL` reporting, but does not find or launch test files. Tests that require host features should say so in comments and should avoid network or destructive behavior by default. For the current all-files smoke pass:

```sh
for test_file in tests/*/*.rey; do
    rey-v0 run "$test_file" || exit 1
done
```

## Tiered evolution roadmap

The tiers describe sequencing and design maturity, not a promise that a module already exists. Each addition still needs a clear proposal, importable implementation, tests, and documentation.

### Tier 1 — core everyday building blocks

Keep the current foundation dependable and approachable: `math`, `string`, `collections`, `fs`, `io`, `process`, and the `test` helpers. Priorities are consistent signatures, boundary behavior, focused tests, and clear host-operation notes. Fix reliability and documentation gaps before multiplying similar helpers.

### Tier 2 — typed data and time foundations

The first `json` and `time` helpers now establish narrow starting points. Next, design any parser and clock/measurement APIs around explicit contracts before adding convenience layers. For JSON, settle representation, parse failure reporting, and string/number semantics using the types v0.3 can honestly express. For time, settle units, clock source, UTC conventions, and deterministic testing. Additional capability may need compiler/runtime support first.

### Tier 3 — reliable integration primitives

Strengthen network and system integration once core contracts are stable. For HTTP, likely areas include request timeouts, safer argument passing, status/error modeling, header/query encoding, and tests using a controlled local endpoint. Prefer runtime support that avoids shell-string construction when available. `std::cli` and `std::log` provide initial focused APIs; structured process support remains a separate area for future proposals.

### Tier 4 — domain systems

Only after core types, errors, and integration boundaries can support them, consider larger domains such as cryptography, compression, database clients, image/audio handling, or specialized math. These carry substantial compatibility and maintenance costs. Keep them separate, carefully scoped, and independently reviewable; a focused package may be a better home than `std`.

## Extending the architecture

For a new module, begin with a short design note: who needs it, the smallest useful API, its runtime or external requirements, how errors are represented, and how behavior will be tested. Then follow [`CONTRIBUTING.md`](CONTRIBUTING.md), create `src/<module>/main.rey` and its README, add tests under `tests/<module>/`, and update the root matrix. Keep the first review focused; defer speculative convenience methods until there is a real use case.
