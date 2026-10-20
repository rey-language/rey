# `std::compress`

Run-length encoding is implemented in pure Rey. Gzip and tar operations invoke
the host's command-line tools through `std::process`.

## Functions

- `rleCompress(data: string): string` encodes each byte run as `count:byte`.
- `rleDecompress(compressed: string): string` decodes that format and asserts
  on malformed input. Empty input round-trips as an empty string.
- `gzip(inputPath: string, outputPath: string): bool` and
  `gunzip(inputPath: string, outputPath: string): bool` stream between paths.
- `tarCreate(archivePath: string, files: Vec<string>): bool` creates an archive
  containing the listed paths.
- `tarExtract(archivePath: string, destDir: string): bool` extracts an archive
  under the requested destination directory.

The gzip/tar wrappers require a POSIX shell and the `gzip`, `gunzip`, and `tar`
commands on `PATH`; they return false on Windows or a nonzero command exit.
Command arguments are quoted before execution. The caller must ensure the
archive contents are appropriate to extract into the selected directory.
RLE follows Rey's byte-indexed string operations and is not grapheme-aware.
