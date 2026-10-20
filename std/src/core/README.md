# std::core

Core diagnostics and convenience helpers for Rey's built-in `Option` and
`Result` values.

## API

- `panic(msg: string): void` writes `panic: <msg>` to standard error and exits
  the process with status 1.
- `todo(msg: string): void` exits with a `TODO: <msg>` panic message.
- `unreachable(msg: string): void` exits with an `unreachable: <msg>` panic
  message.
- `unwrapOr(opt: any, defaultValue: any): any` returns an `Option`'s payload
  or a `Result`'s success value, falling back to `defaultValue` for `None` or
  `Err`.
- `isSome(opt: any): bool` and `isNone(opt: any): bool` inspect an `Option`.
- `isOk(res: any): bool` and `isErr(res: any): bool` inspect a `Result`.

The Option and Result helpers expect the corresponding built-in value; passing
another kind of value causes a runtime method error. The diagnostic functions
terminate the current process, so tests for their messages should run in a
separate process.

```rey
import std::core;

var maybeName = Option::Some("Rey");
if core.isSome(maybeName) {
    println(core.unwrapOr(maybeName, "anonymous"));
}
```
