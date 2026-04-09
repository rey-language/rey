# Rey Language Syntax Reference (v0.2.0)

This document describes the syntax and runtime behavior currently implemented by the Rust reference interpreter at `compiler/v1`.

Notes:
- Rey supports optional type annotations. When present, they are enforced by the interpreter’s typechecker.
- Some type annotations (notably generic type params like `Vec<String>`) are accepted but **erased** at runtime.
- The bootstrap compiler under `rey-compiler/` is experimental and currently supports only a small subset for native codegen (see “Bootstrap Compiler Subset”).

## Contents
- Lexical structure
- Types
- Variables
- Operators
- Control flow
- Functions and lambdas
- Collections
- Strings
- Structs
- Enums and match
- Imports and visibility
- Builtins
- Diagnostics
- Bootstrap compiler subset

## Lexical structure

### Comments
```rey
// single-line comment
```

### Strings
```rey
var a = "hello";
var b = """
line 1
line 2
""";
```

### Chars
```rey
var c: char = 'x';
```

## Types

### Primitive type names
- `int` (i64)
- `float` / `double` (f64)
- `bool`
- `String`
- `char`
- `Void`
- `null` (literal)

Additional numeric type names may parse in annotations, but `int`/`float`/`double` are the core runtime numeric kinds.

### Type forms
- Nullable: `T?` (ex: `int?`)
- Arrays: `[T]` (ex: `[int]`)
- Dict type tags: `{K:V}` (ex: `{String:int}`) — used mainly for annotations
- Union: `A | B` (ex: `int | String`)
- Tuples: `Tuple` (runtime tuples exist; tuple typing is dynamic)

### Generic type annotations (erased)
Generic parameter syntax is accepted in type positions and ignored:
```rey
struct LexerState {
    pub tokens: Vec<Token>;
    pub map: HashMap<String, Vec<int>>;
}

func f(xs: Vec<String>): Vec<int> {
    return [];
}
```

The runtime collections (`Vec`, `HashMap`, etc.) use dynamic dispatch; type parameters are not represented at runtime.

## Variables

```rey
var x = 10;
var y: int = 20;
const pi: float = 3.14;
```

- `var` declares a mutable binding
- `const` declares an immutable binding (cannot be reassigned)
- `const` works at global scope and inside functions

## Operators

### Arithmetic
- `+`, `-`, `*`, `/`, `%`

Division rules:
- `int / int` performs integer division (truncates)
- any float operand produces a float result (`10.0 / 3.0`, `10 / 3.0`)

### Comparison
- `==`, `!=`, `<`, `<=`, `>`, `>=`

Strings support lexicographic comparisons.

### Logical
- `&&`, `||`, `!`

### Assignment / update
- `=`, `+=`, `-=`, `*=`, `/=`, `%=`
- `++`, `--` (prefix/postfix on variables)

### Type check
- `instanceof`

```rey
if (value instanceof String) {
    println("got a string");
}
```

## Control flow

### if / else if / else
Parentheses around conditions are optional.

```rey
if x > 10 {
    println("big");
} else if (x > 5) {
    println("mid");
} else {
    println("small");
}
```

### Loops
- `while`
- `loop` (infinite loop)
- `for n in range(start, end)`
- `for item in arrayExpr`

Loop control:
- `break`
- `continue`

## Functions and lambdas

### Declarations
```rey
func add(a: int, b: int): int {
    return a + b;
}
```

### Default parameters
```rey
func add(a: int, b: int = 10): int {
    return a + b;
}
```

### Variadics
```rey
func sum(nums:...int): int {
    var total = 0;
    for n in nums {
        total += n;
    }
    return total;
}
```

### Lambdas
```rey
var mul = (x: int, y: int) => x * y;
println(mul(3, 4));
```

## Collections

### Arrays (`[T]`)
```rey
var xs: [int] = [1, 2, 3];
println(xs[0]);
xs[0] = 9;
```

Array functions:
- `len(xs)`
- `push(xs, v)`
- `pop(xs)`

Array methods:
- `xs.length()`
- `xs.push(v)`

### Dicts (dynamic)
Dict literals accept identifier keys and string keys:
```rey
var user = {name: "Rey", "id": 1};
println(user.name);
println(user["id"]);
user.name = "ReyLang";
```

### Vec
`Vec` is a builtin dynamic collection type:
```rey
var v = Vec.new();
v.push(1);
v.push(2);
println(v.len());
```

Methods:
- `push(value)`
- `pop()`
- `len()`
- `get(index)`
- `set(index, value)`
- `contains(value)`
- `indexOf(value)`
- `map(lambda)`
- `filter(lambda)`
- `reduce(lambda, init)`
- `reverse()` (in-place)
- `sort()` (in-place; comparable types)
- `slice(start, end)`
- `join(separator)`

### HashMap
`HashMap` is a builtin key/value map with string keys:
```rey
var m = HashMap.new();
m.set("a", 1);
println(m.get("a"));
```

Methods:
- `set(key: String, value)`
- `get(key: String)`
- `delete(key: String)`
- `has(key: String)`
- `len()`
- `keys()` -> `Vec`
- `values()` -> `Vec`
- `entries()` -> `Vec` of `[key, value]` pairs

## Strings

### Concatenation and interpolation
```rey
var name = "Rey";
println("hello " + name);
println("hello {name}");
```

### Indexing
`s[i]` returns a 1-character `String`.

### Methods
- `length()` / `len()`
- `upper()`
- `lower()`
- `trim()`
- `contains(sub: String)` -> `bool`
- `split(delim: String)` -> `[String]`
- `startsWith(prefix: String)` -> `bool`
- `endsWith(suffix: String)` -> `bool`
- `replace(from: String, to: String)` -> `String`
- `slice(start: int, end: int)` -> `String`
- `indexOf(sub: String)` -> `int` (returns `-1` if not found)
- `repeat(n: int)` -> `String`
- `padLeft(width: int, char: String)` -> `String`
- `padRight(width: int, char: String)` -> `String`
- Conversions:
  - `toString()`
  - `toInt()`
  - `toFloat()`

## Structs

### Declaration and fields
```rey
struct Player {
    pub health: int,
    pub name: String,
}
```

Fields are private by default. Prefix a field with `pub` to allow external mutation.

### Struct literals
```rey
var p = Player { name: "Hero", health: 100 };
```

### Methods
```rey
struct Player {
    pub health: int,
    pub name: String,

    pub func takeDamage(amount: int): Void {
        health -= amount;
    }
}
```

Runtime behavior:
- Field names are in scope directly inside instance methods (no `self.` required).
- Mutations to fields inside methods are written back to the instance.
- `StructName.method(...)` is used for static-style calls (constructor-style helpers are commonly named `create`).

Limitations:
- Nested field assignment like `obj.inner.field = ...` is currently rejected (you must assign through temporaries).

## Enums and match

### Enums
```rey
enum Direction {
    North,
    South,
    East,
    West
}
```

### Match
```rey
match dir {
    Direction::North => { println("north"); },
    Direction::South => { println("south"); },
    _ => { println("other"); }
}
```

Supported pattern kinds:
- Enum variant (`Type::Variant` or unqualified `Variant`)
- Struct pattern (`StructName { field: pattern, ... }`)
- Literals (`1`, `"x"`, `true`, `null`)
- Variable binding (`n`)
- Wildcard (`_`)
- Or patterns (`A | B`)

## Imports and visibility

### Visibility modifiers
- `func name()` -> private
- `pub func name()` -> public inside file/module, not importable
- `export pub func name()` -> importable

### Import forms
File imports:
```rey
import file.symbol;
import file.{symbolA, symbolB};
```

Module imports:
```rey
import module;
import module::file;
import module::{fileA, fileB};
```

Resolver order:
1. Current file directory
2. Project root (entry file directory)
3. `~/.reyc/std/src` (for `std` module resolution)
4. `~/.reyc/packages`

Diagnostics include missing files/modules/symbols, non-exported symbols, circular imports, and duplicates.

## Builtins

### IO / process
- `print(...)`
- `println(...)`
- `input()` / `input(prompt: String)`
- `args()` -> `Vec` of CLI args
- `exit(code: int)`
- `exec(cmd: String)` -> `Result` (see standard library notes)

### Filesystem / environment
- `readFile(path: String)` -> `String`
- `writeFile(path: String, content: String)`
- `appendFile(path: String, content: String)`
- `fileExists(path: String)` -> `bool`
- `deleteFile(path: String)`
- `mkdir(path: String)`
- `listDir(path: String)` -> `Vec`
- `getEnv(name: String)` -> `String?`

### Assertions
```rey
assert(x > 0, "x must be > 0");
```

If the condition is false, the interpreter prints an `error[assert]` with a line number and exits with code 1.

### Utility
- `len(value)` for strings, arrays, dicts
- `push(array, value)`
- `pop(array)`
- `abs(number)`
- `max(a, b)`
- `min(a, b)`
- `random()` -> float in `[0,1)`
- `range(start, end)` (used in `for` loops)

Math:
- `floor(n)`
- `ceil(n)`
- `round(n)`
- `sqrt(n)`
- `pow(base, exp)`
- `log(n)`
- `sin(n)`, `cos(n)`, `tan(n)`

## Diagnostics

Error categories include:
- `error[lexer]`
- `error[syntax]`
- `error[type]`
- `error[import]`
- `error[runtime]`
- `error[assert]`

Most errors include file/line/column spans. Method calls on `null` error explicitly and include a line number.

## Bootstrap compiler subset (native codegen)

The experimental bootstrap compiler in `rey-compiler/` can compile a limited subset of Rey to a native binary for the e2e fixtures in `rey-compiler/tests/e2e/`.

Supported (current minimum):
- `func` declarations, parameters, recursion (int-only calling convention for now)
- `var`/`const` for `int`/`String` in simple cases
- Integer arithmetic and comparisons
- `println(int)` and `println(String)` via `printf`
- `if/else`
- `while`, `loop`, `break`, `continue`
- Minimal `for (x in [..])` by unrolling literal arrays
- Minimal structs (int fields), struct literals, field access
- Minimal enums as integer tags (`Enum.Variant`)
- Minimal string-literal `import "path.rey";` via AST merge

Run the current e2e suite:
```bash
rey-compiler/tests/e2e/run.sh
```
