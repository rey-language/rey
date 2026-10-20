# std::math

Math utilities for Rey v0.3. Public constants are exposed as functions because `pub const` is not supported.

## Number helpers

- `clamp(value, minVal, maxVal)` bounds a value.
- `sign(n)` returns `-1`, `0`, or `1`.
- `floor(n)`, `ceil(n)`, and `round(n)` return integer rounding results.
- `abs(n)`, `max(a, b)`, and `min(a, b)` provide common comparisons.
- `pi()`, `e()`, and `tau()` return the corresponding mathematical constants.

## Trigonometry and conversions

- `sin(x)`, `cos(x)`, and `tan(x)` use radians; sine and cosine use Taylor series approximations.
- `toRadians(deg)` and `degToRad(deg)` convert degrees to radians.
- `toDegrees(rad)` and `radToDeg(rad)` convert radians to degrees.
- `trunc(x)` returns an `int` rounded toward zero.
- `fract(x)` returns `x - floor(x)`, the fractional part in `[0, 1)`.

## Statistics

- `sum(nums: ...int)` and `mean(nums: ...int)` operate on variadic integer arguments. `mean` returns an integer and returns `0` for no arguments.
- `median(nums: Vec<int>): float` returns the middle value or midpoint of the two middle values. Empty input returns `0.0`.
- `variance(nums: Vec<int>): float` computes population variance. Empty input returns `0.0`.
- `stdDev(nums: Vec<int>): float` computes population standard deviation. Empty input returns `0.0`.

## Other utilities

- `gcd(a: int, b: int): int` returns the greatest common divisor.
- `lcm(a: int, b: int): int` returns the least common multiple, or `0` if either input is `0`.
- `factorial(n: int): int` computes factorial for non-negative integers and asserts for negative input.
- `sqrt(x: float)`, `pow(base, exp)`, and `log(x: float)` wrap the compiler's math operations.
- `random(): float` returns a random value.
- `lerp(a: float, b: float, t: float)` interpolates between `a` and `b`.
- `hypot(a: float, b: float)` computes the hypotenuse from two components.
