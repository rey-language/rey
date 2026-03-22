# Rey

> Rust power without Rust complexity.

Rey is a fast, modern programming language designed to be powerful without being painful. Write clean, readable code that compiles to blazing-fast native binaries — without fighting a borrow checker or managing memory manually.

---

## Why Rey?

Every language makes a tradeoff:

- **Python** — readable and simple, but slow
- **Rust** — incredibly fast and safe, but complex
- **Go** — simple and fast, but limited
- **C** — bare metal speed, but dangerous at scale

Rey bets that you shouldn't have to choose. The compiler handles the hard parts so you don't have to.

---

## What it looks like

```rey
struct Player {
    health: int,
    name: String,

    pub func create(n: String, h: int): Player {
        return Player { name: n, health: h };
    }

    pub func takeDamage(amount: int): Void {
        health -= amount;
        println("{name} took {amount} damage. HP: {health}");
    }

    pub func isAlive(): bool {
        return health > 0;
    }
}

func main(): Void {
    var p = Player.create("Misbah", 100);
    p.takeDamage(30);
    println(p.isAlive());
}
```

---

## Features

- **Clean syntax** — readable, minimal ceremony
- **Strong type system** — optional annotations, enforced when specified
- **Structs with methods** — data and behavior together, no inheritance complexity
- **Smart memory** — the compiler manages memory, you don't
- **Multidomain** — one language for web, CLI, games, AI, systems
- **reyc** — one tool for everything (build, run, package management)

---

## Status

Rey is under active development. Current version: **v0.1.0**

| Feature | Status |
|---------|--------|
| Variables & types | ✅ Done |
| Control flow | ✅ Done |
| Functions | ✅ Done |
| Arrays & Dicts | ✅ Done |
| Structs | ✅ Done |
| String interpolation | ✅ Done |
| Error messages | ✅ Done |
| Import system | ✅ Done |
| Standard library | 🔨 In progress |
| Package manager (reyc) | 📅 Planned |
| LLVM backend | 📅 Planned |
| Self-hosting compiler | 📅 Planned |

---

## Getting started

### Install

Download the latest binary from [releases](./releases):

**macOS (Apple Silicon)**
```bash
curl -L https://github.com/rey-language/rey/releases/latest/download/rey-v0-macos-arm64 -o rey
chmod +x rey
sudo mv rey /usr/local/bin/rey
```

**Windows (x86_64)**
Download `rey-v0-windows-x86_64.exe` from releases and add to PATH.

### Run a file

```bash
rey hello.rey
```

### Write your first program

```rey
func main(): Void {
    println("hello from rey!");
}
```

---

## Building from source

Requires Rust and Cargo.

```bash
git clone https://github.com/rey-language/rey.git
cd rey/compiler/v1
cargo build --release
./target/release/rey-v0 <file>.rey
```

---

## Contributing

Rey is open source and contributions are welcome. See [CONTRIBUTING.md](./CONTRIBUTING.md).

- Found a bug? Open an issue.
- Want to add a feature? Open a PR to the `contri` branch.
- Want to discuss the language design? Start a discussion.

---

## License

MIT — see [LICENSE](./LICENSE).

---

*Built by [@IMisbahk](https://github.com/IMisbahk) and contributors.*
