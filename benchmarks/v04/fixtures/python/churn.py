import sys
from array import array


def main():
    if len(sys.argv) != 2:
        return 2
    try:
        rounds = int(sys.argv[1], 10)
    except ValueError:
        return 2
    if rounds < 0 or rounds > 4096:
        return 2
    checksum = 0
    for round_index in range(rounds):
        values = array("q")
        for index in range(32):
            values.append((round_index * 17 + index * 13) % 1009)
        for value in values:
            checksum += value
    print(f"checksum={checksum} processed={rounds * 32}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
