# std::time

Small timing helpers for delays and readable benchmark output.

## API

- `sleep(ms: int): void` pauses the current process for the requested number of milliseconds.
- `formatDurationMs(ms: int): string` returns values under one second as `N ms`, and longer values as seconds with three fractional digits, such as `1.250 s`.
- `formatDurationSeconds(sec: float): string` returns the standard Rey float representation followed by ` s`, such as `1.25 s`.

Duration formatters are intended for nonnegative elapsed times. They format a
provided value; they do not measure wall-clock time.
