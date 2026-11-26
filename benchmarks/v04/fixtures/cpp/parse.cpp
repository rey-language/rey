#include <cerrno>
#include <cstdint>
#include <cstdio>
#include <cstdlib>

static int64_t parse_i64(const char *text, bool *ok) {
    char *end = nullptr;
    errno = 0;
    long long value = std::strtoll(text, &end, 10);
    *ok = errno == 0 && end != text && *end == '\0';
    return static_cast<int64_t>(value);
}

int main(int argc, char **argv) {
    if (argc != 10) return 2;
    bool ok = false;
    int64_t size = parse_i64(argv[1], &ok);
    if (!ok || size < 0 || size > 5000000) return 2;
    int64_t checksum = 0;
    for (int64_t i = 0; i < size; ++i) {
        int64_t value = parse_i64(argv[(i % 8) + 2], &ok);
        if (!ok) return 2;
        checksum += value;
    }
    std::printf("checksum=%lld processed=%lld\n", static_cast<long long>(checksum), static_cast<long long>(size));
    return 0;
}
