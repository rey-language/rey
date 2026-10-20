# Native self-host forensics

Observed 2026-10-16 on macOS arm64. The committed native executable was treated as read-only. Test inputs and compiler outputs were confined to `/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/selfhost-recon/`; every potentially hanging compiler/debugger command was alarm-guarded. Build commands put `/usr/bin` first in `PATH`, so the compiler's `clang` fallback resolves to `/usr/bin/clang` (`llc` is absent).

## CLI actually accepted by the native binary

`rey-compiler/main.rey:238-264` reads raw `args()` including argv[0]. It requires at least four entries, reads `cmd = cliArgs[2]`, `path = cliArgs[3]`, and derives the runtime directory from `cliArgs[1]` (`main.rey:240-249`). Thus the native invocation is:

```text
rey-compiler/main <compiler-script-path> <build|run|check> <source-file>
```

For example, from the repository root:

```sh
./rey-compiler/main ./rey-compiler/main.rey check ./rey-compiler/src/lexer/token.rey
```

This is not the usual `<binary> <cmd> <file>` CLI. `--help` is not implemented. The README (`rey-compiler/README.md:61-69`) instead describes invoking the compiler through `rey-v0`, so it does not document this native argv convention.

Reproduction of the short forms mentioned in the defect ledger:

```text
$ cd /Volumes/SSD/Developer/rey-language/rey-lang/rey-compiler
$ perl -e 'alarm 12; exec @ARGV' -- ./main --help
Usage: reyc <build|run|check> <file>
EXIT=0
$ perl -e 'alarm 12; exec @ARGV' -- ./main build rey-compiler/main.rey
Usage: reyc <build|run|check> <file>
EXIT=0
```

The `check` success path generates IR in memory, returns `outputPath: "a.out"` (`main.rey:235`), and prints `Success: a.out` (`main.rey:252-253`); it does not call `compileLLVM` or write that file. Confirmed from the scratch working directory:

```text
a.out_before=absent
Success: a.out
CHECK_EXIT=0
a.out_after=absent
```

Exact command:

```sh
ROOT=/Volumes/SSD/Developer/rey-language/rey-lang
SCR=/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/selfhost-recon
cd "$SCR/cases"
PATH=/usr/bin:/bin:/usr/sbin:/sbin perl -e 'alarm 18; exec @ARGV' -- \
  "$ROOT/rey-compiler/main" "$ROOT/rey-compiler/main.rey" check "$SCR/cases/b00-main.rey"
```

## 1. Self-build SIGSEGV

To avoid the default output path colliding with committed `rey-compiler/main`/`main.bin`, I copied `main.rey` byte-for-byte into scratch and symlinked its `src` directory to the read-only repo source tree. `cmp` returned 0. This exercises the same compiler source and imports while keeping `.ll`, `.o`, and executable outputs outside the repository.

Exact guarded command:

```sh
ROOT=/Volumes/SSD/Developer/rey-language/rey-lang
SCR=/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/selfhost-recon
cd "$ROOT"
PATH=/usr/bin:/bin:/usr/sbin:/sbin perl -e 'alarm 60; exec @ARGV' -- \
  "$ROOT/rey-compiler/main" "$ROOT/rey-compiler/main.rey" build "$SCR/compiler/main.rey"
```

Observed output: none; `EXIT=139` (SIGSEGV). The alarm did not fire.

LLDB was able to stop at `main` and then capture the signal. Command (also alarm-guarded at 120 seconds):

```sh
PATH=/usr/bin:/bin:/usr/sbin:/sbin perl -e 'alarm 120; exec @ARGV' -- lldb --batch \
  -o 'breakpoint set -n main' \
  -o 'run /Volumes/SSD/Developer/rey-language/rey-lang/rey-compiler/main.rey build /private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/selfhost-recon/compiler/main.rey' \
  -o continue -k 'register read x0 x1' -k 'thread backtrace all' \
  -- /Volumes/SSD/Developer/rey-language/rey-lang/rey-compiler/main
```

Top of the captured backtrace:

```text
stop reason = EXC_BAD_ACCESS (code=1, address=0x0)
frame #0: rey_hm_has + 44
  ldrb w11, [x1]
x0 = 0x00000076cd8b3a60
x1 = 0x0000000000000000
frame #1: genExprValue + 2184
frame #2: genExprValue + 21344
frame #3: genStmt2 + 68
frame #4: genStmt2 + 17984
frame #5: genFunc2 + 1340
frame #6: codegenProgram2 + 1268
frame #7: generateIR + 20
frame #8: compileFile + 892
frame #9: _rey_main + 188
frame #10: main + 12
frame #11: dyld`start + 6688
```

The fault is during IR generation (`codegenProgram2`/`generateIR`), before `compileLLVM` or clang. Evidence establishes that `rey_hm_has` receives a null key; it does **not** establish which source expression produced it. Disassembly around the caller shows a `rey_struct_instanceof` check followed by `rey_hm_has` at `genExprValue+2180`, consistent with `genExprValue`'s identifier branch at `rey-compiler/src/codegen/main.rey:202-209` (`ctx.locals.has(expr.name)`). That is a candidate, not a source-line attribution: this executable has no source-line debug mapping, and other nearby map lookups should also be checked. The runtime hash path dereferences its key in `rey-compiler/runtime/hashmap.c:26-29` (`hm_hash`) and is called by `rey_hm_has` at lines 104-105.

**Next step:** stop at `rey_hm_has` during this reproducer and record the caller expression/key before lookup; then fix the null-producing AST/codegen path (rather than treating the runtime null dereference as the whole root cause). Rebuild and repeat stage-1/stage-2 self-host with scratch outputs.

## 2. `check token.rey` bounds panic

Exact guarded command:

```sh
cd /Volumes/SSD/Developer/rey-language/rey-lang
perl -e 'alarm 30; exec @ARGV' -- \
  ./rey-compiler/main ./rey-compiler/main.rey check rey-compiler/src/lexer/token.rey
```

Observed:

```text
rey panic: Vec index out of bounds
EXIT=1
```

LLDB breakpoint on `rey_panic_cstr` captured:

```sh
PATH=/usr/bin:/bin:/usr/sbin:/sbin perl -e 'alarm 60; exec @ARGV' -- lldb --batch \
  -o 'breakpoint set -n rey_panic_cstr' \
  -o 'run /Volumes/SSD/Developer/rey-language/rey-lang/rey-compiler/main.rey check /Volumes/SSD/Developer/rey-language/rey-lang/rey-compiler/src/lexer/token.rey' \
  -o 'thread backtrace all' -o quit \
  -- /Volumes/SSD/Developer/rey-language/rey-lang/rey-compiler/main
```

```text
frame #0: rey_panic_cstr
frame #1: rey_vec_get + 48
frame #2: codegenProgram2 + 4764
frame #3: generateIR + 20
frame #4: compileFile + 892
frame #5: _rey_main + 188
frame #6: main + 12
frame #7: dyld`start + 6688
```

This is a codegen vector access, not evidence of a lexer failure. The runtime check is at `rey-compiler/runtime/vec.c:47-50`. A smaller reproducer is `struct Empty {}\nfunc main() {}`; `build` and `check` both reach the same `Vec index out of bounds` panic. A plain enum does not.

The source/disassembly pair identifies a concrete codegen dispatch defect for this case:

- `rey-compiler/src/codegen/main.rey:1956-1966` iterates `ctx.structDefs.keys()` and then calls `ctx.structDefs.get(name)` at line 1961.
- In `genExprValue`, the generic vector `get` arm at lines 858-864 takes any `get` whose sole argument has type `i64`; the HashMap `get` arm is later at lines 964-977. This receiver dispatch does not distinguish a Vec from a HashMap.
- LLDB disassembly at `codegenProgram2+4680..4760` shows `rey_hm_keys`, a Vec length/get for the key, then a **second `rey_vec_get`** using the `ctx.structDefs` map handle and that key. The second call faults at `rey_vec_get`.

This explains why even an empty struct declaration triggers the panic: the struct-definition map is nonempty and the type-emission pass performs the miscompiled map lookup. It also explains the token fixture, which contains several structs. **Action:** dispatch `.get` by the receiver's resolved type (or typed lowering), with regressions for `struct Empty {}` and `check rey-compiler/src/lexer/token.rey`.

## 3. `check enum_match.rey` hang

Exact guarded reproduction:

```sh
cd /Volumes/SSD/Developer/rey-language/rey-lang
perl -e 'alarm 25; exec @ARGV' -- \
  ./rey-compiler/main ./rey-compiler/main.rey check tests/enum_match.rey
```

Observed: no output before the guard; `EXIT=142` (SIGALRM).

For a live sample, I launched only the native child with a 45-second alarm, sampled that PID for three seconds, then sent SIGTERM to that child and waited for it:

```sh
ROOT=/Volumes/SSD/Developer/rey-language/rey-lang
SCR=/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/selfhost-recon
cd "$ROOT"
PATH=/usr/bin:/bin:/usr/sbin:/sbin perl -e 'alarm 45; exec @ARGV' -- \
  "$ROOT/rey-compiler/main" "$ROOT/rey-compiler/main.rey" check "$ROOT/tests/enum_match.rey" > "$SCR/enum-hang.log" 2>&1 &
pid=$!
sleep 2
PATH=/usr/bin:/bin:/usr/sbin:/sbin perl -e 'alarm 15; exec @ARGV' -- \
  /usr/bin/sample "$pid" 3 -file "$SCR/enum-hang.sample.txt"
kill -TERM "$pid"
wait "$pid"
```

The sample command returned 0; the native child was then sent SIGTERM by this shell and `wait` returned 143. No other process was signaled.

Observed sample excerpt:

```text
Physical footprint:         2.5G
Physical footprint (peak):  2.5G
Call graph:
    656 isAtEnd  (in main) + 128
      109 parseEnumDecl  (in main) + 300
      98 parseEnumDecl  (in main) + 276
      87 parseEnumDecl  (in main) + 216
      80 toSpan  (in main) + 28
      71 consume  (in main) + 108
```

The smallest tested hang is just:

```rey
enum E { A(x: int) }
```

Exact minimal-repro commands (both alarm-guarded):

```sh
ROOT=/Volumes/SSD/Developer/rey-language/rey-lang
SCR=/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/selfhost-recon
perl -e 'alarm 8; exec @ARGV' -- "$ROOT/rey-compiler/main" "$ROOT/rey-compiler/main.rey" check "$SCR/cases/min-enum-payload.rey"
perl -e 'alarm 12; exec @ARGV' -- "$ROOT/rey-compiler/main" "$ROOT/rey-compiler/main.rey" check "$SCR/cases/min-enum-plain.rey"
```

The payload case produced no output and `EXIT=142`. In contrast, `enum E { A }` produced `Success: a.out`, `EXIT=0`.

The sampled loop maps to `rey-compiler/src/parser/main.rey:330-348`. `parseEnumDecl` consumes an identifier at line 341, then only advances for a comma at lines 343-344 or a right brace at lines 337-340. At `(` in a payload variant, `consume` reports/returns `tokens[0]` without advancing (`parser/main.rey:102-109`), so the loop repeats forever. The repeated parser frames plus source control flow make this a confirmed no-progress loop, not a guess. **Action:** parse variant payloads, or make parse errors advance/abort so every loop iteration makes progress; add the one-variant payload case as a regression.

## Small-input build bisection

Inputs were written under the scratch `cases/` directory. Each build used the same native invocation and an 18-second alarm:

```sh
ROOT=/Volumes/SSD/Developer/rey-language/rey-lang
SCR=/private/var/folders/dt/_kd6szqs4gxct6stfgskt30m0000gn/T/opencode/rey-v03/selfhost-recon
PATH=/usr/bin:/bin:/usr/sbin:/sbin perl -e 'alarm 18; exec @ARGV' -- \
  "$ROOT/rey-compiler/main" "$ROOT/rey-compiler/main.rey" build "$SCR/cases/<case>.rey"
```

| Case / added feature | Minimal relevant source | Result |
|---|---|---|
| `b00-main` baseline | `func main() {}` | Compiled to scratch; exit 0 |
| `b10-typed-local` | `func main() { var n: int = 1; }` | Compiled to scratch; exit 0 |
| `b02-func-only` | `func id(x: int): int { return x; }` plus empty `main` | Compiled to scratch; exit 0 |
| `b11-control-main` | typed local plus `if n > 0 { n = 2; }` in `main` | Compiled to scratch; exit 0 |
| `b07-empty-struct` | `struct Empty {}` plus empty `main` | `rey panic: Vec index out of bounds`; exit 1 — first crash in this ladder |
| `b05-enum` | `enum Choice { None, Other }` plus empty `main` | Compiled to scratch; exit 0 |
| `b06-enum-payload` | `enum Choice { Some(value: int) }` plus empty `main` | Timed out; exit 142 |

Representative successful build output (the same `llc`-missing / clang-fallback sequence appeared on the passing cases):

```text
DEBUG llc: llc -O0 -filetype=obj ".../b00-main.ll" -o ".../b00-main.o"
sh: llc: command not found
DEBUG clang obj: clang -O0 -c -x ir ".../b00-main.ll" -o ".../b00-main.o"
DEBUG clang link: clang ".../b00-main.o" -L".../rey-compiler/runtime" -lrey_rt -o ".../b00-main"
Compiled: .../b00-main.rey
Success: .../b00-main
EXIT=0
```

Separate control/call probe (`b03-control-only`) emitted `call i64 @0(i64 1)`; `/usr/bin/clang` reported `error: use of undefined value '@0'`. The native process still returned `EXIT=0`. This is an additional codegen/exit-status defect, not the first crash in the feature ladder.

## Defect-specific next actions

1. Fix method resolution for Vec-vs-HashMap `.get`; use the receiver's typed kind, not only argument ABI type. Add the empty-struct and token-file regressions.
2. Fix the parser's enum-variant loop progress/payload handling; run the minimal payload reproducer under an alarm until it returns a diagnostic rather than hanging.
3. For self-build, instrument the null `rey_hm_has` key and caller expression. The LLDB evidence proves a null key in `genExprValue`, but does not yet prove whether `IdentExpr.name` at lines 202-204 or another nearby lookup is the producer. Keep this root cause labeled a candidate until the actual source expression is identified.
4. Repair CLI argument contract/exit codes separately: native `--help` is currently only the short-argv usage path, successful `check` prints `a.out`, and clang failures can print `Error` while returning process exit 0.
