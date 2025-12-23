# Types — Rey v0

## Overview

Rey is **dynamically typed by default**.

Types are only enforced when a programmer explicitly annotates a variable or function return value.

Once a type annotation is applied, the type of that binding becomes **immutable**.

---

## Built-in Types (v0)

Rey v0 defines the following built-in types:

- `int` — signed integer values
- `string` — UTF-8 encoded strings
- `bool` — boolean values (`true`, `false`)
- `null` — absence of a value

---

## Type Annotations

A type annotation MAY be applied to a variable or function return value using the `:` operator.

Example:
```rey
var name = "misbah" : string; //binded type

//Type annotations bind to the variable, not the value
var age = 18; //non-binded mutable
age = "prime"; //this is allowed 

```
# Typed vs Untyped Bindings
## Untyped Variable

```rey
var x = 10;
x = "hello";   // allowed
```

## Rules:
- the variable has no fixed type.
- the runtime type of the value MAY change.
- no compile time type enforcement occurs.

---

## Typed Variable
```rey
var y = 10 : int;
y = 20;        // allowed
y = "hello";   // compile-time error
```

Rules:
- the variable’s type becomes fixed at declaration.
- reassignment MUST be type-compatible.
- violations MUST be reported at compile time.
  

# Type Compatibility

An assignment is type-compatible if:

- The assigned value’s runtime type matches the declared type exactly.
- Implicit coercion between types is not permitted in v0.

# Type Errors

Type errors occur when:

A typed variable is assigned an incompatible value.
A typed function returns a value of the wrong type.
Type errors in Rey v0 are compile-time errors wherever possible.