#!/usr/bin/env python3
"""Stage verbatim runtime functions reachable from the scratch probe APIs."""

from pathlib import Path
import re
import sys


MODULES = ("layout", "managed", "regions")
ROOTS = {
    "layout": ("layoutRegistryCreate", "layoutRegisterFixed", "layoutRegistryRelease"),
    "managed": (
        "managedCreate", "managedDestroy", "managedProgramRegion",
        "managedRegionAllocate", "managedRegionCreateCohort", "managedRegionEnd",
        "managedRegionScratchUsed", "managedLivePayloadBytes",
    ),
}


def find_function_end(source, opening):
    depth = 0
    quote = None
    escaped = False
    line_comment = False
    block_comment = False
    index = opening
    while index < len(source):
        char = source[index]
        next_char = source[index + 1] if index + 1 < len(source) else ""
        if line_comment:
            if char == "\n":
                line_comment = False
        elif block_comment:
            if char == "*" and next_char == "/":
                block_comment = False
                index += 1
        elif quote is not None:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
        elif char == "/" and next_char == "/":
            line_comment = True
            index += 1
        elif char == "/" and next_char == "*":
            block_comment = True
            index += 1
        elif char in ("\"", "'"):
            quote = char
        elif char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return index + 1
        index += 1
    raise ValueError("unterminated function body")


def parse_module(name, path):
    source = path.read_text()
    imported = {}
    for match in re.finditer(r"(?m)^import\s+(\w+)::\{([^}]*)\};", source):
        target = match.group(1)
        for symbol in match.group(2).split(","):
            imported[symbol.strip()] = target
    functions = {}
    order = []
    pattern = re.compile(r"(?m)^(?:pub\s+)?func\s+([A-Za-z_]\w*)\s*\(")
    for match in pattern.finditer(source):
        opening = source.find("{", match.end())
        if opening < 0:
            raise ValueError(f"missing function body in {path}:{match.group(1)}")
        end = find_function_end(source, opening)
        function_name = match.group(1)
        if function_name in functions:
            raise ValueError(f"duplicate function {function_name} in {path}")
        functions[function_name] = {
            "source": source[match.start():end],
            "body": source[opening + 1:end - 1],
        }
        order.append(function_name)
    return {"name": name, "path": path, "imports": imported, "functions": functions, "order": order}


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: stage-runtime.py RUNTIME_ROOT DESTINATION")
    runtime_root, destination = map(Path, sys.argv[1:])
    modules = {
        name: parse_module(name, runtime_root / name / "main.rey")
        for name in MODULES
    }
    selected = {name: set() for name in MODULES}
    selected_imports = {name: {} for name in MODULES}
    pending = [(name, function) for name, functions in ROOTS.items() for function in functions]
    pending.extend(("regions", name) for name in (
        "regionContextCreate", "regionContextDestroy", "regionProgram",
        "regionAllocateFixed", "regionCreateCohort", "regionEnd",
        "regionScratchUsed", "regionLivePayloadBytes",
    ))
    while pending:
        module_name, function_name = pending.pop()
        module = modules[module_name]
        if function_name in selected[module_name]:
            continue
        function = module["functions"].get(function_name)
        if function is None:
            raise ValueError(f"missing reachable function {module_name}::{function_name}")
        selected[module_name].add(function_name)
        for call in re.findall(r"\b([A-Za-z_]\w*)\s*\(", function["body"]):
            if call in module["functions"]:
                pending.append((module_name, call))
            elif call in module["imports"]:
                target = module["imports"][call]
                if target not in modules:
                    raise ValueError(f"unsupported runtime dependency {module_name}::{call} -> {target}")
                selected_imports[module_name].setdefault(target, set()).add(call)
                pending.append((target, call))

    destination.mkdir(parents=True, exist_ok=True)
    counts = []
    for name in MODULES:
        module = modules[name]
        output = destination / name / "main.rey"
        output.parent.mkdir(parents=True, exist_ok=True)
        lines = []
        for target in sorted(selected_imports[name]):
            symbols = sorted(selected_imports[name][target])
            lines.append(f"import {target}::{{{', '.join(symbols)}}};")
        if lines:
            lines.append("")
        for function_name in module["order"]:
            if function_name in selected[name]:
                lines.append(module["functions"][function_name]["source"])
                lines.append("")
        output.write_text("\n".join(lines).rstrip() + "\n")
        counts.append(f"{name}={len(selected[name])}/{len(module['functions'])}")
    print("staged scratch runtime functions: " + " ".join(counts))


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError) as error:
        print("FAIL scratch runtime staging: " + str(error), file=sys.stderr)
        raise SystemExit(1)
