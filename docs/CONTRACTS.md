# Rey v0.3 compiler and runtime contracts

Status: interface handoff for Phase B, checked against `SPEC.md`, the charter,
and source files listed at the end. SPEC/charter are language authority. This
document distinguishes **CURRENT** source facts from **PROPOSED** typed-AST/
lowering rules. A proposal is not evidence that the old compiler already
implements it. Anything not decidable without inventing an ABI is marked
**OPEN — resolve in Phase B**.

> Current owner rules recorded in `SPEC.md` and charter §10 (2026-10-17): enum
> payload declarations allow all-positional
> (`Count(int)`) or all-named (`Count(value: int)`) slots, never mixed; scalar
> lambda captures snapshot at creation and bindings are read-only while
> aggregates remain shared references; signed integer overflow (including
> increment/decrement and `INT64_MIN / -1`) is a runtime error; strings preserve
> UTF-8 source bytes and raw `\xNN` bytes with ASCII-only case conversion; failed
> numeric string conversions are runtime errors. These are normative language
> rules; their implementation is not implied by this contract.

## 1. Parser → checker → codegen handoff

### 1.1 Current parser AST (source facts)

The parser returns concrete Rey structs from `src/parser/ast/main.rey`; despite
the `Expr`/`Stmt` tag structs, the current nodes are separate structs tested by
`instanceof`. `Program` is `{ statements: [Stmt], span: Span }`. All listed spans
are the lexer `Span { start, end, line, column }`; offsets are zero-based,
half-open byte positions and line/column are one-based ints.
Current `Program`/`Span` carry no source path or AST node identity.
For the charter's declaration terminology, `FuncDecl`, `StructDecl`,
`EnumDecl`, `ImportDecl`, and `VarDecl` correspond to the current concrete names
`FuncDeclStmt`, `StructDeclStmt`, `EnumDeclStmt`, `ImportStmt`, and
`VarDeclStmt` (with `ConstDeclStmt` for const).
The following are the fields currently present (not typechecker annotations):

| Current node | Fields |
|---|---|
| `Literal` | `kind: LiteralKind`, `text: string`; `LiteralKind = String, Char, Int, Float, Bool, Null` |
| `Expr` | `kind: ExprKind`, `span`; tags are `Error, Literal, Identifier, Binary, Unary, Update, Call, EnumVariant, Index, FieldAccess, Lambda, Match, If, Tuple, Array, Dict, StructLiteral` |
| `TypeExpr` | `kind: TypeExprKind`, `name: string`, `inner: TypeExpr?`, `members: [TypeExpr]`, `params: [TypeExpr]`, `result: TypeExpr?`, `span`; kinds `Error, Named, Nullable, Union, Tuple, Function, Applied` |
| `ErrorExpr` | `message: string`, `span` |
| `LiteralExpr` | `kind: LiteralKind`, `text`, `span` |
| `IdentExpr` | `name`, `span` |
| `BinaryExpr` | `left: Expr`, `op: string`, `right: Expr`, `span`; `instanceof` is currently encoded as `op == "instanceof"` with an `IdentExpr` on the right |
| `UnaryExpr` | `op: string`, `right: Expr`, `span` |
| `UpdateExpr` | `target: Expr`, `op: string` (`++` or `--`), `prefix: bool`, `span` |
| `AssignExpr` (proposed) | `target: Expr`, `op: string`, `value: Expr`, `span`; assignments are expressions in SPEC grammar. Current AST represents assignment as `AssignStmt`. |
| `CallExpr` | `callee: Expr`, `args: [Expr]`, `span` |
| `EnumVariantExpr` | `enumName: string`, `variant: string`, `args: [Expr]`, `span` |
| `IndexExpr` | `target: Expr`, `index: Expr`, `span` |
| `FieldAccessExpr` | `object: Expr`, `name: string`, `span` |
| `InterpolatedStringExpr` (proposed) | `parts: [StringPart]`, `span` |
| `StringPart` (proposed) | `kind: StringPartKind (Text | Expr)`, `text: string`, `expr: Expr?`, `span` |
| `LambdaExpr` | `params: [string]`, `body: [Stmt]`, `paramTypeExprs: [TypeExpr?]`, `bodyExpr: Expr?`, `span`; SPEC syntax is `(x: int) => expression` or block; `()` is valid. |
| `MatchExpr` | `expr: Expr`, `arms: [MatchArm]`, `span` |
| `MatchArm` | Current: `pattern: Pattern`, `guard: Expr`, `body: Expr`, `span`. SPEC has no guard syntax; v0.3 parser must not accept/evaluate guards. Proposed body is a `Stmt`: `BlockStmt` or `ExprStmt` for the SPEC block/expression-statement alternatives. |
| `IfExpr` | `condition`, `thenBranch`, `elseBranch`: `Expr`; `span` |
| `TupleExpr`, `ArrayExpr` | `elements: [Expr]`, `span` |
| `DictExpr` | `entries: [DictEntry]`, `span` |
| `DictEntry` | `key: Expr`, `value: Expr`, `span` |
| `StructLiteralExpr` | `name: string`, `fields: [StructInitField]`, `span` |
| `StructInitField` | `name: string`, `value: Expr`, `span` |
| `Stmt` | `kind: StmtKind`, `span`; tags are `VarDecl, ConstDecl, FuncDecl, StructDecl, EnumDecl, Import, If, Assign, Return, Break, Continue, While, For, Loop, Block, ExprStmt, Match` |
| `MatchStmt` | `scrutinee: Expr`, `arms: [MatchStmtArm]`, `span` |
| `MatchStmtArm` | `pattern: Pattern`, `body: Stmt`, `span`; body is `BlockStmt` or `ExprStmt` per parser |
| `VarDeclStmt`, `ConstDeclStmt` | `name: string`, `typeTag: string`, `typeExpr: TypeExpr?`, `value: Expr`, `span` |
| `FuncDeclStmt` | `name: string`, `params: [FuncParam]`, `returnType: string`, `returnTypeExpr: TypeExpr?`, `isPub: bool`, `body: [Stmt]`, `span` |
| `FuncParam` | `name: string`, `typeTag: string`, `typeExpr: TypeExpr?`, `default: Expr`, `hasDefault: bool`, `variadic: bool` |
| `StructDeclStmt` | `name: string`, `fields: [StructField]`, `methods: [FuncDeclStmt]`, `isPub: bool`, `span` |
| `StructField` | `name: string`, `typeTag: string`, `typeExpr: TypeExpr?`, `isPub: bool` |
| `EnumDeclStmt` | `name: string`, `variants: [string]`, `variantDecls: [EnumVariantDecl]`, `isPub: bool`, `span` |
| `EnumVariantDecl` | `name: string`, `payloadNames: [string]`, `payloadTypes: [TypeExpr]`, `span` |
| `ImportStmt` | `path: string`, `items: [string]`, `sourceKind: ImportSourceKind`, `namespaceName: string`, `span`; `ImportSourceKind = File | Module` |
| `ReturnStmt` | `value: Expr`, `span` (no-value return currently uses Null literal sentinel) |
| `BreakStmt`, `ContinueStmt` | `label: string`, `span` (parser supplies empty label) |
| `WhileStmt` | `condition: Expr`, `body: [Stmt]`, `span` |
| `ForStmt` | `variable: string`, `iterable: Expr`, `body: [Stmt]`, `span`; proposed typed annotation `iteration: IterationInfo?` |
| `LoopStmt` | `body: [Stmt]`, `span` |
| `BlockStmt` | `statements: [Stmt]`, `span` |
| `ExprStmt` | `expr: Expr`, `span` |
| `IfStmt` | `condition: Expr`, `thenBranch: BlockStmt`, `elseBranch: BlockStmt?`, `span` |
| `AssignStmt` | `target: Expr`, `op: string`, `value: Expr`, `span` |
| `Pattern` | `kind: PatternKind`, `span`; kinds include `Error, Wildcard, Literal, Variable, EnumVariant, Struct, Or, Range` |
| `ErrorPattern` | `kind: PatternKind`, `span` |
| `WildcardPattern` | `kind: PatternKind`, `span` |
| `LiteralPattern` | `kind: PatternKind`, `literal: Literal`, `span` |
| `VariablePattern` | `kind: PatternKind`, `name: string`, `span` |
| `EnumVariantPattern` | `kind: PatternKind`, `enumName: string`, `variant: string`, `payloads: [Pattern]`, `span` |
| `RangePattern` | `kind: PatternKind`, `start: Literal`, `end: Literal`, `inclusive: bool`, `span` |
| `StructPattern` | `kind: PatternKind`, `structName: string`, `fields: [PatternField]`, `span` |
| `PatternField` | `name: string`, `pattern: Pattern`, `span` |
| `OrPattern` | `kind: PatternKind`, `left: Pattern`, `right: Pattern`, `span` |
| `ParseError` | `message: string`, `span` |

The current shared AST/parser now contains TypeExpr, UpdateExpr, enum
construction/payload patterns, `MatchStmt`, `RangePattern`, tuple parsing, and
the `StructLiteral` tag. These are **existing source changes**, not a promise
that the checker/codegen implement them. No interpolation expression/parts or
assignment-expression node exists yet. `ExprKind.Match`/`MatchExpr`, `IfExpr`,
and `OrPattern` are legacy/unreachable for v0.3: SPEC excludes match
expressions, does not define if-expressions, and its pattern grammar has no
or-pattern. Preserve structs if bootstrap needs them; do not accept them as
v0.3 syntax.

### 1.2 Existing syntax additions and remaining gaps

The shared parser/AST now has `TypeExpr`, typed declaration annotation fields,
pub flags, variadic/default parameter metadata, enum payload declarations and
construction/patterns, `UpdateExpr`, `MatchStmt`/`RangePattern`, canonical import
fields, tuple parsing, and the `StructLiteral` tag. Preserve these current fields
and exported parser APIs; they are additive progress, not evidence of complete
typechecking or codegen.
Native `EnumVariantDecl.payloadNames` is parallel to `payloadTypes` (empty name
means positional). The current parser accepts a name independently per slot;
checker/parser must enforce one mode per variant and reject mixed named/positional
payload slots.

Still add an `Interpolation` tag to `ExprKind` and explicit
`InterpolatedStringExpr { parts: [StringPart], span }`,
with `StringPart { kind: Text | Expr, text: string, expr: Expr?, span }`;
ordinary-string parts evaluate once left-to-right, `{{`/`}}` escape braces, and
triple-quoted raw strings do not interpolate. No interpolation syntax/node was
found in the current lexer/parser/AST. Assignment is currently only
`AssignStmt`; SPEC grammar includes assignment expressions, so add an `Assign`
tag and explicit `AssignExpr { target, op, value, span }` only if required by the parser path,
keeping statement assignment compatible.

`MatchStmt` is the v0.3 form; its current `MatchStmtArm.body: Stmt` represents a
block or semicolon-terminated `ExprStmt`. Legacy `MatchExpr`/`MatchArm.guard`
are not in SPEC and must not affect v0.3 parsing or lowering. `RangePattern`
endpoints are integer `Literal`s with `start <= end` and inclusive semantics.
The current TypeExpr struct's `members` field stores union/tuple/applied args;
`params`/`result` store function types. Keep `typeTag` spelling fields alongside
`typeExpr` during migration; codegen must consume checked types, never parse
type strings. `export` remains an error with a remove-`export` fix-it.

SPEC dict keys are strings; the current `DictEntry.key` is still an arbitrary
`Expr`. Parser/checker must canonicalize bare identifier keys as their string
names, and decode quoted keys, rather than resolve a bare identifier as a value.
Current `ReturnStmt` still uses a Null-expression sentinel for `return;`; keep
`return;` distinct from `return null;` with a presence bit/optional value in the
typed handoff.

For cross-engine payload compatibility, the current Rust oracle AST is
`Stmt::EnumDeclWithPayload { name, variants: Vec<EnumVariantDecl> }`, with
`EnumVariantDecl { name, payloads: Vec<EnumPayloadDecl>, span }` and
`EnumPayloadDecl { name: Option<String>, ty: Type, span }` from
`compiler/v1/src/ast/stmt.rs`. Its expression form is
`Expr::EnumVariant { enum_name: Option<String>, variant, args, span }`; its
pattern form is `Pattern::EnumVariantPayload { enum_name: Option<String>,
variant, payloads }`. The native AST parallels this with
`variantDecls`/`payloadNames`/`payloadTypes`; retain optional/empty-name
ambiguity until checker resolution rather than inventing an enum qualifier for
bare match variants. Payload mode is all-positional or all-named per variant,
never mixed.

### 1.3 Type-expression schema

`TypeExpr` is present in the current parser AST; it is syntax, distinct from
the checked `Type` which the checker must produce.

| `TypeExprKind` | Fields |
|---|---|
| `Named` | `name: string` (primitive names and nominal user types) |
| `Nullable` | `inner: TypeExpr` |
| `Union` | `members: [TypeExpr]` (checker canonicalizes/deduplicates) |
| `Tuple` | `members: [TypeExpr]`; one-element tuple syntax requires trailing comma `(T,)` |
| `Function` | `params: [TypeExpr]`, `result: TypeExpr?` (`(int) -> string`; `->` only here) |
| `Applied` | `name: string`, `members: [TypeExpr]`; accept/erase `Vec<T>`, `HashMap<K,V>` (checker requires `K == string`), `Option<T>`, and `Result<T,E>`. Bare Vec/HashMap use `any` elements/values; no generic declarations/instantiation. |
| `Error` | error kind with empty payloads and the source span |
| Common struct | `kind: TypeExprKind`, `name`, `inner`, `members`, `params`, `result`, `span` |

Canonical primitive names are lowercase (`int`, `float`, `bool`, `string`,
`void`, `char`, `any`). Capitalized aliases are errors with fix-it per charter.
Current checked `Type` is `{ kind: string, name: string, sourcePath: string,
moduleId: string, inner: Type?, key: Type?, value: Type?, params: [Type],
ret: Type? }`; `TypeKind` includes
`Any, Void, Bool, Int, Float, String, Char, Null, Array, Dict, Tuple, Struct,
Enum, Function, Option, Result, Nullable, Union, Unknown`. `Symbol` is
`{ name: string, ty: Type, isConst: bool }`. These are checker-internal types,
not annotations stored in the parser AST.

### 1.4 Current checker/driver/codegen API packet

The checker source declares these exported APIs:

```rey
export pub func typecheckGraph(graph: SourceGraph): TypecheckResult
export pub func typecheckAt(program: Program, sourcePath: string, source: string): TypecheckResult
export pub func typecheck(program: Program): TypecheckResult
struct TypecheckResult { pub ok: bool, pub errors: [TypeError], pub checkedProgram: CheckedProgram? }
struct TypeError { pub message: string, pub span: int, pub sourcePath: string, pub moduleId: string }
struct SourceUnit { pub sourcePath: string, pub moduleId: string, pub source: string, pub isEntry: bool, pub program: Program }
struct ImportEdge { pub fromPath: string, pub toPath: string, pub sourceKind: ImportSourceKind, pub namespaceName: string, pub items: [string], pub span: Span }
struct SourceGraph { pub entryPath: string, pub units: [SourceUnit], pub edges: [ImportEdge], pub errors: [string] }
struct CheckedProgram { pub entryPath: string, pub units: [SourceUnit], pub edges: [ImportEdge], pub expressions: [CheckedExprInfo], pub bindings: [CheckedBindingInfo], pub symbols: [ResolvedSymbol] }
struct CheckedExprInfo { pub sourcePath: string, pub moduleId: string, pub nodeId: int, pub expression: Expr, pub ty: Type, pub symbol: ResolvedSymbol?, pub receiverType: Type, pub structIndex: int, pub fieldIndex: int, pub enumIndex: int, pub variantIndex: int }
struct CheckedBindingInfo { pub sourcePath: string, pub moduleId: string, pub nodeId: int, pub declaration: any, pub name: string, pub ty: Type, pub kind: string, pub isConst: bool, pub parameterIndex: int }
struct ResolvedSymbol { pub sourcePath: string, pub moduleId: string, pub sourceName: string, pub llvmName: string, pub kind: string, pub intrinsicId: string, pub isPub: bool, pub signature: Type, pub callShape: CallShape, pub declarationIndex: int, pub receiverType: Type, pub receiverPassing: string }
struct CallShape { pub requiredCount: int, pub fixedCount: int, pub variadicIndex: int }
struct FunctionSig { pub params: [Type], pub required: int, pub variadic: bool, pub variadicType: Type, pub ret: Type, pub inferReturn: bool, pub owner: string, pub sourcePath: string, pub moduleId: string, pub isPub: bool, pub declarationIndex: int }
```

The source-level `typecheckGraph` path records checked
expressions for a single source unit with no import edges: it sets
`recording = true`, constructs `CheckedProgram`, and returns it in
`TypecheckResult.checkedProgram`. Importing graphs are currently
rejected with cross-module-resolution errors and an empty checked-expression /
symbol table. `typecheckAt` wraps one Program with a source path/source string;
`typecheck(program)` is a compatibility wrapper using `"<single-file>"` and
empty source text. The CLI driver builds a graph with real source units.

`CheckedExprInfo` is a side table containing source/module/node id, source Expr,
checked Type, optional symbol/receiver, and struct/field/enum/variant indexes.
It has no true/false narrowing fields, although branch narrowing is checked
transiently. `recordExprInfo` allocates node IDs during traversal and searches
for existing rows by `(sourcePath,moduleId,expression == expr)` rather than a
parser-provided node identity; keep that identity collision risk explicit.
`ResolvedSymbol` carries source/module paths, source/LLVM names, kind/intrinsic,
`isPub`, signature/call shape, declaration index, and receiver convention. Its
producer is currently single-unit only; `TypecheckContext` starts blank and is
seeded from `SourceUnit` only in `typecheckSingle`.

`TypeError.span` remains a raw `Span.start` offset, with sourcePath/moduleId
fields. Driver formatting now finds the matching `SourceUnit`, but if span is
negative/unknown it still prints `?`. The single-file compatibility
`typecheck(program)` uses a placeholder path and no source. `FunctionSig` carries
module/pub/declaration fields, but cross-module checking is still rejected. The
`create` method-resolution path still reports that module ownership metadata is
required.

The codegen source declares these exported APIs:

```rey
export pub func generateIR(program): string
export pub func generateIRChecked(checked: CheckedProgram): Result<string,string>
export pub func compileLLVM(ir: string, outputPath: string, release: bool, rtDir: string): Result<string,string>
```

The checked entrypoint accepts `CheckedProgram`, rejects graphs other than a
single import-free unit, runs internal `codegenProgramChecked`, converts its
`REY_CODEGEN_ERROR` marker to `Result.Err`, and otherwise returns `Result.Ok(ir)`.
That internal helper sets the checked wrapper/source/module on `CodegenContext`;
`cgFindExprInfo` looks up `CheckedExprInfo` and checked direct/field paths use
resolved metadata. `generateIR(program)` remains the legacy untyped path through
`codegenProgram2`. Current `CompileResult` is `{ok: bool, errors: any, ir: string,
outputPath: string}`. These are source-level interfaces; the checked path has
not been behavior-verified through a native compiler build/run.

The current driver now calls `typecheck.typecheckGraph(graph)`, checks `ok`,
returns silently for `check`, and for build/run passes
`typeResult.checkedProgram` to `codegen.generateIRChecked`. That producer call
works for a single unit without import edges; `typecheckGraph` deliberately
returns cross-module-resolution errors for imports. The driver and current
codegen now share the `generateIRChecked(CheckedProgram): Result<string,string>`
call in source for import-free builds. Neither import-free checked behavior nor
multi-module checked behavior is verified through a native compiler build/run.
Keep `generateIR(Program)` as a compatibility API; do not route the typed build
path back through raw AST or source-name lookup.

### 1.5 Current checked metadata and handoff gaps

At the inspected checker snapshot, a single source unit without imports uses
`typecheckGraph` to set
`TypecheckContext.recording = true`, records `CheckedExprInfo` rows, and returns a
`CheckedProgram` containing that unit, expression rows, and collected symbols.
This is real producer-side metadata, not merely a proposed schema. It records
sourcePath/moduleId/nodeId, inferred `ty`, resolved symbol/receiver, and struct/
field/enum/variant indexes. `CheckedProgram.bindings` also contains binding IDs,
types, kind, constness, and parameter index for locals/globals/params/loop vars.
There are no explicit true/false narrowing fields; branch environments narrow
types transiently.

Important producer limits:

* `typecheckGraph` only calls `typecheckSingle` when there is exactly one unit and
  no import edges. Any import edge currently returns errors saying cross-module
  checked resolution is not implemented (with empty expression/symbol arrays).
  Imported program lowering therefore is not supported by this producer yet.
* `recordExprInfo` allocates nodeId during checking, but identifies an existing
  record by `(sourcePath,moduleId, expression == expr)` rather than a parser
  node identity. Preserve the source/module pair and ensure structurally equal
  AST nodes cannot collapse; span alone is never a safe key.
* `typecheckSingle` sets sourcePath/moduleId from its SourceUnit. The public
  `typecheck(program)` wrapper instead calls `typecheckAt` with
  `"<single-file>"` and empty source. The CLI uses `typecheckGraph`, not that
  wrapper.
* TypeError carries raw start offset plus sourcePath/moduleId, but no Span/line/
  column. Driver reconstructs line/column from the matching SourceUnit; unknown
  offsets still become `?`.
* `ResolvedSymbol` is populated with source/module name, llvmName, kind, intrinsic,
  pub, signature, call shape, declaration and receiver metadata. `FunctionSig`
  also stores owner/source/module/pub. Nevertheless current registration keys
  top-level functions by bare name and methods by `Struct.method`; cross-module
  visibility/name resolution has not been completed. `create` still diagnoses
  missing module ownership metadata.

Consumer status is partial: the driver passes `typeResult.checkedProgram` to
`codegen.generateIRChecked` for build/run, and the current codegen export
consumes that wrapper for a single import-free unit. `codegenProgramChecked` and
`cgFindExprInfo` use checked expressions/symbols for call and field lowering.
However, `cgFindExprInfo` compares `expression == expr` with source/module and
does not use stored nodeId as its lookup key; it must use
`(sourcePath,moduleId,nodeId)` to avoid structural-equality collisions. Codegen
does not currently read `CheckedProgram.bindings`; local storage remains
identifier-name keyed. Multi-module codegen remains blocked by the checker's
cross-module error path.

**OPEN integration detail:** complete the existing exported checked-codegen
consumer for multi-unit graphs after typechecker name/privacy resolution exists.
Keep `CheckedProgram` source units and `ResolvedSymbol` ownership; private
declarations remain usable within their module, imports enforce `isPub`.
`CheckedExprInfo.ty` carries a checked use type, but explicit branch-edge
narrowing facts are not stored.
Default-argument expansion point (call plan vs callee prologue) is OPEN;
variadic `CallShape` packs extras into one Vec slot.

## 2. LLVM IR conventions

### 2.1 Evidence and scalar ABI

`rey-compiler/main.bin.ll` is the checked-in self-build IR. It shows private
string globals `@.strN`, LLVM scalar/layout conventions, and a C entry wrapper
calling `_rey_main`. Its compiler-internal named calls (e.g. `@compileFile`)
do **not** prove emitted user-program calls work: `docs/NATIVE-ROOTCAUSE.md`
traces the stored emitter's direct-call target extraction to zero and records
the user repro that emitted `@0`. The current codegen source tries to assemble
`@<callee name>`, but the self-compiled emitter reads nested `IdentExpr.name`
as zero. The current codegen source emits `%tN` temporaries and `thenN` / `elseN` /
`endifN` / `while.condN` / `loop.bodyN`-style labels. It resets its shared
counter per function. Preserve these spellings where practical.

| Rey value | Current/source evidence | Handoff rule |
|---|---|---|
| `int` | `i64` in `llvmTypeFromTagCtx`; self-build calls/params use `i64` | Keep `i64`. |
| `float` | `double` in current type map and `rey_float_to_str(double)` ABI | Use `double`; float arithmetic is missing in current `genExprValue`. Use `fadd/fsub/fmul/fdiv/frem` and `fcmp` where checked types require them. SPEC says mixed int/float `%` returns float; `int % int` remains int. Divide/remainder by zero and signed integer overflow (including `++`/`--` and `INT64_MIN / -1`) are runtime errors per current SPEC §6.3 / charter §10. |
| `bool` | Current codegen type helpers map source bool to `i64`; `main.bin.ll` uses `i1` for comparisons/branches then `zext i1 ... to i64` for values/returns. Runtime bool-like C APIs return `int64_t`. | Keep `i64` normalized 0/1 for stored/source-level values, params/returns and runtime ABI; use `i1` only for LLVM predicates/branches; zext/trunc explicitly at boundaries. Do not pass `i1` to C `int64_t`. |
| `string` | `i8*` data pointer; `rey_str_new` wraps literal bytes | Keep `i8*` pointing at data, not header. Normalize source type names to lowercase; current codegen still accepts capitalized `String`/`Void` aliases for compatibility, but checker must reject them. |
| `void` | LLVM `void`; `_rey_main` is `void` | Keep `void` return only; no value slot. |
| `char` | Literal kind exists; `rey_str_char_at` returns `char*` containing one byte, not a C scalar | SPEC/owner freezes char as one byte and preserves string bytes; LLVM/source-value representation remains **OPEN — resolve in Phase B**. Do not silently treat a one-byte Rey string pointer as an integer char. |
| struct | current code emits `%struct.Name*`; heap data pointer is 16 bytes after tag header | Preserve typed pointer in LLVM until opaque storage boundary; use casts at those boundaries. |
| enum | current codegen stores payload-free variants as zero-based `i64` ordinal | Preserve that encoding only for payload-free enums. Owner/SPEC payload forms are positional or named (all-or-none per variant); their storage/discriminant layout is **OPEN — resolve in Phase B** because current runtime/codegen does not define it. |
| Vec / HashMap / Result / Option | `i64` opaque heap pointer | Keep `i64` at current C/runtime boundary; statically known values can be cast/decoded for element types. |

`main.bin.ll` struct declarations show ordinary LLVM field order and generated
functions with exact typed params/returns. It does not establish a general
public/foreign function ABI beyond those emitted declarations.

### 2.2 Symbols, functions, globals, labels

* **Function symbols:** current source intends direct top-level names as LLVM
  `@<source-name>`, but stored-emitter/root-cause evidence proves the extracted
  name can be zero for user calls; clean compiler-internal named calls are not
  evidence of a working user ABI. **Minimal proposal, not a verified ABI:** keep
  source names for unique top-level functions where LLVM permits, and let
  `ResolvedSymbol.llvmName` carry the exact chosen symbol. Every checked call
  emits that field; codegen must not read callee names, perform field-name
  heuristics, or fallback to `@0`. Charter §2 / SPEC §4.4 pin only `create`: a
  module-level `pub func create(...)` whose explicit or inferred return type is
  that module's sole struct resolves both `T.create(...)` and bare `create(...)`
  to the same declaration. No other name magic.
* Imported resolution must carry the exact declaration's defining
  `sourcePath`/`moduleId`, `isPub`, call kind, signature, and `llvmName` to
  codegen. The import resolver has fixed duplicate-import idempotence, but its
  flattened statement list loses privacy/ownership context unless those fields
  are retained on declarations/symbols. `pub` gates access across module
  boundaries; private declarations remain emitted and callable within their
  defining module. **OPEN — resolve in Phase B:** deterministic LLVM spelling
  for duplicate function names from different modules, methods, and linkage.
  Do not solve privacy by globally dropping private functions or invent a
  mangling silently.
* SPEC methods have no source-level `self` parameter and access fields in method
  scope. A conventional lowering is an explicit hidden receiver as the first
  LLVM argument after the resolved method symbol; source args follow it. This
  hidden receiver convention is a proposal, while the LLVM symbol spelling is
  still **OPEN — resolve in Phase B** with module/method mangling.
* Current ordinary function locals use `%name.addr` allocas; params are stored
  in entry allocas, and local allocas are hoisted into function entry. Current
  function `main` is renamed `_rey_main`; generated C `main(i32, i8**)` calls
  `rey_init` then `_rey_main` and returns 0. This wrapper is proven in
  `main.bin.ll` and codegen source.
* **Globals:** no LLVM global variables are emitted in inspected `main.bin.ll`;
  current `codegenProgram2` only visits top-level struct/enum/function
  declarations and ignores top-level variable/executable statements. SPEC §8
  now requires every module's executable top-level statements, including
  `var`/`const` initializers, to run once in dependency post-order, each module
  after the modules it imports, before the entry file's top-level statements.
  Then entry `main()` runs once if defined. An imported `main` is not called.
  A module reached by several imports runs once. **Decided:** private module
  variables are initialized by that run. There is no lazy guard on later reads.
  Minimum lowering: one LLVM global per resolved module binding, one init
  function per module, and a driver that calls those inits in the §8 post-order,
  then entry statements, then entry `main`. Duplicate-name linkage across
  modules remains the open mangling item above.
* Labels: `entry:` per function; generated control labels use existing prefixes
  plus the monotonic per-function counter. SSA temporaries `%tN` share that
  counter in current source. LLVM basic-block labels and temporary names must be
  unique in a function; branches must target emitted labels and every block must
  end in a terminator.

### 2.3 Values and aggregate lowering

* **Strings:** global literal storage is `@.strN = private unnamed_addr
  constant [N x i8] c"...\00", align 1`. Codegen computes a GEP to the first
  byte and calls `rey_str_new(i8*, i64 length excluding NUL)`; resulting Rey
  string is the data pointer (header at data - 16). Preserve source UTF-8 bytes
  and raw `\xNN` bytes exactly; embedded NUL is length-aware for Rey string APIs,
  though C-string-only runtime paths cannot preserve it. Case conversion is
  ASCII-only; no Unicode case mapping is part of v0.3.
* **Structs:** `%struct.Name = type { field types in declaration order }`.
  Allocate `sizeof(struct)+16` bytes with `rey_alloc`, write runtime tag at
  allocation byte 0, and use data pointer `raw+16` cast to `%struct.Name*`.
  `rey_struct_instanceof` receives that data pointer and checks tag at -16.
  Field/struct identity comes from checker indexes, never field-name scans.
  Existing codegen field-name heuristics are transitional and must be removed
  from typed lowering.
* **Vec:** `i64` runtime handle; elements are stored in `int64_t` slots. Existing
  C ABI has no element-type parameter. Box pointer values with `ptrtoint`; unbox
  with `inttoptr` using checked element type. **OPEN — resolve in Phase B:**
  precise float/tuple/char boxing; current codegen does not define a universal
  boxing rule.
* **For iteration:** SPEC `for name in Vec/array` iterates in order; its reserved
  `range(start,end)` form iterates ints start-inclusive/end-exclusive and is not
  a first-class range value. Checker resolves a `ForStmt` iteration mode and
  element type; for range, the `range` call intrinsic is legal only in the
  iterable position. Codegen must not dispatch by source callee text. Current
  codegen emits only literal `ArrayExpr` loops and returns empty code for other
  iterables.
* **HashMap / dict:** `i64` runtime handle; SPEC freezes string keys and erased
  value types. Current C implementation accepts Rey-string keys and `i64`
  values. Bare identifier literal keys become their source-name strings. Use
  checker type info to box/unbox value slots; the exact float/tuple/char boxing
  rule remains **OPEN — resolve in Phase B**. Runtime has no value-type metadata.
* **Tuple:** parser AST exists but current codegen falls through to literal 0;
  no runtime tuple API/layout exists. **OPEN — proposed lowering:** LLVM
  literal/named aggregate `{ T0, T1, ... }` by value when statically typed;
  heap-box only when forced through `i64` collection/opaque ABI. Checker must
  supply element types and tuple arity. Confirm aggregate storage/call behavior
  against the chosen bootstrap LLVM version before freezing.
* **Match:** current parser builds `MatchStmt`; current codegen has no match lowering.
  Lower statement arms to an ordered decision chain over the checked scrutinee;
  each successful arm branches to a shared end label. There are no guards in SPEC.
  Enum patterns inspect variant and bind payloads declared either fully
  positional (`Count(int)`) or fully named (`Count(value: int)`), never mixed;
  range
  patterns are inclusive integer-literal ranges (`>= start && <= end`); wildcard
  is catch-all. Checker enforces SPEC exhaustiveness for enums, bool, and
  nullable types; infinite domains require wildcard/binding. **OPEN — resolve in
  Phase B:** concrete payload-enum memory layout and how typed payload fields are
  extracted from it.
* **Lambdas:** current AST carries parameter names/types and an expression body;
  current codegen
  emits no implementation (charter defect ledger says lambdas become `0`).
  Required model is lifting, not mutable closure capture. Proposed scheme:
  assign deterministic per-module `__rey_lambda_N` lifted symbol; check the
  declared lambda parameter types and infer/check a
  concrete function signature; compute immutable free variables in source order;
  emit an environment struct with one field per capture; allocate/fill it at
  lambda creation (owner rule: scalar binding values snapshot at creation; captured
  bindings are read-only, while referenced aggregates remain shared); lifted
  function receives environment as hidden first arg,
  followed by user args; call sites load captures from that environment. The
  lambda expression's type is function type and its value is a closure (code +
  environment), not a bare function pointer when captures exist. **OPEN — resolve
  in Phase B before codegen:** closure value storage/layout and callback adapters.
  In particular current `rey_vec_map/filter(i64 vec, i64 fn_ptr)` invokes a
  one-argument C callback and has no environment parameter; it cannot directly
  invoke a capturing closure. Do not pass a closure object as `fn_ptr`.

### 2.4 Numeric ops and expression semantics

The current codegen handles int arithmetic with `add/sub/mul/sdiv/srem` and uses
integer predicates, with `i1` comparison results. SPEC requires `int / int`
truncating toward zero; mixed int/float arithmetic and remainder produce float.
Typed lowering chooses opcodes from checked operand/result types, not LLVM
value-name heuristics. For doubles use `fadd`, `fsub`, `fmul`, `fdiv`, `frem`,
`fcmp`; integer ops remain signed. Owner ruling: signed overflow is a runtime
error for arithmetic, unary/update operations, including `INT64_MIN / -1`.
Current codegen source calls `cgCheckedIntBin`/`cgCheckedDivRem` for arithmetic,
negation, updates, and compound assignments; div/rem checks zero and min/-1.
This is source-level evidence only; the recorded bootstrap gate never reached a
native runtime test. `string + string` uses `rey_str_concat`;
both `string + non-string` and `non-string + string` are **type errors** per
the owner ruling recorded in charter §10 and `docs/STRING-ABI-NOTES.md`.
Interpolation is different: evaluate each expression once, convert its statically
checked type with `toString()` semantics (`rey_int_to_str`, `rey_float_to_str`,
`rey_bool_to_str`, literal `"null"`, identity for strings), then concatenate in
source order. Never route mixed `+` through `rey_val_to_str`. **OPEN — resolve
in Phase B:** safe dynamic `any.toString()` / interpolation conversion and
stringification of aggregate types; the current one-argument registry bridge
cannot discriminate arbitrary values. Float text is the SPEC §9.1 spelling
(`rey_float_to_str`): Rust 1.98 `f64` Debug, with a trailing `.0` removed on a
plain integer spelling, and `inf` / `-inf` / `NaN` for non-finite values. A
`%g` or `{:.0}` full fixed-point spelling is not that contract.

### 2.5 Runtime archive handoff

Current exported signature remains `compileLLVM(ir, outputPath, release,
rtDir)`. In current `codegen/main.rey`, `rtDir` means the **directory containing**
`librey_rt.a`; codegen checks `rtDir/librey_rt.a` and links with `-L rtDir -lrey_rt`.
It does not build by running `make -C` on the runtime source directory.
`scripts/build-runtime.sh [--output-dir DIR]` builds the archive outside runtime
sources and prints the full `.../librey_rt.a` path. Driver/bootstrap integration
must capture that path and pass its parent directory as `rtDir`, preserving the
existing `compileLLVM` signature. Passing the archive filename or source
directory directly is incorrect. Current CLI runtime resolution checks for an
already-present archive and does not itself invoke the builder; wiring the
builder into bootstrap/driver is an integration dependency.

## 3. Runtime C ABI (current public header)

ABI types below are the exact C declarations in `runtime/rey_rt.h`; on the
verified aarch64 macOS target, `int64_t` maps to LLVM `i64`, pointers to `i8*`
where opaque, and `double` to LLVM `double`. Strings passed to Rey APIs are Rey
data pointers, not arbitrary C strings, unless the contract says C string.
Returned allocations are process-lifetime unless otherwise noted; there is no
general Rey object free API. `rey_alloc` zeroes memory and exits on OOM.

### 3.1 Memory and strings

| C symbol and exact signature | Caller / result contract (source: `runtime/mem.c`, `string.c`, header) |
|---|---|
| `void* rey_alloc(int64_t size)` | `size >= 0`; zero-filled malloc allocation; exits 1 on OOM; caller owns no free operation through public ABI. Internal vec/map growth frees replaced C buffers. |
| `void rey_panic_cstr(const char* msg)` | Raw NUL-terminated C string, non-null in callers; prints `rey panic: ...` to stderr and exits 1. Does not return. |
| `void rey_panic(const char* msg)` | Rey string data pointer or null; prints its length-aware contents (or `(null)`) and exits 1. Does not return. |
| `char* rey_str_new(const char* data, int64_t len)` | Copies exactly `len` bytes and appends NUL; `data` may be null (zero-filled bytes); require nonnegative length. Returns allocated Rey string data pointer; header is 16 bytes before it. |
| `char* rey_str_cstr(const char* cstr)` | Input is NUL-terminated C string; null becomes empty string. Copies into Rey string. |
| `char* rey_str_empty(void)` | Returns newly allocated empty Rey string. |
| `int64_t rey_str_len(const char* s)` | Rey data pointer or null; null returns 0; otherwise reads length header. |
| `char* rey_str_concat(const char* a, const char* b)` | Rey strings; either may be null (treated empty). Returns new allocated Rey string. |
| `char* rey_str_slice(const char* s, int64_t start, int64_t end)` | Rey string or null; null/empty or start>=end returns empty. Negative start clamps to 0; end above length clamps to length. Half-open byte range. Returns new string. |
| `char* rey_str_char_at(const char* s, int64_t i)` | Rey string or null; out-of-range/null returns empty; otherwise returns a newly allocated one-byte Rey string (not scalar char). |
| `int64_t rey_str_index_of(const char* s, const char* sub)` | Rey strings; null operand => -1; returns byte offset, first match; empty substring => 0. |
| `int64_t rey_str_starts_with(const char* s, const char* prefix)` | Rey strings; null operand => 0; returns 0/1. |
| `int64_t rey_str_ends_with(const char* s, const char* suffix)` | Rey strings; null operand => 0; returns 0/1. |
| `char* rey_str_trim(const char* s)` | Rey string or null; null => empty; trims C `isspace` bytes; returns new string. |
| `char* rey_str_replace(const char* s, const char* from, const char* to)` | Rey strings; null `s` => empty; null/empty `from` returns a copy of `s`; null `to` is treated as empty by `rey_str_concat`; returns new string. |
| `char* rey_str_repeat(const char* s, int64_t n)` | Rey string or null; null/nonpositive count => empty; returns new string. |
| `char* rey_str_to_upper(const char* s)` | Rey string or null; null => empty; ASCII lowercase bytes map to uppercase, other bytes unchanged; returns new string. |
| `char* rey_str_to_lower(const char* s)` | Rey string or null; null => empty; ASCII uppercase bytes map to lowercase, other bytes unchanged; returns new string. |
| `int64_t rey_str_eq(const char* a, const char* b)` | Rey strings; pointer-equal => 1; null mismatches => 0; compares stored lengths and bytes; returns 0/1. |
| `int64_t rey_str_split(const char* s, const char* delim)` | Returns `i64` Vec<String>, items are Rey data pointers cast to i64. Null source => empty Vec; null/empty delimiter splits into one-byte strings. |
| `char* rey_int_to_str(int64_t n)` | Returns allocated Rey decimal string. |
| `char* rey_val_to_str(int64_t val)` | Current one-argument compatibility bridge checks a process-global exact-pointer registry populated by runtime string constructors; registered string pointers return unchanged, other values format as decimal integers. It is not a typed value ABI: a full-width integer equal to a live registered string pointer is misclassified; externally fabricated ReyStr pointers are not recognized; Vec/Map/struct are not stringified. Do not use it as a mixed-`+` or arbitrary-pointer fallback. |
| `const char* rey_typed_val_to_str(int64_t kind, int64_t payload)` | Explicit scalar-kind ABI. Kinds are `REY_VALUE_INT=4097`, `BOOL=4098`, `FLOAT=4099`, `STRING=4100`, `CHAR=4101`, `NULL=4102`: INT payload is signed i64; BOOL exactly 0/1; FLOAT raw binary64 bits; STRING a non-null valid ReyStr data pointer (returned borrowed); CHAR byte 0..255 (returns a one-byte Rey string); NULL requires payload 0 (returns `"null"`). Valid non-string inputs return allocated Rey strings; unsupported kinds/invalid payloads panic and exit 1. Owner-reported C verification: normal and ASan/UBSan modes passed, aggregate 14/14, runtime fingerprint `9e263d2a22d8fe2f15f10434977ba5ef2afd73d3e14d75e5538303dcb3b067c8`; harness: `scripts/test-runtime-typed-value.sh [normal|sanitize|all]`. |
| `char* rey_float_to_str(double f)` | Returns an allocated Rey string with the SPEC §9.1 spelling: Rust 1.98 `f64` Debug, drop a trailing `.0` when the spelling is an optional `-`, digits, and `.0` with no exponent. Non-finite results are `inf`, `-inf`, and `NaN`. Locale-independent, decimal point `.`. |
| `char* rey_bool_to_str(int64_t b)` | Nonzero => `"true"`, zero => `"false"`; returns allocated Rey string. |
| `void rey_print(const char* s)` | Rey string data pointer or null; null is ignored; writes exactly stored bytes to stdout, no newline. |
| `void rey_println(const char* s)` | Same input as print; writes string then newline and flushes stdout. |
| `void rey_eprint(const char* s)` | Rey string data pointer or null; null is ignored; writes exactly stored bytes to stderr, no newline. |
| `void rey_eprintln(const char* s)` | Same input as eprint; writes string then newline and flushes stderr. |

### 3.2 Vec and HashMap

| C symbol and exact signature | Caller / result contract (source: `runtime/vec.c`, `hashmap.c`, header) |
|---|---|
| `int64_t rey_vec_new(void)` | Returns opaque i64 pointer to tagged Vec. |
| `void rey_vec_push(int64_t v, int64_t item)` | `v` must be a valid Vec handle; stores raw i64 item. Capacity overflow panics/exits. |
| `int64_t rey_vec_pop(int64_t v)` | Valid nonempty Vec; empty calls `rey_panic_cstr` and exits. |
| `int64_t rey_vec_len(int64_t v)` | Valid Vec handle; returns length. |
| `int64_t rey_vec_get(int64_t v, int64_t i)` | Valid Vec and in-range index; otherwise panics/exits; returns raw slot. |
| `void rey_vec_set(int64_t v, int64_t i, int64_t val)` | Valid Vec and in-range index; otherwise panics/exits; writes raw slot. |
| `int64_t rey_vec_contains(int64_t v, int64_t item)` | Valid Vec; raw integer equality; returns 0/1. Not string equality. |
| `int64_t rey_vec_str_contains(int64_t v, const char* item)` | Vec slots must contain Rey string pointers cast to i64; `item` Rey string; compares with `rey_str_eq`; returns 0/1. |
| `char* rey_vec_join(int64_t v, const char* sep)` | Vec slots must be Rey string pointers cast to i64; separator is Rey string; returns allocated Rey string. |
| `int64_t rey_vec_slice_v(int64_t v, int64_t start, int64_t end)` | Valid Vec; returns a new Vec with slots in half-open range, clamps negative start and end above length. |
| `int64_t rey_vec_reverse(int64_t v)` | Valid Vec; returns a new Vec in reverse slot order. |
| `int64_t rey_vec_map(int64_t v, int64_t fn_ptr)` | `fn_ptr` is a raw C function pointer `int64_t (*)(int64_t)` cast to i64; calls once per item; returns new Vec. No environment/capture argument. |
| `int64_t rey_vec_filter(int64_t v, int64_t fn_ptr)` | Same callback ABI as map; keeps items where callback result is nonzero; returns new Vec. |
| `int64_t rey_hm_new(void)` | Returns opaque i64 pointer to tagged string-keyed map. |
| `void rey_hm_set(int64_t hm, const char* key, int64_t val)` | `hm` valid; key is non-null Rey string data pointer; value raw i64. Current hash walks NUL-terminated bytes while equality is length-aware, so embedded-NUL keys are unsafe/inconsistent. |
| `int64_t rey_hm_get(int64_t hm, const char* key)` | Valid map and non-null Rey string key; returns raw value or 0 when absent (0 is ambiguous with a stored zero). |
| `int64_t rey_hm_has(int64_t hm, const char* key)` | Same key contract; returns 0/1. |
| `void rey_hm_delete(int64_t hm, const char* key)` | Same key contract. Current working implementation marks a tombstone (occupied state 2); lookup probes through tombstones and insertion may reuse them. Header ABI is unchanged. |
| `int64_t rey_hm_keys(int64_t hm)` | Returns Vec<String> handle; slots are key pointers cast to i64. |
| `int64_t rey_hm_values(int64_t hm)` | Returns Vec handle; slots are raw values. |

Vec and maps are heap objects with tag byte at object offset 0. `Vec` slots and
HashMap values are untyped `int64_t`; C runtime does not retain Rey type metadata.

### 3.3 Result, Option, tags, I/O and process

| C symbol and exact signature | Caller / result contract (source: `runtime/result.c`, `io.c`, header) |
|---|---|
| `int64_t rey_ok(int64_t val)` | Allocates tagged Result Ok; stores raw i64 payload. |
| `int64_t rey_err(int64_t val)` | Allocates tagged Result Err; stores raw i64 payload. |
| `int64_t rey_ok_str(const char* val)` | Wraps Rey string pointer as i64 payload in Ok; pointer must remain valid. |
| `int64_t rey_err_str(const char* val)` | Same for Err. |
| `int64_t rey_result_is_ok(int64_t r)` | Null => false; valid Result handle => tag test, 0/1. |
| `int64_t rey_result_is_err(int64_t r)` | Null => true; valid Result handle => Err tag test, 0/1. |
| `int64_t rey_result_unwrap(int64_t r)` | Requires non-null Ok; otherwise panics/exits; returns raw payload. |
| `int64_t rey_result_unwrap_err(int64_t r)` | Public C helper for the normative SPEC `Result.unwrapErr()` method (§9.3): requires non-null Err, otherwise panics/exits; returns the stored i64 payload unchanged, including pointer bits. Codegen/checker method exposure exists, but typed payload unboxing at LLVM use sites still needs verification. |
| `int64_t rey_result_unwrap_or(int64_t r, int64_t def)` | Null/non-Ok => `def`; Ok => raw payload. |
| `const char* rey_result_unwrap_str(int64_t r)` | Requires non-null Ok whose payload is a Rey string pointer; otherwise panics/exits. Returns borrowed string data pointer. |
| `const char* rey_result_unwrap_or_str(int64_t r, const char* def)` | Null/non-Ok => `def`; Ok payload cast to string pointer. Both are Rey string pointers. |
| `int64_t rey_some(int64_t val)` | Allocates tagged Option Some with raw i64 payload. |
| `int64_t rey_none(void)` | Allocates tagged Option None. |
| `int64_t rey_option_is_some(int64_t o)` | Null/None => 0; valid Some => 1. |
| `int64_t rey_option_unwrap(int64_t o)` | Requires non-null Some; otherwise panics/exits; returns raw payload. |
| `int64_t rey_instanceof_tag(int64_t ptr, int64_t tag)` | Null => 0; expects raw runtime object pointer (not string data pointer); reads byte 0 and returns 0/1. |
| `int64_t rey_struct_instanceof(int64_t ptr, int64_t tag)` | Null => 0; expects user-struct data pointer; reads tag at `ptr-16`; returns 0/1. |
| `char* rey_read_file(const char* path)` | `path` is Rey string; returns file contents as Rey string; open failure returns empty string (not Result). |
| `void rey_write_file(const char* path, const char* content)` | Rey strings; open failure panics/exits; writes stored content length. |
| `void rey_append_file(const char* path, const char* content)` | Rey strings; open failure panics/exits; appends stored content length. |
| `int64_t rey_file_exists(const char* path)` | Rey path string; returns 0/1 from `stat`. |
| `void rey_delete_file(const char* path)` | Rey path string; ignores `remove` status. |
| `void rey_mkdir(const char* path)` | Rey path string; mode 0755; ignores status. |
| `int64_t rey_list_dir(const char* path)` | Rey path; returns Vec<String>; failed `opendir` => empty Vec; skips entries beginning `.`; elements are basenames. |
| `void rey_init(int argc, char** argv)` | Stores borrowed process argc/argv; call once from native C `main` before `rey_args`. |
| `int64_t rey_args(void)` | Returns new Vec<String>, including `argv[0]`; each item is a newly copied Rey string pointer cast to i64. |
| `int64_t rey_exec_cmd(const char* cmd)` | Null => Err("exec failed: null command"); otherwise `cmd` is a Rey string used as NUL-terminated `/bin/sh -c` command. Returns tagged Result<String,String> as i64. Captures stdout/stderr concurrently. Exit 0 => Ok(stdout) unchanged and forwards child stderr to parent stderr; failure => Err containing status or signal plus labeled stdout/stderr. Pipe/fork/read/wait failures return descriptive Err. |
| `void rey_exit(int64_t code)` | Exits process using narrowed C `int` code; does not return. |
| `char* rey_get_env(const char* name)` | `name` is Rey string; present env copied to Rey string; missing => empty Rey string. |

### 3.4 Runtime tags and non-ABI files

Header tag constants: `REY_TAG_STR=1`, `VEC=2`, `MAP=3`, `OK=4`, `ERR=5`,
`SOME=6`, `NONE=7`, `STRUCT=8`. `REY_STR_HDR=16`. String data pointer is
header+16; Vec/Map/Result/Option handles are raw object pointers cast to i64;
struct instance pointer is data after 16-byte header. Current codegen assigns
per-struct IDs starting at 8, the same value as `REY_TAG_STRUCT` for the first
type. **OPEN — resolve in Phase B:** whether the generic struct tag denotes all
user structs or is reserved, and how runtime tag IDs stay disjoint as types are
added.
Scalar conversion kinds are a separate namespace, not object tags:
`REY_VALUE_INT=0x1001`, `BOOL=0x1002`, `FLOAT=0x1003`, `STRING=0x1004`,
`CHAR=0x1005`, `NULL=0x1006`.

`runtime/test_rt.c` exports only its test executable `int main(void)`; its test
helpers are static and not part of the library ABI. For completeness, private C
helper signatures inspected are `string.c`: `static size_t
stringPointerHash(uintptr_t)`, `static void stringPointerInsert(uintptr_t*,
size_t, uintptr_t)`, `static void growStringPointers(void)`, `static void
registerStringPointer(const char*)`, `static int isRegisteredStringPointer(uintptr_t)`,
`static ReyStr* data2hdr(const char*)`; `vec.c`: `static ReyVec* vp(int64_t)`,
`static void vec_grow(ReyVec*)`; `hashmap.c`: `static ReyMap* mp(int64_t)`,
`static uint64_t hm_hash(const char*)`, `static void hm_insert_raw(ReyMap*,
const char*, int64_t)`, `static void hm_grow(ReyMap*)`, `static HmEntry*
hm_find(ReyMap*, const char*)`; `result.c`: `static int64_t
tagged_new(uint8_t, int64_t)`, `static ReyTagged* tp(int64_t)`, `static
ReyTagged* op(int64_t)`; `io.c`: `static int rey_io_append(ReyIoBuffer*,
const unsigned char*, size_t)`, `static void rey_io_close(int*)`, `static int64_t
rey_exec_error(const char*, int)`, `static int rey_io_append_text(ReyIoBuffer*,
const char*)`, `static int64_t rey_exec_status_error(int, const ReyIoBuffer*,
const ReyIoBuffer*)`, `static void rey_exec_forward_stderr(const ReyIoBuffer*)`,
`static void rey_exec_child_error(int, const char*, size_t)`; `test_rt.c`:
`static void test_strings(void)`,
`static void test_vec(void)`, `static void test_hashmap(void)`, `static void
test_result(void)`, `static void test_io(void)`, and `int main(void)`. These are implementation/test details, not callable
compiler ABI. `rey_rt.h` is the public signature authority; implementations
inspected: `mem.c`, `string.c`, `vec.c`, `hashmap.c`, `result.c`, `io.c`.

## 4. Diagnostics and check protocol

SPEC §10 / charter §5.2 freeze the rendered line exactly; line and column are
one-based:

```text
<file>:<line>:<col>: error: <message>
<file>:<line>:<col>: warning: <message>
<file>:<line>:<col>: note: <message>
```

One diagnostic per line; no ANSI. Severity words are lowercase. `check` is
silent on success (exit 0); any diagnostic/error exits 1. Usage/unsupported is
exit 2 per charter §3. A valid check result must not emit the current
`Success: a.out` text.

The current diagnostics module defines `Severity { Error, Warning, Note }`,
`Span { start, end, line, column }`, `Diagnostic { severity, code, message,
filePath, span }`, `DiagnosticBag { items }`. Thus the requested logical fields
map without losing current API: `file = filePath`, `line = span.line`,
`col = span.column`, `severity`, `message`; `code` may remain internal. Current
`diagnostics/main.rey` implements `spanFromOffsets`, `formatDiagnostic`, and
`formatDiagnostics`; lexer/parser offsets are converted to one-based coordinates.
`unknownSpan()` uses line/column 0 and formatter prints `?`; current driver uses
that for type errors and locationless failures, which does not satisfy the
frozen numeric `<line>:<col>` shape. Current `TypeError.span` is a raw int offset;
`formatTypeDiagnostics` ignores it, uses the entry path, and substitutes
`unknownSpan()`, including for imported-source errors. **Proposed protocol:**
preserve `typecheck(program)` and `TypecheckResult.ok/errors`, but provide an
originating source path and source span/node identity for every error; the driver
formats each once. Only `ok == true` permits codegen; failed results never
generate IR. `check` is silent on success.

## 5. Cross-stage invariants

1. Parse every supported v0.3 construct into an explicit AST node; do not encode
   struct literals, interpolation, ranges, or match as undocumented name/operator
   conventions.
2. Checker consumes parsed AST and produces a typed AST on success. Codegen
   consumes **only that typed AST**, never parser AST or untyped fallback values.
3. Any type/name/arity/visibility/exhaustiveness error stops codegen. Successful
   check has no errors and `check` prints nothing.
4. Every call site carries a resolved symbol; codegen emits that symbol exactly.
   No `@0`, identifier-name fallback, field-name guessing, or module-name
   heuristic is allowed outside the charter's sole `create` rule.
5. Checker resolves all identifiers, fields, enum variants, builtins, module
   exports, and methods, and records field/variant/declaration indexes. Codegen
   must not search declarations by matching a field name.
6. Lowercase primitive spellings are canonical. Source-level bool is 0/1 `i64`
   at storage/ABI boundaries; `i1` is internal predicate form only.
7. Preserve runtime ABI types: Rey strings are `i8*` data pointers; Vec/Map/
   Result/Option are i64 handles; C boolean-like returns are i64 0/1. Cast only
   at explicitly typed boundaries.
8. Vec/HashMap payloads are untyped i64 slots. Correct boxing/unboxing uses
   checker-provided element/key/value types; pointer guessing is forbidden.
9. Preserve defining source/module identity and `pub` on flattened declarations.
   Private functions remain callable within their defining module; imported
   external access to a private declaration is rejected.
10. Expression annotations use node identity `(sourcePath,nodeId)`, never span
    alone. Spans remain source locations for diagnostics.
11. `string + string` concatenates; either mixed string/non-string `+` is a
    checker error. Interpolation conversions are a separate typed operation;
    never use the untagged string-pointer registry as generic coercion.
12. Struct field order and enum variant ordinal follow declaration order. Match
   exhaustiveness and branch narrowing are checker responsibilities.
13. Lambda captures are immutable and lifted; capture environment must be
     passed explicitly. Do not silently pass an environment-bearing closure to
     current one-argument `rey_vec_map/filter` callbacks.
14. All generated basic blocks have unique labels, correct terminators, and
     valid branch targets. A compiler error is preferable to silently emitting
     literal 0 for an unsupported typed node.
15. All generated runtime declarations match `rey_rt.h` exactly; do not silently
     change C signatures or infer null/ownership guarantees not stated here.

## 6. Integration traps / contradictions found

These observations were made while Phase B sources were changing; qualify them
by the before/after source hashes reported below.

* Current `parser/ast/main.rey` and `parser/main.rey` have already added TypeExpr,
  visibility/type fields, payload enums, enum construction/patterns, update,
  match statements, ranges, imports, and tuple syntax. They still lack expression
  identity/source path, checker annotations, interpolation nodes, and
  AssignExpr. `parse(tokens,path)` currently ignores `path`; `Program` has no
  source identity. Lexer token duplication has been repaired (`Question`,
  `Loop`, `Arrow`, `FatArrow` are now declared in both token files).
* Parser gaps at this live snapshot: `while`/`for` require parentheses despite
  SPEC optional if/while parens and unparenthesized for; lambda parser accepts
  expression bodies only, not block bodies; `parseDict` leaves bare identifier
  keys as `IdentExpr`; string interpolation has no AST/lexer path; assignments
  parse as statements rather than the SPEC expression form. `ReturnStmt` still
  uses a Null literal sentinel. Body shape mismatch remains: function/while/for/
  loop structs declare `[Stmt]` while parser supplies `BlockStmt`; `IfStmt` has
  `elseBranch: BlockStmt?` while `else if` returns `IfStmt`.
* Payload mode: native parser stores parallel `payloadNames`/`payloadTypes` and
  currently permits a name independently per slot. The owner ruling requires
  all-named or all-positional payloads per variant, never mixed; checker/parser
  must reject a mixed variant. Runtime/codegen payload enum representation is
  still **OPEN**.
* The native checker produces a populated `CheckedProgram` for a single source
  unit with no imports. Cross-module `typecheckGraph` currently returns an error
  and empty expression/symbol tables. The driver passes the checked wrapper to
  `generateIRChecked`; source code has an import-free consumer and rejects
  imported/multi-unit checked programs. Neither that path nor module behavior
  has been verified by a native compiler build/run.
* Latest fresh Rust-oracle run in `docs/V03-VERIFICATION.md` built successfully
  on a 442-file unchanged input manifest
  (`e220a166e45c85c166503dc3914ba503a6399e29f1707ce36d88c1f9f0e80b7a`): unit
  tests 104/113 passed and conformance 102/125 passed. Binary:
  `/tmp/rey-v03/runtime-consumer-20261016T191330Z/target/release/rey-v0`
  SHA-256 `cf9e85894d863e76f8bc38ac6b26a6eca76c01d318021460419556b52fdd6f33`.
  `runInjectsScriptAndUserArgsWithoutCliWrapperTokens` and
  `checkRejectsPrivateImportsAndLocatesImportedErrors` passed. The resolved
  runtime CLI entry is wired and exercised; this is not merely source-level
  normalized-argv wiring. The older eight-error Rust build in
  `docs/BOOTSTRAP.md` is superseded by this direct-source oracle run. The full
  native bootstrap/stage-2 gate has not been rerun or verified; do not equate
  the Rust oracle pass with native conformance or self-host success.
* Diagnostics are now implemented in `src/diagnostics/main.rey`; lexer/parser
  errors are converted from offsets to one-based spans and `check` is silent on
  success in the current driver. `TypeError` retains sourcePath/moduleId plus a
  raw start offset; `formatTypeDiagnostics(graph.units, errors)` locates the
  matching source and reconstructs coordinates. Negative/unknown offsets still
  use `unknownSpan()` (`?:?`), which conflicts with the numeric diagnostic form.
* `src/codegen/main.rey` has both the compatibility `generateIR(Program)` path
  and checked `generateIRChecked(CheckedProgram): Result<string,string>` path.
  The checked wrapper supports one import-free source unit; its internal
  `cgFindExprInfo` currently matches expressions by equality plus source/module,
  not nodeId. The unchecked compatibility path still has name-keyed prepasses.
  This is source-level integration evidence, not native behavior verification.
  `docs/NATIVE-ROOTCAUSE.md` remains the stored-emitter evidence: nested direct
  call-target extraction became zero in the old self-compiled emitter and the
  user repro emitted `@0`; clean named compiler-internal calls in `main.bin.ll`
  do not prove user calls correct. Keep symbol/ownership resolution explicit.
* The checker reports mixed `string + non-string` `+` as a type error, and
  codegen also rejects it defensively. The legacy one-argument
  `rey_val_to_str` bridge still has a live-pointer versus full-width-integer
  collision; current codegen declares it but uses `rey_typed_val_to_str` through
  `cgRenderToString` for typed scalar print/string rendering. The C helper is
  verified in normal and ASan/UBSan modes, aggregate 14/14 on runtime fingerprint
  `9e263d2a22d8fe2f15f10434977ba5ef2afd73d3e14d75e5538303dcb3b067c8`, using
  `scripts/test-runtime-typed-value.sh`. Dynamic Any/aggregate `toString()`
  remains OPEN.
* Codegen currently has no float opcodes, interpolation, tuple/dict, match, or
  closure lowering; enum payloads are not lowered; Vec iteration handles only
  literal arrays; methods are not emitted. In the checked path field reads/
  lvalue updates consult `CheckedExprInfo` indexes; the unchecked compatibility
  path still relies on source type/name maps. Current codegen source
  now calls `cgCheckedIntBin`/`cgCheckedDivRem` for integer arithmetic, unary
  negation, updates, and compound assignments; the div/rem helper checks zero
  and MIN/-1. This is source-level implementation evidence, not a native runtime
  verification because the recorded bootstrap gate stops before native execution.
* `main.bin.ll` is a stale successful self-build artifact: compiler-internal
  named calls do not validate user calls. `REY_TAG_STRUCT` is 8 and current
  struct tag allocation starts at 8; generic-vs-type-specific tag use remains
   OPEN. HashMap deletion now uses tombstones, and string conversion now uses an
   exact-pointer compatibility registry; `rey_rt.h` now marks
   `rey_val_to_str` as legacy, but it still has a pointer/integer collision.
   The `rey_vec_contains` header comment says string equality while implementation
   compares raw i64 (use `rey_vec_str_contains` for strings).
  HashMap's NUL-terminated hash versus length-aware key equality remains a risk.
* Current `rey_exec_cmd` captures stdout/stderr concurrently. Success returns
  stdout unchanged and forwards child stderr; failure `Err` includes exit
  status or signal plus labeled `stdout:` and `stderr:` streams. Do not retain
  the former stdout-only contract.
* `rey_result_unwrap_err(int64_t)` supports the normative public
  `Result.unwrapErr()` in SPEC §9.3; the checker and codegen have corresponding
  method paths. The C helper preserves payload bits. Current codegen emits an
  i64 result and rejects unresolved/`any` error types; conversion back to a
  statically known string/aggregate LLVM representation still needs tests. This
  is an implementation verification gap, not an OPEN language decision.
* Runtime semantic gaps vs SPEC/owner rules: empty `replace(from)` currently
  copies instead of inserting at byte boundaries; negative slice start currently
  clamps to zero though SPEC requires nonnegative indices (endpoints past length
  are clamped by both); negative repeat returns empty; missing HashMap get returns
  zero and there is no `rey_hm_len`; `getEnv` returns empty string, not Option;
  file I/O often returns empty/ignores OS errors; `toInt`/`toFloat` have no C
  runtime symbols. The checker recognizes string `.toInt()`/`.toFloat()`, but
  codegen has no parsing/error lowering for them and the checker does not cover
  all SPEC numeric receiver conversions. `rey_typed_val_to_str` is a separate
  formatting ABI, not a parser. Invalid/out-of-range conversions must be runtime
  errors; language-level numeric conversions are not verified. Current `rey_float_to_str`
  still has an integral `{:.0}` fixed-point path (including a 309-digit `DBL_MAX`
  spelling) and a locale-sensitive non-finite path. Those are not the SPEC §9.1 spelling.
  `lexer/main.rey` decodes `\xNN` through `char(value).toString()`; verify this
  preserves one raw byte for values >= 0x80 rather than UTF-8 re-encoding. Runtime
  string storage itself copies byte sequences, and current case conversion is
  ASCII-only.
* Historical C baseline: six categories, 12/12 checks, source fingerprint prefix
  `e9d6…`; this is not the latest runtime result. The typed scalar formatter is
  verified by the normal and ASan/UBSan 14/14 run on runtime fingerprint
  `9e263d2a22d8fe2f15f10434977ba5ef2afd73d3e14d75e5538303dcb3b067c8`. Do not
  conflate that C API test with source-language numeric `.toInt()`/`.toFloat()`
  conversions, which remain an implementation gap. In the latest conformance
  run, the three `result-unwrap-err-*` language fixtures fail before exercising
  `unwrapErr`: the Rust-oracle checker rejects qualified `Result::Ok`/`Result::Err`
  constructors. Rust/runtime implementation paths exist; this constructor
  recognition failure is separate from the normative public method contract.
* Archive linking contract: `scripts/build-runtime.sh --output-dir DIR` prints
  the full archive path. Current `compileLLVM(..., rtDir)` expects the containing
  directory, checks `rtDir/librey_rt.a`, and no longer runs `make -C` on runtime
  sources. Driver/bootstrap must capture the script output and pass its parent
  directory; current driver only resolves an already-present archive.
* The 2026-10-17 owner decisions for payload forms, capture snapshots, signed
  overflow, byte-preserving strings, and explicit conversions are present in the
  current SPEC and charter. Remaining OPEN items above concern ABI/layout or
  implementation, not those owner decisions.

## 7. Evidence / source files cross-checked

* Language authority: `SPEC.md`; charter `docs/V0.3-CHARTER.md` §§2, 3, 5.2,
  6–8 and §10 string `+` ruling.
* Parser AST and parser behavior: `rey-compiler/src/parser/ast/main.rey`,
  `rey-compiler/src/parser/main.rey`, `rey-compiler/src/lexer/main.rey`,
  `rey-compiler/src/lexer/token.rey`.
* Checker/diagnostics/pipeline: current `rey-compiler/src/typecheck/main.rey`,
  `rey-compiler/src/diagnostics/main.rey`, and `rey-compiler/main.rey`. Source
  interface inspection is not native compiler behavior verification.
* LLVM lowering and runtime declarations: `rey-compiler/src/codegen/main.rey`;
  inspected `rey-compiler/main.bin.ll` for named definitions/compiler-internal
  calls, i1 predicates and zexts, string globals, struct layouts, `_rey_main`
  wrapper, and runtime declarations; `docs/NATIVE-ROOTCAUSE.md` for zeroed
  call-target extraction and user-program failure. `main.bin.ll` is evidence of
  a successful prior self-build, not proof emitted user calls work or all v0.3
  features are implemented.
* String value bridge and owner conversion ruling: `docs/STRING-ABI-NOTES.md`.
* Runtime public signatures: `rey-compiler/runtime/rey_rt.h`; typed scalar
  formatter evidence: `rey-compiler/runtime/tests/typed_value_test.c`,
  `scripts/test-runtime-typed-value.sh`, and `docs/STRING-ABI-NOTES.md`.
* Runtime implementation/caller semantics: `rey-compiler/runtime/mem.c`,
  `string.c`, `vec.c`, `hashmap.c`, `result.c`, `io.c`, and public signatures
  in `rey_rt.h`. `runtime/test_rt.c` and `runtime/tests/typed_value_test.c` are
  test-only; typed-value verification is owner-reported on the fingerprint
  above, not a test run in this docs-only task.
* Runtime archive builder: `scripts/build-runtime.sh`; compiler link behavior:
  `rey-compiler/src/codegen/main.rey` and `rey-compiler/main.rey`.
* Oracle/native gate evidence: `docs/V03-VERIFICATION.md` records the latest
  fresh Rust-oracle build, partial unit/conformance results, and passing argv /
  imported-diagnostic regressions. `docs/BOOTSTRAP.md` records an older failed
  bootstrap, superseded for Rust compilation but not followed by a successful
  native bootstrap. The Rust run does not verify native checked-graph/codegen
  execution.
* Oracle AST payload schema: `compiler/v1/src/ast/expr.rs`, `stmt.rs`, `ty.rs`,
  `literal.rs`, `mod.rs`. It defines payload-aware enum declarations, variant
  expressions, and patterns; it does not define a native typed AST or LLVM ABI.

The five 2026-10-17 owner decisions are present in the current SPEC/charter
snapshot. Remaining OPEN items are representation/ABI choices or implementation
gaps, not unresolved owner decisions; none should be treated as a frozen ABI by
inference.
