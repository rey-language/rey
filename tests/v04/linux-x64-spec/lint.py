#!/usr/bin/env python3
"""Lint the v0.4 Linux x64 SPEC surface manifest without invoking a compiler."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import sys
from pathlib import Path, PurePosixPath
from typing import Any


COVERAGE_BEGIN = "<!-- spec-coverage-json:start -->"
COVERAGE_END = "<!-- spec-coverage-json:end -->"
EXAMPLES_BEGIN = "<!-- spec-examples-json:start -->"
EXAMPLES_END = "<!-- spec-examples-json:end -->"

# These byte strings are the frozen Wave 5 §7 stdout oracle. JSON decoding
# below produces the exact bytes, including the final LF, that the examples
# suite will compare in the later native execution slice.
EXPECTED_EXAMPLES = {
    "examples/basic/hello.rey": "hello, rey!\n",
    "examples/basic/variables.rey": (
        "rey version 0.1.1\n"
        "pi = 3.14\n"
        "max hp = 100\n"
        "3\n"
        "3.3333333333333335\n"
    ),
    "examples/basic/functions.rey": (
        "hello, world!\n"
        "hello, rey...\n"
        "10\n"
        "10\n"
    ),
    "examples/advanced/self_cure_stub.rey": (
        "fix soon\n"
        "E001: undefined variable\n"
        "W001: unused import\n"
        "E002: type mismatch\n"
    ),
}

HEADING_RE = re.compile(r"^#{1,3}\s+(\d+(?:\.\d+)*)(?:\.)?\s+(.+?)\s*$")
FENCED_JSON_RE = re.compile(r"\A\s*```json\s*\n(.*?)\n```\s*\Z", re.DOTALL)


class ManifestError(Exception):
    pass


def extract_json_block(document: str, begin: str, end: str, label: str) -> Any:
    if document.count(begin) != 1 or document.count(end) != 1:
        raise ManifestError(f"expected one marked {label} JSON block")
    start = document.index(begin) + len(begin)
    finish = document.index(end)
    if finish <= start:
        raise ManifestError(f"invalid marked {label} JSON block")
    match = FENCED_JSON_RE.fullmatch(document[start:finish].strip())
    if not match:
        raise ManifestError(f"{label} block must contain one JSON fence")
    try:
        return json.loads(match.group(1))
    except json.JSONDecodeError as exc:
        raise ManifestError(f"invalid {label} JSON: {exc}") from exc


def read_spec_headings(spec_path: Path) -> dict[str, str]:
    headings: dict[str, str] = {}
    for line in spec_path.read_text(encoding="utf-8").splitlines():
        match = HEADING_RE.match(line)
        if not match:
            continue
        section_id, title = match.groups()
        if section_id in headings:
            raise ManifestError(f"duplicate numbered SPEC heading: {section_id}")
        headings[section_id] = title
    if not headings:
        raise ManifestError("SPEC.md has no numbered sections")
    return headings


def resolve_manifest_path(root: Path, value: Any, context: str) -> None:
    if not isinstance(value, str) or not value:
        raise ManifestError(f"{context}: expected a non-empty relative path")
    rel = PurePosixPath(value)
    if rel.is_absolute() or ".." in rel.parts:
        raise ManifestError(f"{context}: path must stay inside source: {value}")
    candidate = root.joinpath(*rel.parts)
    try:
        resolved = candidate.resolve(strict=True)
        resolved.relative_to(root.resolve())
    except (FileNotFoundError, ValueError, OSError) as exc:
        raise ManifestError(f"{context}: path does not resolve inside source: {value}") from exc
    if not resolved.is_file():
        raise ManifestError(f"{context}: not a file: {value}")


def validate_manifest(
    root: Path,
    headings: dict[str, str],
    families: Any,
    examples: Any,
) -> list[str]:
    errors: list[str] = []
    if not isinstance(families, list):
        return ["coverage manifest must be a JSON array"]

    rows: dict[str, dict[str, Any]] = {}
    for index, row in enumerate(families):
        if not isinstance(row, dict):
            errors.append(f"coverage row {index + 1} must be an object")
            continue
        section_id = row.get("id")
        if not isinstance(section_id, str) or not section_id:
            errors.append(f"coverage row {index + 1} has no section id")
            continue
        if section_id in rows:
            errors.append(f"duplicate coverage row: {section_id}")
            continue
        rows[section_id] = row

    missing = sorted(set(headings) - set(rows), key=lambda item: [int(n) for n in item.split(".")])
    extra = sorted(set(rows) - set(headings))
    if missing:
        errors.append("missing normative SPEC rows: " + ", ".join(missing))
    if extra:
        errors.append("unknown SPEC row ids: " + ", ".join(extra))

    for section_id, row in rows.items():
        expected_title = headings.get(section_id)
        if expected_title is not None and row.get("family") != expected_title:
            errors.append(
                f"{section_id}: family must match SPEC heading {expected_title!r}"
            )
        status = row.get("status")
        if status not in ("partial", "covered"):
            errors.append(f"{section_id}: status must be 'partial' or 'covered'")
        positive = row.get("positive")
        negative = row.get("negative")
        if not isinstance(positive, list) or not isinstance(negative, list):
            errors.append(f"{section_id}: positive and negative must be arrays")
            continue
        for polarity, cases in (("positive", positive), ("negative", negative)):
            for case_index, case in enumerate(cases):
                context = f"{section_id} {polarity} case {case_index + 1}"
                if not isinstance(case, dict):
                    errors.append(f"{context}: expected an object")
                    continue
                if case.get("mode") not in ("native-check", "native-run"):
                    errors.append(f"{context}: mode must be native-check or native-run")
                try:
                    resolve_manifest_path(root, case.get("path"), context)
                    if case.get("expected") is not None:
                        resolve_manifest_path(root, case["expected"], context + " expected result")
                    elif polarity == "negative":
                        raise ManifestError(f"{context}: negative case is missing its expected result")
                except ManifestError as exc:
                    errors.append(str(exc))

        gap = row.get("gap")
        if status == "covered":
            if not positive or not negative:
                errors.append(f"{section_id}: covered rows need positive and negative cases")
            if gap is not None:
                errors.append(f"{section_id}: covered row must not retain a gap")
        else:
            if not isinstance(gap, dict):
                errors.append(f"{section_id}: partial row needs a named gap and owner")
            elif not all(
                isinstance(gap.get(key), str) and gap[key].strip()
                for key in ("issue", "owner", "detail")
            ):
                errors.append(f"{section_id}: gap needs non-empty issue, owner, and detail")

    if not isinstance(examples, dict):
        errors.append("example stdout manifest must be an object")
    else:
        if set(examples) != set(EXPECTED_EXAMPLES):
            missing_examples = sorted(set(EXPECTED_EXAMPLES) - set(examples))
            extra_examples = sorted(set(examples) - set(EXPECTED_EXAMPLES))
            if missing_examples:
                errors.append("missing example stdout: " + ", ".join(missing_examples))
            if extra_examples:
                errors.append("unknown example stdout: " + ", ".join(extra_examples))
        for example_path, expected_stdout in EXPECTED_EXAMPLES.items():
            if example_path not in examples:
                continue
            resolve_manifest_path(root, example_path, "example source")
            actual_stdout = examples[example_path]
            if not isinstance(actual_stdout, str) or actual_stdout != expected_stdout:
                errors.append(f"{example_path}: stdout differs from the frozen exact output")
            elif not actual_stdout.endswith("\n"):
                errors.append(f"{example_path}: stdout must include its final LF")

    return errors


def run_lint_self_test(
    root: Path,
    headings: dict[str, str],
    families: list[dict[str, Any]],
    examples: dict[str, str],
) -> None:
    if not families:
        raise ManifestError("cannot self-test an empty coverage manifest")
    missing_row = copy.deepcopy(families)
    missing_row.pop()
    if not validate_manifest(root, headings, missing_row, examples):
        raise ManifestError("lint self-test failed to reject a removed coverage row")

    missing_expected_case = copy.deepcopy(families)
    found_negative = False
    for row in missing_expected_case:
        if row.get("negative"):
            del row["negative"][0]["expected"]
            found_negative = True
            break
    if not found_negative or not validate_manifest(
        root, headings, missing_expected_case, examples
    ):
        raise ManifestError("lint self-test failed to reject a removed negative expected result")

    missing_example_result = copy.deepcopy(examples)
    missing_example_result.pop(next(iter(missing_example_result)))
    if not validate_manifest(root, headings, families, missing_example_result):
        raise ManifestError("lint self-test failed to reject a removed example stdout result")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lint", action="store_true", help="lint manifest and run mutation self-tests")
    parser.add_argument("--source", required=True, type=Path, help="source checkout root")
    parser.add_argument("--out", required=True, type=Path, help="external evidence directory")
    args = parser.parse_args()
    if not args.lint:
        parser.error("--lint is required for this slice")

    root = args.source.resolve()
    spec_path = root / "SPEC.md"
    doc_path = root / "docs/V0.4-LINUX-X64-SURFACE.md"
    if not root.is_dir() or not spec_path.is_file() or not doc_path.is_file():
        print("SPEC manifest: source must contain SPEC.md and docs/V0.4-LINUX-X64-SURFACE.md", file=sys.stderr)
        return 1

    try:
        headings = read_spec_headings(spec_path)
        document = doc_path.read_text(encoding="utf-8")
        families = extract_json_block(document, COVERAGE_BEGIN, COVERAGE_END, "coverage")
        examples = extract_json_block(document, EXAMPLES_BEGIN, EXAMPLES_END, "example stdout")
        errors = validate_manifest(root, headings, families, examples)
        if errors:
            raise ManifestError("\n".join(errors))
        run_lint_self_test(root, headings, families, examples)

        output_dir = args.out.resolve()
        output_dir.mkdir(parents=True, exist_ok=True)
        manifest_bytes = doc_path.read_bytes()
        report = {
            "result": "PASS SPEC manifest",
            "spec": "SPEC.md",
            "spec_sha256": hashlib.sha256(spec_path.read_bytes()).hexdigest(),
            "manifest_sha256": hashlib.sha256(manifest_bytes).hexdigest(),
            "normative_families": len(families),
            "example_stdout_records": len(examples),
            "lint_self_tests": [
                "removed coverage row rejected",
                "removed negative expected result rejected",
                "removed example stdout rejected",
            ],
        }
        report_bytes = (json.dumps(report, indent=2, sort_keys=True) + "\n").encode("utf-8")
        report_path = output_dir / (
            "manifest-lint-" + hashlib.sha256(manifest_bytes).hexdigest()[:12] + ".json"
        )
        if report_path.exists():
            if report_path.read_bytes() != report_bytes:
                raise ManifestError(f"existing evidence differs: {report_path}")
        else:
            report_path.write_bytes(report_bytes)
    except (ManifestError, OSError, UnicodeError) as exc:
        print(f"SPEC manifest: {exc}", file=sys.stderr)
        return 1

    print("PASS SPEC manifest")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
