# std::fmt

String formatting, padding, and integer radix conversion helpers.

## API

- `format(template: string, args: Vec<any>): string` substitutes `{}` in order
  and `{0}`, `{1}`, and other decimal indexes from `args`. Values use their
  Rey `toString()` representation. Invalid or out-of-range placeholders are
  kept as written; extra arguments are ignored. In a Rey source string literal,
  write placeholder braces doubled (`{{}}`, `{{0}}`) so the string parser
  passes them through to this function.
- `padLeft(s: string, width: int, pad: string): string` and
  `padRight(s: string, width: int, pad: string): string` pad to a minimum
  string length, repeating and truncating `pad` as needed. Empty `pad` and
  widths no larger than `s` leave `s` unchanged.
- `toHex(val: int): string`, `toBin(val: int): string`, and
  `toOct(val: int): string` convert integers to lowercase hexadecimal, binary,
  and octal digits. Results have no base prefix; negative values have a leading
  minus sign.

```rey
import std::fmt;

var args: Vec<any> = ["Rey", 3];
println(fmt.format("hello {{}}, version 0.{{}}", args));
println(fmt.toHex(255)); // ff
```
