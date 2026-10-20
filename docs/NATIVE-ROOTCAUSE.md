# Native call-emission forensic note

## Finding

The native source generator does **not** hard-code `@0`: it obtains the direct
call target from `expr.callee.name` (`rey-compiler/src/codegen/main.rey:524-526`)
and concatenates that value into `@name` at lines 651-657. The native binary
miscompiles that untyped nested field read. The strongest artifact evidence is
the existing `rey-compiler/main.bin.ll`: in the `genExprValue` direct-call
branch, lines 11403-11415 load `CallExpr.callee`, then line 11416 does
`store i64 0, i64* %name.addr` instead of loading `IdentExpr.name`. This matches
the runtime repro below, where the function definitions are named but calls use
`@0`: the zero target local is concatenated after `@` by the source emitter.

The current source confirms that call generation has no resolved symbol/type
input: its direct path tests `expr.callee instanceof IdentExpr` but then reads
the nested `expr.callee.name` without first binding/casting the callee
(`524-526`). Elsewhere its field-access code handles opaque `i64` by guessing
a struct from a matching field name (`211-282`, especially 243-282). This is a
real unsafe heuristic, but the existing artifact has no source revision/build
manifest; the evidence does **not** prove which exact old lowering rule
produced its zero. The strongest causal statement is that target extraction
has already become zero before source call-string assembly (`653/656`), as
both the artifact IR and current-binary repro show. Thus this is a compiler
miscompile at the untyped call-target/member-resolution boundary, not a
literal `@0` in the current source emitter.

The pipeline reinforces that boundary: it invokes `typecheck.typecheck(...)`
but then calls `codegen.generateIR(parseResult.program)` with the original
untyped parse tree (`main.rey:172-182`, `212-221`). The typecheck result carries
only `ok/errors`; the current checker returns `Any` from `inferExpr` and
unconditional success (`src/typecheck/main.rey:117-132`).

Relevant current-source excerpts (exact lines):

```rey
// src/codegen/main.rey:524-526
if expr instanceof CallExpr {
    if expr.callee instanceof IdentExpr {
        var name = expr.callee.name;
```

```rey
// src/codegen/main.rey:651-657
var tmp = nextTemp(ctx);
if retTy == "void" {
    codeC = codeC + "call void @" + name + "(" + argsOut + ")\n";
    return GenExprResult { ty: "void", val: "0", code: codeC };
}
codeC = codeC + tmp + " = call " + retTy + " @" + name + "(" + argsOut + ")\n";
```

For user-program field reads, the untyped fallback is explicit at
`src/codegen/main.rey:243-282`:

```rey
if sname == "" && obj.ty == "i64" {
    // struct passed as opaque i64 — find struct by field name heuristic
    ...
    if fldItem.name == expr.name {
        ...
    }
    ...
    if sname != "" {
        // cast i64 → struct pointer before GEP
```

### Reproduction

`rey-compiler/README.md:59-67` describes the old Rust-hosted invocation. The
native `main.rey:238-249` instead reads `args()[1]` as the compiler script,
`args()[2]` as command, and `args()[3]` as input. The executable invocation
used was therefore:

```sh
cat > /tmp/rey-v03/call-repro.rey <<'EOF'
func callee(x: int): int {
    return x + 1;
}
func caller(): int {
    return callee(41);
}
func main(): Void {
    println(caller());
}
EOF
PATH=/usr/bin:/bin:/usr/sbin:/sbin python3 -c 'import subprocess; subprocess.run(["$REPO_ROOT/rey-compiler/main", "$REPO_ROOT/rey-compiler/main.rey", "build", "/tmp/rey-v03/call-repro.rey"], timeout=30)'
```

The compiler wrote `call-repro.ll`, then printed (process exit was 0 despite
reporting the compile failure):

```text
DEBUG llc: llc -O0 -filetype=obj "/tmp/rey-v03/call-repro.ll" -o "/tmp/rey-v03/call-repro.o"
DEBUG llc failed: ?
DEBUG clang obj: clang -O0 -c -x ir "/tmp/rey-v03/call-repro.ll" -o "/tmp/rey-v03/call-repro.o"
DEBUG clang obj failed: ?
Error: compile failed
```

Generated IR (`call-repro.ll:81-99`):

```llvm
define i64 @callee(i64 %x) {
define i64 @caller() {
%t0 = call i64 @0(i64 41)
define void @_rey_main() {
%t0 = call i64 @0()
```

The exact verifier command and diagnostic were:

```sh
/usr/bin/clang -c -x ir /tmp/rey-v03/call-repro.ll -o /tmp/rey-v03/call-repro-check.o
```

```text
exit=1
/tmp/rey-v03/call-repro.ll:91:16: error: use of undefined value '@0'
   91 | %t0 = call i64 @0(i64 41)
      |                ^
1 error generated.
```

The field-qualified probe used:

```rey
func target(x: int): int { return x + 1; }
func main(): Void { println(fake.target(5)); }
```

with the same native invocation, replacing the input with
`/tmp/rey-v03/module-call.rey`.
It generated `%t0 = call i64 @0(i64 0, 5 5)` (`module-call.ll:91`) and failed
clang parsing with `error: expected type`. This confirms a field-qualified
callee also reaches `@0`, but malformed argument IR prevents attributing it
more narrowly than that syntax. The current source's candidate
`FieldAccessExpr` paths are `1095-1145` (module-style; target from `fa.name`,
set at 663) and `1147-1167` (receiver-method fallback). A receiver-method probe
containing `Box` exited `-11` before producing IR; receiver-method `@method`
behavior below is source-path evidence, not a successful runtime repro.

## Artifact comparison and provenance

`main` and `main.bin` are byte-identical (SHA-256
`d058091935a43f0d36fd5d089d5160483c89b9fa69af29e1994c159b90713cb4`); both and
`main.bin.ll` have filesystem mtime `2026-06-16 21:18:15`. The `.ll` hash is
`23ff6b046ede65b6eb7ef0bf6752d10c31d733ffa59e2ee506467e02dcf1c5a1`. No
embedded source revision/build manifest was found, so this is evidence about
the stored build artifact, not proof of its exact source input.

Named internal calls in that artifact include:

```llvm
; main.bin.ll:24339
%t24 = call %struct.CompileResult* @compileFile(i8* %t20, i8* %t22, i8* %t23)
```

But in the same `genExprValue` body, the direct call path loads the callee and
then sets the target local to zero:

```llvm
; main.bin.ll:11412-11420
%t2101 = load i64, i64* %expr.addr
%t2102 = inttoptr i64 %t2101 to %struct.CallExpr*
%t2103 = getelementptr inbounds %struct.CallExpr, %struct.CallExpr* %t2102, i32 0, i32 0
%t2104 = load %struct.Expr*, %struct.Expr** %t2103
store i64 0, i64* %name.addr
%t2105 = load i64, i64* %name.addr
```

Thus “clean named calls” in the self-build IR describes compiler-internal
function calls such as `@compileFile`; it does not show that emitted user
program calls were correct. The current repro is the direct evidence for the
user-program failure.

### Which call paths

- **Plain identifier calls:** proven failing. Every such user call enters the
  `IdentExpr` branch at lines 524-526 and emits its extracted `name` at 653/656;
  the repro shows both `callee(41)` and `caller()` become `@0`.
- **Field-qualified callee:** observed `fake.target(5)` emits `@0`, but malformed
  argument IR means the exact candidate branch is not independently established.
  The source has module-style and receiver-method paths at 1095-1167; both
  assemble a target from `fa.name`/`method`, not a resolved symbol.
- **Receiver methods:** source fallback gets `method = fa.name` (663), then
  emits `@method(obj, args...)` (1147-1167). It is not a resolved method
  symbol; the isolated struct probe crashed before IR, so `@0` for this route
  is not claimed as reproduced.
- **Known builtins:** separately hard-coded by name in the direct branch
  (528-614) and field branch (667 onward); this is not typed user-call
  resolution. Do not treat a working builtin as evidence that user calls are
  fixed.

## Silent wrong-code / unsupported paths (source references)

| Path | Exact source behavior | Next fix |
|---|---|---|
| Lambda / dict / tuple values | Parser constructs lambda and dict at `parser/main.rey:756-795`; `TupleExpr` exists in `parser/ast/main.rey:120-124`. `genExprValue` has no matching cases and falls through to `GenExprResult { ty: "i64", val: "0", code: "" }` at `codegen/main.rey:1290`. Tuple construction is also absent from parser grouping (`parser/main.rey:730-735`), so tuple source syntax is not established as a reachable AST today. | Lower each supported value representation; until implemented, diagnose unsupported nodes rather than returning zero. |
| `for x in vec` | `genStmt2` only emits a loop when `iterable instanceof ArrayExpr` (`codegen/main.rey:1651-1678`); all other iterables return `""` at 1679. | Use typed iterable/element type and emit Vec iteration; preserve array-literal support. |
| Index assignment | Assignment handles only `IdentExpr` and `FieldAccessExpr` (`codegen/main.rey:1428-1557`); an `IndexExpr` target reaches empty-string return at 1558. Parser can build indexed assignment targets because `parseAssign` calls `parseCall` (`parser/main.rey:397-405`). | Add typed index lvalue lowering and runtime setter; never silently omit the store. |
| Unknown `instanceof` | Known runtime tags/structs are handled at `codegen/main.rey:326-352`; unknown types return `i1` constant `1` at 353-354. | Checker resolves/validates the right-hand type; codegen consumes its tag/layout or reports an error. |
| Match | `MatchExpr` exists in `parser/ast/main.rey:98-103`, but parser statement dispatch has no Match arm (`parser/main.rey:160-227`) and primary expressions do not parse it (`684-727`). If such a node reaches codegen, it falls through to zero at `codegen/main.rey:1290`. | Parse the frozen match-statement syntax, typecheck exhaustiveness/patterns, then lower explicit branches; reject unsupported matches. |
| Struct field access/assignment | Unknown opaque objects select a struct by field-name heuristic (`codegen/main.rey:243-282`); missing read fields return zero (284-301). Field assignment repeats a first-matching-field heuristic (1477-1509), and returns only RHS code if no field is found (1556). | Consume resolved struct identity + field index/type from typed lowering; remove guess-by-name and emit a diagnostic on invalid layout. |
| Struct methods | Parser stores methods in `StructDeclStmt.methods` (`parser/main.rey:297-327`); codegen's struct pass only stores fields (1817-1832) and its function-emission pass only emits top-level `FuncDeclStmt` (1864-1872). The receiver-method call fallback is name-based (1147-1167). | Preserve receiver/method symbols in typed lowering; emit method definitions and resolved calls explicitly. |
| Optimization | `compileLLVM` defaults to `-O0` and only selects `-O2` when `release` is true (`codegen/main.rey:2053-2061`). Both CLI compile sites pass `false` (`main.rey:185`, `225`), so CLI build/run always use `-O0`. | Add an intentional release/optimization setting; do not claim optimization is active today. |
| Debug output / errors | Tool commands and failure messages are printed at `codegen/main.rey:2072-2092`; errors use `unwrapOr("?")` at 2076, 2082, 2091. `main.rey:259` similarly renders failed run execution as `run error: ?`. `exec` only captures command stdout (`runtime/io.c:76-83`), so diagnostics can be absent from the returned result. | Send diagnostics to stderr with command/status context and preserve subprocess stderr; remove routine DEBUG prints. |

## Phase B fix brief

The checker is currently a stub: `typecheck/main.rey:117-132` returns `Any` from
`inferExpr` and unconditional success from `typecheckProgram`. Typed lowering
must provide, at minimum:

1. **Resolved call target and signature** for every call: stable function
   symbol/LLVM name (not source-text guesses), call kind (user function,
   module export, receiver method, intrinsic), parameter/return types, and
   explicit receiver convention. Preserve builtin selection as explicit
   intrinsic targets.
2. **Resolved field layout**: receiver struct identity, field index, field
   type, and lvalue/read-vs-write information for each field access.
3. **Collection typing**: Vec element type and Dict key/value types, plus
   iterable element type and indexed element/lvalue type. Tuple element types
   and lambda function type/capture data are needed before those forms can be
   lowered safely.
4. **Typed expression/result types and diagnostics** so codegen does not
   guess from `i64`, coerce arbitrary values, or turn unknown cases into valid
   constants.

Minimal codegen changes once that contract exists: have direct and field
call branches consume the resolved target/signature rather than `name`/
`method` strings; use resolved field indices/types instead of the heuristics at
243-282 and 1482-1509; add typed Vec iteration/index stores and the supported
lambda/dict/tuple/match lowerings. For any not-yet-supported typed node, fail
code generation explicitly rather than silently emitting `0`, `true`, or an
empty string. `docs/CONTRACTS.md` was absent when this note was written, so no
schema is presumed; Phase B should align these fields with that contract once
it lands.

## Read-only evidence commands

```text
wc -l rey-compiler/main.rey rey-compiler/src/codegen/main.rey
  266 rey-compiler/main.rey
 2100 rey-compiler/src/codegen/main.rey
cmp -s rey-compiler/main rey-compiler/main.bin  # identical
stat -f 'main inode=%i size=%z mtime=%Sm' -t '%Y-%m-%d %H:%M:%S' rey-compiler/main
stat -f 'main.bin inode=%i size=%z mtime=%Sm' -t '%Y-%m-%d %H:%M:%S' rey-compiler/main.bin
stat -f 'main.bin.ll inode=%i size=%z mtime=%Sm' -t '%Y-%m-%d %H:%M:%S' rey-compiler/main.bin.ll
main inode=791746 size=210336 mtime=2026-06-16 21:18:15
main.bin inode=791747 size=210336 mtime=2026-06-16 21:18:15
main.bin.ll inode=791748 size=1209828 mtime=2026-06-16 21:18:15
shasum -a 256 rey-compiler/main rey-compiler/main.bin rey-compiler/main.bin.ll
d058091935a43f0d36fd5d089d5160483c89b9fa69af29e1994c159b90713cb4  rey-compiler/main
d058091935a43f0d36fd5d089d5160483c89b9fa69af29e1994c159b90713cb4  rey-compiler/main.bin
23ff6b046ede65b6eb7ef0bf6752d10c31d733ffa59e2ee506467e02dcf1c5a1  rey-compiler/main.bin.ll
```
