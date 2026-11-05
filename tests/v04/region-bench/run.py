#!/usr/bin/env python3
"""Native region benchmarks for promotion granularity and reference stores.

Emits Darwin arm64 images from tests/v04/region-bench/workload.rey and records
wall time plus RSS. The workload pauses on stdin after printing its line so
the retained set can be sampled after the timed section.
"""
import hashlib
import json
import os
import select
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SEED = ROOT / "build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME = SEED.parent / "runtime"
RUNTIME_ROOT = ROOT / "runtime-rey"
WORKLOAD = ROOT / "tests/v04/region-bench/workload.rey"
MEMCAP = Path("/tmp/memcap.sh")
OUTPUT = ROOT / "build/v04/region-bench"

CASES = [
    ("promo-crowded", 1, 2048),
    ("promo-control", 1, 0),
    ("list-1000", 2, 1000),
    ("list-10000", 2, 10000),
    ("list-100000", 2, 100000),
    ("vector-1000", 3, 1000),
    ("vector-10000", 3, 10000),
    ("map-256", 4, 256),
    ("map-1000", 4, 1000),
]


def seedEnv():
    env = os.environ.copy()
    env["PATH"] = "/usr/bin:/bin:/usr/sbin:/sbin"
    env["CC"] = "/usr/bin/clang"
    env["CXX"] = "/usr/bin/clang++"
    env["MEMCAP_MB"] = "2048"
    env["MEMCAP_TIMEOUT_SEC"] = "900"
    return env


def runMemcap(args, cwd=None):
    command = [str(MEMCAP), *args]
    completed = subprocess.run(command, cwd=cwd, env=seedEnv(), text=True,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    sys.stderr.write(completed.stdout)
    sys.stderr.write(completed.stderr)
    return completed


def sampleRss(pid):
    completed = subprocess.run(["/bin/ps", "-p", str(pid), "-o", "rss="], text=True,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    text = completed.stdout.strip()
    if not text or not text.isdigit():
        return 0
    return int(text) * 1024


def writeWorkload(kind, count, destination):
    text = WORKLOAD.read_text()
    text = text.replace("var benchKind: int = 1;", "var benchKind: int = %d;" % kind, 1)
    text = text.replace("var benchCount: int = 8;", "var benchCount: int = %d;" % count, 1)
    destination.write_text(text)


def stageSources(stage):
    stage.mkdir(parents=True, exist_ok=True)
    for name in ("layout", "regions", "managed", "containers", "host"):
        target = stage / name
        target.mkdir(parents=True, exist_ok=True)
        source = RUNTIME_ROOT / name / "main.rey"
        (target / "main.rey").write_text(source.read_text())


def emitImage(driver, fixture, stage, image):
    completed = runMemcap([str(driver), str(fixture), str(stage), str(image)])
    if completed.returncode != 0 or not image.is_file():
        raise SystemExit("emit failed for " + image.name)
    os.chmod(image, 0o755)
    signed = subprocess.run(["/usr/bin/codesign", "--sign", "-", "--force", str(image)],
                            text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if signed.returncode != 0:
        sys.stderr.write(signed.stderr)
        raise SystemExit("codesign failed for " + image.name)


def parseLine(line):
    fields = {}
    for part in line.split():
        if "=" not in part:
            continue
        key, value = part.split("=", 1)
        fields[key] = int(value)
    required = ("kind", "mode", "n", "live", "payload", "groups", "requested", "us")
    missing = [name for name in required if name not in fields]
    if missing:
        raise SystemExit("benchmark line missing %s: %s" % (",".join(missing), line))
    return fields


def childPid(parent):
    completed = subprocess.run(["/usr/bin/pgrep", "-P", str(parent)], text=True,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    pids = [int(item) for item in completed.stdout.split() if item.strip().isdigit()]
    return pids[0] if pids else 0


def residentBytes(parent):
    held = childPid(parent)
    child = sampleRss(held) if held else 0
    parentRss = sampleRss(parent)
    return child if child > parentRss else parentRss


def executeImage(image):
    proc = subprocess.Popen([str(MEMCAP), str(image)], stdin=subprocess.DEVNULL,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    os.set_blocking(proc.stdout.fileno(), False)
    started = time.perf_counter()
    chunks = []
    seenLine = False
    retained = 0
    peakSeen = 0
    while not seenLine:
        ready, _, _ = select.select([proc.stdout], [], [], 0.2)
        rss = residentBytes(proc.pid)
        if rss > peakSeen:
            peakSeen = rss
        if ready:
            try:
                chunk = os.read(proc.stdout.fileno(), 4096)
            except BlockingIOError:
                chunk = b""
            if chunk:
                chunks.append(chunk)
                if b"\n" in b"".join(chunks):
                    seenLine = True
                    break
        if proc.poll() is not None and not ready:
            break
    elapsed = time.perf_counter() - started
    deadline = time.perf_counter() + 2.5
    while time.perf_counter() < deadline and proc.poll() is None:
        rss = residentBytes(proc.pid)
        if rss > retained:
            retained = rss
        if rss > peakSeen:
            peakSeen = rss
        time.sleep(0.1)
    os.set_blocking(proc.stdout.fileno(), True)
    rest = b""
    try:
        rest = proc.stdout.read()
    except Exception:
        rest = b""
    stderr = b""
    try:
        stderr = proc.stderr.read()
    except Exception:
        stderr = b""
    proc.wait(timeout=120)
    text = (b"".join(chunks) + (rest or b"")).decode(errors="replace")
    lines = [item for item in text.splitlines() if item.strip()]
    if not lines:
        sys.stderr.write(stderr.decode(errors="replace"))
        sys.stderr.write(text)
        raise SystemExit("benchmark failed (%s): %s" % (proc.returncode, image.name))
    fields = parseLine(lines[0])
    peak = peakSeen if peakSeen > retained else retained
    for line in lines[1:]:
        if line.startswith("peak_rss_bytes="):
            reported = int(line.split("=", 1)[1])
            if reported > peak:
                peak = reported
    if proc.returncode != 0:
        sys.stderr.write(stderr.decode(errors="replace"))
        sys.stderr.write(text)
        raise SystemExit("benchmark failed (%s): %s" % (proc.returncode, image.name))
    fields["wallSeconds"] = round(elapsed, 6)
    fields["peakRssBytes"] = peak
    fields["retainedRssBytes"] = retained
    fields["image"] = image.name
    return fields


def sha256(path):
    digest = hashlib.sha256()
    digest.update(path.read_bytes())
    return digest.hexdigest()


def main():
    if len(sys.argv) != 3 or sys.argv[1] not in ("before", "after"):
        raise SystemExit("usage: run.py before|after driverPath")
    phase = sys.argv[1]
    driver = Path(sys.argv[2])
    if not driver.is_file():
        raise SystemExit("driver missing: " + str(driver))
    if not SEED.is_file():
        raise SystemExit("pinned rey seed is missing")
    OUTPUT.mkdir(parents=True, exist_ok=True)
    phaseDir = OUTPUT / phase
    phaseDir.mkdir(parents=True, exist_ok=True)
    stage = phaseDir / "source"
    records = []
    for name, kind, count in CASES:
        fixture = phaseDir / (name + ".rey")
        image = phaseDir / name
        writeWorkload(kind, count, fixture)
        stageSources(stage)
        print("emit", name, flush=True)
        emitImage(driver, fixture, stage, image)
        print("run", name, flush=True)
        fields = executeImage(image)
        fields["name"] = name
        records.append(fields)
        print(name, fields, flush=True)
    report = {
        "schema": "rey-v04-region-bench-v1",
        "phase": phase,
        "regionsSha256": sha256(RUNTIME_ROOT / "regions" / "main.rey"),
        "records": records,
    }
    (phaseDir / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print("region-bench", phase, "done", len(records), flush=True)


if __name__ == "__main__":
    main()
