# Rey language syntax and API reference

> **Repository snapshot:** 2026-10-22; current **rey-compiler** frontend and **std/src** in this checkout.
> This page describes what the checked-in compiler parses and typechecks, and separately inventories
> the checked-in standard-library exports. It does not imply that every checked declaration works in
> every code-generation backend.
>
> The older [SPEC.md](SPEC.md) is the v0.3.0 language contract and explicitly records that implementation may
> differ. The compiler behavior below was checked against the [lexer](rey-compiler/src/lexer/main.rey),
> [parser](rey-compiler/src/parser/main.rey), and [typechecker](rey-compiler/src/typecheck/main.rey). The v0.4 managed-IR pipeline is experimental and narrower than the established
> frontend/backend path.

## Contents

1. [A small program](#a-small-program)
2. [Lexical rules](#lexical-rules)
3. [Types and values](#types-and-values)
4. [Declarations and visibility](#declarations-and-visibility)
5. [Statements and control flow](#statements-and-control-flow)
6. [Expressions and operators](#expressions-and-operators)
7. [Patterns and matching](#patterns-and-matching)
8. [Modules and imports](#modules-and-imports)
9. [Global built-ins](#global-built-ins)
10. [Built-in methods and constructors](#built-in-methods-and-constructors)
11. [Standard-library exports](#standard-library-exports)
12. [Compiler-only names and implementation gaps](#compiler-only-names-and-implementation-gaps)
13. [Memory-management surface](#memory-management-surface)
14. [Compact grammar](#compact-grammar)

## A small program

~~~rey
import std::string;

func greeting(name: string): string {
    return "Hello, {name}!";
}

func main(): void {
    println(greeting("Rey"));
}
~~~

Top-level statements are allowed. **main** is optional; if an entry module declares it, the checker
requires no parameters and a **void** return type. **pub** makes supported declarations importable
from another module.

## Lexical rules

### Identifiers and keywords

Identifiers are ASCII: a letter or underscore followed by letters, digits, or underscores. Names
are case-sensitive. The current keyword set is:

~~~text
var const func struct enum import export pub return if else while loop for in
break continue match instanceof true false null self
~~~

**export** is recognized only to produce the migration error “export is removed; use pub”. Use **pub**.
Primitive type names are lowercase identifiers: **int**, **float**, **bool**, **string**, **char**,
**void**, and **any**. **String**, **Int**, and other capitalized spellings are not aliases.

### Whitespace and comments

Spaces, tabs, CR, and LF separate tokens. Line comments start with **//**. Block comments use
**/* ... */**, end at the first ***/**, and do not nest. An unterminated block comment is an error.

### Number literals

- Decimal integers: **0**, **42**, **1_000**.
- Decimal floats: **1.**, **1.25**, **1_000.25**.
- Hexadecimal integers: **0x2a** or **0X2A**.
- Binary integers: **0b101010** or **0B101010**.
- **_** may separate valid digits, but may not lead, trail, or repeat.
- A leading minus is the unary **-** operator, not part of the literal.
- Scientific notation such as **1e5** and **1.0e-5** is not accepted.

Integers are checked as signed 64-bit values. The source language has no unsigned integer literal
suffixes.

### Strings, interpolation, and characters

Ordinary strings use double quotes. Supported escapes are newline (**&bsol;n**), tab (**&bsol;t**),
CR (**&bsol;r**), backslash (**&bsol;&bsol;**), double quote (**&bsol;"**), apostrophe (**&bsol;'**),
NUL (**&bsol;0**), and **&bsol;xNN** (two hexadecimal digits). Unknown or incomplete escapes are errors.

**{expression}** interpolates one expression. Braces inside the expression are balanced by the lexer.
In string text, **{{** and **}}** produce literal braces. A lone unmatched **}** is an error.

Triple-quoted strings use three double quotes. Backslash escapes are not decoded in their body, but
interpolation still works. One LF or CRLF immediately after the opening delimiter is omitted.

A character literal uses single quotes and contains exactly one decoded byte, for example **'A'**,
**'&bsol;n'**, or **'&bsol;x41'**. Empty, multi-byte, multi-character, and raw-line-break literals are
invalid. Strings preserve source UTF-8 bytes; indexing a string is byte-oriented, not Unicode
grapheme-aware.

### Punctuation and operators

The lexer recognizes:

~~~text
( ) { } [ ] , . .. ... ; : :: ? | -> =>
= == != < <= > >= + ++ += - -- -= * *= / /= % %=
! && ||
~~~

A lone **&** is invalid. **|** separates types in a union; there are no bitwise operators.

## Types and values

### Primitive and nominal types

- **int**, **float**, **bool**, **string**, **char**, **void**, **any**
- User-declared **struct** and **enum** types
- **void** is only valid as a return type.
- **any** weakens static checking; it is not a generic type parameter.

### Built-in applied types

- **Vec<T>** and **[T]** describe the same vector type.
- **HashMap<string, V>** is a string-keyed map. Map key types other than **string** are rejected.
- **Option<T>** has **Some** and **None** variants.
- **Result<T, E>** has **Ok** and **Err** variants.
- Bare **Vec**, **HashMap**, **Option**, and **Result** use **any** for omitted element/value types; a
  bare map still has string keys.

The parser can form applied type syntax such as **Name<T>**, but the checker only supports its built-in
applied types. User-defined generic declarations, generic functions, constraints, inference, and
monomorphization are not part of this frontend.

### Tuples, functions, nullable types, and unions

Tuple types use **(T, U)** or **(T,)**; the one-element trailing comma is required. Tuple fields are
accessed by numeric members such as **pair.0**.

Function types use **(T1, T2) -> R**, including **() -> R**.

**T?** is nullable shorthand for **T | null**. Union types use **T | U | ...**. **null** is not a
standalone declared type. The checker narrows locals in supported **instanceof** and null comparisons.

### Type inference and conversions

- Annotated variables, fields, parameters, and returns are checked against their declared types.
- An unannotated local takes the initializer's type; reassignment does not change that type.
- Unannotated function parameters default to **any**.
- An omitted function return type is inferred from returned expressions; a function with no value return
  is **void**. Mixing value-returning and void paths is rejected.
- Numeric conversions are explicit; the built-ins **int**, **float**, **string**, and **char** perform
  conversions subject to the checker/backend limits described below.

## Declarations and visibility

### Variables and constants

Both forms require an initializer:

~~~rey
var count: int = 0;
const title = "Rey";
~~~

**var** may be reassigned. **const** may not. A constant binding does not deep-freeze a referenced
vector, map, or struct.

### Functions, parameters, and lambdas

~~~rey
pub func greet(name: string, punctuation: string = "!"): string {
    return "Hello, {name}{punctuation}";
}

func firstOr(values: Vec<string>, fallback: string): string {
    if values.len() == 0 { return fallback; }
    return values[0];
}

const add = (left: int, right: int) => left + right;
~~~

Function declarations have the form **[pub] func name(parameters) [: returnType] { ...}**. Parameter
types are optional for named functions; omitted types are **any**. A default may be written as
**name = expression** or **name: Type = expression**. Defaults must be trailing and compatible with
the parameter type.

A variadic parameter is written **name: ...Type**, and must be the last parameter. Inside the function
it has type **Vec<Type>**. At most one variadic parameter is allowed. This is not written
**...name: Type**.

Lambdas use an expression body and require a type on each parameter. Block-bodied lambdas are not
parsed. A lambda may capture immutable **const** bindings; capturing a mutable **var** is rejected.

Top-level functions, structs, and enums may be public with **pub**. Nested function/type/import
declarations are rejected by the current typechecker, even where the parser can build an AST node.

### Structs and methods

~~~rey
pub struct Person {
    pub name: string,
    age: int,

    pub func birthday(): void {
        age = age + 1;
    }
}
~~~

Fields are **name: Type** with an optional **pub**; methods use **func** and may also be **pub**. A
method does not declare an explicit **self** parameter; its body has access to the instance and its
fields. Private fields and methods are restricted to the owning struct's methods. A struct literal
names its fields:

~~~rey
var person = Person { name: "Ada", age: 36 };
~~~

Each field must be supplied once. The special **Type.create(...)** form is accepted only when the
module provides the supported public **create** factory for its sole struct.

### Enums

~~~rey
pub enum Message {
    Empty,
    Count(int),
    Named(label: string, value: int),
}
~~~

Variants may be unit variants, positional-payload variants, or named-payload variants. A single enum
cannot mix positional and named payload styles. Constructors use **Enum::Variant** or
**Enum::Variant(arguments)**. A bare **Variant(...)** constructor is accepted when its name is
unambiguous in the module.

**Option::Some(value)**, **Option::None**, **Result::Ok(value)**, and **Result::Err(error)** are
built-in constructors. Dot forms such as **Option.Some(value)** are also rewritten to these
constructors.

## Statements and control flow

Simple statements end in semicolons. Blocks and control-flow declarations do not need a trailing
semicolon.

~~~rey
if ready {
    println("go");
} else if waiting {
    println("wait");
} else {
    println("stop");
}

while index < limit {
    index = index + 1;
}

loop {
    if done { break; }
}

for item in items {
    println(item);
}
~~~

Supported statements include variable and constant declarations, expression statements, blocks,
**if**/**else**, **while**, **loop**, **for**, **match**, **return**, **break**, and **continue**.
**for** also accepts **for (item in items) { ... }**. Conditions must be **bool**. **break** and
**continue** take no labels. The **for** binding is immutable.

**return;** and **return expression;** both require a semicolon. **match** is a statement in this
parser; there is no supported match expression.

## Expressions and operators

### Expression forms

Expressions include literals, names, calls, field access, tuple numeric access, indexing, array/vector
literals, dictionary literals, tuple literals, struct literals, lambdas, interpolation, enum/Option/
Result constructors, and assignment.

- **[a, b]** constructs a vector.
- **{ "key": value, bareName: value }** constructs a dictionary. Keys are string literals or bare
  identifier key names.
- **(a, b)** constructs a tuple; **(a,)** is the one-element tuple. **()** as a value is unsupported.
- **value.field**, **pair.0**, and **values[index]** access members/elements.
- String indexing is typed as **char**.
- Calls do not support named arguments.

### Precedence, tightest to loosest

| Level | Operators/forms | Associativity |
| --- | --- | --- |
| Postfix | calls, **::Variant**, struct literals, **.field** / **.0**, **[index]**, postfix **++** / **--** | left to right |
| Prefix | **!**, unary **-**, prefix **++** / **--** | right to left |
| Multiplicative | *****, **/**, **%** | left to right |
| Additive | **+**, **-** | left to right |
| Comparison | **<**, **<=**, **>**, **>=**, **instanceof** | left to right, same level |
| Equality | **==**, **!=** | left to right |
| Logical AND | **&&** | left to right, short-circuit |
| Logical OR | **||** | left to right, short-circuit |
| Assignment | **=**, **+=**, **-=**, ***=**, **/=**, **%=** | right to left |

There are no bitwise operators, exponentiation operator, ternary operator, optional chaining, or
**switch**. String concatenation with **+** requires strings; use interpolation or **string(value)** for
conversion. Integer arithmetic uses checked operations in the current IR lowering. A zero integer
divisor is an error. Structs, vectors, and maps use reference identity for equality; tuples and enum
payloads compare structurally.

**instanceof** is parsed at the comparison level. Its right operand is a bare type identifier, not an
applied type expression. The checker supports the primitive names and selected built-in/container/
struct types; enums are not valid **instanceof** targets.

## Patterns and matching

A match is written as a statement:

~~~rey
match result {
    Result::Ok(value) => { println(value); },
    Result::Err(message) => println(message);
}
~~~

Supported patterns include **_**, literal patterns, bindings, enum variants (qualified or
unqualified), variant payload patterns, struct field patterns, and inclusive integer ranges such as
**1..5**. Range endpoints must be integer literals. Bare variants must be unambiguous.

The checker validates coverage for enum variants, **Option**, **Result**, booleans, and nullable
values. Otherwise include **_** or a binding to cover unmatched values. Or-patterns are not
implemented. Match arms have no guard syntax. An arm body is a block or an expression followed by
**;**.

## Modules and imports

Every import ends with **;**:

~~~rey
import std::string;
import app::config::{load, Config};
import helpers.parse;
import helpers.{trim, split};
import "relative/path/file.rey";
~~~

- **import a::b::c;** binds the final module namespace.
- **import a::b::{name, ...};** selects public names from a module; if the module file is absent, the
  group can resolve child modules.
- Dotted imports select symbols from a **.rey** file.
- A string-path import binds the imported file under its file-stem name.

**std::...** uses the caller's configured standard-library root. Module lookup checks **main.rey** and
single-file modules under that root and its **src** directory. A plain module path searches the
importer's directory, its **src**, the project root, and project **src**; a nested **parent::child**
path also checks the child **.rey** file. Dotted and string imports resolve **.rey** files only.

Imports are loaded depth-first; a circular import is an error. Only public declarations cross a module
boundary. Top-level module initialization follows the loaded dependency order.

## Global built-ins

These names are registered by the current [typechecker](rey-compiler/src/typecheck/main.rey) in ordinary
source. They are not imported from **std**.

| Name/signature | Result and notes |
| --- | --- |
| **print(...values: any): void** | Writes values to standard output. |
| **println(...values: any): void** | Writes values and a newline. |
| **eprint(...values: any): void** | Writes values to standard error. |
| **eprintln(...values: any): void** | Writes values and a newline to standard error. |
| **assert(condition: bool, message: any): void** | Fails when condition is false. |
| **readFile(path: string): string** | Reads a file. |
| **writeFile(path: string, content: string): void** | Replaces or creates a file. |
| **appendFile(path: string, content: string): void** | Appends text to a file. |
| **fileExists(path: string): bool** | Checks whether the path exists; the native implementation uses stat success. |
| **deleteFile(path: string): void** | Removes a file path. |
| **mkdir(path: string): void** | Creates a directory. |
| **listDir(path: string): Vec<string>** | Lists names; backend behavior differs on errors. |
| **getEnv(name: string): Option<string>** | Environment lookup; backend support is incomplete. |
| **exec(command: string): Result<string, string>** | Runs a shell command in the LLVM backend. |
| **args(): Vec<string>** | Returns process arguments. |
| **exit(code: int): void** | Terminates the process. |
| **int(value: any): int** | Checker permits the integer/char/any family; backend conversions differ. |
| **float(value: any): float** | Numeric conversion; backend support differs. |
| **string(value: any): string** | String conversion; backend support differs. |
| **char(value: int): char** | Checker also accepts char/any through its permissive builtin signature. |
| **range(start: int, end: int): Vec<int>** | Registered by the checker; code generation is incomplete. |
| **input([prompt: string]): string** | Checker permits zero or one string argument; backend emission is incomplete. |
| **abs(value: number): number** | Returns the same numeric kind. |
| **min(a: number, b: number): number** | Numeric values must satisfy checker/backend compatibility rules. |
| **max(a: number, b: number): number** | Numeric values must satisfy checker/backend compatibility rules. |
| **floor(value: number): int** | Numeric conversion. |
| **ceil(value: number): int** | Numeric conversion. |
| **round(value: number): int** | Numeric conversion. |
| **sqrt(value: number): float** | LLVM backend includes domain checks. |
| **pow(base: number, exponent: number): float** | Returns a float. |
| **log(value: number): float** | LLVM backend includes domain checks. |
| **sin(value: number): float** | Numeric function. |
| **cos(value: number): float** | Numeric function. |
| **tan(value: number): float** | Numeric function. |
| **random(): float** | Registered by the checker; LLVM emission is incomplete. |

**number** above is documentation shorthand for **int | float**, not a declared Rey type. The
typechecker initially registers permissive **any** signatures for several conversion/math built-ins and
then applies additional call checks. The table describes the checked call shape, not a promise that
every backend implements it.

## Built-in methods and constructors

The compiler [typechecker](rey-compiler/src/typecheck/main.rey) provides the following method signatures.
**T**, **V**, and **E** stand for the receiver's checked types.

### Any value and structs

- **value.toString(): string** is accepted for all receiver types.
- A struct may define its own methods. A struct without a visible **toString** gets the synthesized
  zero-argument **toString(): string** signature.
- For an **any** receiver, the checker accepts any method name and arguments and returns **any**. This
  is a dynamic type-checking escape hatch, not a defined method catalog.

### string

~~~text
len(): int
toString(): string
toUpper(): string
toLower(): string
trim(): string
contains(needle: string): bool
startsWith(prefix: string): bool
endsWith(suffix: string): bool
indexOf(needle: string): int
slice(start: int, end: int): string
replace(old: string, new: string): string
repeat(count: int): string
split(separator: string): Vec<string>
toInt(): int
toFloat(): float
hash(): int
padLeft(width: int, fill: string): string
padRight(width: int, fill: string): string
~~~

### int and float

~~~text
toInt(): int
toFloat(): float
~~~

### Vec<T>

~~~text
new(): Vec<any>                 // Vec.new() constructor
len(): int
push(value: T): void
get(index: int): T
set(index: int, value: T): void
pop(): T
contains(value: T): bool
indexOf(value: T): int
slice(start: int, end: int): Vec<T>
reverse(): void
sort(): void
join(separator: string): string // accepted only for Vec<string>
map((T) -> U): Vec<U>
filter((T) -> bool): Vec<T>
reduce((U, T) -> U, initial: U): U
~~~

### HashMap<string, V>

~~~text
new(): HashMap<string, any>    // HashMap.new() constructor
len(): int
get(key: string): V
set(key: string, value: V): void
has(key: string): bool
delete(key: string): void
keys(): Vec<string>
values(): Vec<V>
entries(): Vec<any>
~~~

The runtime's **entries()** element shape is not accurately expressed by the checker signature.

### Option<T> and Result<T, E>

~~~text
Option::Some(value: T)
Option::None
isSome(): bool
isNone(): bool
unwrap(): T
unwrapOr(fallback: T): T

Result::Ok(value: T)
Result::Err(error: E)
isOk(): bool
isErr(): bool
unwrap(): T
unwrapErr(): E
unwrapOr(fallback: T): T
~~~

Option/Result constructors also accept dot spelling (**Option.Some(value)** and equivalent forms).
**unwrap** behavior on an error/none value is runtime-specific. **unwrapStr** and **unwrapOrStr** are
not accepted by the current typechecker.

### Collection method behavior

- The checker types **reverse()** as **void**; code generation currently has behavior that can
  allocate and return a reversed value, so do not treat it as a portable in-place contract.
- The checker accepts **Vec.set** and indexed assignment on supported source paths; individual
  backend support varies.
- The checker does not provide Vec **insert**, **remove**, or **clear**, nor HashMap **clear**.
- **entries()** is typed **Vec<any>** even though one runtime constructs nested vectors.

## Standard-library exports

This catalog is generated from public **pub func**, **pub struct**, and **pub enum** declarations in
[std/src](std/src) in this checkout. It inventories source declarations (including importable public
exports), not proof that each function typechecks or runs in every backend. Private helper functions
are excluded because they are not exported API.

**Snapshot count:** 312 public function declarations and 10 public struct declarations across 28
top-level modules and 4 directly addressable **math** submodules.

The compiler's standard-library loader resolves one module file per import. A submodule such as
**std::math::basic** is importable by its own path; it is not automatically re-exported from
**std::math**.
### Public types

#### std::image types


Source: [std/src/image/main.rey](std/src/image/main.rey)


~~~text
struct Image { width: int, height: int, pixels: Vec<int> }
~~~


#### std::linalg types


Source: [std/src/linalg/main.rey](std/src/linalg/main.rey)


~~~text
struct Vec2 { x: float, y: float }
struct Vec3 { x: float, y: float, z: float }
~~~


#### std::mem types


Source: [std/src/mem/main.rey](std/src/mem/main.rey)


~~~text
struct Span { buffer: Vec<any>, offset: int, length: int }
~~~


#### std::net types


Source: [std/src/net/main.rey](std/src/net/main.rey)


~~~text
struct SocketAddr { host: string, port: int }
~~~


#### std::sql types


Source: [std/src/sql/main.rey](std/src/sql/main.rey)


~~~text
struct Connection { path: string, driver: string }
struct QueryResult { columns: Vec<string>, rows: Vec<Vec<string>>, changes: int, ok: bool, error: string }
~~~


#### std::sync types


Source: [std/src/sync/main.rey](std/src/sync/main.rey)


~~~text
struct AtomicInt { val: int }
struct Mutex { locked: bool }
struct Channel { queue: Vec<any> }
~~~


### Public functions by module


#### std::binary


Source: [std/src/binary/main.rey](std/src/binary/main.rey)


~~~rey
pub func packU16Le(val: int): Vec<int>
pub func unpackU16Le(bytes: Vec<int>, offset: int): int
pub func packU32Le(val: int): Vec<int>
pub func unpackU32Le(bytes: Vec<int>, offset: int): int
pub func packU16Be(val: int): Vec<int>
pub func unpackU16Be(bytes: Vec<int>, offset: int): int
pub func packU32Be(val: int): Vec<int>
pub func unpackU32Be(bytes: Vec<int>, offset: int): int
pub func toHex(bytes: Vec<int>): string
pub func fromHex(hex: string): Vec<int>
~~~


#### std::cli


Source: [std/src/cli/main.rey](std/src/cli/main.rey)


~~~rey
pub func parseArgs(args: Vec<string>): any
pub func hasFlag(parsed: any, flagName: string): bool
pub func getOption(parsed: any, optName: string, defaultValue: string): string
pub func getPositionals(parsed: any): Vec<string>
pub func renderHelp(appName: string, desc: string, flags: Vec<Vec<string>>): string
~~~


#### std::collections


Source: [std/src/collections/main.rey](std/src/collections/main.rey)


~~~rey
pub func range(start: int, end: int, step: int = 1): Vec<int>
pub func zip(a: Vec<any>, b: Vec<any>): Vec<Vec<any>>
pub func flatten(nested: Vec<Vec<any>>): Vec<any>
pub func unique(arr: Vec<any>): Vec<any>
pub func groupBy(arr: Vec<any>, keyFn: (any) -> any): HashMap<string, any>
pub func chunk(arr: Vec<any>, size: int): Vec<Vec<any>>
pub func partition(arr: Vec<any>, predicate: (any) -> bool): Vec<Vec<any>>
pub func find(arr: Vec<any>, predicate: (any) -> bool): any
pub func findIndex(arr: Vec<any>, predicate: (any) -> bool): int
pub func every(arr: Vec<any>, predicate: (any) -> bool): bool
pub func some(arr: Vec<any>, predicate: (any) -> bool): bool
pub func count(arr: Vec<any>, predicate: (any) -> bool): int
pub func maxBy(arr: Vec<any>, keyFn: (any) -> any): any
pub func minBy(arr: Vec<any>, keyFn: (any) -> any): any
pub func sum(arr: Vec<any>): any
pub func sumBy(arr: Vec<any>, fn: (any) -> any): any
pub func frequency(arr: Vec<any>): HashMap<string, int>
pub func take(arr: Vec<any>, n: int): Vec<any>
pub func drop(arr: Vec<any>, n: int): Vec<any>
pub func concat(a: Vec<any>, b: Vec<any>): Vec<any>
pub func compact(arr: Vec<any>): Vec<any>
pub func map(arr: Vec<any>, fn: (any) -> any): Vec<any>
pub func filter(arr: Vec<any>, predicate: (any) -> bool): Vec<any>
pub func reduce(arr: Vec<any>, fn: (any, any) -> any, initial: any): any
pub func reverse(arr: Vec<any>): Vec<any>
pub func contains(arr: Vec<any>, item: any): bool
~~~


#### std::compress


Source: [std/src/compress/main.rey](std/src/compress/main.rey)


~~~rey
pub func rleCompress(data: string): string
pub func rleDecompress(compressed: string): string
pub func gzip(inputPath: string, outputPath: string): bool
pub func gunzip(inputPath: string, outputPath: string): bool
pub func tarCreate(archivePath: string, files: Vec<string>): bool
pub func tarExtract(archivePath: string, destDir: string): bool
~~~


#### std::core


Source: [std/src/core/main.rey](std/src/core/main.rey)


~~~rey
pub func panic(msg: string): void
pub func todo(msg: string): void
pub func unreachable(msg: string): void
pub func unwrapOr(opt: any, defaultValue: any): any
pub func isSome(opt: any): bool
pub func isNone(opt: any): bool
pub func isOk(res: any): bool
pub func isErr(res: any): bool
~~~


#### std::crypto


Source: [std/src/crypto/main.rey](std/src/crypto/main.rey)


~~~rey
pub func fnv1a(data: string): int
pub func djb2(data: string): int
pub func md5(data: string): string
pub func sha256(data: string): string
pub func randomBytes(n: int): Vec<int>
pub func toHex(bytes: Vec<int>): string
~~~


#### std::csv


Source: [std/src/csv/main.rey](std/src/csv/main.rey)


~~~rey
pub func parse(text: string): Vec<Vec<string>>
pub func parseWithDelimiter(text: string, delim: string): Vec<Vec<string>>
pub func stringify(rows: Vec<Vec<string>>): string
pub func stringifyWithDelimiter(rows: Vec<Vec<string>>, delim: string): string
~~~


#### std::env


Source: [std/src/env/main.rey](std/src/env/main.rey)


~~~rey
pub func get(key: string): any
pub func getOr(key: string, defaultVal: string): string
pub func set(key: string, value: string): void
pub func remove(key: string): void
pub func has(key: string): bool
pub func all(): any
~~~


#### std::fmt


Source: [std/src/fmt/main.rey](std/src/fmt/main.rey)


~~~rey
pub func format(template: string, args: Vec<any>): string
pub func padLeft(s: string, width: int, pad: string): string
pub func padRight(s: string, width: int, pad: string): string
pub func toHex(val: int): string
pub func toBin(val: int): string
pub func toOct(val: int): string
~~~


#### std::fs


Source: [std/src/fs/main.rey](std/src/fs/main.rey)


~~~rey
pub func read(path: string): string
pub func write(path: string, content: string): void
pub func append(path: string, content: string): void
pub func exists(path: string): bool
pub func remove(path: string): void
pub func mkdir(path: string): void
pub func mkdirAll(path: string): void
pub func removeDir(path: string): void
pub func removeDirAll(path: string): void
pub func isFile(path: string): bool
pub func isDir(path: string): bool
pub func list(path: string): Vec<string>
pub func listPaths(path: string): Vec<string>
pub func join(a: string, b: string): string
pub func basename(path: string): string
pub func dirname(path: string): string
pub func extname(path: string): string
pub func copy(src: string, dst: string): void
pub func move(src: string, dst: string): void
pub func touch(path: string): void
pub func readLines(path: string): Vec<string>
pub func writeLines(path: string, lines: Vec<string>): void
pub func ensureDir(path: string): void
pub func cleanPath(path: string): string
~~~


#### std::http


Source: [std/src/http/main.rey](std/src/http/main.rey)


~~~rey
pub func request( method: string, url: string, headers: HashMap<string, string>, body: string ): any
pub func get(url: string): any
pub func post(url: string, body: string): any
pub func put(url: string, body: string): any
pub func del(url: string): any
~~~


#### std::image


Source: [std/src/image/main.rey](std/src/image/main.rey)


~~~rey
pub func createImage(width: int, height: int): Image
pub func setPixel(img: Image, x: int, y: int, r: int, g: int, b: int): void
pub func getPixel(img: Image, x: int, y: int): Vec<int>
pub func toPpm(img: Image): string
pub func writePpm(img: Image, path: string): bool
~~~


#### std::io


Source: [std/src/io/main.rey](std/src/io/main.rey)


~~~rey
pub func printRed(msg: string): void
pub func printGreen(msg: string): void
pub func printYellow(msg: string): void
pub func printBlue(msg: string): void
pub func printMagenta(msg: string): void
pub func printCyan(msg: string): void
pub func printBold(msg: string): void
pub func clear(): void
pub func printTable(headers: Vec<string>, rows: Vec<Vec<string>>): void
pub func progressBar(current: int, total: int, width: int): string
pub func printBox(title: string, content: string): void
~~~


#### std::json


Source: [std/src/json/main.rey](std/src/json/main.rey)


~~~rey
pub func escapeString(s: string): string
pub func stringifyString(s: string): string
pub func stringifyInt(n: int): string
pub func stringifyFloat(f: float): string
pub func stringifyBool(b: bool): string
pub func stringifyArray(items: Vec<string>): string
pub func stringifyObject(entries: Vec<Vec<string>>): string
~~~


#### std::linalg


Source: [std/src/linalg/main.rey](std/src/linalg/main.rey)


~~~rey
pub func vec2(x: float, y: float): Vec2
pub func vec2Add(a: Vec2, b: Vec2): Vec2
pub func vec2Sub(a: Vec2, b: Vec2): Vec2
pub func vec2Scale(v: Vec2, s: float): Vec2
pub func vec2Dot(a: Vec2, b: Vec2): float
pub func vec2Length(v: Vec2): float
pub func vec2Distance(a: Vec2, b: Vec2): float
pub func vec2Normalize(v: Vec2): Vec2
pub func vec3(x: float, y: float, z: float): Vec3
pub func vec3Add(a: Vec3, b: Vec3): Vec3
pub func vec3Sub(a: Vec3, b: Vec3): Vec3
pub func vec3Scale(v: Vec3, s: float): Vec3
pub func vec3Dot(a: Vec3, b: Vec3): float
pub func vec3Cross(a: Vec3, b: Vec3): Vec3
pub func vec3Length(v: Vec3): float
pub func vec3Normalize(v: Vec3): Vec3
~~~


#### std::log


Source: [std/src/log/main.rey](std/src/log/main.rey)


~~~rey
pub func levelTrace(): int
pub func levelDebug(): int
pub func levelInfo(): int
pub func levelWarn(): int
pub func levelError(): int
pub func setLogLevel(level: int): void
pub func getLogLevel(): int
pub func trace(msg: string): void
pub func debug(msg: string): void
pub func info(msg: string): void
pub func warn(msg: string): void
pub func error(msg: string): void
pub func formatLog(level: string, msg: string): string
~~~


#### std::math


Source: [std/src/math/main.rey](std/src/math/main.rey)


~~~rey
pub func pi(): float
pub func e(): float
pub func tau(): float
pub func clamp(value, minVal, maxVal)
pub func sign(n)
pub func floor(n)
pub func ceil(n)
pub func round(n)
pub func toRadians(deg: float): float
pub func toDegrees(rad: float): float
pub func sin(x: float): float
pub func cos(x: float): float
pub func tan(x: float): float
pub func sum(nums: ...int): int
pub func mean(nums: ...int): int
pub func max(a, b)
pub func min(a, b)
pub func abs(n)
pub func sqrt(x: float): float
pub func pow(base, exp)
pub func log(x: float): float
pub func random(): float
pub func lerp(a: float, b: float, t: float): float
pub func hypot(a: float, b: float): float
pub func median(nums: Vec<int>): float
pub func variance(nums: Vec<int>): float
pub func stdDev(nums: Vec<int>): float
pub func gcd(a: int, b: int): int
pub func lcm(a: int, b: int): int
pub func factorial(n: int): int
pub func degToRad(deg: float): float
pub func radToDeg(rad: float): float
pub func trunc(x: float): int
pub func fract(x: float): float
~~~


#### std::math::basic


Source: [std/src/math/basic.rey](std/src/math/basic.rey)


~~~rey
pub func clamp(value, min, max)
pub func sign(n)
pub func floor(n)
pub func ceil(n)
pub func round(n)
~~~


#### std::math::extra


Source: [std/src/math/extra.rey](std/src/math/extra.rey)


~~~rey
pub func sqrt(x)
pub func pow(base, exp)
pub func log(x)
pub func random()
~~~


#### std::math::stats


Source: [std/src/math/stats.rey](std/src/math/stats.rey)


~~~rey
pub func sum(nums: ...int)
pub func mean(nums: ...int)
pub func max(a, b)
pub func min(a, b)
pub func abs(n)
~~~


#### std::math::trig


Source: [std/src/math/trig.rey](std/src/math/trig.rey)


~~~rey
pub func sin(x)
pub func cos(x)
pub func tan(x)
pub func toRadians(deg)
pub func toDegrees(rad)
~~~


#### std::mem


Source: [std/src/mem/main.rey](std/src/mem/main.rey)


~~~rey
pub func sizeOfInt(): int
pub func sizeOfFloat(): int
pub func sizeOfBool(): int
pub func sizeOfChar(): int
pub func makeSpan(buffer: Vec<any>, offset: int, length: int): Span
pub func spanGet(s: Span, index: int): any
pub func spanSlice(s: Span, start: int, end: int): Span
pub func copy(dest: Vec<any>, destOffset: int, src: Vec<any>, srcOffset: int, count: int): void
~~~


#### std::net


Source: [std/src/net/main.rey](std/src/net/main.rey)


~~~rey
pub func socketAddr(host: string, port: int): SocketAddr
pub func formatAddr(addr: SocketAddr): string
pub func isLoopback(addr: SocketAddr): bool
pub func parseIpv4(host: string): Vec<int>
pub func isIpv4(host: string): bool
pub func pingHost(host: string): bool
~~~


#### std::os


Source: [std/src/os/main.rey](std/src/os/main.rey)


~~~rey
pub func name(): string
pub func arch(): string
pub func eol(): string
pub func hostname(): string
pub func homedir(): string
pub func tempDir(): string
~~~


#### std::process


Source: [std/src/process/main.rey](std/src/process/main.rey)


~~~rey
pub func args(): Vec<string>
pub func exit(code: int): void
pub func env(key: string): Option<string>
pub func setEnv(key: string, value: string): void
pub func unsetEnv(key: string): void
pub func envAll(): HashMap<string, string>
pub func cwd(): string
pub func chdir(path: string): void
pub func tempDir(): string
pub func sleepMs(ms: int): void
pub func which(name: string): Option<string>
pub func run(cmd: string): Result<string, string>
pub func runDetailed(cmd: string): any
pub func isWindows(): bool
pub func isMac(): bool
pub func isLinux(): bool
~~~


#### std::regex


Source: [std/src/regex/main.rey](std/src/regex/main.rey)


~~~rey
pub func isMatch(pattern: string, text: string): bool
pub func find(pattern: string, text: string): any
pub func replace(pattern: string, text: string, repl: string): string
~~~


#### std::sql


Source: [std/src/sql/main.rey](std/src/sql/main.rey)


~~~rey
pub func connect(path: string): Connection
pub func execute(conn: Connection, sql: string): QueryResult
pub func query(conn: Connection, sql: string): QueryResult
pub func escapeString(val: string): string
pub func close(conn: Connection): void
~~~


#### std::string


Source: [std/src/string/main.rey](std/src/string/main.rey)


~~~rey
pub func trimStart(s: string): string
pub func trimEnd(s: string): string
pub func trim(s: string): string
pub func startsWith(s: string, prefix: string): bool
pub func endsWith(s: string, suffix: string): bool
pub func indexOf(s: string, sub: string): int
pub func lastIndexOf(s: string, sub: string, fromIndex: int = -1): int
pub func contains(s: string, sub: string): bool
pub func replace(s: string, from: string, to: string): string
pub func replaceAll(s: string, from: string, to: string): string
pub func slice(s: string, start: int, end: int): string
pub func substring(s: string, start: int, end: int): string
pub func padLeft(s: string, width: int, fill: string = " "): string
pub func padRight(s: string, width: int, fill: string = " "): string
pub func repeat(s: string, n: int): string
pub func split(s: string, sep: string): Vec<string>
pub func join(parts: Vec<string>, sep: string): string
pub func toLowerCase(s: string): string
pub func toUpperCase(s: string): string
pub func capitalize(s: string): string
pub func reverse(s: string): string
pub func lines(s: string): Vec<string>
pub func words(s: string): Vec<string>
pub func truncate(s: string, maxLength: int, suffix: string = "..."): string
pub func count(s: string, sub: string): int
pub func isNumeric(s: string): bool
pub func isAlpha(s: string): bool
pub func isAlphanumeric(s: string): bool
pub func parseInt(s: string): any
pub func parseFloat(s: string): any
pub func slugify(s: string): string
~~~


#### std::sync


Source: [std/src/sync/main.rey](std/src/sync/main.rey)


~~~rey
pub func atomicInt(initial: int): AtomicInt
pub func atomicGet(a: AtomicInt): int
pub func atomicSet(a: AtomicInt, v: int): void
pub func atomicAdd(a: AtomicInt, delta: int): int
pub func atomicSub(a: AtomicInt, delta: int): int
pub func mutex(): Mutex
pub func lock(m: Mutex): bool
pub func unlock(m: Mutex): void
pub func channel(): Channel
pub func send(ch: Channel, val: any): void
pub func recv(ch: Channel): any
~~~


#### std::test


Source: [std/src/test/main.rey](std/src/test/main.rey)


~~~rey
pub func assertEq(actual, expected, msg: string): void
pub func assertNotEq(actual, expected, msg: string): void
pub func assertTrue(condition: bool, msg: string): void
pub func assertFalse(condition: bool, msg: string): void
pub func assertApprox(actual: float, expected: float, epsilon: float, msg: string): void
pub func pass(msg: string): void
pub func fail(msg: string): void
~~~


#### std::time


Source: [std/src/time/main.rey](std/src/time/main.rey)


~~~rey
pub func sleep(ms: int): void
pub func formatDurationMs(ms: int): string
pub func formatDurationSeconds(sec: float): string
~~~


#### std::toml


Source: [std/src/toml/main.rey](std/src/toml/main.rey)


~~~rey
pub func parse(text: string): HashMap<string, any>
pub func getString(table: HashMap<string, any>, key: string, defaultVal: string): string
pub func getInt(table: HashMap<string, any>, key: string, defaultVal: int): int
pub func getBool(table: HashMap<string, any>, key: string, defaultVal: bool): bool
pub func stringify(table: HashMap<string, any>): string
~~~


## Compiler-only names and implementation gaps

The checked-in source has two distinct compilation paths:

- The established **rey-compiler/main.rey** path typechecks source and emits LLVM/C-runtime output.
- **rey-compiler/ir/lower/main.rey** lowers a narrower experimental managed-IR subset.

A typechecker accepting a call does not mean both paths can emit it. Important current gaps include:

- **input**, **range**, and **random** are registered by the checker but have no LLVM call emission.
  **getEnv** is typed as **Option<string>** but has no LLVM call emission; the existing C helper returns
  a string, not an Option.
- The managed-IR path does not lower many global I/O, process, and numeric built-ins that the frontend
  recognizes.
- Output built-ins differ between backends in accepted renderable values and separators; the checker
  does not treat every possible **any** value as printable.
- Conversion support differs: LLVM and managed lowering do not accept the same values for **int**,
  **float**, **string**, or **char**. LLVM represents **char** as a one-byte string at the runtime
  boundary, while managed lowering uses an integer representation. String indexing is typed as **char**,
  but backend representations and out-of-range behavior differ.
- **HashMap.entries()** has a checker/runtime shape mismatch. **Vec.reverse()** has checker/codegen
  behavior differences, so its return and mutation behavior are not a portable guarantee.
- Standard-library declarations are ordinary source, not injected built-ins. Several source modules
  call helper names such as **nativeArgs**, **nativeSetEnv**, **nativeExecDetailed**, **nativeMkdirAll**,
  **nativeSqrt**, **nativeRandom**, or **nativeSleepMs**. These names are not registered in ordinary
  user compilation, so some exported declarations cannot currently be compiled as written.
- Some std modules rely on host programs or OS behavior. Their presence in the public catalog is a
  source/API inventory, not a portability guarantee.

The following names from the older **syntax.md** are not registered as ordinary compiler built-ins:
**typeof**, **exp**, **wrapAdd**, **wrapSub**, **wrapMul**, **bitAnd**, **bitOr**, **bitXor**, **bitNot**,
**bitShl**, **bitShr**, **copyFile**, **moveFile**, **isFile**, **isDir**, **mkdirAll**, **removeDir**,
**removeDirAll**, **listDirPaths**, **pathJoin**, **basename**, **dirname**, **extname**, **setEnv**,
**unsetEnv**, **envAll**, **cwd**, **chdir**, **tempDir**, **execDetailed**, **which**, and **sleepMs**.
Some are public functions under **std::fs**, **std::process**, **std::env**, **std::os**, or **std::time**;
the standard-library catalog records their real module path. They are not globals.

There are no **LinkedList**, **Stack**, or **Queue** built-in types in this compiler frontend. Runtime C
symbols and conditional raw-runtime intrinsics are compiler internals, not source-level APIs. The
private raw intrinsic names include **__reyRawNull**, **__reyRawMap**, **__reyRawUnmap**,
**__reyRawOffset**, **__reyRawLoadI64**, **__reyRawStoreI64**, **__reyRawLoadByte**,
**__reyRawStoreByte**, **__reyRawEqual**, **__reyRawLoadAddress**, **__reyRawStoreAddress**,
**__reyRawFail**, **__reyRawFromInt**, **__reyRawToInt**, **__reyRawSyscall**, **__reyRawArgCount**,
and **__reyRawArg**. They are conditionally registered for compiler-authorized runtime units and are
not available to ordinary application code.

## Memory-management surface

Ordinary Rey source has no pointer type or syntax for manual pointer arithmetic, **malloc/free**,
ownership or borrowing annotations, explicit region scopes, or promotion statements. There is no
user-facing collector-control API. Programs express values and references; placement and lifetime
management belong to compiler/runtime implementation.

The v0.4 design target for this branch is automatic compiler-inferred regions with escape promotion,
without a trailing tracing collector. Region inference and promotion do not introduce source keywords.
The managed-IR implementation is experimental, so this syntax reference does not claim that the full
memory model or every backend is complete.

## Compact grammar

This is a compact grammar for the current parser surface. It omits the lexical bodies of string and
character tokens, which are specified above. The parser can construct some forms that the typechecker
then rejects; the semantic restrictions in this reference still apply.

~~~ebnf
program          = { topLevelItem }, EOF ;
topLevelItem     = declaration | statement ;
declaration      = functionDecl | structDecl | enumDecl ;
visibility       = [ "pub" ] ;

functionDecl     = visibility, "func", IDENT, "(", [ parameterList ], ")",
                   [ ":", type ], block ;
parameterList    = parameter, { ",", parameter }, [ "," ] ;
parameter        = IDENT, [ ":", [ "..." ], type ], [ "=", expression ] ;

structDecl       = visibility, "struct", IDENT, "{", { structMember }, "}" ;
structMember     = [ "pub" ], ( fieldDecl | methodDecl ) ;
fieldDecl        = IDENT, ":", type, [ "," ] ;
methodDecl       = "func", IDENT, "(", [ parameterList ], ")", [ ":", type ], block, [ "," ] ;

enumDecl         = visibility, "enum", IDENT, "{", [ enumVariant, { ",", enumVariant }, [ "," ] ], "}" ;
enumVariant      = IDENT, [ "(", [ payloadList ], ")" ] ;
payloadList      = namedPayloadList | typeList ;
namedPayloadList = namedPayload, { ",", namedPayload }, [ "," ] ;
namedPayload     = IDENT, ":", type ;

statement        = variableDecl | importDecl | block | ifStmt | whileStmt | loopStmt
                 | forStmt | matchStmt | returnStmt | breakStmt | continueStmt | expression, ";" ;
variableDecl     = ( "var" | "const" ), IDENT, [ ":", type ], "=", expression, ";" ;
block            = "{", { statement | declaration }, "}" ;
ifStmt           = "if", expression, block, [ "else", ( ifStmt | block ) ] ;
whileStmt        = "while", expression, block ;
loopStmt         = "loop", block ;
forStmt          = "for", ( IDENT, "in", expression | "(", IDENT, "in", expression, ")" ), block ;
returnStmt       = "return", [ expression ], ";" ;
breakStmt        = "break", ";" ;
continueStmt     = "continue", ";" ;

matchStmt        = "match", expression, "{", [ matchArm, { ",", matchArm }, [ "," ] ], "}" ;
matchArm         = pattern, "=>", ( block | expression, ";" ) ;
pattern          = "_" | literalPattern | integerRange | enumPattern | structPattern | IDENT ;
integerRange     = [ "-" ], integerLiteral, "..", [ "-" ], integerLiteral ;
enumPattern      = [ IDENT, "::" ], IDENT, [ "(", [ patternList ], ")" ] ;
patternList      = pattern, { ",", pattern }, [ "," ] ;
structPattern    = IDENT, "{", [ patternField, { ",", patternField }, [ "," ] ], "}" ;
patternField     = IDENT, ":", pattern ;

importDecl       = "import", ( moduleImport | dottedImport | stringImport ), ";" ;
moduleImport     = modulePath, [ "::", "{", nameList, "}" ] ;
modulePath       = IDENT, { "::", IDENT } ;
dottedImport     = IDENT, ".", ( IDENT | "{", nameList, "}" ) ;
stringImport     = STRING ;
nameList         = IDENT, { ",", IDENT }, [ "," ] ;

type             = nullableType, { "|", nullableType } ;
nullableType     = typeAtom, [ "?" ] ;
typeAtom         = primitiveType | namedType | appliedType | tupleType | functionType ;
primitiveType    = "int" | "float" | "bool" | "string" | "char" | "void" | "any" ;
namedType        = IDENT ;
appliedType      = IDENT, "<", typeList, ">" ;
typeList         = type, { ",", type } ;
tupleType        = "(", type, ",", [ typeList ], ")" ;
functionType     = "(", [ typeList ], ")", "->", type ;

expression       = assignment ;
assignment       = logicalOr, [ assignOp, assignment ] ;
assignOp         = "=" | "+=" | "-=" | "*=" | "/=" | "%=" ;
logicalOr        = logicalAnd, { "||", logicalAnd } ;
logicalAnd       = equality, { "&&", equality } ;
equality         = comparison, { ( "==" | "!=" ), comparison } ;
comparison       = additive, { ( "<" | "<=" | ">" | ">=" | "instanceof" ), additive } ;
additive         = multiplicative, { ( "+" | "-" ), multiplicative } ;
multiplicative   = unary, { ( "*" | "/" | "%" ), unary } ;
unary            = ( "!" | "-" | "++" | "--" ), unary | postfix ;
postfix          = primary, { callSuffix | memberSuffix | indexSuffix | updateSuffix } ;
callSuffix       = "(", [ argumentList ], ")" ;
memberSuffix     = "." , ( IDENT | integerLiteral ) | "::", IDENT ;
indexSuffix      = "[", expression, "]" ;
updateSuffix     = "++" | "--" ;
argumentList     = expression, { ",", expression }, [ "," ] ;

primary          = literal | IDENT | lambda | tupleOrGroup | vectorLiteral | dictLiteral | structLiteral ;
lambda           = "(", [ lambdaParameters ], ")", "=>", expression ;
lambdaParameters = typedParameter, { ",", typedParameter }, [ "," ] ;
typedParameter   = IDENT, ":", type ;
tupleOrGroup     = "(", expression, ")" | "(", expression, ",", [ expressionList ], ")" ;
expressionList   = expression, { ",", expression }, [ "," ] ;
vectorLiteral    = "[", [ expressionList ], "]" ;
dictLiteral      = "{", [ dictEntry, { ",", dictEntry }, [ "," ] ], "}" ;
dictEntry        = ( IDENT | STRING ), ":", expression ;
structLiteral    = IDENT, "{", [ structEntry, { ",", structEntry }, [ "," ] ], "}" ;
structEntry      = IDENT, ":", expression ;

literal          = integerLiteral | floatLiteral | STRING | CHAR | "true" | "false" | "null" ;
literalPattern   = literal ;
integerLiteral   = decimalInteger | hexInteger | binaryInteger ;
decimalInteger   = decimalDigits ;
hexInteger       = ("0x" | "0X"), hexDigits ;
binaryInteger    = ("0b" | "0B"), binaryDigits ;
floatLiteral     = decimalDigits, ".", [ decimalDigits ] ;
decimalDigits    = digit, { digit | "_", digit } ;
hexDigits        = hexDigit, { hexDigit | "_", hexDigit } ;
binaryDigits     = binaryDigit, { binaryDigit | "_", binaryDigit } ;
digit            = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;
binaryDigit      = "0" | "1" ;
hexDigit         = digit | "a" | "b" | "c" | "d" | "e" | "f" | "A" | "B" | "C" | "D" | "E" | "F" ;
IDENT            = identifierStart, { identifierContinue } ;
identifierStart  = ASCII_LETTER | "_" ;
identifierContinue = ASCII_LETTER | DIGIT | "_" ;
~~~

Grammar notes:

- A function parameter's **...** appears after its colon, as in **rest: ...int**. The checker requires it
  to be final and unique.
- Enum payloads are positional types or all-named **name: type** entries inside parentheses; enum
  variants do not use struct-literal braces.
- **instanceof** shares comparison precedence, but the checker requires its right side to be a bare
  supported type identifier.
- The grammar does not encode context rules such as only top-level public declarations, enum
  exhaustiveness, lvalue checks, import visibility, or the restrictions on generic types.
- Or-patterns, match guards, block-bodied lambdas, match expressions, empty tuple values, and numeric
  exponent notation are not supported.
