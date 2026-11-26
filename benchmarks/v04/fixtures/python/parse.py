import sys

TOKENS = ("11", "-23", "37", "-41", "53", "-67", "79", "-89")

if len(sys.argv) != 10:
    raise SystemExit(2)
size = int(sys.argv[1])
if size < 0 or size > 5_000_000:
    raise SystemExit(2)
checksum = 0
for index in range(size):
    checksum += int(sys.argv[(index % 8) + 2], 10)
print(f"checksum={checksum} processed={size}")
