from pathlib import Path
import sys

path = Path(sys.argv[1])
source = path.read_text()
needle = """pub func hostWrite(fd: int, base: __rey_raw_address, length: int): int {
    return __reyRawToInt(__reyRawHostSyscall(1, hostRawInt(fd), base, hostRawInt(length), __reyRawNull(), __reyRawNull(), __reyRawNull()));
}
"""
replacement = """var hostTestWriteMode = 0;
var hostTestWriteCalls = 0;

pub func hostTestSetWriteMode(mode: int): void {
    hostTestWriteMode = mode;
    hostTestWriteCalls = 0;
}

func hostTestRealWrite(fd: int, base: __rey_raw_address, length: int): int {
    return __reyRawToInt(__reyRawHostSyscall(1, hostRawInt(fd), base, hostRawInt(length), __reyRawNull(), __reyRawNull(), __reyRawNull()));
}

pub func hostWrite(fd: int, base: __rey_raw_address, length: int): int {
    if hostTestWriteMode == 1 {
        if hostTestWriteCalls == 0 {
            hostTestWriteCalls = 1;
            return -4;
        }
        hostTestWriteCalls = hostTestWriteCalls + 1;
        var shortLength = length;
        if shortLength > 13 { shortLength = 13; }
        return hostTestRealWrite(fd, base, shortLength);
    }
    if hostTestWriteMode == 2 {
        if hostTestWriteCalls == 0 {
            hostTestWriteCalls = 1;
            var shortLength = length;
            if shortLength > 5 { shortLength = 5; }
            return hostTestRealWrite(fd, base, shortLength);
        }
        hostTestWriteCalls = hostTestWriteCalls + 1;
        return -5;
    }
    return hostTestRealWrite(fd, base, length);
}
"""
count = source.count(needle)
if count != 1:
    raise SystemExit(f"expected one Linux hostWrite definition in staged module, found {count}")
path.write_text(source.replace(needle, replacement))
