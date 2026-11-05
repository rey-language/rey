#!/usr/bin/env python3
"""Evaluate v0.4 hard release conditions from evidence files.

This script does not build, execute, or publish. A missing or failed gate is
blocked. It writes readiness.json in the evidence directory and never turns an
aspirational target into an observation.
"""
import json
import sys
from pathlib import Path

platforms = (
    "darwin-arm64",
    "darwin-x86_64",
    "linux-aarch64",
    "linux-x86_64",
    "windows-x86_64",
)
gates = ("build", "run", "bootstrap", "package")
forbidden = (
    "faster than every",
    "faster than all",
    "fastest language",
)


def loadEvidence(directory, platform):
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


def acceptPlatform(data):
    reasons = []
    for gate in gates:
        if data.get(gate) != "pass":
            reasons.append(gate + " is not pass")
    if data.get("nativeHardware") is not True:
        reasons.append("nativeHardware is not true")
    if data.get("emulator") is not False:
        reasons.append("emulator is not false")
    return reasons


def loadFacts(directory):
    path = directory / "release-facts.json"
    if not path.is_file():
        return {}
    try:
        data = json.loads(path.read_text())
    except json.JSONDecodeError:
        return {}
    if not isinstance(data, dict):
        return {}
    return data


def hardConditions(facts, platformReports):
    imageWriter = facts.get("imageWriter") == "rey" and facts.get("externalLinkerForImages") is False
    driverIsNative = facts.get("driverBuiltBy") == "rey-native-bootstrap" and facts.get("seedInvokesClang") is False
    conditions = [
        {
            "id": 1,
            "name": "Rey production compiler and native image generation, with no hidden linker",
            "status": "pass" if imageWriter and driverIsNative else "blocked",
            "reason": "scalar image writers are Rey and do not invoke a linker"
            if imageWriter else "image writer facts are missing or name an external linker",
        },
        {
            "id": 2,
            "name": "Current conformance for every advertised language feature",
            "status": "pass" if facts.get("conformanceComplete") is True else "blocked",
            "reason": "this package does not carry a complete conformance record",
        },
        {
            "id": 3,
            "name": "Precise roots, cyclic reclamation, regions, promotion, and concurrency boundaries",
            "status": "pass" if facts.get("concurrencyBoundaries") == "pass" else "blocked",
            "reason": "concurrency boundaries are not evidenced by this package",
        },
        {
            "id": 4,
            "name": "Reproducible S2/S3 native bootstrap",
            "status": "pass" if facts.get("bootstrapReproduced") is True else "blocked",
            "reason": "the experimental driver is still built by the v0.3.1 seed",
        },
        {
            "id": 5,
            "name": "Five platforms pass build, run, bootstrap, and package gates on native hardware",
            "status": "pass" if platformReports and all(item["status"] == "pass" for item in platformReports) else "blocked",
            "reason": "one or more platform gates are missing or blocked",
        },
        {
            "id": 6,
            "name": "Cache invalidation, trusted isolation, and semantic optimizations",
            "status": "pass" if facts.get("cacheInvalidation") == "pass" and facts.get("trustedIsolation") == "pass" else "blocked",
            "reason": "cache invalidation and trusted isolation are not evidenced here",
        },
        {
            "id": 7,
            "name": "Measured scorecard with regressions and shortfalls resolved",
            "status": "pass" if facts.get("regressionsResolved") is True and int(facts.get("scorecardSamples") or 0) >= 9 else "blocked",
            "reason": "no nine-sample scorecard is attached; aspirational targets are not observations",
        },
    ]
    if imageWriter and not driverIsNative:
        conditions[0]["reason"] = "image writers are Rey, but the driver was built by " + str(facts.get("driverBuiltBy", "an unknown host compiler"))
    return conditions


def main():
    if len(sys.argv) != 2:
        print("usage: v04-release-readiness.py <evidence-directory>", file=sys.stderr)
        return 2
    directory = Path(sys.argv[1])
    directory.mkdir(parents=True, exist_ok=True)
    blocked = []
    platformReports = []
    for platform in platforms:
        data, error = loadEvidence(directory, platform)
        if error:
            blocked.append(platform + ": " + error)
            platformReports.append({"platform": platform, "status": "blocked", "reasons": [error]})
            continue
        reasons = acceptPlatform(data)
        status = "pass" if not reasons else "blocked"
        if reasons:
            blocked.append(platform + ": " + "; ".join(reasons))
        platformReports.append({
            "platform": platform,
            "status": status,
            "reasons": reasons,
            "build": data.get("build"),
            "built": data.get("built", data.get("build")),
            "structural": data.get("structural"),
            "executed": data.get("executed"),
            "run": data.get("run"),
            "bootstrap": data.get("bootstrap"),
            "package": data.get("package"),
            "nativeHardware": data.get("nativeHardware"),
            "emulator": data.get("emulator"),
            "executionKind": data.get("executionKind"),
            "returnCode": data.get("returnCode"),
            "evidence": data.get("evidence", ""),
            "note": data.get("note", ""),
        })
        print(
            platform
            + " release=" + status
            + " built=" + str(data.get("built", data.get("build")))
            + " structural=" + str(data.get("structural"))
            + " executed=" + str(data.get("executed"))
            + " kind=" + str(data.get("executionKind"))
        )
    notes = directory / "release-notes.md"
    if notes.is_file():
        text = notes.read_text().lower()
        for phrase in forbidden:
            if phrase in text:
                blocked.append("release notes contain an unsupported speed claim: " + phrase)
    else:
        blocked.append("missing release-notes.md naming provenance and limitations")
    facts = loadFacts(directory)
    conditions = hardConditions(facts, platformReports)
    for condition in conditions:
        if condition["status"] != "pass":
            blocked.append("hard condition " + str(condition["id"]) + ": " + condition["reason"])
    report = {
        "released": False,
        "published": False,
        "packageContract": "rey-package-1",
        "verdict": "pass" if not blocked else "blocked",
        "platforms": platformReports,
        "hardConditions": conditions,
        "facts": facts,
    }
    (directory / "readiness.json").write_text(json.dumps(report, indent=2) + "\n")
    if blocked:
        print("v0.4 release blocked")
        for item in blocked:
            print("- " + item)
        print("wrote " + str(directory / "readiness.json"))
        return 2
    print("v0.4 evidence files satisfy the five-platform gate and the hard conditions")
    print("this script did not publish artifacts or measure performance")
    return 0


if __name__ == "__main__":
    sys.exit(main())
