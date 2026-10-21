#include <errno.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static int parse_size(const char *text, int64_t *out) {
    char *end = NULL;
    errno = 0;
    long long value = strtoll(text, &end, 10);
    if (errno != 0 || end == text || *end != '\0' || value < 0 || value > 10000000) return 0;
    *out = (int64_t)value;
    return 1;
}

int main(int argc, char **argv) {
    if (argc != 2) return 2;
    int64_t size;
    if (!parse_size(argv[1], &size)) return 2;
    int64_t state = 1;
    int64_t checksum = 0;
    for (int64_t i = 0; i < size; i++) {
        state = (state * 48271) % 2147483647;
        checksum += state % 1024;
    }
    printf("checksum=%" PRId64 " processed=%" PRId64 "\n", checksum, size);
    return 0;
}
