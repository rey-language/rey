#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct {
    int64_t *items;
    size_t length;
    size_t capacity;
} IntVector;

static int push(IntVector *vector, int64_t value) {
    if (vector->length == vector->capacity) {
        size_t next = vector->capacity == 0 ? 4 : vector->capacity * 2;
        if (next < vector->capacity || next > SIZE_MAX / sizeof(int64_t)) return 0;
        int64_t *grown = realloc(vector->items, next * sizeof(int64_t));
        if (grown == NULL) return 0;
        vector->items = grown;
        vector->capacity = next;
    }
    vector->items[vector->length++] = value;
    return 1;
}

int main(int argc, char **argv) {
    if (argc != 2) return 2;
    char *end = NULL;
    long long parsed = strtoll(argv[1], &end, 10);
    if (end == argv[1] || *end != '\0' || parsed < 0 || parsed > 4096) return 2;
    int64_t rounds = (int64_t)parsed;
    int64_t checksum = 0;
    for (int64_t round = 0; round < rounds; round++) {
        IntVector values = {0};
        for (int64_t index = 0; index < 32; index++) {
            int64_t value = (round * 17 + index * 13) % 1009;
            if (!push(&values, value)) {
                free(values.items);
                return 3;
            }
        }
        for (size_t index = 0; index < values.length; index++) checksum += values.items[index];
        free(values.items);
    }
    printf("checksum=%" PRId64 " processed=%" PRId64 "\n", checksum, rounds * 32);
    return 0;
}
