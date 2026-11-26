#include <cstdint>
#include <cstdio>
#include <cstdlib>

int main(int argc, char **argv) {
    if (argc != 2) return 2;
    char *end = nullptr;
    long long parsed = std::strtoll(argv[1], &end, 10);
    if (end == argv[1] || *end != '\0' || parsed < 0 || parsed > 10000000) return 2;
    int64_t size = static_cast<int64_t>(parsed);
    int64_t state = 1;
    int64_t checksum = 0;
    for (int64_t i = 0; i < size; ++i) {
        state = (state * 48271) % 2147483647;
        if (state % 7 == 0) checksum = (checksum * 31 + state) % 2147483647;
    }
    std::printf("checksum=%lld processed=%lld\n", static_cast<long long>(checksum), static_cast<long long>(size));
    return 0;
}
