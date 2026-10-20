# std::sql

`std::sql` provides a small SQL connection and query interface backed by the
`sqlite3` command-line program when it is available.

```rey
import std::sql;

var conn = sql.connect("app.db");
var result = sql.query(conn, "SELECT name FROM users");
if result.ok {
    println(result.columns);
    println(result.rows);
}
sql.close(conn);
```

## API

- `connect(path: string): Connection` selects the `sqlite3` driver when the CLI
  is on `PATH`; otherwise it selects the `memory` mock driver.
- `execute(conn: Connection, sql: string): QueryResult` runs SQL and returns
  headers, pipe-separated text rows, a change count, and status/error fields.
- `query(conn: Connection, sql: string): QueryResult` is an alias for
  `execute`.
- `escapeString(val: string): string` doubles single quotes for embedding a
  value inside a SQL single-quoted string literal. It does not add the quotes.
- `close(conn: Connection): void` removes the temporary backing file used for
  a `:memory:` connection. Ordinary CLI connections do not hold an open handle.

The SQLite backend invokes `sqlite3 -header -separator '|'` through the process
shell. It also appends `SELECT changes()` to report the last statement's change
count. Shell arguments are quoted, but SQL itself is intentionally executed as
SQL and this API does not provide parameterized statements. The text format
cannot distinguish embedded pipe characters or newlines in values, and SQL
`NULL` is represented as an empty string.

When `sqlite3` is unavailable, the `memory` driver is a no-op mock: calls
succeed with empty columns and rows and zero changes. It does not parse SQL or
retain data. A `:memory:` connection uses a generated temporary SQLite
file because each CLI call starts a separate process; call `close` to remove
that file.
