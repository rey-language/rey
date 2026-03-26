# Rey Language Syntax Reference (v0.1.1)

This document reflects the behavior currently implemented in `compiler/v1`.

## Table of Contents
1. Variables and Types
2. Operators
3. Control Flow
4. Functions
5. Imports and Visibility
6. Collections
7. Strings
8. Structs
9. Enums and Match
10. Built-ins
11. Diagnostics

## Variables and Types
Rey supports explicit and inferred typing.

```rey
var x = 10;
var y: int = 20;
const pi: float = 3.14;
```

- `var` declares a mutable variable
- `const` declares an immutable variable (cannot be reassigned)

Implemented primitive type names:
- `int`
- `uint`
- `byte`
- `float`
- `double`
- `String`
- `bool`
- `char`
- `null`
- `Void`

Implemented type forms:
- Nullable: `int?`
- Array: `[int]`
- Dictionary: `{String:int}`
- Union: `int | String`

Comments:
```rey
// single-line comment
```

Tuples are supported as literals and index access:

```rey
var t = (1, "ok", true);
println(t.0);
println(t.1);
println(t.2);
```

## Operators
Arithmetic:
- `+`, `-`, `*`, `/`, `%`

Division rules:
- `int / int` performs integer division (truncates)
- any `float` operand produces a `float` result (`10.0 / 3.0`, `10 / 3.0`)

Comparison:
- `==`, `!=`, `<`, `<=`, `>`, `>=`

Logical:
- `&&`, `||`, `!`

Update / assignment:
- `=`, `+=`, `-=`, `*=`, `/=`, `%=`
- `++`, `--` (prefix and postfix on variables)

Type check:
- `instanceof`

## Control Flow
Conditionals:

```rey
if (x > 10) {
    println("big");
} else if (x > 5) {
    println("mid");
} else {
    println("small");
}
```

Parentheses around `if`/`while` conditions are optional.

Loops:
- `while`
- `loop` (infinite loop)
- `for i in range(start, end)`
- `for item in arrayExpr`

Loop control:
- `break`
- `continue`

## Functions
Function declarations:

```rey
func add(a: int, b: int): int {
    return a + b;
}
```

Default parameters:

```rey
func add(a: int, b: int = 10): int {
    return a + b;
}
```

Variadic parameter (must be last):

```rey
func sum(nums:...int): int {
    var total = 0;
    for n in nums {
        total += n;
    }
    return total;
}
```

Lambda expressions:

```rey
var mul = (x: int, y: int) => x * y;
println(mul(3, 4));
```

## Imports and Visibility
Function visibility modifiers:
- `func name()` -> private
- `pub func name()` -> public inside file/module, not importable
- `export pub func name()` -> importable

File imports:

```rey
import actuator.name;
import actuator.{name, other};
```

Module imports:

```rey
import action;
import action::walk;
import action::{walk, run};
```

Resolver order:
1. Current file directory
2. Project root (entry file directory)
3. `~/.reyc/std/src` (for `std` module resolution)
4. `~/.reyc/packages`

Module rules:
- `import module` requires `module/main.rey`
- `import module` injects a namespace object used as `module.func()`
- `import module::item` injects `item` namespace used as `item.func()`
- Only `export pub` functions are importable

Compile-time import diagnostics include:
- file not found
- missing module `main.rey`
- function not found
- function is `pub` but not `export pub`
- circular import
- duplicate import

## Collections
Arrays:

```rey
var xs: [int] = [1, 2, 3];
println(xs[0]);
xs[0] = 9;
push(xs, 4);
println(pop(xs));
println(xs.length());
```

Array methods:
- `length()`

Dictionaries (identifier or string keys in literals):

```rey
var user = {name: "Rey", id: 1};
println(user.name);
println(user["id"]);
user.name = "ReyLang";
```

Dictionary methods:
- `length()`

## Strings
Regular and multiline strings:

```rey
var a = "hello";
var b = """
line 1
line 2
""";
```

Char literals:
```rey
var c: char = 'x';
```

Interpolation:

```rey
var hp = 100;
println("HP: {hp}");
println("Buffed: {hp + 50}");
```

String methods:
- `length()`
- `upper()`
- `lower()`
- `contains(str)`
- `split(str)`
- `toString()`
- `toInt()`
- `toFloat()`

## Structs
Struct declaration:

```rey
struct Player {
    health: int,
    name: String,

    pub func create(n: String, h: int): Player {
        return Player { name: n, health: h };
    }

    pub func takeDamage(amount: int): Void {
        health -= amount;
    }
}
```

Struct literal:

```rey
var p = Player { name: "Hero", health: 100 };
```

Field mutation:
- `obj.field = value` works for `pub` struct fields
- `obj.field += value` works for `pub` struct fields
- nested field assignment like `obj.inner.field = value` is currently rejected

Implemented method behavior:
- Instance method calls inject fields into method scope by field name.
- Mutated field names are written back to the instance.
- Static calls are parsed as `StructName.method(...)`.
- In current parser behavior, methods named `create` that are `pub` and return the struct type are treated as static.

## Enums and Match
Enum declaration:

```rey
enum Direction {
    North,
    South,
    East,
    West
}
```

Match:

```rey
match dir {
    Direction::North => { println("north"); },
    Direction::South => { println("south"); },
    _ => { println("other"); }
}
```

Pattern kinds:
- enum variant (`Type::Variant` or unqualified `Variant`)
- struct pattern (`StructName { field: pattern, ... }`)
- literal (`1`, `"x"`, `true`, `null`)
- variable binding (`n`)
- wildcard (`_`)

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

## Diagnostics
Compiler and runtime errors are printed with category labels such as:
- `error[lexer]`
- `error[syntax]`
- `error[import]`
- `error[runtime]`

Parser/lexer/import errors include file/line/column spans.
