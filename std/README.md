# Rey Standard Library (`std`)

The Rey Standard Library (`std`) provides the foundational, high-performance building blocks for Rey applications and compilers. This guide targets the **Rey language v0.3.0** syntax and runtime.

Organized under [`src/`](src/), all 28 standard modules can be imported directly:
```rey
import std::collections;
import std::math;
import std::string;
import std::test;
```

---

## Complete 28-Module Matrix

### Tier 1: Core Runtime & Essentials (Day-1 / Self-Hosting Base)
| Module | Status | Highlights & Capabilities |
|---|---|---|
| [`std::core`](src/core/README.md) | ✅ Complete | Language-level assertions, `panic`, `todo`, `unreachable`, `Option`/`Result` inspection (`isSome`, `isNone`, `isOk`, `isErr`, `unwrapOr`). |
| [`std::mem`](src/mem/README.md) | ✅ Complete | Memory layout introspection (`sizeOf*`), sliceable buffer views (`Span`), and overlap-safe memory copying. |
| [`std::string`](src/string/README.md) | ✅ Complete | UTF-8 string manipulation: `trim`, `replace`, `replaceAll`, `slice`, `substring`, `split`, `join`, `padLeft`, `padRight`, `lines`, `words`, `truncate`, `parseInt`, `parseFloat`, `slugify`. |
| [`std::fmt`](src/fmt/README.md) | ✅ Complete | String template formatting (`format`), padding, and integer radix conversion (`toHex`, `toBin`, `toOct`). |
| [`std::collections`](src/collections/README.md) | ✅ Complete | Dynamic algorithms: `range`, `zip`, `flatten`, `unique`, `groupBy`, `chunk`, `partition`, `find`, `every`, `some`, `count`, `maxBy`, `minBy`, `sum`, `frequency`, `take`, `drop`, `compact`. |
| [`std::io`](src/io/README.md) | ✅ Complete | Terminal ANSI styling (`printRed`, `printBold`, etc.), table output (`printTable`), progress indicators (`progressBar`), and boxed banners. |
| [`std::fs`](src/fs/README.md) | ✅ Complete | Filesystem engine: `read`, `write`, `append`, `exists`, `remove`, `touch`, `readLines`, `writeLines`, `ensureDir`, `cleanPath`, `copy`, `move`, `listPaths`, `mkdirAll`. |
| [`std::process`](src/process/README.md) | ✅ Complete | Process args, environment variables, working directory, command execution (`run`, `runDetailed`), OS detection, and exit control. |

### Tier 2: Systems, OS & Networking
| Module | Status | Highlights & Capabilities |
|---|---|---|
| [`std::net`](src/net/README.md) | ✅ Complete | Network address helpers (`SocketAddr`), IPv4 parsing/validation, loopback detection, and host reachability (`pingHost`). |
| [`std::http`](src/http/README.md) | ✅ Complete | REST client verbs (`get`, `post`, `put`, `del`), custom headers, response parsing, and timeout configuration. |
| [`std::time`](src/time/README.md) | ✅ Complete | High-resolution millisecond sleep (`sleepMs`) and human-readable duration formatting (`formatDurationMs`, `formatDurationSeconds`). |
| [`std::env`](src/env/README.md) | ✅ Complete | Dedicated OS environment access: `get`, `getOr`, `set`, `remove`, `has`, `all`. |
| [`std::sync`](src/sync/README.md) | ✅ Complete | Concurrency & cooperative primitives: `AtomicInt` (atomic add, sub, get, set), `Mutex` (try-lock, unlock), and FIFO `Channel`. |
| [`std::crypto`](src/crypto/README.md) | ✅ Complete | Fast non-crypto hashes (`fnv1a`, `djb2`), command-backed cryptographic digests (`sha256`, `md5`), random bytes, and hex encoding. |
| [`std::os`](src/os/README.md) | ✅ Complete | Host platform identification (`name`, `arch`, `hostname`, `homedir`, `tempDir`, `eol`). |

### Tier 3: Data Formats & Developer Productivity
| Module | Status | Highlights & Capabilities |
|---|---|---|
| [`std::json`](src/json/README.md) | ✅ Complete | JSON string escaping and data structure stringifiers (`stringifyString`, `stringifyInt`, `stringifyFloat`, `stringifyArray`, `stringifyObject`). |
| [`std::toml`](src/toml/README.md) | ✅ Complete | Pure Rey TOML configuration parser and serializer for `rey.toml` and `capy.toml`. |
| [`std::csv`](src/csv/README.md) | ✅ Complete | RFC 4180 compliant CSV & TSV parser and generator with custom delimiter and quote support. |
| [`std::regex`](src/regex/README.md) | ✅ Complete | Linear-time pattern matcher supporting literals, `.` wildcards, `*`, `+`, `?` quantifiers, character classes `[a-z]`, anchors `^`, `$`. |
| [`std::cli`](src/cli/README.md) | ✅ Complete | Declarative CLI flag and option parser (`parseArgs`, `hasFlag`, `getOption`, `getPositionals`, `renderHelp`). |
| [`std::log`](src/log/README.md) | ✅ Complete | Leveled structured logging (`trace`, `debug`, `info`, `warn`, `error`) with configurable runtime verbosity. |
| [`std::test`](src/test/README.md) | ✅ Complete | Built-in test assertion framework (`assertEq`, `assertNotEq`, `assertTrue`, `assertFalse`, `assertApprox`, `pass`, `fail`). |
| [`std::binary`](src/binary/README.md) | ✅ Complete | Little-endian and Big-endian binary packing/unpacking (`packU16Le`, `packU32Le`, etc.) and hex byte conversions. |

### Tier 4: High-Performance Domains & Extensions
| Module | Status | Highlights & Capabilities |
|---|---|---|
| [`std::math`](src/math/README.md) | ✅ Complete | Trigonometry, statistics (`mean`, `median`, `variance`, `stdDev`), number theory (`gcd`, `lcm`, `factorial`), interpolation (`lerp`, `hypot`), constants (`pi`, `e`, `tau`). |
| [`std::linalg`](src/linalg/README.md) | ✅ Complete | 2D and 3D vector arithmetic (`Vec2`, `Vec3`, add, sub, scale, dot, cross, length, distance, normalize). |
| [`std::compress`](src/compress/README.md) | ✅ Complete | Pure Rey run-length encoding (RLE) plus host `gzip`, `gunzip`, `tarCreate`, and `tarExtract` integrations. |
| [`std::image`](src/image/README.md) | ✅ Complete | 24-bit RGB pixel buffer (`Image`, `setPixel`, `getPixel`), Netpbm PPM image serializer, and file emitter. |
| [`std::sql`](src/sql/README.md) | ✅ Complete | Database query and execution abstraction with SQLite CLI backend, memory database support, and safe SQL escaping. |

---

## 🚀 Quickstart Example

```rey
import std::collections;
import std::math;
import std::string;
import std::test;

func main(): void {
    const raw: string = "  rey  ";
    var name = string.trim(raw);
    var steps = collections.range(1, 6);
    var score = math.clamp(steps.len() * 4, 0, 10);

    test.assertEq(name, "rey", "trim removes surrounding whitespace");
    println("hello " + string.toUpperCase(name));
    println(steps.len().toString() + " steps; score " + score.toString());
    test.pass("quickstart complete");
}
```

---

## 🧪 Master Test Suite (28 / 28 Passing)

Execute the entire test suite across all 28 modules with:

```bash
cd /Volumes/SSD/Developer/rey-language/std
./tests/run_all.sh
```

```text
==========================================
 ALL 28 STANDARD LIBRARY MODULES PASSED!
 Total: 28 / 28 test suites passed, 0 failed.
==========================================
```
