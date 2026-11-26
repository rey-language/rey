#include <errno.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static const int64_t parse_i64(const char *text, int *ok) {
    char *end = NULL;
    errno = 0;
    long long value = strtoll(text, &end, 10);
    *ok = errno == 0 && end != text && *end == '\0';
    return (int64_t)value;
}

int main(int argc, char **argv) {
    if (argc != 10) return 2;
    int ok = 0;
    int64_t size = parse_i64(argv[1], &ok);
    if (!ok || size < 0 || size > 5000000) return 2;
    int64_t checksum = 0;
    for (int64_t i = 0; i < size; i++) {
        int token_ok = 0;
        int64_t value = parse_i64(argv[(i % 8) + 2], &token_ok);
        if (!token_ok) return 2;
        checksum += value;
    }
    printf("checksum=%" PRId64 " processed=%" PRId64 "\n", checksum, size);
    return 0;
}
