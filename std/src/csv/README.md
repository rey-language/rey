# std::csv

RFC 4180 style CSV parsing and serialization implemented in Rey.

```rey
import std::csv;

var rows = csv.parse("name,quote\r\nRey,\"hello, world\"");
var output = csv.stringify(rows);
```

`parse` and `stringify` use comma delimiters. `parseWithDelimiter` and
`stringifyWithDelimiter` accept a one-character delimiter. Parsed fields retain
their content, including line breaks inside quoted fields. Serialization uses
CRLF between records and quotes fields containing the delimiter, a quote, or a
line break.
