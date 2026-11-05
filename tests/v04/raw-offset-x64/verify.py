import pathlib
import os
import subprocess
import sys


def main() -> None:
    directory = pathlib.Path(sys.argv[1])
    expected = {
        "valid": 103,
        "negative-base": 1,
        "negative-offset": 1,
        "overflow": 1,
        "null-base": 1,
    }
    for name, status in expected.items():
        path = directory / f"{name}.elf"
        data = path.read_bytes()
        if not data.startswith(b"\x7fELF"):
            raise SystemExit(f"{name}: output is not an ELF image")
        path.chmod(0o755)
        result = subprocess.run([str(path)], check=False)
        if result.returncode != status:
            raise SystemExit(f"{name}: expected exit {status}, got {result.returncode}")
        print(f"native raw-offset {name}: exit {result.returncode}")


if __name__ == "__main__":
    main()
