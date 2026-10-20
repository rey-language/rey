# `std::string`

Pure helpers for common string transformations, searching, validation, and parsing.
Import the module with `import std::string;` and call functions as
`string.trim(value)`.

## API

| Signature | Behavior |
| --- | --- |
| `trim(s: string): string` | Removes spaces, tabs, CR, and LF from both ends. |
| `trimStart(s: string): string` | Removes leading spaces, tabs, CR, and LF. |
| `trimEnd(s: string): string` | Removes trailing spaces, tabs, CR, and LF. |
| `startsWith(s: string, prefix: string): bool` | Checks the beginning; an empty prefix matches. |
| `endsWith(s: string, suffix: string): bool` | Checks the ending; an empty suffix matches. |
| `replace(s: string, from: string, to: string): string` | Replaces the first match. An empty `from` leaves `s` unchanged. |
| `replaceAll(s: string, from: string, to: string): string` | Replaces every non-overlapping match. An empty `from` leaves `s` unchanged. |
| `slice(s: string, start: int, end: int): string` | Extracts `[start, end)`; negative offsets count from the end and bounds are clamped. |
| `substring(s: string, start: int, end: int): string` | Extracts with clamped nonnegative bounds; swaps bounds when `start > end`. |
| `padLeft(s: string, width: int, fill: string = " "): string` | Pads to a minimum width; multi-character fill is clipped at the boundary. |
| `padRight(s: string, width: int, fill: string = " "): string` | Pads to a minimum width; multi-character fill is clipped at the boundary. |
| `repeat(s: string, n: int): string` | Repeats `s`; nonpositive counts return `""`. |
| `indexOf(s: string, sub: string): int` | First match index, `0` for an empty needle, or `-1`. |
| `lastIndexOf(s: string, sub: string, fromIndex: int = -1): int` | Last match at or before `fromIndex`; the default searches from the end. |
| `contains(s: string, sub: string): bool` | Checks whether `sub` occurs in `s`. |
| `split(s: string, sep: string): Vec<string>` | Splits on a delimiter and preserves empty fields; an empty delimiter splits characters. |
| `join(parts: Vec<string>, sep: string): string` | Joins strings with `sep`. |
| `toLowerCase(s: string): string` | Converts ASCII `A` through `Z` to lowercase. |
| `toUpperCase(s: string): string` | Converts ASCII `a` through `z` to uppercase. |
| `capitalize(s: string): string` | Uppercases the first character and lowercases the remainder (ASCII case conversion). |
| `reverse(s: string): string` | Reverses the indexed string units. |
| `lines(s: string): Vec<string>` | Splits on LF, removes CR from CRLF endings, and omits a final empty line. |
| `words(s: string): Vec<string>` | Splits on runs of space, tab, CR, and LF; omits empty words. |
| `truncate(s: string, maxLength: int, suffix: string = "..."): string` | Limits total width, including the suffix. |
| `count(s: string, sub: string): int` | Counts non-overlapping matches; an empty needle counts as zero. |
| `isNumeric(s: string): bool` | Accepts a signed decimal with at least one digit; surrounding whitespace is ignored. |
| `isAlpha(s: string): bool` | True for a nonempty ASCII-letter-only string. |
| `isAlphanumeric(s: string): bool` | True for a nonempty ASCII-letter-and-digit-only string. |
| `parseInt(s: string): any` | Returns `Option.Some(int)` or `Option.None()` for invalid input. |
| `parseFloat(s: string): any` | Returns `Option.Some(float)` or `Option.None()` for invalid input. |
| `slugify(s: string): string` | Lowercases ASCII text, turns separator runs into `-`, and keeps ASCII letters/digits. |

## Examples

```rey
import std::string;

var title = string.trim("  Rey Standard Library  ");
var slug = string.slugify(title);                    // "rey-standard-library"
var short = string.truncate(title, 12);              // "Rey Stand..."
var fields = string.split("one,two,three", ",");
var sentence = string.join(fields, " / ");           // "one / two / three"

if string.startsWith(title, "Rey") {
    println(string.replaceAll(title, " ", "-"));
}

var maybePort = string.parseInt("8080");
if maybePort.isSome() {
    println(maybePort.unwrap().toString());
}
```

## Notes

- String indices and lengths follow Rey's built-in string indexing behavior. The
  case conversion, alphabetic checks, and slug normalization are ASCII-focused.
- `parseInt` accepts an optional sign and decimal digits only. `parseFloat`
  accepts an optional sign and one decimal point; neither parser accepts exponents
  or trailing non-whitespace characters.
- Numeric parsers return an `Option` wrapped in `any` because Rey v0.3 does not
  expose a typed result signature for these hand-written parsers.
