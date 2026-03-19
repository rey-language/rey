# Rey Language Syntax Reference

Welcome to the comprehensive guide for the **Rey Language (v1)**.

---

## Table of Contents

1. [Variables & Types](#variables--types)
2. [Null Safety](#null-safety)
3. [Operators](#operators)
4. [Control Flow](#control-flow)
5. [Functions](#functions)
6. [Collections (Arrays & Dicts)](#collections-arrays--dicts)
7. [Strings (Interpolation, Multiline, Methods)](#strings)
8. [Structs](#structs)
9. [Built-in Functions](#built-in-functions)
10. [Error Diagnostics](#error-diagnostics)

---

## Variables & Types

### Core Types

Rey heavily leverages implicit type inference but supports full explicit typing.

| Type | Example | Description |
|------|---------|-------------|
| `int` | `42`, `-10` | Integer numbers |
| `float` | `3.14`, `-0.5` | Floating-point numbers |
| `String` | `"hello"` | String literals |
| `bool` | `true`, `false` | Boolean values |
| `null` | `null` | Nullable default |
| `Void` | `Void` | Function return type (no value) |
| `[T]` | `[1, 2]` | Array of type `T` |
| `{K:V}` | `{"a": 1}` | Dictionary mapping keys to values |

### Variable Declaration

Variables are defined using the `var` keyword. Unannotated `var` declarations are **Dynamically Typed** and can change their type later, while annotated ones are strictly typed to their annotation. You can also declare immutable constants using `const`.

```rey
// Dynamically typed (can mutate types later)
var x = 10;           
x = "Now I'm a string!"; 

// Explicitly strictly typed (cannot change type later)
var id: int = 1234;
// id = "string"; // ❌ Type error

// Immutable Constants (cannot be reassigned)
const MAX_LIVES = 3;
// MAX_LIVES = 5; // ❌ Type error
const GRAVITY: float = 9.81;
```

### Type Conversion Methods

You can explicitly convert basic types using methods:
- `.toString()`: Converts any value into a string.
- `.toInt()`: Attempts to convert a numeric string or float into an integer.
- `.toFloat()`: Attempts to convert a numeric string or int into a float.

```rey
var strAge = (25).toString();    // "25"
var actualAge = "30".toInt();    // 30
var piVal = "3.1415".toFloat();  // 3.1415
```

---

## Null Safety

Rey incorporates native Null Safety by explicitly distinguishing between types that can hold `null` values versus those that cannot. Standard types (`String`, `int`) **cannot** be assigned `null`. 

To allow a variable to be `null`, append a `?` to its type annotation:

```rey
// Standard restricted types:
// var msg: String = null; // ❌ Type error

// Nullable Types:
var name: String? = null;  // ✅ Valid
name = "Rey";

var count: int? = null;
count = 50;
```

---

## Operators

Rey comes full-featured with standard arithmetic, logic, comparison, and unary operations.

### Arithmetic & Assignment

| Operator | Description | Sub-types | Example |
|----------|-------------|-----------|---------|
| `+` | Addition / Concat | `+=` (compound) | `a + b` / `a += 5` |
| `-` | Subtraction | `-=` (compound) | `a - b` / `a -= 5` |
| `*` | Multiplication | `*=` (compound) | `a * b` / `a *= 2` |
| `/` | Division | `/=` (compound) | `a / b` / `a /= 2` |
| `%` | Modulo | `%=` (compound) | `a % 2` / `a %= 2` |
| `++` / `--` | Increment / Decrement | — | `x++` / `--y` |

> *Note: Mixed-type String concatenation is fully supported! (`"HP: " + 100` compiles to `"HP: 100"`).*

### Comparisons & Logical Operations

| Operator | Action | Example | Logic | Action | Example |
|----------|--------|---------|-------|--------|---------|
| `==` | Equality | `a == b` | `&&` | AND | `a && b` |
| `!=` | Inequality | `a != b` | `\|\|` | OR | `a \|\| b` |
| `<` | Less | `a < b` | `!` | NOT | `!a` |
| `<=` | Less/Eq | `a <= b` | `-` | Negate | `-a` |
| `>` | Greater | `a > b` |
| `>=` | Greater/Eq | `a >= b` |

---

## Control Flow

### If / Else
Standard branching logic. Conditions **must** be wrapped in parentheses.

```rey
var score = 85;

if (score >= 90) {
    println("A");
} else if (score >= 80) {
    println("B");
} else {
    println("C");
}
```

### While Loop

Repeats execution as long as the condition evaluates to `true`:

```rey
var i = 0;
while (i < 5) {
    println("Current: ", i);
    i++;
}
```

### For Loop

Iterates over a predefined sequence using the `range(inclusive, exclusive)` generator:

```rey
for index in range(0, 10) {
    println(index); // Prints 0 through 9
}
```

### Break and Continue

Interrupt or skip loop iterations easily:

```rey
for n in range(0, 100) {
    if (n % 2 == 0) {
        continue; // Skip even numbers
    }
    if (n > 50) {
        break; // Stop completely after 50
    }
    println(n);
}
```

---

## Functions

Functions are defined using the `func` keyword. Parameters and return types can be typed or left implicitly untyped (`Any`).

```rey
// Fully typed function
func calculateDamage(base: int, multiplier: float): float {
    return base * multiplier;
}

// Implicit Void return type and 'Any' parameters
func greet(name) {
    println("Hello, " + name + "!");
}

func main(): Void {
    greet("Wizard");
    var dmg = calculateDamage(15, 1.5);
}
```

---

## Collections (Arrays & Dicts)

### Arrays

Arrays are defined using square brackets `[]`.

```rey
var items = [10, 20, 30];               // Untyped inference
var names: [String] = ["Goblin", "Orc"]; // Strictly typed Array

println(items[0]);                      // Retrieval

// Push and Pop builtin operations
push(items, 40);                        // Appends 40
var lastElement = pop(items);           // Stores 40 and removes it
```

### Dictionaries

Dictionaries define string-keyed property objects. Elements can be retrieved via indices or dynamic property access.

```rey
var player = {"hp": 100, "name": "Hero"};
var strictDict: {String:int} = {"gold": 50};

println(player["hp"]);    // Index bracket notation
println(player.name);     // Shorthand dot notation
```

---

## Strings

### Multiline Strings
Rey supports standard C-style strings via `""`, but also robust multiline strings wrapping content in `""" """`:

```rey
var query = """
SELECT * 
FROM users 
WHERE active = true;
""";
```

### String Interpolation
Rey natively resolves dynamically bound variables directly inside string text via brackets `{}`:

```rey
var playerName = "Mage";
var hp = 100;

// Variables injected automatically!
println("Welcome {playerName}! Your HP is: {hp}");

// Math expressions are also supported inline:
println("HP buff: {hp + 50}");
```

### String Methods
Strings come with comprehensive native methods (Note: `length()`, not `len()` for strings!).

```rey
var msg = " Rey Language ";

println(msg.length());       // Returns character count
println(msg.upper());        // -> " REY LANGUAGE "
println(msg.lower());        // -> " rey language "
println(msg.contains("ey")); // -> true
println(msg.split(" ")[1]);  // -> "Rey"
```

---

## Built-in Functions

In addition to collection modifications, Rey provides several global functions natively inside the interpreter context.

### Standard Evaluators
- `print(args...)`: Prints multiple args natively without a trailing newline.
- `println(args...)`: Prints multiple args terminating with a newline.
- `input(prompt)`: Blocks terminal execution and reads string input from the user.
- `len(target)`: General length evaluator that works for Arrays, Strings, and Dicts.

```rey
print("Connecting");
println("...", "Done!");
var entry = input("Confirm? (y/n): ");
var size = len([1, 2, 3]);
```

### Temporary Math Utilities
Rey dynamically ships with math constants. 
*Note: These will eventually be migrated to an isolated `std` package module.*
- `abs(num)`: Returns absolute integer/float.
- `max(a, b)`: Returns greater value.
- `min(a, b)`: Returns smaller value.
- `random()`: Automatically creates a highly precise randomized fraction between `0.00` and `0.99`.

---

## Structs

Structs are the primary way to define custom data structures and behavior in Rey. They support fields, methods (instance and static), and a unique scoping model.

### Declaration

Structs are declared using the `struct` keyword. Fields are declared with `name: type`. Methods are declared with `func`. By default, fields and methods are **private**. Use the `pub` keyword to make them accessible from outside the struct.

```rey
struct Player {
    health: int,
    name: String,

    // Static method (returns the struct type)
    pub func create(n: String, h: int): Player {
        return Player { name: n, health: h };
    }

    // Instance method
    // Note: fields are accessed directly by name!
    pub func takeDamage(amount: int): Void {
        health -= amount;
        println("{name} took {amount} damage. HP: {health}");
    }
}
```

### Construction

Structs are instantiated using a literal syntax `StructName { field: value, ... }`.

```rey
var p = Player { name: "Hero", health: 100 };
```

### Methods & Scoping

- **Instance Methods**: When a method is called on an instance (`p.takeDamage(10)`), the struct's fields are injected into the method's local scope. You access them directly by their name (e.g., `health`). Any mutations to these variables are written back to the instance after the method finishes.
- **Static Methods**: Methods that return the struct type and are marked `pub` can be called directly on the struct name (e.g., `Player.create("Hero", 100)`).
- **Visibility**: Only `pub` fields and methods can be accessed via dot notation from outside.

```rey
var p = Player.create("Hero", 100);
p.takeDamage(20);
println(p.health); // Accessing pub field
```

---

## Error Diagnostics

Rey leverages visually stunning Rust/Miette-like compiler diagnostics! Gone are the days of parsing confusing log stacks!

If you write malformed code (such as leaving a string literal unterminated or throwing syntactical bugs), the compiler will extract exactly what happened, and highlight the faulty column ranges actively in your console:

```text
error[lexer]: Unterminated string literal
 --> line 3:19
  |
3 |     var message = "Hello, world
  |                   ^^^^^^^^^^^^^

error[syntax]: Expected ';' after expression.
 --> line 39:24
  |
39 |     var playerAtk: int = 15;0
   |                        ^
```
