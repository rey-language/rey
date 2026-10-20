#define _POSIX_C_SOURCE 200809L
#include "rey_rt.h"
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static int64_t assertions;

#define CHECK(condition, message) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s (line %d)\n", message, __LINE__); \
        exit(1); \
    } \
} while (0)

static void on_timeout(int signal_number) {
    static const char message[] = "FAIL: hashmap regression test timed out\n";
    (void)signal_number;
    (void)write(STDERR_FILENO, message, sizeof(message) - 1);
    _Exit(124);
}

static char* make_key(const char* prefix, int id) {
    char buf[64];
    snprintf(buf, sizeof(buf), "%s-%05d", prefix, id);
    return rey_str_cstr(buf);
}

static uint64_t test_hash(const char* key) {
    uint64_t hash = 14695981039346656037ULL;
    while (*key) {
        hash ^= (uint8_t)*key++;
        hash *= 1099511628211ULL;
    }
    return hash;
}

static char** make_collision_keys(int count) {
    char** keys = (char**)calloc((size_t)count, sizeof(*keys));
    uint64_t target = UINT64_MAX;
    int found = 0;
    CHECK(keys != NULL, "allocate collision keys");
    for (int candidate = 0; found < count; candidate++) {
        char buf[64];
        snprintf(buf, sizeof(buf), "collision-%d", candidate);
        uint64_t bucket = test_hash(buf) % 16;
        if (target == UINT64_MAX) target = bucket;
        if (bucket == target) {
            keys[found] = rey_str_cstr(buf);
            CHECK(keys[found] != NULL, "allocate collision string");
            found++;
        }
    }
    return keys;
}

static void test_collisions_delete_and_reuse(void) {
    char** keys = make_collision_keys(10);
    int64_t map = rey_hm_new();

    for (int i = 0; i < 7; i++) rey_hm_set(map, keys[i], 100 + i);
    for (int i = 0; i < 7; i++)
        CHECK(rey_hm_get(map, keys[i]) == 100 + i, "initial colliding lookup");

    rey_hm_delete(map, keys[1]);
    rey_hm_delete(map, keys[4]);
    CHECK(!rey_hm_has(map, keys[1]), "deleted colliding key misses");
    CHECK(!rey_hm_has(map, keys[4]), "second deleted colliding key misses");
    for (int i = 0; i < 7; i++) {
        if (i == 1 || i == 4) continue;
        CHECK(rey_hm_has(map, keys[i]) && rey_hm_get(map, keys[i]) == 100 + i,
              "collision chain survives deletion");
    }

    rey_hm_set(map, keys[6], 106);
    CHECK(rey_hm_get(map, keys[6]) == 106, "update past tombstone retains value");
    CHECK(rey_vec_len(rey_hm_keys(map)) == 5, "update past tombstone does not duplicate key");

    rey_hm_delete(map, rey_str_cstr("missing-collision-key"));
    CHECK(rey_vec_len(rey_hm_keys(map)) == 5, "missing delete does not change map length");

    rey_hm_set(map, keys[1], 901);
    CHECK(rey_hm_has(map, keys[1]) && rey_hm_get(map, keys[1]) == 901,
          "delete then reinsert same key");
    CHECK(rey_vec_len(rey_hm_keys(map)) == 6, "reinsert restores one live key");

    for (int i = 0; i < 7; i++) rey_hm_delete(map, keys[i]);
    for (int i = 0; i < 7; i++) CHECK(!rey_hm_has(map, keys[i]), "drained key misses");
    CHECK(rey_vec_len(rey_hm_keys(map)) == 0, "full drain has no keys");
    CHECK(rey_vec_len(rey_hm_values(map)) == 0, "full drain has no values");

    for (int i = 0; i < 10; i++) rey_hm_set(map, keys[i], 2000 + i);
    for (int i = 0; i < 10; i++)
        CHECK(rey_hm_has(map, keys[i]) && rey_hm_get(map, keys[i]) == 2000 + i,
              "map is reusable after full drain");
    printf("PASS: collision chains, missing delete, delete/reinsert, full drain/reuse\n");
}

static int stress_key_index(const char* key, int original_count, int extra_count) {
    int id;
    if (sscanf(key, "stress-%d", &id) == 1 && id >= 0 && id < original_count) return id;
    if (sscanf(key, "resize-%d", &id) == 1 && id >= 0 && id < extra_count)
        return original_count + id;
    return -1;
}

static int stress_value_index(int64_t value, int original_count, int extra_count) {
    if (value >= 100000 && value < 100000 + original_count)
        return (int)(value - 100000);
    if (value >= 200000 && value < 200000 + extra_count)
        return original_count + (int)(value - 200000);
    return -1;
}

static void verify_stress_map(int64_t map, char** keys, int* live, int64_t* values,
                              int original_count, int extra_count, int expected_count) {
    int total = original_count + extra_count;
    int* seen_keys = (int*)calloc((size_t)total, sizeof(*seen_keys));
    int* seen_values = (int*)calloc((size_t)total, sizeof(*seen_values));
    int64_t key_vec = rey_hm_keys(map);
    int64_t value_vec = rey_hm_values(map);

    CHECK(seen_keys != NULL && seen_values != NULL, "allocate vector verification state");
    CHECK(rey_vec_len(key_vec) == expected_count, "keys vector length matches live count");
    CHECK(rey_vec_len(value_vec) == expected_count, "values vector length matches live count");

    for (int64_t i = 0; i < rey_vec_len(key_vec); i++) {
        const char* key = (const char*)(uintptr_t)rey_vec_get(key_vec, i);
        int index = stress_key_index(key, original_count, extra_count);
        int valid = index >= 0 && live[index] && !seen_keys[index];
        CHECK(valid, "keys vector contains each live key exactly once");
        if (valid) {
            seen_keys[index] = 1;
            CHECK(rey_hm_has(map, key) && rey_hm_get(map, key) == values[index],
                  "enumerated key resolves to expected value");
        }
    }

    for (int64_t i = 0; i < rey_vec_len(value_vec); i++) {
        int index = stress_value_index(rey_vec_get(value_vec, i), original_count, extra_count);
        int valid = index >= 0 && live[index] && !seen_values[index];
        CHECK(valid, "values vector contains each live value exactly once");
        if (valid) seen_values[index] = 1;
    }

    for (int i = 0; i < total; i++) {
        if (live[i]) {
            CHECK(seen_keys[i], "keys vector did not omit a live key");
            CHECK(seen_values[i], "values vector did not omit a live value");
        }
    }
    free(seen_keys);
    free(seen_values);
}

static void test_12k_interleaved_deletes_and_resize(void) {
    const int original_count = 12000;
    const int extra_count = 9000;
    const int total = original_count + extra_count;
    char** keys = (char**)calloc((size_t)total, sizeof(*keys));
    int* live = (int*)calloc((size_t)total, sizeof(*live));
    int64_t* values = (int64_t*)calloc((size_t)total, sizeof(*values));
    int64_t map = rey_hm_new();
    int expected_count = original_count;

    CHECK(keys != NULL && live != NULL && values != NULL, "allocate stress state");
    for (int i = 0; i < original_count; i++) {
        keys[i] = make_key("stress", i);
        values[i] = 100000 + i;
        live[i] = 1;
        rey_hm_set(map, keys[i], values[i]);
    }

    for (int i = 0; i < original_count; i++) {
        if (i % 10 < 3) {
            rey_hm_delete(map, keys[i]);
            live[i] = 0;
            expected_count--;
            CHECK(!rey_hm_has(map, keys[i]), "interleaved deleted key misses");
            CHECK(rey_hm_get(map, keys[i]) == 0, "deleted get returns default");
        } else {
            CHECK(rey_hm_has(map, keys[i]) && rey_hm_get(map, keys[i]) == values[i],
                  "interleaved survivor lookup");
        }
    }

    rey_hm_delete(map, rey_str_cstr("missing-stress-key"));
    for (int i = 0; i < original_count; i++) {
        if (live[i])
            CHECK(rey_hm_has(map, keys[i]) && rey_hm_get(map, keys[i]) == values[i],
                  "remaining stress key resolves");
        else
            CHECK(!rey_hm_has(map, keys[i]), "every deleted stress key misses");
    }
    verify_stress_map(map, keys, live, values, original_count, 0, expected_count);

    /* These additions cross the load threshold while old tombstones remain. */
    for (int i = 0; i < extra_count; i++) {
        int index = original_count + i;
        keys[index] = make_key("resize", i);
        values[index] = 200000 + i;
        live[index] = 1;
        rey_hm_set(map, keys[index], values[index]);
        if (i % 997 == 0)
            CHECK(rey_hm_get(map, keys[index]) == values[index], "lookup during tombstone resize");
    }
    verify_stress_map(map, keys, live, values, original_count, extra_count,
                      expected_count + extra_count);
    printf("PASS: %d keys, 30%% interleaved deletions/lookups, tombstone resize, keys/values\n",
           original_count);
    free(keys);
    free(live);
    free(values);
}

static void test_long_run_tombstone_resizes(void) {
    const int rounds = 12;
    char** active = (char**)calloc(65536, sizeof(*active));
    int live_count = 0;
    int inserted = 0;
    int capacity = 16;
    int64_t map = rey_hm_new();

    CHECK(active != NULL, "allocate long-run active keys");
    for (int round = 0; round < rounds; round++) {
        int threshold = capacity / 2;
        if (round == 0) {
            while (live_count < threshold) {
                active[live_count] = make_key("longrun", inserted);
                rey_hm_set(map, active[live_count], ++inserted);
                live_count++;
            }
        }

        for (int i = 0; i < 2; i++) {
            rey_hm_delete(map, active[i]);
            CHECK(!rey_hm_has(map, active[i]), "long-run deleted entry misses");
        }
        memmove(active, active + 2, (size_t)(live_count - 2) * sizeof(*active));
        live_count -= 2;

        while (live_count < threshold) {
            active[live_count] = make_key("longrun", inserted);
            rey_hm_set(map, active[live_count], ++inserted);
            live_count++;
        }

        /* The next set reaches the load threshold and grows with tombstones present. */
        active[live_count] = make_key("longrun", inserted);
        rey_hm_set(map, active[live_count], ++inserted);
        live_count++;
        for (int i = 0; i < live_count; i++)
            CHECK(rey_hm_get(map, active[i]) != 0, "long-run survivors resolve after resize");

        char absent[64];
        snprintf(absent, sizeof(absent), "longrun-absent-%02d", round);
        rey_hm_delete(map, rey_str_cstr(absent));
        CHECK(!rey_hm_has(map, rey_str_cstr(absent)), "long-run missing delete stays absent");
        capacity *= 2;
    }
    printf("PASS: %d growth rounds / %d inserted keys with tombstones at every resize\n",
           rounds, inserted);
    free(active);
}

int main(void) {
    signal(SIGALRM, on_timeout);
    alarm(120);
    test_collisions_delete_and_reuse();
    test_12k_interleaved_deletes_and_resize();
    test_long_run_tombstone_resizes();
    alarm(0);
    printf("PASS: all hashmap regression tests (%lld assertions)\n", (long long)assertions);
    return 0;
}
