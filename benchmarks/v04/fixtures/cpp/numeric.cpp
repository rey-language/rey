#include <cstdint>
#include <iostream>
#include <string>

int main(int argc, char **argv) {
    if (argc != 2) return 2;
    std::size_t used = 0;
    long long parsed;
    try {
        parsed = std::stoll(argv[1], &used, 10);
    } catch (...) {
        return 2;
    }
    if (used != std::string(argv[1]).size() || parsed < 0 || parsed > 10000000) return 2;
    const int64_t size = static_cast<int64_t>(parsed);
    int64_t state = 1;
    int64_t checksum = 0;
    for (int64_t i = 0; i < size; ++i) {
        state = (state * 48271) % 2147483647;
        checksum += state % 1024;
    }
    std::cout << "checksum=" << checksum << " processed=" << size << '\n';
    return 0;
}
