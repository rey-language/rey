# `std::collections`

Pure helpers for working with Rey `Vec` values. Import with
`import std::collections;`. Functions that accept callbacks use typed Rey arrow
functions such as `(value: any) => value > 0`.

## API

| Signature | Behavior |
| --- | --- |
| `range(start: int, end: int, step: int = 1): Vec<int>` | Integer sequence with exclusive `end`; supports negative steps. A zero step returns an empty vector. |
| `zip(a: Vec<any>, b: Vec<any>): Vec<Vec<any>>` | Pairs items up to the shorter input length. Each pair is `[a[i], b[i]]`. |
| `flatten(nested: Vec<Vec<any>>): Vec<any>` | Flattens one vector level, preserving order. |
| `unique(arr: Vec<any>): Vec<any>` | Removes duplicates, keeping the first occurrence of each value. |
| `groupBy(arr: Vec<any>, keyFn: (any) -> any): HashMap<string, any>` | Groups values by `keyFn`; keys use each result's `toString()` representation. |
| `chunk(arr: Vec<any>, size: int): Vec<Vec<any>>` | Splits into consecutive groups; the last group may be shorter. Nonpositive sizes return empty. |
| `partition(arr: Vec<any>, predicate: (any) -> bool): Vec<Vec<any>>` | Returns `[matching, remaining]`, preserving input order in both groups. |
| `find(arr: Vec<any>, predicate: (any) -> bool): any` | Returns `Option.Some(item)` for the first match, else `Option.None()`. |
| `findIndex(arr: Vec<any>, predicate: (any) -> bool): int` | First matching index, or `-1`. |
| `every(arr: Vec<any>, predicate: (any) -> bool): bool` | True when all values match; true for an empty vector. |
| `some(arr: Vec<any>, predicate: (any) -> bool): bool` | True when at least one value matches. |
| `count(arr: Vec<any>, predicate: (any) -> bool): int` | Number of matching values. |
| `maxBy(arr: Vec<any>, keyFn: (any) -> any): any` | `Option.Some(item)` with the greatest comparable key, or `Option.None()` when empty. |
| `minBy(arr: Vec<any>, keyFn: (any) -> any): any` | `Option.Some(item)` with the least comparable key, or `Option.None()` when empty. |
| `sum(arr: Vec<any>): any` | Adds numeric values; empty input returns `0`. |
| `sumBy(arr: Vec<any>, fn: (any) -> any): any` | Adds the numeric result of `fn` for every item. |
| `frequency(arr: Vec<any>): HashMap<string, int>` | Counts values by their Rey `toString()` representation. |
| `take(arr: Vec<any>, n: int): Vec<any>` | First `n` values; clamps negative counts to zero and large counts to length. |
| `drop(arr: Vec<any>, n: int): Vec<any>` | Values after the first `n`; clamps the starting index to the input bounds. |
| `concat(a: Vec<any>, b: Vec<any>): Vec<any>` | New vector containing `a` followed by `b`. |
| `compact(arr: Vec<any>): Vec<any>` | Drops `null`, `false`, numeric zero, `""`, and `Option.None()` values. |

The module also retains `map`, `filter`, `reduce`, `reverse`, and `contains` for
compatibility with earlier `std::collections` versions.

## Examples

```rey
import std::collections;

var nums: Vec<any> = Vec.new();
nums.push(3);
nums.push(8);
nums.push(5);

var evens = collections.filter(nums, (value: any) => value % 2 == 0);
var total = collections.sum(nums);                    // 16
var top = collections.maxBy(nums, (value: any) => value);
var groups = collections.groupBy(nums, (value: any) => value % 2);
var firstTwo = collections.take(nums, 2);

if top.isSome() {
    println(top.unwrap().toString());                  // 8
}
println(groups.get("1").len().toString());             // 2
```

## Notes

- `range` excludes `end`, matching Rey's range iteration convention. A positive
  step with `start >= end`, or a negative step with `start <= end`, yields empty.
- `groupBy` and `frequency` return string-keyed maps because Rey `HashMap` keys
  are strings. Distinct values with the same string representation share a key.
- `maxBy` and `minBy` compare callback results with Rey's numeric ordering
  operators. Return an `Option` so an empty input is represented safely.
- Collection helpers create result vectors and preserve source order unless the
  operation explicitly reverses or groups values.
