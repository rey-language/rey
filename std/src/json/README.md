# std::json

Helpers for escaping strings and assembling basic JSON values. This module
does not parse JSON or validate pre-serialized values.

## API

- `escapeString(s: string): string` escapes characters that need escaping inside a JSON string, without adding quotes.
- `stringifyString(s: string): string` returns a quoted and escaped JSON string.
- `stringifyInt(n: int): string`, `stringifyFloat(f: float): string`, and `stringifyBool(b: bool): string` serialize primitive values.
- `stringifyArray(items: Vec<string>): string` joins already-serialized JSON values in an array.
- `stringifyObject(entries: Vec<Vec<string>>): string` takes entries shaped as `[plain string key, serialized JSON value]`.

Array items and object values are inserted as supplied, so callers should use
these helpers to serialize nested values and ensure those fragments are valid
JSON. `stringifyFloat` expects a finite value because JSON has no representation
for NaN or infinity.
