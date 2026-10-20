# std::toml

A small pure-Rey TOML reader and writer for common configuration values:
quoted strings, integers, booleans, comments, and single-level section headers.

```rey
import std::toml;

var config = toml.parse("[server]\nport = 8080\nenabled = true");
var port = toml.getInt(config, "server.port", 3000);
```

Section values are stored in the flat map under dotted keys such as
`server.port`. `stringify` emits these as valid dotted-key assignments. Missing
keys and values of the wrong type return the caller's default.
