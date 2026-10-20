# std::fs

Filesystem and path helpers for Rey v0.3. Import with `import std::fs;`.
Filesystem operations delegate to compiler builtins; `cleanPath` is lexical
and does not check whether a path exists or resolve symlinks.

## File and directory operations

- `read(path: string): string` reads a whole file.
- `write(path: string, content: string): void` replaces or creates a file.
- `append(path: string, content: string): void` appends text to a file.
- `touch(path: string): void` creates an empty file only when the path does
  not already exist; it leaves existing contents unchanged.
- `readLines(path: string): Vec<string>` splits on line feeds, strips CRLF
  carriage returns, and does not add an extra item for a final line feed.
- `writeLines(path: string, lines: Vec<string>): void` writes each item with a
  trailing line feed. An empty vector writes an empty file.
- `exists(path: string): bool` checks whether a filesystem entry exists.
- `remove(path: string): void` removes a file.
- `mkdir(path: string): void` creates one directory.
- `mkdirAll(path: string): void` creates a directory and missing parents.
- `ensureDir(path: string): void` creates the directory tree when it is not
  already a directory.
- `removeDir(path: string): void` removes an empty directory.
- `removeDirAll(path: string): void` removes a directory tree.
- `isFile(path: string): bool` and `isDir(path: string): bool` inspect entry
  type.
- `list(path: string): Vec<string>` returns entry names.
- `listPaths(path: string): Vec<string>` returns full entry paths.
- `copy(src: string, dst: string): void` and `move(src: string, dst: string): void`
  copy or move a file.

## Path helpers

- `join(a: string, b: string): string` joins two path segments.
- `cleanPath(path: string): string` collapses repeated separators and `.` / `..`
  segments, normalizes backslashes to forward slashes, and preserves absolute
  roots and drive prefixes.
- `basename(path: string): string` and `dirname(path: string): string` return
  the final path component and its parent.
- `extname(path: string): string` returns the extension without a leading dot.

