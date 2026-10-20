# `std::log`

Simple leveled logging for Rey v0.3. Import with `import std::log;`.

Levels are ordered from most detailed to most severe: trace (0), debug (1),
info (2), warn (3), and error (4). The default threshold is info. A message is
printed to standard output when its level is at least the configured threshold.
`formatLog(level, msg)` returns the stable text form `[LEVEL] message`.

`setLogLevel(level)` accepts only values from 0 through 4. Since Rey functions
cannot share module-level mutable state, the threshold is kept in the current
process environment under `REY_STD_LOG_LEVEL`. It therefore affects later
calls in the same process and may be inherited by child processes.
