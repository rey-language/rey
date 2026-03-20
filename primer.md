# Primer — rey-lang
Last updated: Mar 19, 2026 (session end)

## What this project is
Rey is a custom programming language built by Misbah. Currently on v0 — a tree-walking interpreter written in Rust. The language has C-like syntax with type inference, functions, control flow, and basic builtins. v0 is the working prototype; future versions will likely move toward compilation.

## Key architecture
```
compiler/v1/src/
├── lexer/        — tokenizer (cursor, token, span, error)
├── parser/       — produces AST (parser.rs, error.rs)
├── ast/          — AST node types (expr, stmt, literal, ty)
└── interpreter/  — tree-walker (evaluator, executor, environment, value, function, std, control_flow)
```
Pipeline: source → lexer → tokens → parser → AST → interpreter → output

## Build & run
```bash
cd compiler/v1
cargo build --release
./target/release/rey-v0 <file>.rey

cargo run -- src/tests/variables.rey
```

## What's implemented in v0

### Variables & Types
- Variables with optional type annotations (`var x = 10`, `var x: int = 10`)
- Immutable variables with const (`const pi: float = 3.14`)
- Unannotated variables use dynamic typing (`Ty::Any`)
- Types: `int`, `float`, `String`, `bool`, `char`, `null`, `Void`
- Additional numeric types: `uint`, `double`, `byte`
- Nullable types (`int?`) for null safety

### Operators
- Arithmetic: `+`, `-`, `*`, `/`, `%` (modulo)
- Comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`
- Logical: `&&`, `||`, `!`
- Assignment: `=`, `+=`, `-=`, `*=`, `/=`, `%=`
- Increment/decrement: `++`, `--` (prefix and postfix)
- `instanceof` for type checking

### Control Flow
- `if`/`else if`/`else` — conditional branching
- `while` — standard while loop
- `loop` — infinite loop with `break`
- `for x in range(start, end)` — range iteration
- `for x in array` — array element iteration
- `break`, `continue` — loop control
- `match` — pattern matching on enums and primitives

### Functions
- Function declarations with optional typed params and return types
- Default parameter values
- Variadic parameters
- Lambda expressions: `(x: int, y: int) => x + y`
- First-class functions (functions as values)

### Data Structures
- **Arrays**: literals `[1, 2, 3]`, indexing `arr[0]`, typed arrays `[int]`
  - Methods: `length()`, `push()`, `pop()`
- **Dictionaries**: literals `{"key": value}`, indexing `dict["key"]`
  - Typed dicts: `{String:int}`
- **Tuples**: literals `(1, "a", true)`, access via index `.0`, `.1`
- **Structs**: definitions, literals, static/instance methods
  - Field scoping (fields act as method's "global scope")
  - `pub`/`private` visibility for methods

### Enums
```rey
enum Direction {
    North,
    South,
    East,
    West
}

var dir = North;  // Enum variants are automatically defined
match dir {
    Direction::North => print("Going north"),
    Direction::South => print("Going south"),
    _ => print("Unknown")
}
```

### Strings
- String interpolation: `"Value: {var}"`
- Mixed concatenation: `"HP: " + 10`
- Methods: `length()`, `upper()`, `lower()`, `contains()`, `split()`, `toString()`, `toInt()`, `toFloat()`

### Builtins
- `print()`, `println()` — output
- `len()` — length of arrays/strings
- `push()`, `pop()` — array operations
- `input()` — read user input
- `abs()`, `max()`, `min()` — math functions
- `random()` — random number generation

### Error Handling
- Compile-time type checking for annotated vars/functions
- Rust/Miette-like visual error diagnostics
- Runtime error messages with context

### Entry Point
- If `main()` function exists, it's called automatically

## Test files
`tests/` directory contains `.rey` files for each feature:
- `field_assign.rey` — external field assignment
- `array_index_assign.rey` — array index assignment
- `int_div.rey` — integer division
- `loop.rey` — infinite loop keyword
- `for_in_array.rey` — for-in array iteration
- `enum_match.rey` — enums and match statements

Run any test: `cargo run -- ../../tests/<file>.rey`

## Current status
`rey v0.0.7-pre` complete on `codex` branch:
- All v0.0.7 features implemented
- Bugs fixed: else if chaining, field assignment, array index assignment, integer division
- New features: `loop` keyword, `for x in array`, enums, match statements

## Next up (future releases)
- Build and package binaries for macOS (arm64) and Windows (x86_64)
- Add null safety: `null` comparisons and clean error on `null` access
- Add `try`/`catch` error handling
- Update `syntax.md` documentation
