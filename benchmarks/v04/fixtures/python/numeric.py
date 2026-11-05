import sys


def main():
    if len(sys.argv) != 2:
        return 2
    try:
        size = int(sys.argv[1], 10)
    except ValueError:
        return 2
    if size < 0 or size > 10_000_000:
        return 2
    state = 1
    checksum = 0
    for _ in range(size):
        state = (state * 48271) % 2147483647
        checksum += state % 1024
    print(f"checksum={checksum} processed={size}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
