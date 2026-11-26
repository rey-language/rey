# Self-host profile event log

`scripts/v04-selfhost-profile.sh` summarizes the elapsed time from a producer's
image write through normal teardown and its actual exit. The profile log is
JSON Lines with exactly three ordered events:

```json
{"event":"image_write","timestamp_ns":1000000000}
{"event":"teardown_complete","timestamp_ns":2600000000}
{"event":"producer_exit","timestamp_ns":3000000000,"status":0}
```

Timestamps are monotonic nanoseconds from one clock. The writer rejects missing,
duplicate, reordered, malformed, or nonzero-exit events. `--self-test` checks
the exact synthetic timestamps/durations and proves failed and incomplete logs
are rejected. Each self-test writes its copied inputs and report in a unique
subdirectory of the caller's work directory and retains them.

```sh
sh scripts/v04-selfhost-profile.sh --self-test --work /tmp/rey-profile-test
sh scripts/v04-selfhost-profile.sh --log /path/to/events.jsonl --out /path/to/profile.md
```
