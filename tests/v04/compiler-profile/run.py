#!/usr/bin/env python3
"""Validate self-host profile event logs and report image-to-exit timing."""

from __future__ import annotations

import argparse
import json
import shutil
import sys
import time
from pathlib import Path


HERE = Path(__file__).resolve().parent
EVENTS = ("image_write", "teardown_complete", "producer_exit")


class ProfileError(ValueError):
    pass


def read_events(path: Path) -> dict[str, dict[str, int | str]]:
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        raise ProfileError(f"cannot read event log: {exc}") from exc
    if not lines:
        raise ProfileError("event log is empty")
    parsed: list[dict[str, object]] = []
    for line_number, line in enumerate(lines, start=1):
        try:
            event = json.loads(line)
        except json.JSONDecodeError as exc:
            raise ProfileError(f"line {line_number}: invalid JSON") from exc
        if not isinstance(event, dict):
            raise ProfileError(f"line {line_number}: event must be an object")
        parsed.append(event)
    names = [event.get("event") for event in parsed]
    if tuple(names) != EVENTS:
        raise ProfileError(f"expected exactly ordered events {', '.join(EVENTS)}")
    result: dict[str, dict[str, int | str]] = {}
    for event in parsed:
        name = str(event["event"])
        timestamp = event.get("timestamp_ns")
        if not isinstance(timestamp, int) or isinstance(timestamp, bool) or timestamp < 0:
            raise ProfileError(f"{name}: timestamp_ns must be a non-negative integer")
        result[name] = {"timestamp_ns": timestamp}
        if name == "producer_exit":
            status = event.get("status")
            if not isinstance(status, int) or isinstance(status, bool):
                raise ProfileError("producer_exit: status must be an integer")
            result[name]["status"] = status
    write_ns = int(result["image_write"]["timestamp_ns"])
    teardown_ns = int(result["teardown_complete"]["timestamp_ns"])
    exit_ns = int(result["producer_exit"]["timestamp_ns"])
    if not write_ns <= teardown_ns <= exit_ns:
        raise ProfileError("image write, teardown, and producer exit timestamps are out of order")
    if result["producer_exit"]["status"] != 0:
        raise ProfileError(f"producer exited with status {result['producer_exit']['status']}")
    return result


def report_text(events: dict[str, dict[str, int | str]]) -> str:
    image_ns = int(events["image_write"]["timestamp_ns"])
    teardown_ns = int(events["teardown_complete"]["timestamp_ns"])
    exit_ns = int(events["producer_exit"]["timestamp_ns"])
    return "\n".join(
        [
            "# Self-host producer profile",
            "",
            "| Event | Timestamp (ns) |",
            "|---|---:|",
            f"| Image write complete | {image_ns} |",
            f"| Context teardown complete | {teardown_ns} |",
            f"| Producer exit (status 0) | {exit_ns} |",
            "",
            f"- Image write to teardown: {(teardown_ns - image_ns) / 1_000_000_000:.9f} s",
            f"- Teardown to producer exit: {(exit_ns - teardown_ns) / 1_000_000_000:.9f} s",
            f"- Image write to producer exit: {(exit_ns - image_ns) / 1_000_000_000:.9f} s",
            "",
        ]
    )


def write_report(log_path: Path, output_path: Path) -> None:
    report = report_text(read_events(log_path))
    output_path.parent.mkdir(parents=True, exist_ok=True)
    if output_path.exists():
        raise ProfileError(f"refusing to overwrite existing report: {output_path}")
    output_path.write_text(report, encoding="utf-8")


def self_test(work: Path) -> None:
    if not work.exists():
        work.mkdir(parents=True)
    if not work.is_dir():
        raise ProfileError(f"work path is not a directory: {work}")
    run_dir = work / f"self-test-{time.time_ns()}"
    run_dir.mkdir()
    success = HERE / "fixtures" / "profile-success.jsonl"
    failed = HERE / "fixtures" / "profile-failed.jsonl"
    incomplete = HERE / "fixtures" / "profile-incomplete.jsonl"
    copied: dict[str, Path] = {}
    for source in (success, failed, incomplete):
        if not source.is_file():
            raise ProfileError(f"missing synthetic log fixture: {source}")
        destination = run_dir / source.name
        shutil.copyfile(source, destination)
        copied[source.stem] = destination

    events = read_events(copied["profile-success"])
    if (
        events["image_write"]["timestamp_ns"] != 1_000_000_000
        or events["teardown_complete"]["timestamp_ns"] != 2_600_000_000
        or events["producer_exit"]["timestamp_ns"] != 3_000_000_000
    ):
        raise ProfileError("valid fixture timestamps did not match the expected values")
    report_path = run_dir / "profile.md"
    write_report(copied["profile-success"], report_path)
    report = report_path.read_text(encoding="utf-8")
    for exact in (
        "Image write complete | 1000000000",
        "Context teardown complete | 2600000000",
        "Producer exit (status 0) | 3000000000",
        "Image write to producer exit: 2.000000000 s",
    ):
        if exact not in report:
            raise ProfileError(f"report did not retain expected timestamp or duration: {exact}")

    for key in ("profile-failed", "profile-incomplete"):
        try:
            read_events(copied[key])
        except ProfileError:
            continue
        raise ProfileError(f"accepted {key} synthetic event log")

    print("PASS profile harness")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    self_parser = subparsers.add_parser("self-test")
    self_parser.add_argument("--work", type=Path, required=True)
    report_parser = subparsers.add_parser("report")
    report_parser.add_argument("--log", type=Path, required=True)
    report_parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "self-test":
            self_test(args.work.expanduser().resolve())
        else:
            write_report(args.log.expanduser().resolve(), args.out.expanduser().resolve())
            print(f"PASS profile report: {args.out}")
    except ProfileError as exc:
        print(f"profile error: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
