#define _POSIX_C_SOURCE 200809L
#include "rey_rt.h"
#include <errno.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
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
    static const char message[] = "FAIL: vec regression test timed out\n";
    (void)signal_number;
    (void)write(STDERR_FILENO, message, sizeof(message) - 1);
    _Exit(124);
}

static void test_zero_length_operations(void) {
    int64_t vec = rey_vec_new();
    CHECK(rey_vec_len(vec) == 0, "new vector is empty");
    CHECK(!rey_vec_contains(vec, 7), "empty contains is false");
    CHECK(!rey_vec_str_contains(vec, rey_str_cstr("missing")), "empty string contains is false");
    CHECK(rey_vec_len(rey_vec_slice_v(vec, 0, 0)) == 0, "empty slice is empty");
    CHECK(rey_vec_len(rey_vec_slice_v(vec, 0, 10)) == 0, "oversized end on empty vector is empty");
    CHECK(rey_vec_len(rey_vec_reverse(vec)) == 0, "reverse of empty vector is empty");
    CHECK(rey_vec_len(rey_vec_map(vec, 0)) == 0, "map of empty vector is empty");
    CHECK(rey_vec_len(rey_vec_filter(vec, 0)) == 0, "filter of empty vector is empty");
    CHECK(rey_str_eq(rey_vec_join(vec, rey_str_cstr(",")), rey_str_empty()),
          "join of empty vector is empty string");

    int64_t strings = rey_vec_new();
    rey_vec_push(strings, (int64_t)(uintptr_t)rey_str_cstr("alpha"));
    rey_vec_push(strings, (int64_t)(uintptr_t)rey_str_cstr("beta"));
    CHECK(rey_vec_str_contains(strings, rey_str_cstr("beta")), "string contains compares values");
    CHECK(rey_str_eq(rey_vec_join(strings, rey_str_cstr("/")), rey_str_cstr("alpha/beta")),
          "string vector join preserves items");
    printf("PASS: zero-length operations and string items\n");
}

static void test_growth_and_access(void) {
    const int count = 20001;
    int64_t vec = rey_vec_new();
    for (int i = 0; i < count; i++) rey_vec_push(vec, (int64_t)i * 17 - 3);

    CHECK(rey_vec_len(vec) == count, "length after repeated growth");
    CHECK(rey_vec_get(vec, 0) == -3, "first value after growth");
    CHECK(rey_vec_get(vec, count - 1) == (int64_t)(count - 1) * 17 - 3,
          "last value after growth");
    for (int i = 0; i < count; i += 997)
        CHECK(rey_vec_get(vec, i) == (int64_t)i * 17 - 3, "sampled values survive growth");

    rey_vec_set(vec, 10000, -777);
    CHECK(rey_vec_get(vec, 10000) == -777, "set updates existing element");
    CHECK(rey_vec_contains(vec, -777), "contains sees updated element");
    printf("PASS: append/growth beyond 10k and get/set\n");
}

static void test_slice_and_reverse(void) {
    int64_t vec = rey_vec_new();
    for (int i = 0; i < 10; i++) rey_vec_push(vec, i);

    int64_t slice = rey_vec_slice_v(vec, 3, 7);
    CHECK(rey_vec_len(slice) == 4, "slice length");
    for (int i = 0; i < 4; i++) CHECK(rey_vec_get(slice, i) == i + 3, "slice contents");

    int64_t clamped = rey_vec_slice_v(vec, 0, 50);
    CHECK(rey_vec_len(clamped) == 10, "slice clamps oversized end");
    for (int i = 0; i < 10; i++) CHECK(rey_vec_get(clamped, i) == i, "clamped slice contents");
    CHECK(rey_vec_len(rey_vec_slice_v(vec, 8, 3)) == 0, "reversed slice range is empty");

    int64_t reversed = rey_vec_reverse(vec);
    CHECK(rey_vec_len(reversed) == 10, "reverse length");
    for (int i = 0; i < 10; i++) CHECK(rey_vec_get(reversed, i) == 9 - i, "reverse contents");
    printf("PASS: slice bounds and reverse\n");
}

enum InvalidOperation {
    POP_EMPTY,
    GET_NEGATIVE,
    GET_AT_LENGTH,
    SET_NEGATIVE,
    SET_AT_LENGTH,
    SLICE_NEG_START,
    SLICE_NEG_END
};

static void invoke_invalid_operation(enum InvalidOperation operation) {
    int64_t vec = rey_vec_new();
    if (operation == GET_AT_LENGTH || operation == SET_AT_LENGTH) rey_vec_push(vec, 11);
    switch (operation) {
    case POP_EMPTY: (void)rey_vec_pop(vec); break;
    case GET_NEGATIVE: (void)rey_vec_get(vec, -1); break;
    case GET_AT_LENGTH: (void)rey_vec_get(vec, rey_vec_len(vec)); break;
    case SET_NEGATIVE: rey_vec_set(vec, -1, 11); break;
    case SET_AT_LENGTH: rey_vec_set(vec, rey_vec_len(vec), 11); break;
    case SLICE_NEG_START: (void)rey_vec_slice_v(vec, -4, 50); break;
    case SLICE_NEG_END: (void)rey_vec_slice_v(vec, 0, -1); break;
    }
}

static void expect_child_exit_one(enum InvalidOperation operation, const char* message) {
    fflush(stdout);
    pid_t pid = fork();
    CHECK(pid >= 0, "fork bounds-check child");
    if (pid == 0) {
        (void)freopen("/dev/null", "w", stderr);
        alarm(10);
        invoke_invalid_operation(operation);
        _Exit(0);
    }

    int status = 0;
    pid_t waited;
    do {
        waited = waitpid(pid, &status, 0);
    } while (waited < 0 && errno == EINTR);
    CHECK(waited == pid, "wait for bounds-check child");
    CHECK(WIFEXITED(status) && WEXITSTATUS(status) == 1, message);
}

static void test_bounds_and_empty_pop(void) {
    expect_child_exit_one(POP_EMPTY, "pop on empty vector keeps panic exit status");
    expect_child_exit_one(GET_NEGATIVE, "negative get keeps panic exit status");
    expect_child_exit_one(GET_AT_LENGTH, "get at length keeps panic exit status");
    expect_child_exit_one(SET_NEGATIVE, "negative set keeps panic exit status");
    expect_child_exit_one(SET_AT_LENGTH, "set at length keeps panic exit status");
    expect_child_exit_one(SLICE_NEG_START, "negative slice start exits");
    expect_child_exit_one(SLICE_NEG_END, "negative slice end exits");
    printf("PASS: empty pop and get/set bounds panic in child processes\n");
}

static void test_repeated_drain_and_reuse(void) {
    int64_t vec = rey_vec_new();
    const int rounds = 100;
    const int per_round = 64;
    for (int round = 0; round < rounds; round++) {
        for (int i = 0; i < per_round; i++) rey_vec_push(vec, round * per_round + i);
        CHECK(rey_vec_len(vec) == per_round, "length after refill");
        for (int i = per_round - 1; i >= 0; i--)
            CHECK(rey_vec_pop(vec) == round * per_round + i, "pop drains in reverse insertion order");
        CHECK(rey_vec_len(vec) == 0, "length after drain");
    }
    rey_vec_push(vec, 42);
    CHECK(rey_vec_get(vec, 0) == 42, "vector works after repeated drain/reuse");
    printf("PASS: %d repeated drain/reuse rounds\n", rounds);
}

int main(void) {
    signal(SIGALRM, on_timeout);
    alarm(120);
    test_zero_length_operations();
    test_growth_and_access();
    test_slice_and_reverse();
    test_bounds_and_empty_pop();
    test_repeated_drain_and_reuse();
    alarm(0);
    printf("PASS: all vec regression tests (%lld assertions)\n", (long long)assertions);
    return 0;
}
