#!/usr/bin/env python3
"""Measure peak RSS for repeated region workloads.

Compiles each workload twice with --regions, at N and 10N iterations, and
records Darwin arm64 maximum resident set size. A plateau means the 10N run
stays within 1.5x of the N run and does not add more than 8 MiB.
"""
import json
import os
import platform
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SEED = ROOT / "build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"
RUNTIME = SEED.parent / "runtime"
RUNTIME_ROOT = ROOT / "runtime-rey"
WORKLOADS = ROOT / "tests/v04/memory-rss/workloads"
OUTPUT = ROOT / "build/v04/memory-rss"
BASE_ITERATIONS = 2000
GROWTH_LIMIT = 1.5
ABSOLUTE_SLACK = 8 * 1024 * 1024


def run(args, cwd=None):
    completed = subprocess.run(args, cwd=cwd, env=os.environ.copy(), text=True,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if completed.returncode != 0:
        sys.stderr.write(completed.stdout)
        sys.stderr.write(completed.stderr)
        raise SystemExit("command failed: " + " ".join(args))
    return completed


def peakRss(image):
    completed = subprocess.run(["/usr/bin/time", "-l", str(image)], text=True,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if completed.returncode != 0:
        sys.stderr.write(completed.stdout)
        sys.stderr.write(completed.stderr)
        raise SystemExit("workload failed: " + str(image))
    match = re.search(r"(\d+)\s+maximum resident set size", completed.stderr)
    if not match:
        raise SystemExit("time -l did not report maximum resident set size")
    return int(match.group(1))


def writeLimited(source, destination, limit):
    text = source.read_text()
    updated, count = re.subn(r"var limit: int = \d+;", "var limit: int = %d;" % limit, text, count=1)
    if count != 1:
        raise SystemExit("missing iteration limit in " + source.name)
    destination.write_text(updated)


def main():
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        raise SystemExit("memory-rss requires Darwin arm64")
    if not SEED.is_file() or not RUNTIME.is_dir():
        raise SystemExit("pinned rey seed is missing")
    OUTPUT.mkdir(parents=True, exist_ok=True)
    compiler = OUTPUT / "compiler"
    if compiler.exists():
        shutil.rmtree(compiler)
    shutil.copytree(ROOT / "rey-compiler/src", compiler)
    shutil.copyfile(ROOT / "rey-compiler/native-main.rey", compiler / "main.rey")
    driver = OUTPUT / "native-driver"
    env = os.environ.copy()
    env["PATH"] = "/usr/bin:/bin:/usr/sbin:/sbin"
    env["CC"] = "/usr/bin/clang"
    env["CXX"] = "/usr/bin/clang++"
    built = subprocess.run([str(SEED), "build", str(compiler / "main.rey"), "-o", str(driver),
                            "--runtime-dir", str(RUNTIME)], env=env, text=True,
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if built.returncode != 0:
        sys.stderr.write(built.stdout)
        sys.stderr.write(built.stderr)
        raise SystemExit("native driver build failed")
    os.chmod(driver, 0o755)
    records = []
    for source in sorted(WORKLOADS.glob("*.rey")):
        samples = {}
        for scale, limit in (("n", BASE_ITERATIONS), ("tenN", BASE_ITERATIONS * 10)):
            generated = OUTPUT / ("%s-%s.rey" % (source.stem, scale))
            image = OUTPUT / ("%s-%s" % (source.stem, scale))
            writeLimited(source, generated, limit)
            compiled = subprocess.run([str(driver), "build", str(generated), "-o", str(image),
                                       "--target", "darwin-arm64", "--managed",
                                       "--runtime-root", str(RUNTIME_ROOT)],
                                      env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            if compiled.returncode != 0:
                sys.stderr.write(compiled.stdout)
                sys.stderr.write(compiled.stderr)
                raise SystemExit("compile failed: " + source.name)
            os.chmod(image, 0o755)
            subprocess.run(["/usr/bin/codesign", "--sign", "-", "--force", str(image)], check=False,
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            samples[scale] = peakRss(image)
        ratio = samples["tenN"] / samples["n"] if samples["n"] else 999.0
        growth = samples["tenN"] - samples["n"]
        plateau = ratio <= GROWTH_LIMIT and growth <= ABSOLUTE_SLACK
        records.append({
            "workload": source.name,
            "iterations": BASE_ITERATIONS,
            "rssBytesN": samples["n"],
            "rssBytes10N": samples["tenN"],
            "ratio": round(ratio, 3),
            "growthBytes": growth,
            "plateau": plateau,
            "retained": "loop output is a scalar total; containers and iteration objects are not retained",
        })
        print(source.name, samples["n"], samples["tenN"], "ratio", round(ratio, 3), "plateau", plateau, flush=True)
    passed = all(item["plateau"] for item in records) and len(records) == 4
    report = {
        "schema": "rey-v04-memory-rss-v1",
        "passed": passed,
        "host": "darwin-arm64",
        "baseIterations": BASE_ITERATIONS,
        "growthLimit": GROWTH_LIMIT,
        "absoluteSlackBytes": ABSOLUTE_SLACK,
        "records": records,
    }
    (OUTPUT / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print("memory-rss", "PASS" if passed else "FAIL", len(records), "workloads", flush=True)
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
