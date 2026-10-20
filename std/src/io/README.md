# std::io

Terminal output helpers for Rey v0.3. Import with `import std::io;`.

- `printRed(msg: string): void`
- `printGreen(msg: string): void`
- `printYellow(msg: string): void`
- `printBlue(msg: string): void`
- `printMagenta(msg: string): void`
- `printCyan(msg: string): void`
- `printBold(msg: string): void`

The color and bold helpers wrap a line in ANSI escape sequences. `clear(): void`
prints the ANSI clear-screen and home-cursor sequence.

`printTable(headers: Vec<string>, rows: Vec<Vec<string>>): void` prints a
bordered table. Rows with fewer cells than headers are padded with empty cells;
extra row cells are ignored.

`progressBar(current: int, total: int, width: int): string` returns an ASCII
bar such as `[====  ]`. `width` is the number of interior fill characters.
Progress is clamped to the bar, negative widths become zero, and non-positive
totals produce an empty bar unless `current` is positive (then it is full).

`printBox(title: string, content: string): void` prints the title and
newline-separated content inside a padded ASCII box.

