#!/usr/bin/env python3
"""Static checks for the v0.4 region-only production path.

This is not a build or execution gate. It inspects source and optional IR text
for forbidden collector coupling and required region interfaces.
"""
import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

REGION_RUNTIME = ROOT / "runtime-rey/regions/main.rey"
MANAGED_RUNTIME = ROOT / "runtime-rey/managed/main.rey"
CONTAINER_RUNTIME = ROOT / "runtime-rey/containers/main.rey"
RUNTIME_ENTRY = ROOT / "runtime-rey/main.rey"
RUNTIME_FILES = [REGION_RUNTIME, MANAGED_RUNTIME, CONTAINER_RUNTIME, RUNTIME_ENTRY]

REGION_ENGINE_FILES = [REGION_RUNTIME, CONTAINER_RUNTIME, RUNTIME_ENTRY]
FORBIDDEN_RUNTIME = [
    r"\bimport\s+gc\b",
    r"\bimport\s+heap\b",
    r"\bgcCollect\b",
    r"\bgcCreate\b",
    r"\bgcDestroy\b",
    r"\bgcRegisterRootRange\b",
    r"\bgcUnregisterRootRange\b",
    r"\bgcRootPush\b",
    r"\bgcRootSet\b",
    r"\bgcRootPop\b",
    r"\bheapAllocate",
    r"\bheapCreate\b",
    r"\bheapDestroy\b",
    r"\bmanagedCollect\b",
    r"\bmanagedRootPush\b",
    r"\bmanagedRootSet\b",
    r"\bmanagedRootPop\b",
]
FORBIDDEN_MANAGED = [
    r"\bimport\s+gc\b",
    r"\bimport\s+heap\b",
    r"\bgcCollect\b",
    r"\bgcCreate\b",
    r"\bgcDestroy\b",
    r"\bgcRegisterRootRange\b",
    r"\bgcUnregisterRootRange\b",
    r"\bgcRootPush\b",
    r"\bgcRootSet\b",
    r"\bgcRootPop\b",
    r"\bheapAllocate",
    r"\bheapCreate\b",
    r"\bheapDestroy\b",
    r"\bmanagedCollect\b",
]
REQUIRED_RUNTIME = [
    "managedCreate",
    "managedDestroy",
    "managedProgramRegion",
    "managedCurrentRegion",
    "managedRegionCreate",
    "managedRegionCreateChild",
    "managedRegionEnd",
    "managedRegionAllocate",
    "managedAllocateDestination",
    "managedAllocateProgram",
    "managedStoreReference",
    "managedPromoteReference",
    "managedPromotionCount",
    "managedDependencyCount",
    "managedRegionGroupCount",
    "managedLiveObjects",
    "managedLivePayloadBytes",
    "managedRequestedObjectBytes",
    "managedRegionRequestedBytes",
    "managedRegionUsedBytes",
    "managedRegionObjectCount",
]

REGION_LOWERING = ROOT / "rey-compiler/src/ir/managed/regions/main.rey"
LIFETIME_VERIFIER = ROOT / "rey-compiler/src/ir/managed/lifetime/main.rey"
LINKER = ROOT / "rey-compiler/src/ir/managed/link/main.rey"
NATIVE_DRIVER = ROOT / "rey-compiler/native-main.rey"
IR_DRIVER = ROOT / "rey-compiler/ir-main.rey"
COMPILER_FILES = [REGION_LOWERING, LIFETIME_VERIFIER, LINKER, NATIVE_DRIVER, IR_DRIVER]

FORBIDDEN_COMPILER_ENGINE = [
    r"\bmanagedCollect\b",
    r"\bmanagedRootPush\b",
    r"\bmanagedRootSet\b",
    r"\bmanagedRootPop\b",
    r"\bgcCollect\b",
    r"\bheapAllocate",
    r"runtime-rey/gc",
    r"runtime-rey/heap",
]
IR_FORBIDDEN = [
    r"\bmanagedCollect\b",
    r"\bmanagedRootPush\b",
    r"\bmanagedRootSet\b",
    r"\bmanagedRootPop\b",
    r"\bgcCollect\b",
    r"\bgcRegisterRootRange\b",
    r"\bheapAllocate",
]
IR_REQUIRED = [
    "managedRegionAllocate",
    "managedStoreReference",
    "managedCreate",
    "managedDestroy",
]


def missing(path):
    return not path.is_file()


def scan(paths, patterns):
    hits = []
    for path in paths:
        if missing(path):
            continue
        for index, line in enumerate(path.read_text().splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith("//"):
                continue
            for pattern in patterns:
                if re.search(pattern, line):
                    hits.append(f"{path.relative_to(ROOT)}:{index}: {stripped}")
    return hits


def defined(paths, names):
    text = "\n".join(path.read_text() for path in paths if path.is_file())
    return [name for name in names if not re.search(r"\bfunc\s+" + re.escape(name) + r"\b", text)]


def function_body(text, name):
    match = re.search(r"\bfunc\s+" + re.escape(name) + r"\s*\(", text)
    if not match:
        return ""
    start = text.find("{", match.end())
    if start < 0:
        return ""
    depth = 0
    for index in range(start, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return text[start:index + 1]
    return ""


def runtime_checks(errors):
    for path in RUNTIME_FILES:
        if missing(path):
            errors.append(f"missing required source: {path.relative_to(ROOT)}")
    errors.extend(scan(REGION_ENGINE_FILES, FORBIDDEN_RUNTIME))
    errors.extend(scan([MANAGED_RUNTIME], FORBIDDEN_MANAGED))
    absent = defined([MANAGED_RUNTIME], REQUIRED_RUNTIME)
    if absent:
        errors.append("missing runtime API: " + ", ".join(absent))

    # Root shims are allowed only in the managed facade and must not publish.
    if MANAGED_RUNTIME.is_file():
        text = MANAGED_RUNTIME.read_text()
        for name in ("managedRootPush", "managedRootSet", "managedRootPop"):
            body = function_body(text, name)
            if body and re.search(r"__reyRawStore|gc|heap", body):
                errors.append(f"managed facade root shim {name} publishes or stores state")

    # The production region engine must actually contain promotion and release.
    if REGION_RUNTIME.is_file():
        text = REGION_RUNTIME.read_text()
        for name in ("regionPromote", "regionStoreReference", "regionEnd", "regionCreateChild"):
            if not re.search(r"\bfunc\s+" + name + r"\b", text):
                errors.append(f"missing region runtime interface: {name}")


def compiler_checks(errors):
    for path in COMPILER_FILES:
        if missing(path):
            errors.append(f"missing required source: {path.relative_to(ROOT)}")
    errors.extend(scan([REGION_LOWERING, LIFETIME_VERIFIER], FORBIDDEN_COMPILER_ENGINE))

    if REGION_LOWERING.is_file():
        text = REGION_LOWERING.read_text()
        for name in ("lowerManagedRegions", "lowerManagedRegionsWithPromotionStress"):
            if not re.search(r"\bfunc\s+" + name + r"\b", text):
                errors.append(f"missing compiler interface: {name}")
        body = function_body(text, "lowerUserFunction")
        if "bindings.promoteReference" not in body:
            errors.append("region lowering has no return promotion")
        if "TerminatorKind::Return" not in body:
            errors.append("region lowering does not guard return promotion by terminator")
        promote_at = body.find("bindings.promoteReference")
        end_at = body.find("bindings.regionEnd")
        if promote_at < 0 or end_at < 0 or promote_at > end_at:
            errors.append("return promotion does not precede region end in lowering")

    if LIFETIME_VERIFIER.is_file():
        text = LIFETIME_VERIFIER.read_text()
        if not re.search(r"\bfunc\s+verifyManagedRegionPlan\b", text):
            errors.append("missing compiler interface: verifyManagedRegionPlan")
        if not re.search(r"returned region allocation", text):
            errors.append("lifetime verifier does not reject unproven returned region allocations")
        if not re.search(r"direct raw store", text):
            errors.append("lifetime verifier does not reject raw reference stores")

    if LINKER.is_file():
        text = LINKER.read_text()
        if not re.search(r"\bfunc\s+buildRegionProgram\b", text):
            errors.append("missing compiler interface: buildRegionProgram")
        body = function_body(text, "buildRegionProgram")
        if "lowerManagedRegionsWithPromotionStress" not in body:
            errors.append("buildRegionProgram does not use the region lowerer")
        if not re.search(r"\bfunc\s+buildCompletedRegionProgram\b", text):
            errors.append("missing compiler interface: buildCompletedRegionProgram")
        completed = function_body(text, "buildCompletedRegionProgram")
        for required in ("applyScalarReplacement", "completeManagedMemory", "lowerManagedRegionsCompleted"):
            if required not in completed:
                errors.append("buildCompletedRegionProgram omits " + required)
        units = function_body(text, "regionRuntimeUnits")
        if "/regions/main.rey" not in units or "/managed/main.rey" not in units:
            errors.append("region runtime units omit the production runtime")
        if "/gc/" in units or "/heap/" in units:
            errors.append("region runtime units load gc or heap")
        load = function_body(text, "loadRegionRuntime")
        if "regionRuntimeUnits" not in load:
            errors.append("loadRegionRuntime does not use regionRuntimeUnits")
        legacy = function_body(text, "buildManagedProgram")
        if "buildRegionProgram" not in legacy or "does not lower region placements" not in legacy:
            errors.append("legacy builder does not fail closed on region placements")
        if "forbiddenRuntimeProgram" not in text:
            errors.append("region linker has no gc/heap runtime guard")

    for path in (NATIVE_DRIVER, IR_DRIVER):
        if not path.is_file():
            continue
        text = path.read_text()
        if "buildCompletedRegionProgram" not in text:
            errors.append(f"{path.relative_to(ROOT)} does not use buildCompletedRegionProgram")
        if not re.search(r"buildCompletedRegionProgram\s*\([^;]*promotionStress", text, re.S):
            errors.append(f"{path.relative_to(ROOT)} does not pass promotionStress to buildCompletedRegionProgram")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--ir", action="append", default=[], help="IR text dump to inspect")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    report = {"ok": True, "errors": []}
    runtime_checks(report["errors"])
    compiler_checks(report["errors"])
    for path in args.ir:
        p = Path(path)
        if not p.is_file():
            report["errors"].append(f"missing IR dump: {path}")
            continue
        text = p.read_text()
        for pattern in IR_FORBIDDEN:
            if re.search(pattern, text):
                report["errors"].append(f"{path}: forbidden generated symbol {pattern}")
        for name in IR_REQUIRED:
            if name not in text:
                report["errors"].append(f"{path}: missing generated symbol {name}")
    report["ok"] = not report["errors"]
    if args.json:
        print(json.dumps(report, indent=2))
    else:
        if report["ok"]:
            print("v04 region contract check ok")
        else:
            for error in report["errors"]:
                print("FAIL " + error)
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
