#!/usr/bin/env python3
"""Capture opt-in compiler phase markers with monotonic timestamps."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import threading
import time
from pathlib import Path


PREFIX = b"REY_RETENTION event="


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--events", type=Path, required=True)
    parser.add_argument("--stdout", type=Path, required=True)
    parser.add_argument("--stderr", type=Path, required=True)
    parser.add_argument("--status", type=Path, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        parser.error("missing compiler command")

    for path in (args.events, args.stdout, args.stderr, args.status):
        path.parent.mkdir(parents=True, exist_ok=True)

    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    errors: list[BaseException] = []

    def drain(stream, output_path: Path, capture_events: bool) -> None:
        try:
            with output_path.open("wb") as output:
                event_file = args.events.open("w", encoding="utf-8") if capture_events else None
                try:
                    while True:
                        line = stream.readline()
                        if not line:
                            break
                        output.write(line)
                        output.flush()
                        if event_file is not None and line.startswith(PREFIX):
                            event_file.write(json.dumps({
                                "timestamp_ns": time.monotonic_ns(),
                                "line": line.decode("utf-8", errors="replace").rstrip("\r\n"),
                            }, sort_keys=True) + "\n")
                            event_file.flush()
                finally:
                    if event_file is not None:
                        event_file.close()
        except BaseException as exc:  # propagate reader errors after process completion
            errors.append(exc)

    started = time.monotonic_ns()
    stdout_thread = threading.Thread(target=drain, args=(process.stdout, args.stdout, False))
    stderr_thread = threading.Thread(target=drain, args=(process.stderr, args.stderr, True))
    stdout_thread.start()
    stderr_thread.start()
    status = process.wait()
    stdout_thread.join()
    stderr_thread.join()
    ended = time.monotonic_ns()
    if errors:
        raise errors[0]
    args.status.write_text(
        json.dumps({"status": status, "started_ns": started, "ended_ns": ended}, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return status


if __name__ == "__main__":
    raise SystemExit(main())
