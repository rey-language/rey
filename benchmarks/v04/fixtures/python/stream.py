import sys

if len(sys.argv) != 2:
    raise SystemExit(2)
size = int(sys.argv[1])
if size < 0 or size > 10_000_000:
    raise SystemExit(2)
state = 1
checksum = 0
for _ in range(size):
    state = (state * 48271) % 2147483647
    if state % 7 == 0:
        checksum = (checksum * 31 + state) % 2147483647
print(f"checksum={checksum} processed={size}")
