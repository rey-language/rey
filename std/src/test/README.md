# std::test

Small assertion and reporting helpers for Rey test programs.

## Assertions

- `assertEq(actual, expected, msg: string)` checks equality.
- `assertNotEq(actual, expected, msg: string)` checks inequality.
- `assertTrue(condition: bool, msg: string)` checks that a condition is true.
- `assertFalse(condition: bool, msg: string)` checks that a condition is false.
- `assertApprox(actual: float, expected: float, epsilon: float, msg: string)` checks that the absolute difference is at most `epsilon`.

Assertions stop the test program with the supplied message when a check fails.

## Reporting

- `pass(msg: string)` prints a `PASS` line.
- `fail(msg: string)` prints a `FAIL` line and stops the test program.

```rey
import std::test;

test.assertEq("ready", "ready", "status matches");
test.assertApprox(3.14, 3.14159, 0.01, "pi approximation");
test.pass("example checks");
```
