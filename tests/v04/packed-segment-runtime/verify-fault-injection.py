from pathlib import Path
import subprocess
import sys

binary = Path(sys.argv[1])
out = Path(sys.argv[2])
expected = b"\x00\x80\xffA" + bytes((i * 131 + 17) & 0xff for i in range(5003))

eintr_path = out / "eintr-short-writes.bin"
eintr_path.write_bytes(b"old contents")
subprocess.run([str(binary), str(eintr_path), "e"], check=True, timeout=60)
if eintr_path.read_bytes() != expected:
    raise SystemExit("EINTR plus short writes did not produce exact packed output")

failure_path = out / "persistent-write-failure.bin"
failure_path.write_bytes(b"old contents")
result = subprocess.run([str(binary), str(failure_path), "f"], check=False, timeout=60)
if result.returncode == 0:
    raise SystemExit("persistent host write failure was incorrectly reported as success")
if failure_path.read_bytes() != expected[:5]:
    raise SystemExit("hard write failure did not leave the expected five-byte partial prefix")

print("packed segment hostWrite EINTR, short-write, and failure checks passed")
