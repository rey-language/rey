#!/usr/bin/env python3
"""Gate v0.4 release claims on per-platform evidence files.

This script does not build, execute, or publish. A missing or failed gate is
a blocked platform. It never turns an aspirational target into an observation.
"""
import json
import sys
from pathlib import Path

PLATFORMS = (
    "darwin-arm64",
    "darwin-x86_64",
    "linux-aarch64",
    "linux-x86_64",
    "windows-x86_64",
)
GATES = ("build", "run", "bootstrap", "package")
FORBIDDEN = (
    "faster than every",
    "faster than all",
    "fastest language",
)


def load_evidence(directory, platform):
    path = directory / (platform + ".json")
    if not path.is_file():
        return None, "missing " + str(path)
    try:
        data = json.loads(path.read_text())
    except json.JSONDecodeError as error:
        return None, str(path) + " is not JSON: " + str(error)
    if data.get("platform") != platform:
        return None, str(path) + " names a different platform"
    return data, ""


def accept(data):
    reasons = []
    for gate in GATES:
        if data.get(gate) != "pass":
            reasons.append(gate + " is not pass")
    if data.get("nativeHardware") is not True:
        reasons.append("nativeHardware is not true")
    if data.get("emulator") is not False:
        reasons.append("emulator is not false")
    return reasons


def main():
    if len(sys.argv) != 2:
        print("usage: v04-release-readiness.py <evidence-directory>", file=sys.stderr)
        return 2
    directory = Path(sys.argv[1])
    blocked = []
    for platform in PLATFORMS:
        data, error = load_evidence(directory, platform)
        if error:
            blocked.append(platform + ": " + error)
            continue
        reasons = accept(data)
        if reasons:
            blocked.append(platform + ": " + "; ".join(reasons))
    notes = directory / "release-notes.md"
    if notes.is_file():
        text = notes.read_text().lower()
        for phrase in FORBIDDEN:
            if phrase in text:
                blocked.append("release notes contain an unsupported speed claim: " + phrase)
    else:
        blocked.append("missing release-notes.md naming provenance and limitations")
    if blocked:
        print("v0.4 release blocked")
        for item in blocked:
            print("- " + item)
        return 2
    print("v0.4 evidence files satisfy the five-platform gate")
    print("this script did not publish artifacts or measure performance")
    return 0


if __name__ == "__main__":
    sys.exit(main())
