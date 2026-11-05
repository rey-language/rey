#include <cstdint>
#include <iostream>
#include <string>
#include <vector>

int main(int argc, char **argv) {
    if (argc != 2) return 2;
    std::size_t used = 0;
    long long parsed;
    try {
        parsed = std::stoll(argv[1], &used, 10);
    } catch (...) {
        return 2;
    }
    if (used != std::string(argv[1]).size() || parsed < 0 || parsed > 4096) return 2;
    const int64_t rounds = static_cast<int64_t>(parsed);
    int64_t checksum = 0;
    for (int64_t round = 0; round < rounds; ++round) {
        std::vector<int64_t> values;
        for (int64_t index = 0; index < 32; ++index) values.push_back((round * 17 + index * 13) % 1009);
        for (int64_t value : values) checksum += value;
    }
    std::cout << "checksum=" << checksum << " processed=" << rounds * 32 << '\n';
    return 0;
}
