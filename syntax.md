# Rey Language Syntax Reference

**Version:** 0.2.0  
**Compiler:** rey-macos-arm64 (native) / rey-v0 (Rust interpreter)

---

## Table of Contents

1. [Lexical Structure](#1-lexical-structure)
2. [Types](#2-types)
3. [Variables and Constants](#3-variables-and-constants)
4. [Operators](#4-operators)
5. [Control Flow](#5-control-flow)
6. [Functions](#6-functions)
7. [Lambdas](#7-lambdas)
8. [Structs](#8-structs)
9. [Enums](#9-enums)
10. [Pattern Matching](#10-pattern-matching)
11. [Collections — Vec](#11-collections--vec)
12. [Collections — HashMap](#12-collections--hashmap)
13. [Strings](#13-strings)
14. [Result and Option](#14-result-and-option)
15. [instanceof](#15-instanceof)
16. [Imports](#16-imports)
17. [Built-in Functions](#17-built-in-functions)
18. [I/O and Process](#18-io-and-process)

<<<<<<< HEAD
---

## 1. Lexical Structure
=======
Semicolons are required statement terminators.

```rey
var x = 10;
var y: int = 20;
const pi: float = 3.14;
```

- `var` declares a mutable variable
- `const` declares an immutable variable (cannot be reassigned)
- Statements must end with `;`:
  - variable declarations
  - assignments
  - returns
  - expression statements
>>>>>>> 81dc245 (.)

### Comments
```rey
// single-line comment
```

### Identifiers
Identifiers start with a letter or underscore, followed by letters, digits, or underscores.
```rey
myVariable  _private  count2
```

Convention: `camelCase` for variables and functions, `PascalCase` for types.

### Literals

| Kind | Examples |
|------|----------|
| Integer | `0`, `42`, `-7`, `1_000_000` |
| Float | `3.14`, `-0.5`, `2.0` |
| String | `"hello"`, `"line\n"`, `"tab\t"` |
| Bool | `true`, `false` |
| Null | `null` |

<<<<<<< HEAD
String escape sequences: `\n`, `\t`, `\\`, `\"`, `\r`
=======
Equality semantics:
- primitives (`int`, `float`, `bool`, `char`) use value equality
- strings use value equality
- arrays use reference equality
- structs use reference equality

Logical:
- `&&`, `||`, `!`
>>>>>>> 81dc245 (.)

### Keywords
```
var  const  func  struct  enum  match  import  export  pub
if  else  while  loop  for  in  break  continue  return
true  false  null  instanceof
```

---

## 2. Types

### Primitive types

| Type | Description | Example |
|------|-------------|---------|
| `int` | 64-bit integer | `42` |
| `float` | 64-bit double | `3.14` |
| `bool` | Boolean | `true` |
| `String` | Heap string | `"hello"` |
| `Void` | No return value | return type only |

### Compound types

| Type | Description |
|------|-------------|
| `Vec` | Dynamic array |
| `HashMap` | String-keyed hash map |
| `Result` | Ok/Err tagged union |
| `Option` | Some/None tagged union |
| `Any` | Escape hatch — no type checking |

### Type annotations

Type annotations are optional. When present, they are checked at compile time.

```rey
var x: int = 42;
var name: String = "Rey";
func add(a: int, b: int): int { return a + b; }
```

Unannotated variables are inferred from the initializer:
```rey
var x = 42;       // inferred: int
var s = "hello";  // inferred: String
```

### Nullable types

Append `?` to make a type nullable:
```rey
var maybe: int? = null;
var name: String? = "Rey";
```

### Union types

Use `|` for union types:
```rey
var value: int | String = 42;
var value2: int | String = "hello";
```

---

## 3. Variables and Constants

```rey
var x = 10;             // mutable, inferred type
var y: int = 20;        // mutable, explicit type
const PI = 3.14159;     // immutable
const MAX: int = 1000;  // immutable, typed
```

Assignment:
```rey
x = 30;        // reassign
x = x + 1;    // no ++ operator; use explicit arithmetic
```

Compound assignment:
```rey
x += 5;
x -= 2;
x *= 3;
x /= 4;
x %= 7;
```

---

## 4. Operators

### Arithmetic
```rey
a + b    // addition (also string concat)
a - b    // subtraction
a * b    // multiplication
a / b    // integer division when both operands are int
a % b    // modulo
-a       // negation
```

### Comparison
```rey
a == b   // equal
a != b   // not equal
a < b    // less than
a <= b   // less than or equal
a > b    // greater than
a >= b   // greater than or equal
```

### Logical
```rey
a && b   // and
a || b   // or
!a       // not
```

### String concatenation
```rey
"Hello, " + name + "!"
42.toString() + " items"
```

---

## 5. Control Flow

### if / else

```rey
if condition {
    // ...
}

if x > 0 {
    println("positive");
} else if x < 0 {
    println("negative");
} else {
    println("zero");
}
```

### while

```rey
var i = 0;
while i < 10 {
    println(i.toString());
    i = i + 1;
}
```

### loop

Infinite loop with explicit `break`:
```rey
var i = 0;
loop {
    if i >= 10 { break; }
    println(i.toString());
    i = i + 1;
}
```

### for / in

Iterate over an array literal or Vec:
```rey
for item in [1, 2, 3, 4, 5] {
    println(item.toString());
}

for name in names {
    println(name);
}
```

### break / continue

```rey
loop {
    if done { break; }
    if skip { continue; }
    doWork();
}
```

---

## 6. Functions

### Declaration

```rey
func greet(name: String): String {
    return "Hello, " + name + "!";
}

// void return — use Void annotation or omit return type
func log(msg: String): Void {
    println(msg);
}

// inferred return type
func double(x: int) {
    return x * 2;
}
```

### Calling

```rey
var message = greet("Rey");
log(message);
var result = double(21);
```

### Multiple parameters

```rey
func clamp(value: int, min: int, max: int): int {
    if value < min { return min; }
    if value > max { return max; }
    return value;
}
```

### Recursion

```rey
func fib(n: int): int {
    if n <= 1 { return n; }
    return fib(n - 1) + fib(n - 2);
}
```

### Exports (for modules)

```rey
export pub func add(a: int, b: int): int {
    return a + b;
}
```

---

## 7. Lambdas

```rey
var double = (x: int) => x * 2;
var add = (a: int, b: int) => a + b;

// lambda body can be a block
var clamp = (v: int, lo: int, hi: int) => {
    if v < lo { return lo; }
    if v > hi { return hi; }
    return v;
};
```

Passing lambdas:
```rey
func apply(f: (int) -> int, x: int): int {
    return f(x);
}

var result = apply((n) => n * n, 5);  // 25
```

---

## 8. Structs

### Declaration

```rey
struct Point {
    pub x: int,
    pub y: int,
}

struct Person {
    pub name: String,
    pub age: int,
}
```

<<<<<<< HEAD
### Instantiation
=======
Fields are private by default. Prefix a field with `pub` to allow external mutation.
Struct field declarations are comma-separated. Trailing commas are allowed.

Struct literal:
>>>>>>> 81dc245 (.)

```rey
var p = Point { x: 3, y: 4 };
var alice = Person { name: "Alice", age: 30 };
```

### Field access

```rey
println(p.x.toString());     // 3
println(alice.name);         // Alice
```

### Field mutation

```rey
p.x = 10;
alice.age = alice.age + 1;
```

### Nested structs

```rey
struct Rectangle {
    pub topLeft: Point,
    pub bottomRight: Point,
}

var r = Rectangle {
    topLeft: Point { x: 0, y: 0 },
    bottomRight: Point { x: 100, y: 50 },
};
println(r.topLeft.x.toString());
```

---

## 9. Enums

```rey
enum Direction {
    North,
    South,
    East,
    West,
}

enum Color {
    Red,
    Green,
    Blue,
}
```

<<<<<<< HEAD
Accessing variants:
```rey
var dir = Direction.North;
var col = Color.Red;
```

Variants are represented as integers (0-indexed in declaration order) in the native compiler.

---

## 10. Pattern Matching
=======
Enum variants are comma-separated. Trailing commas are allowed.

Match:
>>>>>>> 81dc245 (.)

```rey
match direction {
    Direction.North => println("going north"),
    Direction.South => println("going south"),
    Direction.East  => println("going east"),
    Direction.West  => println("going west"),
}
```

<<<<<<< HEAD
Wildcard:
```rey
match value {
    0 => println("zero"),
    1 => println("one"),
    _ => println("other"),
}
```

Multi-arm with block body:
```rey
match color {
    Color.Red => {
        println("red");
        println("stop!");
    }
    Color.Green => println("green"),
    _ => println("other"),
}
```
=======
Match arms must be comma-separated.

Pattern kinds:
- enum variant (`Type::Variant` or unqualified `Variant`)
- struct pattern (`StructName { field: pattern, ... }`)
- literal (`1`, `"x"`, `true`, `null`)
- variable binding (`n`)
- wildcard (`_`)

If no match arm applies and there is no `_` fallback arm, runtime raises:
- `error[match]: non-exhaustive patterns`

## Built-ins
Global built-ins:
- `print(...)`
- `println(...)`
- `input()` / `input(promptString)`
- `len(value)` — works on strings, arrays, dictionaries
- `push(array, value)`
- `pop(array)`
- `abs(number)`
- `max(a, b)` — two numbers
- `min(a, b)` — two numbers
- `random()` — returns float in [0, 1)
- `range(start, end)` — used inside `for` loops (see Control Flow)
>>>>>>> 81dc245 (.)

Matching structs (Rust interpreter only):
```rey
match point {
    Point { x: 0, y: 0 } => println("origin"),
    Point { x, y }       => println("at " + x.toString() + "," + y.toString()),
}
```

<<<<<<< HEAD
---

## 11. Collections — Vec

```rey
// create
var v = Vec.new();

// push / pop
v.push(1);
v.push(2);
v.push(3);
var last = v.pop();   // 3

// access by index
var first = v[0];     // 1

// length
var n = v.length();   // 2

// contains
var has = v.contains(1);    // true (Vec<int>)

// join (Vec<String>)
var words = Vec.new();
words.push("hello");
words.push("world");
var s = words.join(", ");   // "hello, world"

// map
var doubled = v.map((x) => x * 2);

// filter
var evens = v.filter((x) => x % 2 == 0);

// array literal (shorthand)
var nums = [10, 20, 30];
for n in nums {
    println(n.toString());
}
```

---

## 12. Collections — HashMap

```rey
// create
var m = HashMap.new();

// set / get
m.set("name", "Rey");
m.set("version", "0.2.0");

var name = m.get("name");

// check existence
if m.has("name") {
    println("found: " + m.get("name"));
}

// delete
m.delete("version");

// keys / values
var keys = m.keys();     // Vec<String>
var vals = m.values();   // Vec<Any>
```

---

## 13. Strings

```rey
var s = "Hello, World!";

// length
var n = s.len();           // 13 (also: s.length())

// concat
var greeting = "Hi " + name + "!";

// index (single character as String)
var c = s[0];              // "H"

// slice
var sub = s.slice(0, 5);   // "Hello"

// search
var idx = s.indexOf(",");  // 5
var yes = s.startsWith("Hello");
var no  = s.endsWith("?");

// transform
var up  = s.toUpper();
var lo  = s.toLower();
var tr  = "  hi  ".trim();

// replace / repeat
var r = s.replace("World", "Rey");
var rep = "ab".repeat(3);          // "ababab"

// split
var parts = "a,b,c".split(",");    // Vec<String>

// convert to string
var i = 42;
var f = 3.14;
var b = true;
println(i.toString());
println(f.toString());
println(b.toString());   // "true" or "false"
```

---

## 14. Result and Option

### Result

```rey
// constructors
var ok  = Result.Ok(42);
var err = Result.Err("something went wrong");

// check and unwrap
if ok.isOk() {
    println(ok.unwrap().toString());
}

if err.isErr() {
    println(err.unwrapOr(0).toString());
}

// shorthand with file I/O
var content = readFile("data.txt");
if content.isOk() {
    println(content.unwrap());
} else {
    println("error: " + content.unwrapOr("unknown"));
}
```

### Option

```rey
var some = Option.Some(99);
var none = Option.None();

if some.isSome() {
    println(some.unwrap().toString());
}
```

---

## 15. instanceof

Check the runtime type of a value:

```rey
if value instanceof CallExpr {
    var call: CallExpr = value;
    // use call.callee, call.args, etc.
}

if x instanceof Vec {
    println("it's a vec");
}

if r instanceof Result {
    println("it's a result");
}
```

Supported types: any declared `struct`, `Vec`, `HashMap`, `Result`, `Option`.

---

## 16. Imports

### Module import

```rey
import lexer;
import parser;

var tokens = lexer.tokenize(source, "file.rey");
```

### Named symbol import

```rey
import lexer.tokenize;

var tokens = tokenize(source, "file.rey");
```

### Group import

```rey
import lexer.{tokenize, LexerResult};
```

### Relative file import

```rey
import "./utils";
import "./ast/nodes";
```

### Export

```rey
export pub func tokenize(source: String, path: String): LexerResult {
    // ...
}
```

Module resolution order:
1. Same directory as importing file
2. `<dir>/src/<module>/main.rey`
3. `<dir>/<module>/main.rey`
4. Project-root variants of the above

---

## 17. Built-in Functions

| Function | Description |
|----------|-------------|
| `println(value)` | Print with newline. Accepts `int`, `String`, `bool`, etc. |
| `print(value)` | Print without newline |
| `assert(cond, msg)` | Panic if `cond` is false |
| `typeof(value)` | Return type name as `String` (Rust interpreter only) |

---

## 18. I/O and Process

```rey
// file I/O
var content = readFile("path/to/file.txt");   // Result<String, String>
writeFile("out.txt", "hello\n");
appendFile("log.txt", "entry\n");
var exists = fileExists("file.txt");           // bool (as int: 1/0)
deleteFile("temp.txt");
mkdir("new-dir");
var entries = listDir(".");                    // Result<Vec<String>, String>

// process
var argv = args();        // Vec<String> — command-line arguments
var result = exec("ls -la");  // Result<String, String> — stdout
exit(0);                  // terminate with exit code

// environment
var home = getEnv("HOME");  // String
```

---

## Grammar Summary (EBNF sketch)

```
program    = stmt* EOF
stmt       = varDecl | constDecl | funcDecl | structDecl | enumDecl
           | assignStmt | exprStmt | ifStmt | whileStmt | loopStmt
           | forStmt | returnStmt | breakStmt | continueStmt | importStmt
           | blockStmt

varDecl    = "var" IDENT (":" type)? "=" expr ";"
constDecl  = "const" IDENT (":" type)? "=" expr ";"
funcDecl   = ("export" "pub")? "func" IDENT "(" params? ")" (":" type)? block
structDecl = "struct" IDENT "{" (field ",")* "}"
enumDecl   = "enum" IDENT "{" (IDENT ",")* "}"
importStmt = "import" importPath ";"

expr       = assign | ternary
assign     = IDENT assignOp expr
ternary    = or ("?" expr ":" expr)?
or         = and ("||" and)*
and        = eq ("&&" eq)*
eq         = cmp (("==" | "!=") cmp)*
cmp        = add (("<" | "<=" | ">" | ">=") add)*
add        = mul (("+" | "-") mul)*
mul        = unary (("*" | "/" | "%") unary)*
unary      = ("!" | "-") unary | postfix
postfix    = primary (call | index | field | "instanceof" type)*
call       = "(" args? ")"
primary    = NUMBER | STRING | BOOL | "null" | IDENT | lambda | array
           | structLiteral | "(" expr ")"
```

---

## Version Notes

- **v0.2.0** — native compiler; all features above compile to native via LLVM
- **v0.1.0** — Rust interpreter; full feature set including struct pattern matching  
- The type checker is a stub in v0.2.0 — type errors are caught at runtime
=======
Parser/lexer/import errors include file/line/column spans.

Runtime safety checks:
- null dereference raises `null dereference at line <n>`
- array/string bounds errors raise `index out of bounds (i=..., len=...)`
>>>>>>> 81dc245 (.)
