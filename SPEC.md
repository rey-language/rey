# Rey Language Specification — v0.3.0

**Status:** canonical language contract for Rey 0.3.0.  
**Authority:** this file and `docs/V0.3-CHARTER.md` supersede `syntax.md` and every file in `spec/`. Those files remain historical, non-normative material.  
**Scope:** this specifies the v0.3 language, not a claim that every feature is implemented. Implementation gaps noted below are not alternate rules.

## 1. Vocabulary and source files

- A **program** is an entry `.rey` source file plus its imports. A **module** is one source file: `directory/main.rey` for a directory module, or a single `.rey` file when that file itself is the import target. Other `.rey` files in the directory are separate modules. They are not loaded as part of the directory module.
- A **declaration** introduces a name; a **statement** performs an action; an **expression** produces a value; an **lvalue** is a mutable variable, field, or indexed collection element.
- Source files are UTF-8. LF and CRLF line endings are accepted. Diagnostics use one-based line and column numbers.
- Top-level statements are allowed. File extension is `.rey`.

## 2. Lexical structure

### 2.1 Whitespace, comments, identifiers

Whitespace separates tokens. `//` comments continue to line end. `/* ... */` comments end at the first `*/`; comments do not nest. Unterminated block comments are errors.

Identifiers start with a Unicode alphabetic character or `_`, followed by Unicode alphanumeric characters or `_`. Names are case-sensitive. Reserved keywords are:

```text
var const func struct enum import pub export return if else while loop for in
break continue match instanceof true false null self
```

`export` is reserved only to report the required migration error; it is not a valid declaration modifier. Primitive type names (`int`, `float`, `bool`, `string`, `void`, `char`, `any`) are identifiers recognized in type positions. Capitalized primitive spellings are errors with a fix-it to lowercase.

### 2.2 Numeric literals

- Decimal integers have one or more decimal digits; decimal floats have digits, `.`, then zero or more digits (`1`, `1.`, `1.25` are valid; `.25` is not).
- A member `.` does not start a float. After a member `.`, a decimal integer is a tuple index, and a following `.` starts another member access. `nested.0.0` reads index `0` of the value at `nested.0`. `1.5` is one float only when its `.` is not a member operator.
- Hex integers use `0x`/`0X` and hex digits; binary integers use `0b`/`0B` and binary digits.
- `_` may separate two valid digits in a number, but cannot lead, trail, or repeat. Separators are removed before conversion.
- A leading `-` is unary negation, not part of a literal. Suffixes are forbidden. Integers are signed 64-bit; floats are IEEE-754 binary64.
- Invalid prefixes/separators and out-of-range integers are errors. **SPEC choice:** v1 currently scans decimal digits and dots only; this rule formalizes charter-required separators and bases.

### 2.3 Strings, escapes, interpolation

Ordinary strings use `"`. Source characters contribute their original UTF-8 bytes without normalization. Exactly these escapes are recognized: `\n`, `\t`, `\r`, `\\`, `\"`, `\'`, `\0`, and `\xNN` (`N` is a hex digit; `\xNN` contributes one raw byte, including NUL or invalid UTF-8). Unknown, incomplete, and unterminated escapes are errors. **SPEC choice:** v1 currently preserves an unknown escape's escaped character; v0.3 rejects it so the escape set is closed and testable.

A single unescaped source `{` begins an interpolation, terminated by its matching `}`. While scanning the embedded text, every `{` increments brace depth and every `}` decrements it, including braces inside quoted substrings; the first return to depth zero ends the interpolation. The interpolation must contain one valid expression. Interpolations evaluate once, left-to-right, and are converted by `toString()` semantics before concatenation. In string text only, doubled `{{` or `}}` yields one literal brace and does not open/close interpolation; inside an interpolation, each brace changes depth. An unmatched single source `}` in string text is an error. **SPEC choice:** bytes produced by `\xNN` are literal content and do not become interpolation delimiters. For example, `"{{x}}"` is the literal `{x}`, while `"{x + 1}"` interpolates.

**SPEC choice:** v1's parser currently starts interpolation at every `{` and balances nested braces, but defines no literal-brace escape. Doubling both braces is the chosen, testable escape.

Triple-quoted `"""..."""` strings preserve source UTF-8 bytes: escapes are not processed; an initial LF or CRLF directly after the opener is omitted. Their contents use the same interpolation/brace rules as ordinary strings. **SPEC choice:** v1 routes triple-quoted literals through the same interpolation parser; “raw” here means only that backslash escapes are not decoded.

### 2.4 Character literals

A char literal is exactly one byte between single quotes: e.g. `'a'`, `'\n'`, `'\x41'`, `'\''`. UTF-8 source text in a literal contributes its original UTF-8 bytes; an unescaped source character that encodes to multiple bytes is therefore not a valid `char`. `\xNN` contributes one raw byte, including NUL or a byte that is not valid UTF-8. Empty, multi-byte, multi-character, raw-line-terminator, or unterminated literals are errors. The §2.3 escape set is valid in chars.

Strings preserve source UTF-8 bytes and `\xNN` bytes exactly, without normalization; string values may contain NUL and need not be valid UTF-8. This is the v0.3 byte-string contract, not a deferred Unicode feature. Unicode-aware operations are not provided; see §11.

### 2.5 Punctuation and terminators

Delimiters/operators are `(` `)` `{` `}` `[` `]` `,` `.` `..` `...` `:` `::` `;` `?` `|` `+` `-` `*` `/` `%` `!` `=` `==` `!=` `<` `<=` `>` `>=` `&&` `||` `=>` `->` `++` `--` `+=` `-=` `*=` `/=` `%=`. A lone `&` is invalid. A single `|` is a union separator; `||` is logical OR. Every simple statement requires `;`; block-ending declarations/control statements do not.

## 3. Types and values

### 3.1 Type set

Built-ins are lowercase `int`, `float`, `bool`, `string`, `void`, `char`, and `any`. `null` is a value/type member, not a standalone declared type. `void` is only a function return type. `any` accepts every value and weakens static checks.

Nominal types are structs and enums. Structural types are tuples `(T1, T2, ...)`, Vec/array, string-keyed HashMap/dictionary, function `(T1, T2, ...) -> R`, nullable `T?`, and union `T | U | ...`. A one-element tuple type is `(T,)`. Tuple elements are immutable and accessed by zero-based numeric member (`pair.0`, `pair.1`).

`Vec<T>`, `HashMap<K,V>`, `Option<T>`, and `Result<T,E>` are built-in applied types whose arguments are retained for static checking and erased only from the runtime representation. Discarding these arguments before static checking is an implementation bug, not the target behavior. `Vec<T>` uses the same logical element-type checking as `[T]` (for example, `Vec<string>` checks elements as `string`). `Option<T>` and `Result<T,E>` check constructors and payload-bearing operations against their declared payload types. HashMap keys remain string-only: `K` must be `string`, while `V` is checked as the value type. Bare `Vec`, `Option`, and `Result` use `any` for element/payload types; bare `HashMap` defaults to `HashMap<string, any>`. Applied arguments may compose with nullable, union, function, and tuple types. These built-ins do not introduce a general generic system: user-defined generic declarations/functions, constraints, generic inference, and monomorphization are unsupported.

### 3.2 Inference and compatibility

Annotated variables, parameters, fields, and returns are statically checked. An unannotated local infers its initializer's type, which does not later change. Unannotated parameters are `any`. Unannotated return types are inferred as the union of their reachable value-return types; a function with no value return is `void`, and mixing value and void returns is an error. Calls check arity and argument types. Explicit return types must be satisfied on all return paths. **SPEC choice:** the union rule makes inferred returns deterministic; the charter requires return inference but does not specify its join rule.

No implicit numeric conversion is allowed for assignments, parameters, or returns. Mixed int/float arithmetic is an explicit exception and returns float. `any` is compatible with all types. `T` is assignable to `T | U`; `null` is assignable to `T?`. Nullable values require narrowing before non-null operations. Capitalized primitive type spellings such as `String` are errors with lowercase fix-its. Unknown types and type aliases are unsupported.

### 3.3 Nullable, union, and `instanceof`

`T?` is shorthand for `T | null`. `instanceof` produces bool. In its true branch, a tested local is narrowed to the tested type. In the paired `else` branch, that type is removed from the local's union. For example, when `value` is `int | string`, the `else` of `value instanceof string` has type `int`, so `value + 1` is int addition. A `== null` or `!= null` test narrows only its true branch; the `else` keeps the type from before the test. The tested `instanceof` type must be a declared struct or supported built-in runtime type.

### 3.4 Structs, enums, Option, Result

Struct literals supply each declared field exactly once; missing/duplicate fields are errors. Field access and mutation use `.`. Methods are declared in the struct, called on instances with `.`, and have no explicit `self` parameter; their fields are in method scope. `pub` fields/methods are externally accessible, otherwise private.

Enums are nominal finite variant sets. A payload variant may declare positional types, as `Count(int)`, or named typed slots, as `Count(value: int)`. Each variant uses exactly one style; mixing named and positional slots in one variant is an error. Constructors and match patterns are always positional, in declaration order, for either declaration style (e.g. `Enum::Count(7)` and `Count(n)`). When exactly one enum in the module declares a variant name, `Variant(args)` is that constructor, so `Count(8)` is `Message::Count(8)` when `Message` is the only enum with `Count`. If two enums declare the same variant name, the bare call is a compile error and `Enum::Variant(args)` is required. A zero-payload variant is `Enum::Variant`. The same value may be written `Enum::Variant()`. `Option::None` and `Option::None()` are that none value. A payload constructor still requires its arguments; empty parentheses do not drop them. A variant-only pattern covers any payload of that variant.

`Option<T>` has `Option::Some(value)` and `Option::None` / `Option::None()` constructors. `Result<T,E>` has `Result::Ok(value)` and `Result::Err(error)`. `Some`, `Ok`, and `Err` take one argument. `None` takes zero. These constructor calls are expressions. They may be nested inside other arguments, as in `Box::Hold(Option::Some(7))` and `Message::Maybe(Option::None)`.

## 4. Declarations

### 4.1 Variables and constants

`var name [: T] = expression;` declares a mutable binding; `const name [: T] = expression;` declares an immutable one. Both require an initializer. Types are inferred when omitted, then fixed. Reassigning a const is an error; const does not deep-freeze referenced objects.

### 4.2 Functions and variadics

```rey
pub func greet(name: string): string { return "hello " + name; }
func log(message: string): void { println(message); }
func sum(first: int, rest: ...int): int { return first; }
```

A function is `[pub] func name(parameters) [: returnType] block`. Parameters are comma-separated, optionally typed, and may have default expressions. Variadic syntax is `name: ...T`; it is final, at most one, and receives extras as a Vec. Defaults must be trailing and type-compatible. Return annotations use `: T`; `->` is only for function types. `return;` returns void; `return expression;` returns a value.

### 4.3 Lambdas and capture

Lambdas use `(x: int) => x + 1`; `() => expression` has no parameters. A lambda body is an expression. Lambdas have function types and may be passed/returned. **SPEC choice:** v1 parses expression-bodied lambdas only; block-bodied lambda syntax in old drafts is not in v0.3.

Lambdas use immutable snapshot captures at creation. Scalar values (`int`, `float`, `bool`, `char`, and `string`) are copied as they existed when the lambda was created; later reassignment of the outer binding does not change the captured value. Reference aggregates (structs, arrays/Vecs, and HashMaps/dictionaries) copy the reference and retain normal shared identity, so later mutation through any alias remains visible. Tuple/enum/Option/Result values are copied by value, while reference-valued payloads they contain remain shared. A lambda cannot reassign a captured binding. For example, a closure capturing `x = 1` continues to read `1` after the outer `x` is set to `2`; a closure capturing a Vec observes later `push` operations on that same Vec. Closure representation/lifting is an implementation detail; these observable rules are normative.

### 4.4 Structs, methods, frozen `create` rule

```rey
struct Point {
    pub x: int,
    y: int,
    pub func move(dx: int, dy: int): void { x += dx; y += dy; }
}
```

Fields/methods are private unless marked `pub`; field declarations are comma-separated and may trail with a comma. Methods use field names as variables and write updated fields back to the receiver.

The exact frozen heuristic is: a **module-level** `pub func create(...)` whose return type is struct `T` declared in the same module is callable as both `T.create(...)` and bare `create(...)`, only when that module declares exactly one struct. The return type may be explicit or inferred. If the module declares zero or multiple structs, `create` is an ordinary function. It does not apply to private `create`, a differently named function, or a method inside a struct. A function written inside a struct is always an instance method, including a method named `create`. There is no user-declared static method. No other name-based static-call rule exists.

**Implementation note:** v1 currently recognizes static calls differently (uppercase receiver plus call), and its typechecker treats them as `any`; that is not conformance. The module-level charter rule above is definitive, not OPEN.

### 4.5 Enums

```rey
enum Shape { Point, Circle(radius: float), Pair(int, int), }
var c: Shape = Shape::Circle(2.5);
match c { Circle(r) => println(r); }
```

Variants are comma-separated and may trail with a comma. Payloads use either positional types (`Pair(int, int)`) or all-named slots (`Circle(radius: float)`); mixing forms within a variant is an error. Constructors and patterns bind payload slots positionally in declaration order, including named-slot declarations. Enum access is `Enum::Variant`, not `Enum.Variant`. Bare variant names occur only in a match arm whose known scrutinee enum makes the variant unambiguous. **SPEC choice:** payload forms extend v1's payload-free enum syntax.

## 5. Statements and control flow

### 5.1 Blocks and conditionals

A block is `{ statement* }`. `if condition { ... } [else if ... | else { ... }]` and `while condition { ... }` allow optional parentheses around the condition. Conditions must have type `bool`; no truthiness conversion exists. Branches/bodies introduce lexical scopes.

### 5.2 Loops and control transfer

`loop { ... }` repeats until broken. `for name in expression { ... }` iterates a Vec/array or `range(start, end)`. `range` yields ints start-inclusive and end-exclusive; endpoints must be integers or integer-valued floats. `start >= end` yields no iterations. `break;` exits and `continue;` advances the innermost loop. Either outside a loop is an error. `return;` and `return expression;` leave the current function/lambda; return outside one is an error. All require semicolons.

### 5.3 Match statement and patterns

`match expression { pattern => arm, ... }` is a statement, not an expression. Arms are comma-separated (optional trailing comma), tested in order. An arm is a block or an expression statement ending in `;`. **SPEC choice:** the expression-arm semicolon follows the frozen mandatory-terminator rule; old drafts omit it in some examples. Patterns are enum variants (`E::V` with optional payload patterns), a bare variant on a statically known enum scrutinee, literals (`int`, `float`, `string`, `char`, `bool`, `null`), inclusive integer range `a..b`, struct field patterns, bindings, and `_`. Range endpoints are integer literals and `a <= b`.

Match is exhaustive: every enum variant must be covered by a variant-only pattern (which ignores any payload), or `_`; payload-destructuring patterns alone never establish coverage. Bool requires true and false or `_`; nullable requires null and an exhaustive non-null case or `_`. Infinite/open domains, including int/string/char and general unions, require `_`/binding unless every finite member is statically provable. Literal/range sets alone do not exhaust an infinite domain. Statically provable duplicate/unreachable arms are errors; otherwise arms retain order.

## 6. Expressions and operators

### 6.1 Forms

Expressions include literals, names, groups, tuples, array/Vec and dictionary literals, struct literals, calls, member access, indexing, enum construction, lambdas, `instanceof`, assignments, unary and binary operators. Calls/member/index operations may chain. `array[i]`/`Vec[i]` are zero-based; `string[i]` returns a char; map indexing uses string keys. Mutable collections support index assignment. Invalid indices are runtime errors.

### 6.2 Precedence and associativity

Highest to lowest; rows associate left-to-right except assignment (right-to-left):

| Level | Operators/forms |
|---:|---|
| 1 | call `()`, index `[]`, member `.`/`::`, postfix `++`/`--` |
| 2 | prefix `!`, unary `-`, prefix `++`/`--` |
| 3 | `*` `/` `%` |
| 4 | `+` `-` |
| 5 | `<` `<=` `>` `>=` |
| 6 | `instanceof` |
| 7 | `==` `!=` |
| 8 | `&&` |
| 9 | `||` |
| 10 | `=` `+=` `-=` `*=` `/=` `%=` |

Comparison chains parse left-to-right and must typecheck at each operation. `..` is only a match-range delimiter. `=>` is lambda/match delimiter; `->` is function-type delimiter. String interpolation is not an infix operator.

### 6.3 Operator semantics

- `+` adds numbers, concatenates strings, or concatenates a string with the textual form of the other value. `-`, `*`, `/`, `%` require numbers.
- `int / int` truncates toward zero and returns int. If either operand is float, `/` returns float. `%` preserves int for two ints and returns float if either operand is float. For two ints, when `b` is not zero and `a / b` does not overflow, `a == (a / b) * b + (a % b)`. The remainder has the sign of `a`. For example, `-17 / 5` is `-3` and `-17 % 5` is `-2`.
- Division and remainder by zero, for `int` and for `float`, are runtime errors. They do not produce `inf` or `NaN`. `1.0 / 0.0` and `1.0 % 0.0` exit 1. Every signed 64-bit arithmetic result outside `[-2^63, 2^63-1]` is a runtime overflow error, including addition, subtraction, multiplication, unary negation, `++`/`--`, and compound assignments. `INT64_MIN / -1` and `INT64_MIN % -1` are also runtime overflow errors. `abs(INT64_MIN)` is a runtime error. Engines must give the same behavior in all build modes, must not wrap, and must not rely on undefined behavior or LLVM poison. Mixed int/float arithmetic is the explicit conversion exception. Finite float operations use IEEE-754 rounding and are not fused: a multiply followed by an add is two operations. Float equality is IEEE-754 equality: `NaN == NaN` is false, and `-0.0 == 0.0` is true.
- For example, `var min: int = -9223372036854775807 - 1; var q = min / -1;` must raise a runtime error, never wrap or produce an LLVM poison value.
- Numeric comparison accepts int/float pairs. String ordering is bytewise ASCII. Two `char` values order by unsigned byte value, from 0 through 255: `'a' < 'b'` is true, and `'\xFF' > '\x00'` is true. Ordering between `char` and `int`, `float`, or `string` is a type error; a `char` is not converted for comparison. Equality is value equality for primitives and strings; enum values, including `Option` and `Result`, compare by constructor and payload recursively; tuples compare elementwise. Structs, arrays/Vecs, and dictionaries/HashMaps compare by reference identity; null equals only null. **SPEC choice:** tuple equality is elementwise; payload-enum equality is structural; HashMap identity follows heap-reference representation. v1 does not currently define tuple, payload-enum, or dict equality. Function values are not comparable (**SPEC choice:** the v1 evaluator has no function-equality operation).
- `&&`/`||` require bool and short-circuit. `!` requires bool; unary `-` a number.
- Compound assignments evaluate the corresponding operation once, then store its result to a mutable lvalue.
- Prefix/postfix `++`/`--` are valid only for mutable int lvalues: variables, fields, or index targets. They change the value by one. Postfix yields old value then increments/decrements; prefix updates first and yields new value. Target is evaluated once.

## 7. Modules and imports

### 7.1 Spellings and behavior

The following are accepted and canonicalized to one import representation:

```rey
import std::math;
import std::math::{sin, cos};
import a.b;
import a.{b, c};
import "path/file.rey";
```

- `import module;` imports a module namespace using its final component. The namespace contains the public declarations of that module's one file (`module/main.rey`). Sibling `module/*.rey` files are not merged into it. `import math` does not fall back to a flat `math.rey`; that spelling is a directory module and is an error when `math/main.rey` is missing.
- `import parent::child;` imports the selected child module as a namespace named `child`. This is a module path. It resolves whether or not `parent/main.rey` exists: prefer `parent/child/main.rey`, then `parent/child.rey`.
- A brace group on a module path (`import math::{sin, cos}`, `import std::math::{sin, cos}`, `import modules::{account}`, `import shapes::solid::{area, perimeter}`) uses one rule. Resolve that module path to its `main.rey`. If the file exists, each name is a public declaration of that file only. Sibling files are not searched, and a sibling file does not shadow a declaration in `main.rey`. If `main.rey` does not exist, each name is its own child module under that path: prefer `path/name/main.rey`, then `path/name.rey` (folder before file), and the bound name is that child module's namespace. `import modules::{account}` and `import shapes::solid::{area, perimeter}` are the missing-file form. `import std::math::{sin, cos}` is the declaration form when `math/main.rey` exists. To load a sibling file while `main.rey` exists, write the unbraced child path, as in `import action::run` for `action/run.rey`.
- `import file.symbol;` / `import file.{symbols};` import named public declarations from `file.rey`.
- A string-path import imports a file's public declarations as a namespace named by the file stem; it names a file, not a directory, and interpolation is not evaluated in import paths.

Only `pub` declarations are exposed. A named import of missing/private declarations and duplicate imported names are errors. Re-import of the same canonical file/module is idempotent. Circular imports are errors at the import that re-enters a module already on the import stack, including a module whose resolved file is already on that stack. The diagnostic includes the text `Circular import detected` and the cycle chain of module paths separated by ` -> `, ending with the repeated module. No top-level statement runs when resolution fails.

Imports are permitted only at module scope, before executable top-level statements; an import nested in a function or block is an error.

**SPEC choice:** the charter freezes spellings but not their binding details. Dotted forms select a file and symbols; `::` selects module paths. This choice makes file and folder resolution deterministic.

### 7.2 Resolution order and file/folder priority

Resolve relative to the importing file first, then the entry file's project root. For component `m`, try in order:

1. importer directory: `m.rey` for file imports, `m/main.rey` for module imports;
2. importer `src/`: `m.rey` or `m/main.rey`;
3. project root: `m.rey` or `m/main.rey`;
4. project-root `src/`: `m.rey` or `m/main.rey`;
5. for the `std` root, `$HOME/.reyc/std/src/`; for package roots, `$HOME/.reyc/packages/`.

A module import tries only `m/main.rey` at each of those roots. A file import tries only `m.rey`. The `or` in items 2–4 names those two shapes; a module import does not fall back to the flat file, and a file import does not switch to a directory.

For `parent::item`, resolve parent as a directory, then prefer `parent/item/main.rey` over `parent/item.rey`. Thus module folder wins over same-named file for a module item. Path components are case-sensitive. A resolved module is that one file; resolution does not scan sibling `.rey` files for more declarations. For a namespace import, `::` selects module folders; dotted imports select `.rey` files. Explicit string file paths resolve as files and never switch to a directory. **SPEC choice:** folder-before-file follows the v1 module-item resolver candidate order. One file per module keeps import resolution to a fixed candidate list.

### 7.3 Visibility and removed keyword

`pub` means importable; no modifier means private. `pub` may mark functions, structs, enums, fields, and methods. Private declarations are module-local. A private method may be called from any function in the defining module. A struct literal in the defining module may initialize private fields, including from a function that is not a method. A struct literal outside that module may not name a private field. After construction, a private field may be read or written only inside that struct's methods, not from other functions in the same module and not from another module. Top-level `var`/`const` bindings are module-private; `pub var` and `pub const` are not valid v0.3 declarations. A function in a module may read that module's `var` and `const` after those bindings' declarations. Those bindings are not importable. `export` is a hard error with a fix-it: remove `export`, retaining `pub` if importability is intended. **SPEC choice:** the two-tier rule applies to types and their fields/methods, not only functions, so an imported public API can name its public types. Globals remain private, matching the v1 resolver's importable-function model.

## 8. Program execution shape

Import resolution finishes before any top-level statement runs. A cycle, a missing import, or any other resolution error is a compile error. Nothing runs.

Function, struct, and enum declarations are visible throughout their module, so a top-level statement may call a function declared later in that module. A `var` or `const` is visible only after its declaration. Using it earlier is a compile error. Its initializer is an executable statement.

After resolution succeeds, each module's executable top-level statements run exactly once. Import declarations and function, struct, and enum declarations are not executable statements. `var` and `const` initializers run in source order together with the module's other top-level statements, such as `println(...)`.

Order across modules is a depth-first post-order of the import graph. The walk follows each module's imports in source order. A module runs after every module it imports. A module reached by more than one path runs once. The entry file runs after every imported module. If the entry imports `left` and then `right`, and both import `leaf`, the order is `leaf`, `left`, `right`, then the entry file. If the entry imports `right` and then `left`, `right` runs before `left`, and `leaf` still runs once before both. Alphabetical order is not the rule.

A later read of a module binding sees the value stored by that one run. There is no second initialization and no lazy guard on the read.

If the entry file defines `main`, call it once after that file's top-level statements. Entry `main` takes no parameters and returns `void`; `pub` is not required. Any other entry `main` shape is a compile error. An imported module's `main` is an ordinary function. It is not called automatically. Other code may call it only under the normal visibility rules.

**SPEC choice:** one initialization in dependency order needs no per-read guard, and a module-level `var` is already stored before any dependent call. Only the entry `main` starts the program.

## 9. Built-in API inventory

This is the complete v0.3 built-in inventory, grounded in `compiler/v1/src/interpreter/std.rs`, `compiler/v1/src/interpreter/executor.rs`, and `rey-compiler/runtime/rey_rt.h` / `runtime/*.c`. Charter-frozen spellings take priority over old implementation names. `native*` aliases are internal and not source APIs. Anything marked **not in v0.3** is not callable.

### 9.1 Strings and conversions

| Receiver | v0.3 methods and behavior |
|---|---|
| any printable value | `toString()`; bools yield `true`/`false`, null yields `null`, ints yield decimal integers, a `char` yields its single raw byte (so `'\xFF'.toString().len()` is `1`, and byte 0 is a NUL byte, not the spelling `\x00`), floats use the spelling below. Interpolation and `print` / `eprint` use the same bytes. `null.toString()` is the four characters `null` and does not dereference null. |
| string | `len()` (byte count), `toUpper()`, `toLower()`, `trim()`, `split(delimiter)`, `contains(needle)`, `startsWith(prefix)`, `endsWith(suffix)`, `indexOf(needle)` (byte index or -1), `replace(from,to)`, `slice(start,end)` (half-open byte slice), `repeat(count)`, `toInt()`, `toFloat()`, `hash()`, `padLeft(width,pad)`, `padRight(width,pad)`. |
| int | `toInt()` (identity), `toFloat()` (binary64 conversion). |
| float | `toInt()` (truncate toward zero), `toFloat()` (identity). |
| string/Vec | `substring(...)` **not in v0.3**; use `slice`. |
| any receiver | `length()` **not in v0.3**. Use `.len()`. |
| string | `upper()` / `lower()` **not in v0.3**. Use `toUpper` / `toLower`. |
| global | `len(value)` **not in v0.3**; use `.len()`. |

`split("")` splits into one-byte strings. `trim()` removes ASCII whitespace bytes (HT, LF, VT, FF, CR, and space). Case conversion changes ASCII letters only; all non-ASCII bytes are preserved unchanged. Search, prefix/suffix checks, replace, split, slice, repeat, and padding operate on bytes. `replace` replaces non-overlapping occurrences left-to-right; an empty `from` inserts `to` at each byte boundary. `slice` indices must be nonnegative and are clamped to length; if the end is no greater than start, the result is empty. `repeat` requires a nonnegative count. `string.toInt()` parses ASCII decimal numeric text and truncates toward zero; `string.toFloat()` parses ASCII decimal floating-point text. `int.toInt()` and `float.toFloat()` are identities; `int.toFloat()` converts to binary64; `float.toInt()` truncates toward zero. For example, `"12.8".toInt()` yields `12`, while `"x".toInt()` is a runtime error. A valid conversion returns the target scalar; malformed input, non-finite/out-of-range float-to-int, and out-of-range string conversions are runtime errors and exit with status 1. Calls to `toInt()`/`toFloat()` on unsupported receiver types (for example, `Vec.new().toInt()`) are static type errors. `hash()` returns the first byte's numeric value and errors on an empty string. Pad width is a nonnegative byte count and pad text must be nonempty; its first byte is repeated. Float `toString()` is locale-independent and uses `.` as the decimal point. For a finite value, start from the characters of Rust 1.98 `format!("{:?}", value)` for that binary64 number (shortest round-trip Debug). If that spelling is an optional `-`, one or more digits, and a trailing `.0`, with no exponent, omit `.0`. Normative results: `0.0` is `0`; `-0.0` is `-0`; `1.0` is `1`; `-1.0` is `-1`; `1.5` is `1.5`; `0.1` is `0.1`; `1e-4` is `0.0001`; `1e-5` is `1e-5`; `1e16` is `1e16`; `2^53` (`9007199254740992`) is `9007199254740992`; `2^70` is `1.1805916207174113e21`; the largest finite value is `1.7976931348623157e308`; the smallest positive subnormal (the value of integer bits `1`) is `5e-324`. Non-finite values are `inf`, `-inf`, and `NaN` (every NaN uses that spelling). `print`, `println`, `eprint`, `eprintln`, and interpolation of a float use this same spelling.

No `length()` receiver alias is provided, including on Vecs.

### 9.2 Vec and HashMap

| Receiver | v0.3 methods and behavior |
|---|---|
| `Vec<T>` | `push(value)`, `pop()`, `get(index)`, `set(index,value)`, `len()`, `contains(value)`, `indexOf(value)`, `join(separator)`, `slice(start,end)`, `reverse()`, `sort()`, `map(fn)`, `filter(fn)`, `reduce(fn,initial)`. Ordered, mutable; empty pop and invalid indices are errors. `sort` supports homogeneous int/float/string/bool values; reverse/sort mutate and return void; slice returns a new Vec. |
| `Vec<T>` | `isEmpty()` **not in v0.3**. |
| `HashMap<string,V>` | `set(key,value)`, `get(key)`, `has(key)`, `delete(key)`, `keys()`, `values()`, `entries()`, `len()`. String keys; missing `get` is a runtime error; keys/values return Vecs; entries returns a Vec of two-element `[string, value]` pairs. |
| `HashMap<K,V>` | `insert(key,value)` **not in v0.3**; use `set`. |

Vec and HashMap `slice(start,end)` clamp endpoints to collection length; negative indices are errors; end no greater than start yields an empty collection. `Vec.new()` and `HashMap.new()` make empty containers. `[a,b,...]` is a `Vec<T>` value, the same value kind as `Vec.new()`, not a separate array type. Every `Vec` method applies to it, including `map`, `filter`, and `reduce`. `{key: value,...}` is a string-keyed dictionary; bare identifier keys become their names; `{}` is empty.

### 9.3 Option and Result

| Type | v0.3 API |
|---|---|
| `Option<T>` | `Option::Some(value)`, `Option::None` or `Option::None()`, `.unwrap()`, `.unwrapOr(default)`, `.isSome()`, `.isNone()` |
| `Result<T,E>` | `Result::Ok(value)`, `Result::Err(error)`, `.unwrap()`, `.unwrapErr()`, `.unwrapOr(default)`, `.isOk()`, `.isErr()` |

`Option.unwrap()` on None and `Result.unwrap()` on Err raise runtime errors. `Result.unwrapErr()` returns the Err payload unchanged and raises a runtime error when called on Ok. `unwrapOr(default)` returns the payload or supplied default for Option and Result. `Option.map(...)` and `Result.map(...)` are **not in v0.3**.

### 9.4 Free functions

| Names | v0.3 contract |
|---|---|
| `print(...values)`, `println(...values)` | Write zero or more values to stdout, separated by one ASCII space with no leading or trailing separator; only `println` appends LF. |
| `eprint(...values)`, `eprintln(...values)` | Same formatting to stderr; only `eprintln` appends LF. |
| `input()`, `input(prompt:string)` | Read a line; optional prompt is printed first. |
| `assert(condition:bool,message)` | Runtime assertion failure if false; exactly two args. |
| `exit(code:int)` | Terminate process with status. |
| `int(value)`, `char(value)`, `float(value)`, `string(value)` | Scalar conversions, distinct from the receiver methods above. `int` accepts a `char` and returns its unsigned byte, 0 through 255 (`int('A')` is `65`). `char` accepts an `int` and returns that byte when the int is in `0..=255` (`char(66)` is `'B'`, and `char(0)` and `char(255)` succeed); any other int, including `char(256)` and `char(-1)`, is a runtime error and exits 1. `float` accepts an `int` or `float` and returns the binary64 value (`float(4) == 4.0`). `string` of a `char` is that one raw byte; `string` of an `int`, `float`, or `bool` uses `toString()`; `string` of a `string` is that string. A call with the wrong argument type is a static type error. |
| `abs`, `min`, `max`, `floor`, `ceil`, `round`, `sqrt`, `pow`, `log`, `sin`, `cos`, `tan`, `random` | v1 numeric builtins; invalid arity/type/domain is runtime error. `floor` returns the greatest int no greater than the value (toward −∞); `ceil` returns the least int no less than the value (toward +∞); `round` returns int (`round(1.2)` is `1`, `round(1.8)` is `2`). An out-of-range result is a runtime error. `sqrt`, `pow`, `log`, trig, and `random` return float; `random()` is in `[0,1)`. |
| `readFile(path)`, `writeFile(path,contents)`, `appendFile(path,contents)`, `fileExists(path)`, `deleteFile(path)`, `mkdir(path)`, `listDir(path)` | File API. `readFile`/`listDir` return values directly; OS errors are runtime errors. `fileExists` returns bool; mutators return void. |
| `args()`, `exec(command)`, `getEnv(name)` | Process/env API. args is `Vec<string>`; exec is `Result<string,string>`; getEnv is `Option<string>`. `exec` runs the command. A process that starts with a success status is `Ok` of the captured stdout bytes. A non-zero status, and a failure to start the process, are `Err`, not an uncaught runtime error. `unwrapErr` on that `Err` returns the error string. |
| `range(start,end)` | Reserved `for` iterable form, start-inclusive/end-exclusive; accepts int or integer-valued float endpoints; not a general first-class range value. |

`typeof(value)` and `clock()` are **not in v0.3**. Free `push(array,value)`, `pop(array)`, and `len(value)` are **not in v0.3**; use receiver methods. The v1-only helpers `cwd`, `chdir`, `tempDir`, `mkdirAll`, `removeDir`, `removeDirAll`, `copyFile`, `moveFile`, `isFile`, `isDir`, `listDirPaths`, `pathJoin`, `basename`, `dirname`, `extname`, `setEnv`, `unsetEnv`, `envAll`, `sleepMs`, `execDetailed`, and `which`, and `LinkedList`/`Stack`/`Queue` constructors/methods are **not in v0.3**.

## 10. Errors, Results, diagnostics

Lexical, parse, import, and type errors are compile/check errors. Runtime errors include invalid operations not caught statically, divide/remainder by zero, null dereference, bounds errors, failed conversion/I/O, assertion failure, and invalid `unwrap`. Errors do not silently become null. Expected recoverable failure uses `Result<T,E>` or `Option<T>` and must be explicitly inspected.

Malformed or out-of-range supported explicit conversions are runtime errors and exit with status 1. A conversion requested on an unsupported source type is a static type error.

Diagnostics use `file:line:col: error: message`; warnings/notes use the same form. Lines/columns are one-based; ANSI is not required. `check` is silent on success. Exit status: `0` success, `1` compile/check/runtime error, `2` usage or unsupported operation.

## 11. Semantics notes

- Strings preserve UTF-8 source bytes and raw escaped bytes exactly, without normalization; values may contain NUL or invalid UTF-8. Stored byte length defines contents; NUL is data, not a terminator. Length, indexing, comparison, search, slicing, splitting, and other string operations are byte-based. `string[i]` returns the single byte at `i` as a `char`; UTF-8 sequences may therefore be observed one byte at a time. Case conversion is ASCII-only, and no operation is Unicode-aware. This is the v0.3 conformance model, not deferred Unicode support.
- For example, source string `"é"` consists of UTF-8 bytes `0xC3 0xA9`, so its length is 2 and index 0 yields char byte `0xC3`; `"\xFF"` has length 1 and preserves byte `0xFF`.
- `char` is one byte, not a Unicode code point/grapheme, and compares by unsigned byte value, including `<`, `<=`, `>`, and `>=` (§6.3). `int` is signed 64-bit; `float` is binary64. Signed integer overflow is a runtime error (§6.3).
- Strings compare by value. Primitive and enum values compare by value. Structs, arrays/Vecs, and dictionaries/HashMaps alias on assignment and compare by reference identity; distinct struct allocations with equal fields are unequal. Const does not freeze referenced objects.
- `&&`, `||`, interpolation parts, function arguments, and match arms evaluate left-to-right; short-circuit right operands may not execute.
- `for range` is half-open; match ranges are inclusive.

## 12. Non-goals

Out of scope: `uint`, `byte`, `double`, full generics, mutable lambda captures, bitwise operators, Unicode-aware string/character operations, match expressions, truthiness, implicit numeric coercion, and operator overloading. Strings nevertheless preserve UTF-8 source bytes and raw escaped bytes and operate on bytes as required by v0.3. The separate stdlib repository, website, VS Code extension, optimizer, and cross-platform support are outside the v0.3 sprint.

## 13. EBNF grammar

This grammar describes the complete v0.3 surface. `IDENT` excludes reserved keywords; `NUMBER` means `integerLiteral | decimalFloat`; `stringLiteral` and `characterLiteral` produce the decoded `STRING` and `CHAR` tokens specified in §2. Semantic constraints (typing, visibility, arity, exhaustiveness, lvalues, resolution) are specified above.

```ebnf
program             = { topLevelItem } , EOF ;
topLevelItem        = declaration | statement ;
identifier          = identifierStart , { identifierContinue } ;
identifierStart     = Unicode_Alphabetic | "_" ;
identifierContinue  = Unicode_Alphanumeric | "_" ;
lineTerminator      = LF | CRLF | CR ;
whitespace          = Unicode_Whitespace ;
lineComment         = "//" , { nonLineTerminator } , [ lineTerminator ] ;
blockComment        = "/*" , { blockCommentUnit } , "*/" ; (* comments do not nest *)
blockCommentUnit    = anySourceCharacterExceptAsterisk | "*" , anySourceCharacterExceptSlash ;
stringLiteral       = ordinaryString | multilineString ;
ordinaryString      = "\"" , { ordinaryStringUnit } , "\"" ;
ordinaryStringUnit  = stringCharacter | escapeSequence | braceEscape | interpolation ;
stringCharacter     = anySourceCharacterExceptQuoteBackslashOrBrace ; (* literal source scalar contributes its UTF-8 bytes *)
multilineString     = "\"\"\"" , { multilineStringUnit } , "\"\"\"" ;
multilineStringUnit = rawStringCharacter | braceEscape | interpolation ;
rawStringCharacter  = anySourceCharacterExceptTripleQuoteOrBrace ; (* literal source scalar contributes its UTF-8 bytes *)
braceEscape         = "{{" | "}}" ;
interpolation       = "{" , expression , "}" ; (* nested braces balance before expression parse *)
escapeSequence     = "\\n" | "\\t" | "\\r" | "\\\\" | "\\\"" | "\\'" | "\\0"
                    | "\\x" , hexDigit , hexDigit ;
characterLiteral   = "'" , ( oneByteCharacter | escapeSequence ) , "'" ;
oneByteCharacter   = one ASCII source byte except quote, backslash, or line terminator ;
declaration         = variableDecl | functionDecl | structDecl | enumDecl | importDecl ;
variableDecl        = ( "var" | "const" ) , IDENT , [ ":" , type ] , "=" , expression , ";" ;
functionDecl        = [ "pub" ] , "func" , IDENT , "(" , [ parameters ] , ")" , [ ":" , type ] , block ;
parameters          = parameter , { "," , parameter } , [ "," ] ;
parameter           = IDENT , [ ":" , [ "..." ] , type ] , [ "=" , expression ] ;
structDecl          = [ "pub" ] , "struct" , IDENT , "{" , { structMember } , "}" ;
structMember        = [ "pub" ] , ( fieldDecl | methodDecl ) ;
fieldDecl           = IDENT , ":" , type , [ "," ] ;
methodDecl          = "func" , IDENT , "(" , [ parameters ] , ")" , [ ":" , type ] , block , [ "," ] ;
enumDecl            = [ "pub" ] , "enum" , IDENT , "{" , [ enumVariant , { "," , enumVariant } , [ "," ] ] , "}" ;
enumVariant         = IDENT , [ "(" , enumPayloadList , ")" ] ;
enumPayloadList     = positionalPayloadList | namedPayloadList ;
positionalPayloadList = type , { "," , type } , [ "," ] ;
namedPayloadList    = namedPayload , { "," , namedPayload } , [ "," ] ;
namedPayload        = IDENT , ":" , type ;

importDecl          = "import" , ( moduleImport | fileSymbolImport | stringFileImport ) , ";" ;
moduleImport        = modulePath , [ "::" , "{" , nameList , "}" ] ;
modulePath          = IDENT , { "::" , IDENT } ;
fileSymbolImport    = IDENT , "." , ( IDENT | "{" , nameList , "}" ) ;
stringFileImport    = STRING ;
nameList            = IDENT , { "," , IDENT } , [ "," ] ;

statement           = block | ifStmt | whileStmt | loopStmt | forStmt | matchStmt | returnStmt | breakStmt | continueStmt | simpleStmt ;
block               = "{" , { statement | declaration } , "}" ;
ifStmt              = "if" , condition , block , [ "else" , ( ifStmt | block ) ] ;
whileStmt           = "while" , condition , block ;
condition           = expression | "(" , expression , ")" ;
loopStmt            = "loop" , block ;
forStmt             = "for" , IDENT , "in" , expression , block ;
matchStmt           = "match" , expression , "{" , [ matchArm , { "," , matchArm } , [ "," ] ] , "}" ;
matchArm            = pattern , "=>" , ( block | expression , ";" ) ;
returnStmt          = "return" , [ expression ] , ";" ;
breakStmt           = "break" , ";" ;
continueStmt        = "continue" , ";" ;
simpleStmt          = expression , ";" ;

pattern             = "_" | literalPattern | rangePattern | enumPattern | structPattern | IDENT ;
literalPattern      = decimalFloat | signedInteger | stringLiteral | characterLiteral | "true" | "false" | "null" ;
rangePattern        = signedInteger , ".." , signedInteger ;
signedInteger       = [ "-" ] , integerLiteral ;
enumPattern         = [ IDENT , "::" ] , IDENT , [ "(" , [ pattern , { "," , pattern } ] , ")" ] ;
structPattern       = IDENT , "{" , [ patternField , { "," , patternField } , [ "," ] ] , "}" ;
patternField        = IDENT , ":" , pattern ;

type                = unionType ;
unionType           = nullableType , { "|" , nullableType } ;
nullableType        = typeAtom , [ "?" ] ;
typeAtom            = primitiveType | "null" | IDENT , [ genericArgs ] | functionType | tupleType ;
primitiveType       = "int" | "float" | "bool" | "string" | "void" | "char" | "any" ;
genericArgs         = "<" , type , { "," , type } , ">" ;
functionType        = "(" , [ typeList ] , ")" , "->" , type ;
tupleType           = "(" , type , "," , [ typeList ] , ")" ;
typeList            = type , { "," , type } ;

expression          = assignment ;
assignment          = logicOr , [ assignOp , assignment ] ;
assignOp            = "=" | "+=" | "-=" | "*=" | "/=" | "%=" ;
logicOr             = logicAnd , { "||" , logicAnd } ;
logicAnd            = equality , { "&&" , equality } ;
equality            = instanceofExpr , { ( "==" | "!=" ) , instanceofExpr } ;
instanceofExpr      = comparison , { "instanceof" , typeAtom } ;
comparison          = additive , { ( "<" | "<=" | ">" | ">=" ) , additive } ;
additive            = multiplicative , { ( "+" | "-" ) , multiplicative } ;
multiplicative      = unary , { ( "*" | "/" | "%" ) , unary } ;
unary               = ( "!" | "-" | "++" | "--" ) , unary | postfix ;
postfix             = primary , { callSuffix | indexSuffix | memberSuffix | updateSuffix } ;
callSuffix          = "(" , [ argumentList ] , ")" ;
indexSuffix         = "[" , expression , "]" ;
memberSuffix        = ( "." , ( IDENT | tupleIndex ) ) | ( "::" , IDENT ) ;
tupleIndex          = decimalInteger ;
updateSuffix        = "++" | "--" ;
argumentList        = expression , { "," , expression } , [ "," ] ;

primary             = literal | IDENT | lambda | tupleOrGroup | arrayLiteral | dictLiteral | structLiteral ;
literal             = numberLiteral | stringLiteral | characterLiteral | "true" | "false" | "null" ;
lambda              = "(" , [ lambdaParameters ] , ")" , "=>" , expression ;
lambdaParameters    = lambdaParameter , { "," , lambdaParameter } , [ "," ] ;
lambdaParameter     = IDENT , ":" , type ;
tupleOrGroup        = "(" , expression , ")" | "(" , expression , "," , [ expressionList ] , ")" ;
expressionList      = expression , { "," , expression } , [ "," ] ;
arrayLiteral        = "[" , [ expressionList ] , "]" ;
dictLiteral         = "{" , [ dictEntry , { "," , dictEntry } , [ "," ] ] , "}" ;
dictEntry           = ( IDENT | STRING ) , ":" , expression ;
structLiteral       = IDENT , "{" , [ structEntry , { "," , structEntry } , [ "," ] ] , "}" ;
structEntry         = IDENT , ":" , expression ;

numberLiteral       = integerLiteral | decimalFloat ;
integerLiteral      = decimalInteger | hexadecimalInteger | binaryInteger ;
decimalInteger      = decimalDigits ;
hexadecimalInteger  = ( "0x" | "0X" ) , hexDigit , { hexDigit | "_" , hexDigit } ;
binaryInteger       = ( "0b" | "0B" ) , binaryDigit , { binaryDigit | "_" , binaryDigit } ;
decimalFloat        = decimalDigits , "." , [ decimalDigits ] ;
decimalDigits       = digit , { digit | "_" , digit } ;
digit               = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;
binaryDigit         = "0" | "1" ;
hexDigit            = digit | "a" | "b" | "c" | "d" | "e" | "f" | "A" | "B" | "C" | "D" | "E" | "F" ;
```

`modulePath` without a group names a module namespace of that one module file. A group on an existing module file selects public declarations of that file. A group whose module file is absent selects child modules, one per name (§7.1). Dotted import selects file symbols. A string path binds a namespace named by its file stem, which must be a valid identifier. `null` may occur only as a member of a union/nullable type. `rangePattern` endpoints are integer literals only. Known type names disambiguate struct literals from dictionaries; in a match, a known scrutinee enum disambiguates bare variant names from bindings.

## 14. Conformance status and open items

At the initial source review, v1 gaps/differences included: decimal-only number scanning; absent `\xNN`/NUL string decoding; no `..` range token; no string-path/chained module imports; `export pub` still parsed; enums without payloads; legacy method aliases `length`, `upper`, `lower`; `++`/`--` limited to variable nodes; no module-level `create`; char literals typed as `any`; static calls typed as `any`. The inspected Rust implementation uses UTF-8 `String` and Unicode `char`, so it cannot represent every arbitrary byte string required here; that is an engine conformance gap, not a deferred feature. The inspected native C ABI also lacks direct entries for some v1 methods (for example Vec sort/indexOf, HashMap len/entries, and Option isNone); native conformance must still provide the frozen public API. In `runtime/string.c`, negative slice indices are clamped and empty-needle replace is a no-op, unlike the v1 baseline specified above. Other agents are editing engines concurrently, so this is a review snapshot, not a claim about their current working-tree state. These are implementation gaps, not alternate rules.

The six follow-up decisions recorded in `docs/V0.3-CHARTER.md` are applied in this specification; none of those six decisions remain unresolved. They define enum payload forms, capture snapshots, signed overflow, byte-preserving strings/chars, explicit conversion errors, and static checking with runtime erasure for built-in type arguments. The `create` rule remains separately frozen in §4.4.

The 2026-10-17 decisions in charter §10 are also applied here: imported-module initialization (§8), one file per module and brace resolution (§§1, 7), float `toString` spelling (§9.1), and unsigned `char` ordering (§6.3). Those sections are the rules. An engine that still rejects a program those sections accept, or still prints a different float spelling, is behind the spec.
