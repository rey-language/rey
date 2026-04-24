# Rey Programming Language

**Rey** is a statically-typed, compiled programming language with clean syntax, built for clarity and hackability. The compiler is written in Rey itself — it bootstraps from a Rust reference interpreter and emits LLVM IR compiled to native binaries via `clang`.

```rey
func greet(name: String): String {
    return "Hello, " + name + "!";
}

func main(): Void {
    println(greet("Rey"));
}
```

---

## Status

| Component | State |
|-----------|-------|
| Lexer | ✅ Complete |
| Parser | ✅ Complete |
| Type checker | 🚧 Stub (inference only) |
| LLVM backend | ✅ Complete |
| C runtime library | ✅ Complete |
| Self-hosting | ✅ Phase H — native compiler passes all e2e tests |

The native compiler (`rey-compiler/main.rey`) compiles to a standalone binary using `clang`. No Rust is required at runtime.

---

## Quick Start

### Prerequisites

- macOS arm64 (other platforms: build from source)
- `clang` on `PATH`

### Pre-built binary (macOS arm64)

```sh
# Use the pre-built binary from releases/
chmod +x releases/0.2.0/rey-macos-arm64

# Compile a program
./releases/0.2.0/rey-macos-arm64 rey-compiler/main.rey build examples/basic/hello.rey

# Run a program
./releases/0.2.0/rey-macos-arm64 rey-compiler/main.rey run examples/basic/hello.rey
```

### Build from source

```sh
# 1. Build the Rust bootstrap interpreter
cd compiler/v1 && cargo build --release && cd ../..

# 2. Compile the Rey compiler using the bootstrap
./compiler/v1/target/release/rey-v0 rey-compiler/main.rey build rey-compiler/main.rey

# 3. Sign the output (macOS)
codesign -s - rey-compiler/main.bin

# 4. Run a Rey program with the native compiler
./rey-compiler/main.bin rey-compiler/main.rey run examples/basic/hello.rey
```

---

## Language Features

### Variables and constants

```rey
var x = 42;
var name: String = "Rey";
const PI: float = 3.14159;
```

### Functions and lambdas

```rey
func add(a: int, b: int): int {
    return a + b;
}

var double = (x: int) => x * 2;
```

### Control flow

```rey
if x > 0 { println("positive"); } else { println("non-positive"); }

while i < 10 { i = i + 1; }

loop { if done { break; } }

for item in [1, 2, 3] { println(item.toString()); }
```

### Structs

```rey
struct Point {
    pub x: int,
    pub y: int,
}

var p = Point { x: 3, y: 4 };
```

### Enums and match

```rey
enum Color { Red, Green, Blue }

match color {
    Color.Red   => println("red"),
    Color.Green => println("green"),
    _           => println("other"),
}
```

### Collections

```rey
var v = Vec.new();
v.push("a");
v.push("b");
println(v.join(", "));   // a, b

var m = HashMap.new();
m.set("count", 42);
println(m.get("count").toString());
```

### Result and Option

```rey
var r = readFile("data.txt");
if r.isOk() {
    println(r.unwrap());
} else {
    println("error: " + r.unwrapOr("unknown"));
}
```

### Imports

```rey
import lexer;
import parser;

var tokens = lexer.tokenize(source, "file.rey");
var ast    = parser.parse(tokens, "file.rey");
```

---

## Repository Layout

```
rey-lang/
├── compiler/v1/          # Rust bootstrap interpreter (not needed at runtime)
│   └── src/              # Lexer, parser, tree-walk interpreter
├── rey-compiler/         # Self-hosted LLVM compiler (written in Rey)
│   ├── main.rey          # Compiler entry point and pipeline
│   ├── src/
│   │   ├── lexer/        # Tokenizer
│   │   ├── parser/       # Recursive-descent parser + AST definitions
│   │   ├── typecheck/    # Type checker (stub)
│   │   └── codegen/      # LLVM IR emitter
│   ├── runtime/          # C runtime library (librey_rt)
│   │   ├── rey_rt.h      # Public ABI header
│   │   ├── string.c      # String operations
│   │   ├── vec.c         # Dynamic arrays
│   │   ├── hashmap.c     # Hash maps
│   │   ├── result.c      # Result/Option/instanceof
│   │   ├── io.c          # File I/O and process
│   │   └── mem.c         # Allocator
│   └── tests/e2e/        # End-to-end test suite
├── releases/             # Pre-built binaries (by version)
├── examples/             # Example Rey programs
├── spec/                 # Language specification
└── tests/                # Interpreter-level test fixtures
```

---

## Testing

```sh
# Build runtime first (only needed once)
make -C rey-compiler/runtime

# Run all e2e tests with the native compiler
bash rey-compiler/tests/e2e/run.sh
```

Expected output: `OK`

---

## Docs

- [`ARCHITECTURE.md`](ARCHITECTURE.md) — compiler pipeline and design
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — style guide and how to extend the language
- [`ROADMAP.md`](ROADMAP.md) — future directions and known limitations
- [`releases/0.2.0/RELEASE.md`](releases/0.2.0/RELEASE.md) — v0.2.0 release notes

---

## License

MIT — see [`LICENSE`](LICENSE).
