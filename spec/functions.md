# Functions — Rey v0

## Function Declaration

Functions are declared using the `func` keyword.

Syntax:
```rey
func name(parameters) : returnType {
    body
}
func greet(name: string) : string {
    return name;
}
```
## Parameters

Parameters MAY be annotated with types.

Untyped parameters are dynamically typed.

Typed parameters MUST receive type-compatible arguments.

```rey

func echo(x) {
    return x;
}

func add(a: int, b: int) : int {
    return a + b;
}
```

## Return Types

A function MAY declare a return type.

If declared, all return paths MUST produce a compatible value.

Violations MUST result in a compile-time error.

```rey
func bad() : int {
    return "hello";   // compile-time error
}
```