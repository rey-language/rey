Mirrored import fixtures retained at their historical nested path. Run from
`compiler/v1`:

```bash
cargo run -- run ../../tests/imports/imports/success/main.rey
cargo run -- run ../../tests/imports/imports/errors/duplicate_import.rey
cargo run -- run ../../tests/imports/imports/errors/pub_not_export.rey
cargo run -- check ../../tests/imports/imports/errors/file_not_found.rey
cargo run -- check ../../tests/imports/imports/errors/folder_missing_main.rey
cargo run -- check ../../tests/imports/imports/errors/function_not_found.rey
cargo run -- check ../../tests/imports/imports/errors/group_missing_symbol.rey
cargo run -- check ../../tests/imports/imports/errors/circular/cycle_entry.rey
```

The historical `duplicate_import.rey` and `pub_not_export.rey` paths are
success cases under v0.3: identical imports are idempotent and `pub` is
importable. `export` is invalid; unmodified private declarations are not.
