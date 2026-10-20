# Rey v0.3.0 Release — Multi-Platform & Standard Library Distribution

**Released:** 2026-10-20  
**Version:** v0.3.0  
**Architectures:** macOS arm64/x86_64, Linux musl x86_64/aarch64, Windows x86_64  

---

## 📦 What's in this Release

Rey v0.3.0 is a milestone release delivering **first-class multi-platform portability** and **seamless Standard Library linking** across all major desktop and server operating systems.

### 1. Multi-Platform Binary Packages

Each package includes the standalone compiler (`bin/rey` or `bin/rey.exe`) and the full bundled pure-Rey Standard Library (`std/src/`):

| Package | Platform | Architecture | Binary Format | Linkage |
|---|---|---|---|---|
| `rey-v0.3.0-macos-arm64.tar.gz` | macOS (Apple Silicon) | `aarch64` | Mach-O 64-bit | Dynamic libc |
| `rey-v0.3.0-macos-x86_64.tar.gz` | macOS (Intel) | `x86_64` | Mach-O 64-bit | Dynamic libc |
| `rey-v0.3.0-linux-x86_64.tar.gz` | Linux | `x86_64` | ELF 64-bit | **100% Static (musl)** |
| `rey-v0.3.0-linux-aarch64.tar.gz` | Linux | `aarch64` | ELF 64-bit | **100% Static (musl)** |
| `rey-v0.3.0-windows-x86_64.zip` | Windows | `x86_64` | PE32+ Executable | Native MinGW |

---

## 📚 Standard Library (28 Pure-Rey Modules)

The release includes the pure-Rey standard library directly linked to the distribution. Standard library modules can be imported without external configuration:

```rey
import std::string;
import std::process;
import std::json;
import std::fs;

func main(): void {
    var greeting = "Hello, Rey v0.3.0!";
    if string.startsWith(greeting, "Hello") {
        println(greeting);
    }
}
```

### Module Directory:
- **Core & Runtime**: `std::core`, `std::mem`, `std::sync`, `std::time`, `std::env`, `std::process`, `std::os`
- **Collections & Data**: `std::collections`, `std::string`, `std::binary`, `std::json`, `std::toml`, `std::csv`
- **I/O & Networking**: `std::io`, `std::fs`, `std::net`, `std::http`, `std::sql`
- **Math & Science**: `std::math`, `std::linalg`, `std::image`
- **Utilities**: `std::log`, `std::fmt`, `std::crypto`, `std::regex`, `std::cli`, `std::compress`, `std::test`

---

## ⚡ Module Caching & Resolution

### In-Memory AST Caching (Active)
During compilation, all imported modules are memoized via `ResolverState::cache`. Diamond dependencies and shared domain schemas are parsed, resolved, and type-checked **only once** per compilation session.

### Automatic Portable Stdlib Discovery
The compiler resolves `std::*` using prioritized fallbacks:
1. Workspace root / local path (`./src/`, `./std/src`)
2. Relative binary path (`bin/../std/src`)
3. `REY_STD_PATH` environment variable override
4. User cache directories (`~/.capy/std/src`, `~/.reyc/std/src`)

### Incremental On-Disk Compilation (Roadmap)
Persistent disk-backed object compilation caching (`.capy/cache/`) is designed as part of the upcoming Capybara toolchain (`capy build`).

---

## 🚀 Quick Start

```sh
# Unpack the archive
tar -xzf rey-v0.3.0-macos-arm64.tar.gz
cd rey-v0.3.0-macos-arm64

# Run a Rey program
./bin/rey run app.rey

# Check syntax and typecheck
./bin/rey check app.rey
```
