# Rey Language Comprehensive Syntax & API Reference

**Specification Target:** Rey Language v0.3.0  
**Compiler:** `rey` (self-hosted native compiler via LLVM) & `rey-v0` (reference implementation)  
**Standard Character Encoding:** UTF-8  

---

## Table of Contents

1. [Lexical Structure](#1-lexical-structure)
   - [Statements and Semicolons](#11-statements-and-semicolons)
   - [Comments](#12-comments)
   - [Identifiers and Naming Conventions](#13-identifiers-and-naming-conventions)
   - [Keywords](#14-keywords)
   - [Literals and Escapes](#15-literals-and-escapes)
2. [Types and Type System](#2-types-and-type-system)
   - [Primitive Types](#21-primitive-types)
   - [Compound & Structural Types](#22-compound--structural-types)
   - [Nullable Types](#23-nullable-types)
   - [Union Types](#24-union-types)
   - [Type Narrowing & `instanceof`](#25-type-narrowing--instanceof)
3. [Variables, Constants & Scope](#3-variables-constants--scope)
   - [Declarations](#31-declarations)
   - [Assignments & Compound Operators](#32-assignments--compound-operators)
   - [Increment and Decrement](#33-increment-and-decrement)
4. [Operators & Expressions](#4-operators--expressions)
   - [Precedence & Associativity Table](#41-precedence--associativity-table)
   - [Arithmetic & Overflow Semantics](#42-arithmetic--overflow-semantics)
   - [Comparison & Equality](#43-comparison--equality)
   - [Logical Operators](#44-logical-operators)
5. [Control Flow](#5-control-flow)
   - [Conditionals: `if`, `else if`, `else`](#51-conditionals-if-else-if-else)
   - [Loops: `while`, `loop`, `for ... in`](#52-loops-while-loop-for--in)
   - [Control Transfer: `break`, `continue`, `return`](#53-control-transfer-break-continue-return)
   - [Pattern Matching: `match`](#54-pattern-matching-match)
6. [Functions & Lambdas](#6-functions--lambdas)
   - [Function Declarations](#61-function-declarations)
   - [Default Parameters & Variadics](#62-default-parameters--variadics)
   - [Lambdas & Closures](#63-lambdas--closures)
7. [Nominal Data Types: Structs & Enums](#7-nominal-data-types-structs--enums)
   - [Structs & Methods](#71-structs--methods)
   - [Enums & Algebraic Data Types](#72-enums--algebraic-data-types)
8. [Modules, Imports & Visibility](#8-modules-imports--visibility)
   - [Import Syntax](#81-import-syntax)
   - [Module Resolution Order & Priority](#82-module-resolution-order--priority)
   - [Visibility: `pub` vs Private](#83-visibility-pub-vs-private)
   - [Execution Lifecycle & Entry Point](#84-execution-lifecycle--entry-point)
9. [Collections](#9-collections)
   - [`Vec<T>`](#91-vect)
   - [`HashMap<string, V>`](#92-hashmapstring-v)
   - [`Option<T>`](#93-optiont)
   - [`Result<T, E>`](#94-resultt-e)
   - [`LinkedList`, `Stack`, `Queue`](#95-linkedlist-stack-queue)
10. [Complete Standard & Built-in Function Catalog](#10-complete-standard--built-in-function-catalog)
    - [Scalar Conversions](#101-scalar-conversions)
    - [Numeric & Math Builtins](#102-numeric--math-builtins)
    - [Bitwise & Wrapping Arithmetic](#103-bitwise--wrapping-arithmetic)
    - [Standard I/O & Diagnostics](#104-standard-io--diagnostics)
    - [File System Operations](#105-file-system-operations)
    - [Process, Environment & System Execution](#106-process-environment--system-execution)
11. [Methods Quick Reference](#11-methods-quick-reference)
    - [String Methods](#111-string-methods)
    - [Vec Methods](#112-vec-methods)
    - [HashMap Methods](#113-hashmap-methods)
    - [Option & Result Methods](#114-option--result-methods)

---

## 1. Lexical Structure

### 1.1 Statements and Semicolons
Semicolons (`;`) are **mandatory statement terminators** for all declarations, assignments, returns, break/continue statements, and expression statements. Block-closing braces (`}`) do not require a trailing semicolon.

```rey
var count: int = 10;
count += 5;
println("Count: " + count.toString());
```

### 1.2 Comments
Rey supports single-line and non-nesting multi-line block comments:

```rey
// Single-line comment extending to the end of the line

/* Multi-line block comment
   spanning multiple lines */
```

### 1.3 Identifiers and Naming Conventions
- Identifiers start with an ASCII/Unicode letter or underscore (`_`), followed by letters, digits, or underscores.
- Names are strictly case-sensitive.
- **Conventions:**
  - `camelCase` for variable names, field names, and function names.
  - `PascalCase` for struct types, enum types, and namespaces.
  - `SCREAMING_SNAKE_CASE` or `camelCase` for constants.

### 1.4 Keywords
The following tokens are reserved keywords in Rey v0.3:

```text
var       const     func      struct    enum      match
import    pub       return    if        else      while
loop      for       in        break     continue  instanceof
true      false     null      self      export
```
*(Note: `export` is reserved exclusively to emit a compile-time migration diagnostic pointing users to `pub`.)*

### 1.5 Literals and Escapes

#### Integers and Floats
- **Decimal Integers:** `0`, `42`, `-100`, `1_000_000` (underscores are allowed between digits for readability).
- **Hexadecimal Integers:** `0x1A3F`, `0xFF`
- **Binary Integers:** `0b1010_0110`, `0b0`
- **Floats (64-bit IEEE-754 binary64):** `3.14159`, `0.0`, `-0.5`, `1.0e-5`

#### Strings and Characters
- **Character Literals (`char`):** Exactly one raw byte enclosed in single quotes: `'a'`, `'Z'`, `'\n'`, `'\t'`, `'\''`, `'\x41'`.
- **String Literals (`string`):** Enclosed in double quotes (`"..."`). UTF-8 encoded byte strings.
- **Escape Sequences:**
  - `\n` (newline, ASCII 10)
  - `\t` (tab, ASCII 9)
  - `\r` (carriage return, ASCII 13)
  - `\\` (backslash)
  - `\"` (double quote)
  - `\'` (single quote)
  - `\0` (NUL byte)
  - `\xNN` (raw byte with two hexadecimal digits, e.g. `\x1B`)
- **String Interpolation:**
  Unescaped single braces `{expr}` inside strings interpolate expressions at runtime. Doubled braces `{{` or `}}` produce literal brace characters.
  ```rey
  var name = "Rey";
  var version = 3;
  println("Welcome to {name} v0.{version}!"); // "Welcome to Rey v0.3!"
  println("Use {{x}} for syntax templates."); // "Use {x} for syntax templates."
  ```
- **Triple-Quoted Strings:**
  Enclosed by `"""..."""`. Preserves source newlines and whitespace verbatim without decoding backslash escapes. Interpolation with `{expr}` remains active.

---

## 2. Types and Type System

### 2.1 Primitive Types
Primitive type keywords must be written in lowercase. Capitalized spellings (e.g. `String`, `Int`) produce a compile-time error.

| Type | Description | Size / Representation |
|---|---|---|
| `int` | 64-bit signed two's-complement integer | 8 bytes (`i64`) |
| `float` | 64-bit IEEE-754 double precision float | 8 bytes (`double`) |
| `bool` | Boolean value (`true` or `false`) | 1 byte (`i1` / `u8`) |
| `char` | Exactly one 8-bit byte | 1 byte (`u8`, unsigned 0..255) |
| `string` | Immutable UTF-8 / raw byte string | Heap object pointer (`i8*`) |
| `void` | Absence of value (function return type only) | 0 bytes |
| `any` | Dynamic escape-hatch type | Tagged dynamic value box |

### 2.2 Compound & Structural Types

| Kind | Syntax Example | Notes |
|---|---|---|
| **Tuples** | `(int, string, bool)` | Fixed-size immutable heterogeneous product type. Element access via `.0`, `.1`. |
| **Vectors** | `Vec<T>` or `[T]` | Dynamic growable array of homogeneous elements. |
| **HashMaps** | `HashMap<string, V>` | Key-value associative hash table (keys are always `string`). |
| **Options** | `Option<T>` | Null-safe optional container (`Option::Some(v)` or `Option::None`). |
| **Results** | `Result<T, E>` | Error-handling union (`Result::Ok(v)` or `Result::Err(e)`). |
| **Functions** | `(int, string) -> bool` | Callable function signature. |

### 2.3 Nullable Types
Append `?` to any type to indicate it may hold `null`:
```rey
var age: int? = null;
var title: string? = "Engineer";

if title != null {
    // title is safely narrowed to string inside this block
    println(title.toUpper());
}
```

### 2.4 Union Types
Use the pipe operator `|` to declare heterogeneous union types:
```rey
var id: int | string = 101;
id = "REY-101"; // Valid
```

### 2.5 Type Narrowing & `instanceof`
The `instanceof` operator tests nominal structs, enums, and built-in dynamic values at runtime. Inside conditional branches, `instanceof` automatically narrows the variable:
```rey
func inspect(val: int | string | Vec<int>): void {
    if val instanceof int {
        println("Integer: " + val.toString());
    } else if val instanceof string {
        println("String: " + val);
    } else if val instanceof Vec {
        println("Vector length: " + val.len().toString());
    }
}
```

---

## 3. Variables, Constants & Scope

### 3.1 Declarations
- `var`: Declares a mutable variable.
- `const`: Declares an immutable variable (cannot be reassigned after initialization).

```rey
var counter: int = 0;
counter = counter + 1; // Valid

const MAX_USERS: int = 1024;
// MAX_USERS = 2048; // Compile Error: cannot reassign const
```

*Type annotations are optional when the initializer can be inferred:*
```rey
var score = 100;       // Inferred as int
var message = "Hello"; // Inferred as string
```

### 3.2 Assignments & Compound Operators
Rey supports standard assignment and compound in-place arithmetic:
```rey
var x = 10;
x += 5;  // x = 15
x -= 3;  // x = 12
x *= 2;  // x = 24
x /= 4;  // x = 6
x %= 4;  // x = 2
```

### 3.3 Increment and Decrement
Prefix and postfix `++` and `--` are valid on mutable integer lvalues:
```rey
var n = 5;
n++;   // Postfix: increments n to 6
++n;   // Prefix: increments n to 7
n--;   // Postfix: decrements n to 6
--n;   // Prefix: decrements n to 5
```

---

## 4. Operators & Expressions

### 4.1 Precedence & Associativity Table
From highest (tightest binding) to lowest:

| Level | Operators / Syntactic Forms | Associativity |
|:---:|---|---|
| **1** | Calls `()`, Indexing `[]`, Member Access `.`, Path `::`, Postfix `++`, `--` | Left-to-right |
| **2** | Unary `!`, Unary `-`, Prefix `++`, `--` | Right-to-left |
| **3** | Multiplicative: `*`, `/`, `%` | Left-to-right |
| **4** | Additive: `+`, `-` | Left-to-right |
| **5** | Relational: `<`, `<=`, `>`, `>=` | Left-to-right |
| **6** | Type Introspection: `instanceof` | Left-to-right |
| **7** | Equality: `==`, `!=` | Left-to-right |
| **8** | Logical AND: `&&` | Left-to-right |
| **9** | Logical OR: `\|\|` | Left-to-right |
| **10** | Assignment: `=`, `+=`, `-=`, `*=`, `/=`, `%=` | Right-to-left |

### 4.2 Arithmetic & Overflow Semantics
- **Checked Arithmetic:** Standard operations (`+`, `-`, `*`, `/`, `%`) perform strict runtime overflow checking. Overflowing $2^{63}-1$ or $-2^{63}$ terminates execution with an error diagnostic.
- **Division by Zero:** Division or modulo by zero terminates execution with a runtime panic.
- **Wrapping Arithmetic:** When explicit two's-complement modular wrapping is desired (e.g. hashing, cryptographic primitives, RNG), use the explicit builtins: `wrapAdd(a, b)`, `wrapSub(a, b)`, `wrapMul(a, b)`.

### 4.3 Comparison & Equality
- **Primitive Value Equality:** `int`, `float`, `bool`, `char` compare strictly by numerical/boolean value.
- **String Value Equality:** Strings compare byte-for-byte (`"rey" == "rey"` evaluates to `true`).
- **Tuple & Enum Equality:** Tuples and algebraic enums compare structurally across their members/payloads.
- **Reference Equality:** Heap collections (`Vec`, `HashMap`) and `struct` instances compare by reference identity (pointer address).

### 4.4 Logical Operators
Short-circuit evaluation is guaranteed for `&&` and `||`:
```rey
if ptr != null && ptr.isValid() {
    // ptr.isValid() is only called if ptr is not null
}
```

---

## 5. Control Flow

### 5.1 Conditionals: `if`, `else if`, `else`
Parentheses around conditions are optional. Condition must evaluate to `bool`.
```rey
if score >= 90 {
    println("Grade: A");
} else if score >= 80 {
    println("Grade: B");
} else {
    println("Grade: C");
}
```

### 5.2 Loops: `while`, `loop`, `for ... in`

#### `while` Loop
Repeats as long as the condition evaluates to `true`:
```rey
var i = 0;
while i < 5 {
    println(i.toString());
    i += 1;
}
```

#### Infinite `loop`
Repeats indefinitely until an explicit `break;` statement:
```rey
var n = 0;
loop {
    if n >= 10 {
        break;
    }
    n += 1;
}
```

#### `for ... in` Loop
Iterates through `Vec` elements, arrays, or numeric ranges:
```rey
// Iterating over a collection
for name in ["Alice", "Bob", "Charlie"] {
    println("Hello, " + name);
}

// Iterating over a range (start-inclusive, end-exclusive)
for idx in range(0, 5) {
    println("Index: " + idx.toString()); // 0, 1, 2, 3, 4
}
```

### 5.3 Control Transfer: `break`, `continue`, `return`
```rey
func findFirstEven(nums: Vec<int>): int {
    var found = -1;
    for n in nums {
        if n % 2 != 0 {
            continue; // Skip odd numbers
        }
        found = n;
        break;    // Stop on first match
    }
    return found;
}
```

### 5.4 Pattern Matching: `match`
The `match` statement tests expressions against pattern arms. Pattern matching is **exhaustive**:
- Matches enum variants (with or without payloads)
- Matches integer ranges `a..b`
- Matches scalar literals (`0`, `true`, `"test"`)
- Matches wildcard `_` fallback

```rey
enum Status {
    Idle,
    Processing(int),
    Failed(string)
}

func logStatus(s: Status): void {
    match s {
        Status::Idle => {
            println("System is idle.");
        },
        Status::Processing(progress) => {
            println("Progress: " + progress.toString() + "%");
        },
        Status::Failed(err) => {
            println("Error encountered: " + err);
        },
        _ => {
            println("Unknown state");
        }
    }
}
```

---

## 6. Functions & Lambdas

### 6.1 Function Declarations
Functions are declared using `func`. Return type annotations use `: Type`. If omitted, return type is inferred or `void`.

```rey
func add(a: int, b: int): int {
    return a + b;
}

pub func greet(name: string): void {
    println("Hello, " + name + "!");
}
```

### 6.2 Default Parameters & Variadics
Rey supports trailing default values and final variadic parameters:

```rey
// Default parameter values
func connect(host: string, port: int = 8080, secure: bool = true): void {
    // ...
}

// Variadic parameters (collected as Vec<T>)
func sum(first: int, ...rest: int): int {
    var total = first;
    for val in rest {
        total += val;
    }
    return total;
}
```

### 6.3 Lambdas & Closures
Lambdas use the arrow syntax `(args) => expression`.
- **Capture Semantics:**
  - **Scalars** (`int`, `float`, `bool`, `char`, `string`) are captured by **immutable value snapshot** at creation time.
  - **Aggregates** (`Vec`, `HashMap`, `struct`) capture their reference identity, sharing mutable heap state with the outer scope.

```rey
var multiplier = 3;
var triple = (x: int) => x * multiplier;
println(triple(10).toString()); // 30

var list = [1, 2, 3, 4, 5];
var doubled = list.map((n: int) => n * 2);
```

---

## 7. Nominal Data Types: Structs & Enums

### 7.1 Structs & Methods
Struct fields are private by default; use `pub` to expose fields or methods externally.

```rey
struct Vector2D {
    pub x: float,
    pub y: float,

    pub func length(): float {
        return sqrt(x * x + y * y);
    }

    pub func scale(factor: float): void {
        x *= factor;
        y *= factor;
    }
}

// Construction
var v = Vector2D { x: 3.0, y: 4.0 };
println("Magnitude: " + v.length().toString()); // 5.0
v.scale(2.0);
println("New X: " + v.x.toString());           // 6.0
```

### 7.2 Enums & Algebraic Data Types
Enums can declare pure unit variants or variants with typed payload parameters:

```rey
enum WebEvent {
    PageLoad,
    KeyPress(char),
    Click(int, int),
    Paste(text: string)
}

var ev = WebEvent::Click(100, 250);
```

---

## 8. Modules, Imports & Visibility

### 8.1 Import Syntax
Modules can be imported via directory namespaces, braced symbol groups, or relative file paths:

```rey
// Import an entire module file or folder namespace
import std::math;
var s = math.sin(1.57);

// Import specific symbols from a module
import std::math::{sin, cos, PI};

// Import via relative file path
import "helpers/parser.rey";
var p = parser.create();
```

### 8.2 Module Resolution Order & Priority
When an import `parent::child` or `child` is encountered, the compiler resolves paths in the following priority order:
1. Importer directory: `child/main.rey` (Folder priority) then `child.rey` (Flat file).
2. Importer `src/`: `src/child/main.rey` then `src/child.rey`.
3. Project root: `<root>/child/main.rey` then `<root>/child.rey`.
4. Project-root `src/`: `<root>/src/child/main.rey` then `<root>/src/child.rey`.
5. Standard library directory: `$HOME/.reyc/std/src/`.
6. External packages: `$HOME/.reyc/packages/`.

### 8.3 Visibility: `pub` vs Private
- Declarations without `pub` are strictly private to their defining module.
- `pub func`, `pub struct`, and `pub enum` are importable across module boundaries.
- Struct fields marked `pub` can be read and mutated by importing modules. Private fields are accessible only to the struct's own internal methods.

### 8.4 Execution Lifecycle & Entry Point
1. **Module Static Initialization:** All top-level variable initializers and statements run once per module in depth-first post-order topological dependency order.
2. **Entry Point:** Once imported modules are initialized, the compiler invokes `func main(): void` in the root entry file.

---

## 9. Collections

### 9.1 `Vec<T>`
Dynamic array container storing elements contiguously.
```rey
var nums = Vec.new();
nums.push(10);
nums.push(20);
nums.push(30);

var last = nums.pop();       // 30
var first = nums[0];         // 10
var totalLen = nums.len();   // 2

nums[1] = 99;                // In-place index assignment
```

### 9.2 `HashMap<string, V>`
Fast associative key-value map keyed by strings.
```rey
var userAges = HashMap.new();
userAges.set("Alice", 28);
userAges.set("Bob", 34);

if userAges.has("Alice") {
    var age = userAges.get("Alice");
    println("Alice is " + age.toString());
}

userAges.delete("Bob");
```

### 9.3 `Option<T>`
Type-safe container representing a value that may or may not be present:
```rey
var present: Option<int> = Option::Some(42);
var empty: Option<int> = Option::None;

if present.isSome() {
    println("Value: " + present.unwrap().toString());
}

var fallback = empty.unwrapOr(100); // Evaluates to 100
```

### 9.4 `Result<T, E>`
Type-safe container for operations that can return either success or failure:
```rey
func parsePort(s: string): Result<int, string> {
    if s == "80" { return Result::Ok(80); }
    return Result::Err("Invalid port number");
}

var res = parsePort("80");
if res.isOk() {
    println("Port: " + res.unwrap().toString());
} else {
    println("Error: " + res.unwrapErr());
}
```

### 9.5 `LinkedList`, `Stack`, `Queue`
High-level standard data structures provided by the runtime:
```rey
// Stack: LIFO container
var st = Stack.new();
st.push("first");
st.push("second");
var top = st.pop(); // "second"

// Queue: FIFO container
var q = Queue.new();
q.enqueue(1);
q.enqueue(2);
var item = q.dequeue(); // 1

// LinkedList: Doubly-linked list
var list = LinkedList.new();
list.push(10);
list.unshift(5);
```

---

## 10. Complete Standard & Built-in Function Catalog

### 10.1 Scalar Conversions
| Built-in Function | Parameter Type | Return Type | Description |
|---|---|---|---|
| `int(c)` | `char` | `int` | Converts a 1-byte char to its 0..255 unsigned integer code. |
| `char(i)` | `int` | `char` | Converts an integer in range `0..255` to its single-byte character. Exits 1 if out of range. |
| `float(x)` | `int \| float` | `float` | Converts an integer or float to 64-bit binary64 float. |
| `string(x)` | `scalar` | `string` | Converts any scalar primitive directly to its string representation. |

### 10.2 Numeric & Math Builtins
| Built-in Function | Parameter Type(s) | Return Type | Description |
|---|---|---|---|
| `abs(x)` | `int` or `float` | Same as `x` | Returns absolute value. Panics on `INT64_MIN`. |
| `min(a, b)` | Numbers | Number | Returns the smaller of two numbers. |
| `max(a, b)` | Numbers | Number | Returns the larger of two numbers. |
| `pow(base, exp)` | `(int, int)` or `(float, float)` | Number | Computes base raised to power exponent. |
| `sqrt(x)` | `float` | `float` | Square root. Exits 1 if `x < 0.0`. |
| `exp(x)` | `float` | `float` | Exponential function ($e^x$). |
| `log(x)` | `float` | `float` | Natural logarithm ($\ln x$). Exits 1 if `x <= 0.0`. |
| `sin(x)` | `float` | `float` | Sine in radians. |
| `cos(x)` | `float` | `float` | Cosine in radians. |
| `tan(x)` | `float` | `float` | Tangent in radians. |
| `floor(x)` | `float` | `int` | Greatest integer $\le x$ (rounds towards $-\infty$). |
| `ceil(x)` | `float` | `int` | Least integer $\ge x$ (rounds towards $+\infty$). |
| `round(x)` | `float` | `int` | Rounds to the nearest integer. |
| `random()` | `void` | `float` | Generates a pseudo-random float in range `[0.0, 1.0)`. |

### 10.3 Bitwise & Wrapping Arithmetic
| Built-in Function | Parameter Types | Return Type | Description |
|---|---|---|---|
| `wrapAdd(a, b)` | `(int, int)` | `int` | 64-bit integer addition with two's-complement modular wrapping. |
| `wrapSub(a, b)` | `(int, int)` | `int` | 64-bit integer subtraction with two's-complement modular wrapping. |
| `wrapMul(a, b)` | `(int, int)` | `int` | 64-bit integer multiplication with two's-complement modular wrapping. |
| `bitAnd(a, b)` | `(int, int)` | `int` | Bitwise AND (`a & b`). |
| `bitOr(a, b)` | `(int, int)` | `int` | Bitwise OR (`a \| b`). |
| `bitXor(a, b)` | `(int, int)` | `int` | Bitwise XOR (`a ^ b`). |
| `bitNot(a)` | `int` | `int` | Bitwise bitwise inversion (`~a`). |
| `bitShl(a, n)` | `(int, int)` | `int` | Bitwise shift left (`a << n`). |
| `bitShr(a, n)` | `(int, int)` | `int` | Arithmetic shift right (`a >> n`). |

### 10.4 Standard I/O & Diagnostics
| Built-in Function | Signature | Description |
|---|---|---|
| `print(...values)` | `(...any): void` | Prints values space-separated to standard output without trailing newline. |
| `println(...values)` | `(...any): void` | Prints values space-separated to standard output followed by a newline. |
| `eprint(...values)` | `(...any): void` | Prints values space-separated to standard error without trailing newline. |
| `eprintln(...values)` | `(...any): void` | Prints values space-separated to standard error followed by a newline. |
| `input(prompt?)` | `(string?): string` | Prompts user and reads a line of text from standard input. |
| `assert(cond, msg)` | `(bool, string): void` | Asserts condition is true. Panics with `msg` if false. |
| `typeof(val)` | `(any): string` | Returns the runtime type name of the given value. |

### 10.5 File System Operations
| Built-in Function | Signature | Description |
|---|---|---|
| `readFile(path)` | `(string): string` | Reads entire file contents into a string. Exits 1 on missing file. |
| `writeFile(path, data)`| `(string, string): void` | Writes string contents to file (creates or truncates). |
| `appendFile(path, data)`| `(string, string): void` | Appends string contents to an existing or new file. |
| `fileExists(path)` | `(string): bool` | Returns `true` if file exists on disk, `false` otherwise. |
| `deleteFile(path)` | `(string): void` | Deletes a file from the file system. |
| `copyFile(src, dst)` | `(string, string): void` | Copies file from `src` to `dst`. |
| `moveFile(src, dst)` | `(string, string): void` | Moves or renames file from `src` to `dst`. |
| `isFile(path)` | `(string): bool` | Returns `true` if path is a regular file. |
| `isDir(path)` | `(string): bool` | Returns `true` if path is a directory. |
| `mkdir(path)` | `(string): void` | Creates a single directory. |
| `mkdirAll(path)` | `(string): void` | Recursively creates directories along the path. |
| `removeDir(path)` | `(string): void` | Removes an empty directory. |
| `removeDirAll(path)` | `(string): void` | Recursively removes a directory and all its contents. |
| `listDir(path)` | `(string): Vec<string>`| Returns list of filenames within directory. |
| `listDirPaths(path)` | `(string): Vec<string>`| Returns list of full paths within directory. |
| `pathJoin(a, b)` | `(string, string): string` | Joins two filesystem path segments. |
| `basename(path)` | `(string): string` | Returns filename portion of a path. |
| `dirname(path)` | `(string): string` | Returns directory portion of a path. |
| `extname(path)` | `(string): string` | Returns file extension (including `.`). |

### 10.6 Process, Environment & System Execution
| Built-in Function | Signature | Description |
|---|---|---|
| `args()` | `(): Vec<string>` | Returns list of command-line arguments passed to the program. |
| `getEnv(name)` | `(string): Option<string>` | Reads an environment variable. |
| `setEnv(name, val)` | `(string, string): void` | Sets an environment variable in current process. |
| `unsetEnv(name)` | `(string): void` | Unsets an environment variable. |
| `envAll()` | `(): HashMap<string, string>` | Returns all environment variables as a key-value map. |
| `cwd()` | `(): string` | Returns current working directory. |
| `chdir(path)` | `(string): Result<string, string>`| Changes current working directory. |
| `tempDir()` | `(): string` | Returns system temporary directory path. |
| `exec(cmd)` | `(string): Result<string, string>` | Executes shell command via `/bin/sh -c` and returns stdout. |
| `execDetailed(cmd)` | `(string): ExecResult` | Executes shell command and returns struct with stdout, stderr, and exit code. |
| `which(bin)` | `(string): Option<string>` | Resolves full path to executable binary using system PATH. |
| `sleepMs(ms)` | `(int): void` | Suspends thread execution for specified milliseconds. |
| `exit(code)` | `(int): void` | Terminates process immediately with specified status code. |

---

## 11. Methods Quick Reference

### 11.1 String Methods
| Method | Returns | Description |
|---|---|---|
| `s.len()` | `int` | Returns number of bytes in string. |
| `s[i]` | `string` | Returns 1-character string at index `i`. |
| `s.slice(start, end)` | `string` | Returns half-open substring `[start, end)`. |
| `s.indexOf(sub)` | `int` | Returns first byte index of `sub`, or `-1` if absent. |
| `s.lastIndexOf(sub)` | `int` | Returns last byte index of `sub`, or `-1` if absent. |
| `s.contains(needle)` | `bool` | Returns `true` if `needle` is found in string. |
| `s.startsWith(prefix)` | `bool` | Returns `true` if string begins with `prefix`. |
| `s.endsWith(suffix)` | `bool` | Returns `true` if string ends with `suffix`. |
| `s.toUpper()` | `string` | Converts ASCII characters to uppercase. |
| `s.toLower()` | `string` | Converts ASCII characters to lowercase. |
| `s.trim()` | `string` | Strips leading and trailing ASCII whitespace. |
| `s.trimStart()` | `string` | Strips leading ASCII whitespace. |
| `s.trimEnd()` | `string` | Strips trailing ASCII whitespace. |
| `s.padLeft(w, pad)` | `string` | Pads left side to width `w` using first byte of `pad`. |
| `s.padRight(w, pad)` | `string` | Pads right side to width `w` using first byte of `pad`. |
| `s.replace(from, to)` | `string` | Replaces occurrences of `from` with `to`. |
| `s.repeat(n)` | `string` | Returns string repeated `n` times. |
| `s.split(delim)` | `Vec<string>` | Splits string by delimiter (`""` splits into individual characters). |
| `s.toInt()` | `int` | Parses ASCII decimal string to integer. Exits 1 if invalid. |
| `s.toFloat()` | `float` | Parses ASCII decimal string to float. Exits 1 if invalid. |
| `s.hash()` | `int` | Returns numeric value of first byte. |
| `s.toString()` | `string` | Identity conversion. |

### 11.2 Vec Methods
| Method | Returns | Description |
|---|---|---|
| `v.push(item)` | `void` | Appends item to end of vector. |
| `v.pop()` | `T` | Removes and returns last item. Exits 1 if empty. |
| `v.len()` | `int` | Returns number of elements in vector. |
| `v.get(i)` | `T` | Returns item at index `i` (checked bounds). |
| `v.set(i, val)` | `void` | Overwrites item at index `i` (checked bounds). |
| `v.contains(item)` | `bool` | Returns `true` if vector contains item. |
| `v.indexOf(item)` | `int` | Returns index of item, or `-1` if absent. |
| `v.insert(i, item)` | `void` | Inserts item at index `i`. |
| `v.remove(i)` | `T` | Removes and returns item at index `i`. |
| `v.clear()` | `void` | Removes all elements from vector. |
| `v.reverse()` | `void` | In-place reverses the order of elements. |
| `v.sort()` | `void` | In-place stable sort (homogeneous ints, floats, strings, bools). |
| `v.slice(start, end)`| `Vec<T>` | Returns a new sub-vector from `start` to `end`. |
| `v.join(sep)` | `string` | Joins string vector elements separated by `sep`. |
| `v.map(fn)` | `Vec<U>` | Transforms elements via mapping function `(T) -> U`. |
| `v.filter(fn)` | `Vec<T>` | Filters elements satisfying predicate `(T) -> bool`. |
| `v.reduce(fn, init)` | `U` | Folds elements using accumulator `(U, T) -> U`. |

### 11.3 HashMap Methods
| Method | Returns | Description |
|---|---|---|
| `m.set(key, val)` | `void` | Inserts or updates key-value pair. |
| `m.get(key)` | `V` | Retrieves value for key (panics if key absent). |
| `m.has(key)` | `bool` | Returns `true` if key exists in map. |
| `m.delete(key)` | `void` | Removes key and its associated value from map. |
| `m.keys()` | `Vec<string>` | Returns vector of all keys in map. |
| `m.values()` | `Vec<V>` | Returns vector of all values in map. |
| `m.entries()` | `Vec<Vec<any>>` | Returns vector of `[key, value]` pairs. |
| `m.len()` | `int` | Returns number of entries in map. |
| `m.clear()` | `void` | Removes all entries from map. |

### 11.4 Option & Result Methods
| Method | Receiver | Returns | Description |
|---|---|---|---|
| `.isSome()` | `Option<T>` | `bool` | `true` if value is `Option::Some`. |
| `.isNone()` | `Option<T>` | `bool` | `true` if value is `Option::None`. |
| `.unwrap()` | `Option<T>` | `T` | Returns payload. Panics if `None`. |
| `.unwrapOr(def)` | `Option<T>` | `T` | Returns payload if `Some`, otherwise `def`. |
| `.isOk()` | `Result<T, E>` | `bool` | `true` if value is `Result::Ok`. |
| `.isErr()` | `Result<T, E>` | `bool` | `true` if value is `Result::Err`. |
| `.unwrap()` | `Result<T, E>` | `T` | Returns Ok payload. Panics if `Err`. |
| `.unwrapErr()` | `Result<T, E>` | `E` | Returns Err payload. Panics if `Ok`. |
| `.unwrapOr(def)` | `Result<T, E>` | `T` | Returns Ok payload if `Ok`, otherwise `def`. |
| `.unwrapStr()` | `Result<string, E>` | `string` | Unwraps string Ok payload directly. |
| `.unwrapOrStr(def)` | `Result<string, E>` | `string` | Returns string Ok payload or string default. |
