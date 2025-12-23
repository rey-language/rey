# Rey Language Specification — Version 0 (v0)

This document defines the **Version 0 (v0)** specification of the Rey programming language.

Rey is an experimental programming language designed to be **dynamic by default**, with **optional, explicit type annotations** that are strictly enforced once declared.

## Scope of v0

Rey v0 intentionally defines a **minimal core language**. Its purpose is to establish correct and consistent semantics, not feature completeness.

Rey v0 includes:
- Variables and assignment
- Optional type annotations
- Functions
- Basic execution model
- Basic built-in types

Rey v0 does **not** include:
- Objects or classes
- Contracts or invariants
- Concurrency or async execution
- Manual memory management
- Modules or imports

These features are reserved for future versions.

## Normative Language

The key words **MUST**, **MUST NOT**, **SHOULD**, and **MAY** are to be interpreted as described in RFC 2119.

This specification is **normative**.  
Any implementation of Rey v0 **must conform** to the rules described here.
