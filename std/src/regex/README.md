# `std::regex`

A small pattern-matching engine written in pure Rey. Import it with
`import std::regex;`.

## Functions

- `isMatch(pattern: string, text: string): bool` reports whether the pattern
  matches anywhere in the text. Use `^` and `$` to constrain the match to the
  beginning or end of the text.
- `find(pattern: string, text: string): any` returns `Option.Some(matchedText)`
  for the first match, or `Option.None()` when no match exists. An empty match
  is returned as `Some("")`.
- `replace(pattern: string, text: string, repl: string): string` replaces every
  non-overlapping match. Zero-width matches are replaced once at each possible
  position while ensuring the scan always advances.

Supported syntax is literal characters, `.` for any one character, `^` and `$`
anchors, `*`, `+`, and `?` quantifiers, and character sets such as `[abc]` and
`[a-z]`. Character-set ranges compare character codes. Escapes, grouping,
alternation, negated sets, and Unicode grapheme-aware matching are not
implemented. Quantifiers apply to the preceding single character or set.
