# Contributing to Rey

Rey is an open, ambitious project. We're building a language from scratch — the compiler, the toolchain, the stdlib, the ecosystem. Everything. If that sounds exciting, you're in the right place.

This isn't a finished project looking for minor fixes. It's a living language actively being designed and built. Your contributions don't just fix bugs — they shape what Rey becomes.

---

## The vision

> Rust power without Rust complexity.

Rey is designed to be the language that doesn't make you choose between fast and simple. The compiler handles the hard parts. You write clean code. Read `VISION.md` before anything else — it's the north star for every decision in this project.

---

## Who we're looking for

Anyone who is:

- Excited about language design and compiler engineering
- Willing to learn (you don't need to be an expert)
- Opinionated about how programming should feel
- Able to write clean, reasoned code
- Interested in building something real from the ground up

We don't care about your resume. We care about your ideas and your work.

---

## What needs to be built

Rey is early. Almost everything is fair game. Here's where the work is:

### Compiler (Rust)
The heart of Rey. Written in Rust, lives in `compiler/v1/`.

- **Bug fixes** — see the issue tracker, there's always something broken
- **New syntax** — enums, match, generics, closures, traits
- **Import system** — the module resolution and import pipeline
- **LLVM backend** — the big one. Turn AST → LLVM IR → native binary
- **Error messages** — make them better, clearer, more helpful
- **Type system** — generics, union types, type inference improvements

### Standard Library (Rey)
Lives in `rey-language/std`. Written in Rey itself (once the import system lands).

- `std::fs` — file I/O
- `std::process` — args, exit, env
- `std::math` — math functions
- `std::http` — HTTP client/server
- `std::json` — JSON parsing
- anything else the language needs

### reyc (Rey)
The package manager and toolchain. Think cargo for Rey.

- `reyc run` — compile and run
- `reyc init` — scaffold a project
- `reyc build` — compile to binary
- `reyc add` — install a package
- `reyc publish` — publish to reyc.io

### Tooling
- **VSCode extension** — lives in `rey-language/rey-vscode`. Syntax highlighting is done, LSP is next.
- **rey-website** — rey-lang.com. Docs, playground, getting started.
- **reyc.io** — the package registry website.

### Language design
Not all contributions are code. If you have strong opinions about:
- How memory should work in Rey
- What the import syntax should look like
- How enums and match should behave
- What belongs in the standard library

Open a discussion. Language design decisions matter more than any single implementation.

---

## How to contribute

### 1. Read first
- `README.md` — what Rey is
- `VISION.md` — where Rey is going
- `syntax.md` — what the language looks like today
- `primer.md` — current state of the project

### 2. Find something to work on
- Check the issue tracker for open issues
- Look at the bug/feature list in `primer.md`
- Have an idea? Open a discussion first

### 3. Branch discipline
- `master` — stable, owned by the core team
- `claude` — Claude's branch (AI contributor)
- `codex` — Codex's branch (AI contributor)
- your contributions → open a PR to `master`

Create your branch off master:
```bash
git checkout master
git checkout -b your-name/feature-name
```

### 4. Make your changes
- Small, focused commits
- Conventional commit messages: `feat(lexer): add comment tokenization`
- Test your changes — run the test files in `compiler/v1/src/tests/`
- Don't break existing tests

### 5. Open a PR
- Clear title and description
- Explain what you changed and WHY
- Reference related issues
- If it's a language change, update `syntax.md`
- If it's a big change, discuss it first

---

## Commit style

We use conventional commits:

```
feat(scope): what you added
fix(scope): what you fixed
chore(scope): maintenance, cleanup
docs(scope): documentation
test(scope): tests
refactor(scope): refactor without behavior change
```

Examples:
```
feat(structs): implement method overloading
fix(parser): handle empty array literal
chore(release): bump to v0.0.7-pre
docs(syntax): add enum documentation
```

---

## Running the compiler

```bash
cd compiler/v1
cargo build --release
./target/release/rey-v0 your-file.rey
```

Run all tests:
```bash
for f in src/tests/*.rey; do
    echo "Testing $f..."
    ./target/release/rey-v0 $f
done
```

---

## The AI contributors

Rey has two autonomous AI contributors — `claude` and `codex` — each with their own git branch. They commit real code, open real PRs, and are treated as contributors.

This is an experiment in human-AI collaborative development. If you want to set up a similar workflow, read `CLAUDE.md` and `AGENTS.md`.

---

## What we value

**Simplicity** — if it's complex, it needs to justify itself  
**Clarity** — code and decisions should be easy to reason about  
**Boldness** — Rey is making big bets. Don't be afraid to propose big ideas  
**Honesty** — if something is broken or wrong, say so  
**Craft** — care about the details. Languages are used by people.

---

## What we don't want

- Vague PRs with no explanation
- Changes that break existing behavior without discussion
- Overengineered solutions to simple problems
- Ego. Critique ideas, not people.

---

## Communication

- **Issues** — bugs, feature requests, questions
- **Discussions** — language design, big ideas, proposals
- **PRs** — concrete changes with clear rationale

We're a small team. Response times vary. Be patient.

---

## License

By contributing, you agree your contributions will be licensed under the MIT license.

---

Rey is early. The decisions made now will define what this language becomes. If you want to be part of that — welcome.

https://github.com/rey-language/rey