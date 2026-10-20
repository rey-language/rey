# `std::mem`

Memory-oriented helpers for inspecting Rey's documented primitive sizes and
working with bounded views over `Vec<any>` values. Import with
`import std::mem;`.

## API

- `sizeOfInt(): int`, `sizeOfFloat(): int`, `sizeOfBool(): int`, and
  `sizeOfChar(): int` return fixed byte counts: 8, 8, 1, and 4 respectively.
  The `char` value follows the requested module contract; it differs from the
  current v0.3 syntax reference, which describes `char` as one byte.
- `Span { buffer: Vec<any>, offset: int, length: int }` describes a view into a
  vector. Its fields are public so callers can inspect or construct values.
- `makeSpan(buffer, offset, length): Span` validates that the range fits in the
  vector.
- `spanGet(s, index): any` reads a zero-based element within a span.
- `spanSlice(s, start, end): Span` returns a view using a half-open range
  relative to the original span.
- `copy(dest, destOffset, src, srcOffset, count): void` copies existing vector
  elements and stages the source first, so overlapping ranges work like
  `memmove`.

Invalid span and copy ranges fail through `assert`. These helpers operate on
Rey vectors; they do not expose raw pointers or allocate native memory.
