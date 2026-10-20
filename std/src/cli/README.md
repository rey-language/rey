# `std::cli`

Command-line helpers for Rey v0.3. Import with `import std::cli;`.

`parseArgs(args: Vec<string>): any` returns a map with three entries:

- `flags`: a `HashMap<string, bool>`
- `options`: a `HashMap<string, string>`
- `positionals`: a `Vec<string>`

Use `hasFlag(parsed, name)`, `getOption(parsed, name, defaultValue)`, and
`getPositionals(parsed)` to read those values. Names can be passed with or
without leading dashes. Long options support both `--port=8080` and
`--port 8080`. A long switch with no following ordinary value is a flag.
Because this API has no option schema, a long switch followed by an ordinary
token is interpreted as an option/value pair. Put positional arguments before
such switches or after `--` to avoid ambiguity. Short switches such as `-v`
are flags; compact groups such as `-abc` set three flags.

`renderHelp(appName, desc, flags)` renders a usage header and option rows. Each
row is a `Vec<string>` whose first element is the spelling (for example,
`--verbose`) and whose second element, when present, is its description.
