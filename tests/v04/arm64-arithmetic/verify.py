#!/usr/bin/env python3
import platform
import subprocess
import sys
from pathlib import Path


MIN_I64 = -(1 << 63)
MAX_I64 = (1 << 63) - 1


def arithmetic(operation, left, right):
    if operation in ("div", "rem") and right == 0:
        return None
    if operation in ("div", "rem") and left == MIN_I64 and right == -1:
        return None
    if operation == "sub":
        value = left - right
    elif operation == "mul":
        value = left * right
    elif operation == "div":
        value = abs(left) // abs(right)
        if (left < 0) != (right < 0):
            value = -value
    elif operation == "rem":
        quotient = abs(left) // abs(right)
        if (left < 0) != (right < 0):
            quotient = -quotient
        value = left - quotient * right
    else:
        raise AssertionError("unknown arithmetic operation: " + operation)
    if value < MIN_I64 or value > MAX_I64:
        return None
    return value


def expected_cases():
    cases = {
        "sub-min-valid": arithmetic("sub", MIN_I64, MIN_I64),
        "sub-max-zero": arithmetic("sub", MAX_I64, MAX_I64),
        "sub-overflow-positive": arithmetic("sub", MAX_I64, -1),
        "sub-overflow-negative": arithmetic("sub", MIN_I64, 1),
        "mul-positive-positive": arithmetic("mul", 7, 5),
        "mul-negative-positive": arithmetic("mul", -7, 5),
        "mul-positive-negative": arithmetic("mul", 7, -5),
        "mul-negative-negative": arithmetic("mul", -7, -5),
        "mul-min-times-negative-one": arithmetic("mul", MIN_I64, -1),
        "mul-large-overflow": arithmetic("mul", MAX_I64, 2),
        "mul-min-times-one": arithmetic("mul", MIN_I64, 1),
        "div-positive": arithmetic("div", 17, 5),
        "div-negative-numerator": arithmetic("div", -17, 5),
        "div-negative-denominator": arithmetic("div", 17, -5),
        "div-negative-both": arithmetic("div", -17, -5),
        "div-zero": arithmetic("div", 17, 0),
        "div-min-over-negative-one": arithmetic("div", MIN_I64, -1),
        "rem-positive": arithmetic("rem", 17, 5),
        "rem-negative-numerator": arithmetic("rem", -17, 5),
        "rem-negative-denominator": arithmetic("rem", 17, -5),
        "rem-negative-both": arithmetic("rem", -17, -5),
        "rem-zero": arithmetic("rem", 17, 0),
        "rem-min-over-negative-one": arithmetic("rem", MIN_I64, -1),
        "bool-eq-true": 2 if True == True else 0,
        "bool-eq-false": 2 if True == False else 0,
        "bool-ne-true": 2 if True != False else 0,
        "bool-ne-false": 2 if False != False else 0,
        "call-chain": 2,
    }
    return cases


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: verify.py OUTPUT_DIRECTORY")
    if sys.platform != "darwin" or platform.machine() != "arm64":
        raise SystemExit("native execution verification requires macOS arm64")

    output = Path(sys.argv[1])
    cases = expected_cases()
    actual_files = {path.name for path in output.iterdir() if path.is_file()}
    if actual_files != set(cases):
        raise SystemExit(
            "fixture image set mismatch: missing="
            + repr(sorted(set(cases) - actual_files))
            + " extra="
            + repr(sorted(actual_files - set(cases)))
        )

    for name, expected in cases.items():
        image = output / name
        image.chmod(0o755)
        signature = subprocess.run(
            ["/usr/bin/codesign", "--verify", "--strict", str(image)],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if signature.returncode != 0:
            raise SystemExit(name + " codesign verification failed: " + signature.stderr.strip())
        result = subprocess.run([str(image)], capture_output=True, text=True, timeout=10)
        expected_status = 1 if expected is None else expected % 256
        if result.returncode != expected_status:
            raise SystemExit(
                name
                + " returned status "
                + str(result.returncode)
                + ", expected "
                + str(expected_status)
                + (" (checked arithmetic trap)" if expected is None else " (value " + str(expected) + ")")
                + "; stderr="
                + result.stderr.strip()
            )
        print("PASS " + name + " status=" + str(result.returncode))

    print("arm64 arithmetic native executions passed: " + str(len(cases)))


if __name__ == "__main__":
    main()
