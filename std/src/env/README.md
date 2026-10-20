# `std::env`

Convenience functions for reading and changing the current process environment.

```rey
import std::env;

var mode: string = env.getOr("APP_MODE", "development");
env.set("APP_MODE", mode);
```

## Functions

- `get(key: string): any` returns the runtime's `Option<string>` value: use
  `isSome()` and `unwrap()` when the variable is set.
- `getOr(key: string, defaultVal: string): string` returns the value or the
  supplied default.
- `set(key: string, value: string): void` sets a process environment variable.
- `remove(key: string): void` removes a process environment variable.
- `has(key: string): bool` checks whether a variable exists.
- `all(): any` returns the runtime's `HashMap<string, string>` of variables.

Changes affect the current Rey process and its subsequently launched child
processes. Environment values are strings; the underlying runtime determines
which keys and values are valid on the host.
