# Rey Language Syntax — v0

Reference documentation for implemented syntax and features in the Rey v0 interpreter.

---

## Table of Contents

1. [Variables](#variables)
2. [Data Types](#data-types)
3. [Operators](#operators)
4. [Control Flow](#control-flow)
5. [Functions](#functions)
6. [Builtins](#builtins)
7. [Program Structure](#program-structure)

---

## Variables

### Declaration

Variables are declared using the `var` keyword. All variables must be declared before use.

```rey
var x = 10;           // untyped (type inferred)
var name = "Rey";     // untyped
var flag = true;      // untyped
```

### Type Annotations

Variables MAY declare a type using `: type`. Typed variables MUST receive compatible values on reassignment.

```rey
var i: int = 5;       // typed as int
var f: float = 3.14;  // typed as float
var s: String = "hello";  // typed as String
var b: bool = false;  // typed as bool
```

### Reassignment

Variables can be reassigned using `=`. The new value MUST match the declared type if one was specified.

```rey
var x = 10;
x = 20;               // OK - same type

var typed: int = 42;
typed = 100;          // OK - int matches int
```

---

## Data Types

### Core Types

| Type | Example | Description |
|------|---------|-------------|
| `int` | `42`, `-10` | Integer numbers |
| `float` | `3.14`, `-0.5` | Floating-point numbers |
| `String` | `"hello"` | String literals |
| `bool` | `true`, `false` | Boolean values |
| `null` | `null` | Null value |
| `Void` | `Void` | Function return type (no value) |

### Collection Types

Arrays:

```rey
var xs = [1, 2, 3];
var ys: [int] = [1, 2, 3];
println(xs[0]);
```

Dictionaries (string-keyed):

```rey
var d = {name: "Rey", id: 42};
var typed: {String:int} = {"a": 1, "b": 2};
println(d["name"]);
println(d.name);
```

### Type Inference

When no type annotation is provided, the type is inferred from the initializer:

```rey
var x = 10;           // inferred as int
var f = 3.14;         // inferred as float
var s = "text";       // inferred as String
var b = true;         // inferred as bool
```

---

## Operators

### Arithmetic Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `+` | Addition | `a + b` |
| `-` | Subtraction | `a - b` |
| `*` | Multiplication | `a * b` |
| `/` | Division | `a / b` |

### Comparison Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `==` | Equal | `a == b` |
| `!=` | Not equal | `a != b` |
| `<` | Less than | `a < b` |
| `<=` | Less than or equal | `a <= b` |
| `>` | Greater than | `a > b` |
| `>=` | Greater than or equal | `a >= b` |

### Logical Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `&&` | Logical AND | `a && b` |
| `||` | Logical OR | `a || b` |

### Unary Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `-` | Negation | `-x` |
| `!` | Logical NOT | `!flag` |

### Assignment Operator

| Operator | Description | Example |
|----------|-------------|---------|
| `=` | Simple assignment | `x = 5` |

---

## Control Flow

### If/Else

Conditional execution using `if` and `else`. Parentheses are **required** around the condition.

```rey
if (condition) {
    // executed if condition is true
} else {
    // executed if condition is false
}
```

Example:

```rey
if (x > 10) {
    println("big");
} else {
    println("small");
}
```

### While Loops

While loops repeat while the condition is true. Parentheses are **required** around the condition.

```rey
while (condition) {
    // loop body
}
```

Example:

```rey
var i = 0;
while (i < 10) {
    println(i);
    i = i + 1;
}
```

### For Loops

For loops iterate over a range:

```rey
for x in range(start, end) {
    // loop body
}
```

The `range` function produces values from `start` (inclusive) to `end` (exclusive).

### Break and Continue

```rey
while (true) {
    if (done) {
        break;      // exit loop
    }
    if (skip) {
        continue;   // next iteration
    }
}
```

---

## Functions

### Declaration

Functions are declared using the `func` keyword.

```rey
func name(parameters) : returnType {
    body
}
```

### Parameters

Parameters MAY be typed. Untyped parameters accept any value.

```rey
// untyped parameter
func echo(x) {
    return x;
}

// typed parameters
func add(a: int, b: int): int {
    return a + b;
}
```

### Return Types

Functions MAY declare a return type. If declared, all return paths MUST return a compatible value.

```rey
func greet(name: String): String {
    return name;
}

func compute(): Void {
    return;     // or omit return entirely
}
```

### Calling Functions

```rey
func main(): Void {
    var result = add(2, 3);
    println(result);
}
```

---

## Builtins

### `println`

Print a value to stdout.

```rey
println("Hello, World!");
println(42);
println(true);
```

### `len`

Get the length of a string, array, or dictionary.

```rey
println(len("abc"));
println(len([1, 2, 3]));
println(len({a: 1, b: 2}));
```

### `push` / `pop`

Mutate arrays.

```rey
var xs: [int] = [1, 2];
push(xs, 3);
println(pop(xs));
```

### `input`

Read a line from stdin (optionally with a prompt).

```rey
var name = input("Enter name: ");
println(name);
```

---

## String Methods

Supported methods on `String`:

```rey
var s: String = "Rey Language";
println(s.length());
println(s.upper());
println(s.lower());
println(s.contains("Lang"));
println(s.split(" ")[0]);
```

---

## Program Structure

### Entry Point

Programs start executing from the `main` function:

```rey
func main(): Void {
    // program entry point
}
```

### Complete Example

```rey
// Variable and function example

func add(a: int, b: int): int {
    return a + b;
}

func main(): Void {
    var x = 10;
    var y = 20;
    var sum = add(x, y);
    println(sum);     // prints 30
}
```

### Fibonacci Example

```rey
func fib(n: int): int {
    if (n < 2) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

func main(): Void {
    var i: int = 0;
    while (i < 10) {
        println(fib(i));
        i = i + 1;
    }
}
```

---

## Not Yet Implemented

The following features are NOT implemented:

| Feature | Status |
|---------|--------|
| Modulo operator (`%`) | Lexer token exists but parser doesn't use it |
| Compound assignment (`+=`, `-=`, etc) | Not implemented |
| Increment/decrement (`++`, `--`) | Not implemented |

---

## Running Programs

Build and run using Cargo:

```bash
cd compiler/v1
cargo build --release
./target/release/rey-v0 <path-to-.rey-file>
```

Or use `cargo run`:

```bash
cd compiler/v1
cargo run -- ../src/tests/variables.rey
```
