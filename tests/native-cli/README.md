# Native CLI checks

Run with a built native compiler and an existing runtime archive:

```sh
sh tests/native-cli/run.sh /absolute/path/to/rey /absolute/path/to/runtime [/scratch/path]
```

The runner copies the runtime into scratch and writes all source/output files there; it does not clean them up. It covers quiet `check`, no check artifacts, build/run output paths with spaces, `-o`/`--output`, runtime paths with spaces, missing values, unknown options, missing input, and import-cycle detection/deduplication. It requires `librey_rt.a` in the supplied runtime directory and does not build it. Expected exits are 0 for help/version and success, 1 for compile/runtime failures, and 2 for invalid usage.

Native and interpreted `args()` are expected as `[program, ...userArgs]`. For the bootstrap call `rey-v0 run <compiler-source> -- build <input> -o <output> --runtime-dir <runtime>`, the interpreter must inject `[<compiler-source>, "build", <input>, "-o", <output>, "--runtime-dir", <runtime>]`; no `run`, source-path, or `--` offsets are consumed by this driver. Runtime lookup uses the explicit `--runtime-dir` when supplied and validates `librey_rt.a` even for `check`; absent an override, build/run accept only a sibling `runtime/librey_rt.a` that exists. A relocated compiler must receive the runtime directory explicitly.

`check` invokes the current checker and honors its `ok`/`errors` result. The current `TypecheckResult` has no typed-program field, so this driver does not claim or pass a typed AST to codegen; checker coverage and the typed-AST handoff remain separate work. The process-level runner requires a native executable with codegen `exit` lowering, runtime `unwrapErr` and stderr intrinsics, and quiet successful compiler output integrated.

Driver diagnostics use `eprintln`; this depends on the pending runtime/core-intrinsic registration. `exec` inherits subprocess stderr while returning captured stdout; run output is preserved on both success and failure via `unwrapErr`. Codegen's current `compileLLVM` also emits `DEBUG` banners directly to stdout, which this driver cannot suppress and must be silenced by its owner.
