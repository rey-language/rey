#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    if (argc != 2) return 2;
    char *end = NULL;
    long long parsed = strtoll(argv[1], &end, 10);
    if (end == argv[1] || *end != '\0' || parsed < 0 || parsed > 10000000) return 2;
    int64_t size = (int64_t)parsed;
    int64_t state = 1;
    int64_t checksum = 0;
    for (int64_t i = 0; i < size; i++) {
        state = (state * 48271) % 2147483647;
        if (state % 7 == 0) checksum = (checksum * 31 + state) % 2147483647;
    }
    printf("checksum=%" PRId64 " processed=%" PRId64 "\n", checksum, size);
    return 0;
}
