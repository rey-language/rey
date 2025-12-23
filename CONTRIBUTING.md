# Contributing to Rey

Thanks for your interest in contributing to **Rey**.  
Rey is an experimental programming language focused on language design, interpreters, and execution models. Contributions are welcome, but the bar for correctness and clarity is intentionally high.

Please read this document carefully before opening an issue or pull request.

---

## Project Philosophy

Rey prioritizes:

- Simplicity over cleverness  
- Explicit design decisions over implicit behavior  
- Small, well-reasoned primitives  
- Clear separation between specification and implementation  

If a change adds complexity, it must justify itself.

---

## Ways to Contribute

You can contribute by:

- Reporting bugs or inconsistencies
- Improving documentation or specifications
- Proposing language features or semantics
- Implementing interpreter or tooling changes
- Adding tests or examples

Speculative or vague contributions are discouraged. Prefer concrete proposals.

---

## Before You Start

1. Read the `README.md`
2. Read the language specification under `spec/`
3. Search existing issues and pull requests
4. Understand the current scope of **v0**

If you’re unsure whether something belongs in v0, open a discussion first.

---

## Issues

When opening an issue:

- Use a clear and descriptive title
- Explain the problem, not just the symptom
- Include examples (code snippets, expected vs actual behavior)
- Reference relevant spec sections if applicable

Feature requests should explain:
- The problem being solved
- Why existing mechanisms are insufficient
- How the proposal fits Rey’s design goals

---

## Pull Requests

All pull requests should:

- Be focused on a single change
- Include a clear explanation of *why* the change exists
- Reference related issues or discussions
- Update documentation/specs if behavior changes
- Avoid unrelated refactors

Large changes should be discussed before implementation.

---

## Code Style

- Prefer clarity over brevity
- Avoid unnecessary abstractions
- Keep functions small and readable
- Name things explicitly
- Comments should explain *why*, not *what*

Consistency matters more than personal preference.

---

## Specifications vs Implementation

- The `spec/` directory defines **what Rey is**
- The implementation defines **how it runs**

Implementation must follow the spec, not the other way around.  
If behavior changes, the spec must be updated first or alongside the change.

---

## Versioning

- v0 is intentionally minimal
- Breaking changes are acceptable in v0, but must be justified
- Experimental features should be clearly marked

---

## Community Expectations

- Be respectful and constructive
- Assume good intent
- Critique ideas, not people
- Keep discussions technical and grounded

This is a learning-focused, long-term project.

---

## License

By contributing, you agree that your contributions will be licensed under the project’s license.

---

## Questions

If you’re unsure about anything, open a discussion instead of guessing.

Thanks for helping build Rey.
