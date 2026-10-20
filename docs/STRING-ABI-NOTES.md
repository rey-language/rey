# Runtime string value ABI handoff

## Legacy compatibility bridge

The public ABI in `rey-compiler/runtime/rey_rt.h` still declares
`char *rey_val_to_str(int64_t val)`. The runtime's current implementation uses a
process-global exact-pointer registry populated by string constructors. It avoids
reading `val - REY_STR_HDR` for arbitrary integers and recognizes pointers that
the runtime itself returned as string data pointers. This is a compatibility
bridge, not a typed representation and not a complete solution to value-kind
discrimination.

The string-value regression suite is `rey-compiler/runtime/tests/string_value_test.c`;
run it with `scripts/test-runtime-string.sh [normal|sanitize|all]`.

## Known limits and lifetime assumptions

- A full-width integer whose numeric value equals a live registered string data
  pointer is indistinguishable from that pointer in the one-argument ABI. The
  registry will classify that integer as a string. No pointer-range or header
  probe can solve that collision safely.
- A valid ReyStr pointer created outside the runtime constructors is not
  registered and is treated as an integer by `rey_val_to_str`.
- The registry is grow-only. The current runtime exposes no string deallocator;
  strings and their registered addresses therefore remain live until process
  exit. If strings gain freeing, a stale registry entry could classify an
  unrelated value after allocator address reuse. Removal must then be coupled
  to object lifetime, or the bridge must be removed in favor of a tagged ABI.
- The regression harness links real `mem.c`, `string.c`, `vec.c`, `io.c`, and
  `result.c`; it does not stub allocation or vector ownership. It omits unrelated
  hashmap code. ASan leak detection is disabled because this runtime currently
  retains string allocations for process lifetime.

The legacy function remains available for binary compatibility. A numeric
integer equal to a live registered string pointer can still be mistaken for a
string, and the pointer registry does not make arbitrary `i64` values typed.

## Explicit-kind scalar ABI

New callers can use this ABI without changing the old function or object
layouts:

```c
const char *rey_typed_val_to_str(int64_t kind, int64_t payload);
```

The stable public kind constants in `rey_rt.h` are deliberately outside the
existing `REY_TAG_*` object-tag range:

| Constant | Value | Payload contract |
|---|---:|---|
| `REY_VALUE_INT` | `0x1001` | Any `int64_t`; formatted by `rey_int_to_str`. |
| `REY_VALUE_BOOL` | `0x1002` | Exactly `0` or `1`; formatted as `false` or `true`. |
| `REY_VALUE_FLOAT` | `0x1003` | Raw 64-bit `double` bit pattern in the `int64_t` payload; runtime reinterprets with `memcpy`. |
| `REY_VALUE_STRING` | `0x1004` | Nonzero pointer to a valid ReyStr data field; returned unchanged. |
| `REY_VALUE_CHAR` | `0x1005` | Integer byte in `0..255`; returned as a one-byte string, including NUL and `0xff`. |
| `REY_VALUE_NULL` | `0x1006` | Payload must be `0`; returns the text `null`. |

Unknown kinds and out-of-range bool/char/null payloads panic with a specific
runtime diagnostic. They never fall through to the legacy pointer registry.
For `REY_VALUE_STRING`, the caller promises a valid ReyStr pointer; the runtime
checks only that it is non-null and does not probe arbitrary addresses.

Float formatting intentionally reuses `rey_float_to_str`, which currently uses
`snprintf("%g")`, whose default is six significant digits and whose formatting
follows the active C locale/libc behavior (including special values). This ABI
adds no new float formatting or round-trip policy.

The typed API does not tag values automatically. Callers must carry the correct
kind from static type information. It does not resolve dynamic `Any` values,
heterogeneous collection elements, or other opaque `i64` payloads whose type is
unknown. String results created by conversion follow the runtime's existing
process-lifetime allocation policy; string-kind conversion returns the borrowed
input pointer.

## Current codegen assumptions (read-only audit)

- `src/codegen/main.rey:181-189` interns literal bytes as an LLVM global, then
  constructs the runtime string with `rey_str_new`. The runtime constructor
  registers the resulting data pointer.
- String-producing runtime constructors (`rey_str_new`, `rey_str_concat`,
  `rey_str_repeat`, `rey_str_to_upper`, and `rey_str_to_lower`) register their
  outputs. Helpers built through these constructors inherit registration.
- String `.toString()` is emitted as identity at `main.rey:793-794`. Other
  `rey_val_to_str` calls remain at `376`, `387`, `808`, and `1090`; the `+`
  lowering at `357-390` can pass a non-string `i64` or cast opaque pointer to
  `i64`, while the helper has no static-kind argument. The generated declaration
  at `1899` also has only one `i64` parameter.

These assumptions explain why the legacy pointer registry recognizes ordinary
runtime strings, but do not make the old ABI type-safe. Native lowering must
select the explicit kind from typed metadata before using the new function; it
must not infer kind from pointer magnitude or fall back for unknown values.

## Compiler handoff

The owner ruling is: a `string + non-string` expression must be a type error, not
a runtime-coercion case. Native lowering should enforce that in the typechecker
and must not use `rey_val_to_str` as a fallback for mixed `+` operands.

The native compiler must update the generated declaration and each generic
conversion callsite to pass a kind selected from the typed AST. String pointers
must be passed with `REY_VALUE_STRING`; integer payloads with `REY_VALUE_INT`;
bool and float payloads must use their corresponding kinds. The emitted LLVM ABI
is `i8* @rey_typed_val_to_str(i64 kind, i64 payload)`. Unknown or unsupported
dynamic kinds must be rejected or handled by an explicitly typed lowering; they
must not use the legacy guess.

Alternatively, codegen can avoid the generic typed helper for known scalar
types by calling the existing scalar formatters directly. Either approach leaves
`rey_val_to_str(int64_t)` available for old binaries, so its collision ambiguity
remains until that legacy ABI is retired. The owner ruling that mixed
`string + non-string` is a type error still applies.

The typed regression suite is `rey-compiler/runtime/tests/typed_value_test.c`;
run it with `scripts/test-runtime-typed-value.sh [normal|sanitize|all]`. Its
sanitizer mode checks ASan/UBSan memory safety, not leaks or complete dynamic
value typing; process-lifetime allocations make LeakSanitizer unsuitable here.
