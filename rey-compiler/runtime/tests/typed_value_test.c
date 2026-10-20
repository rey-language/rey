#define _POSIX_C_SOURCE 200809L
#include "../rey_rt.h"
#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

#define CHECK(cond, name) do { \
    if (!(cond)) { fprintf(stderr, "FAIL: %s\n", name); exit(1); } \
} while (0)

static int contains_bytes(const char* haystack, size_t haystack_len, const char* needle) {
    size_t needle_len = strlen(needle);
    if (needle_len > haystack_len) return 0;
    for (size_t i = 0; i <= haystack_len - needle_len; i++)
        if (memcmp(haystack + i, needle, needle_len) == 0) return 1;
    return 0;
}

static void check_bytes(const char* actual, const unsigned char* expected, size_t len, const char* name) {
    CHECK(rey_str_len(actual) == (int64_t)len, name);
    CHECK(memcmp(actual, expected, len) == 0, name);
}

static void test_kinds_and_pointer_collision(void) {
    CHECK(rey_str_eq(rey_typed_val_to_str(REY_VALUE_INT, 0), rey_str_cstr("0")), "int zero formatting");
    CHECK(rey_str_eq(rey_typed_val_to_str(REY_VALUE_INT, 1), rey_str_cstr("1")), "int one formatting");
    CHECK(rey_str_eq(rey_typed_val_to_str(REY_VALUE_BOOL, 0), rey_str_cstr("false")), "bool false differs from int zero");
    CHECK(rey_str_eq(rey_typed_val_to_str(REY_VALUE_BOOL, 1), rey_str_cstr("true")), "bool true differs from int one");
    CHECK(rey_str_eq(rey_typed_val_to_str(REY_VALUE_INT, INT64_C(200000)), rey_str_cstr("200000")), "large typed integer formatting");

    const unsigned char bytes[] = {'s', 0, 't', 0xff, 'r'};
    char* live_string = rey_str_new((const char*)bytes, (int64_t)sizeof(bytes));
    int64_t same_payload = (int64_t)(uintptr_t)live_string;
    char decimal[32];
    snprintf(decimal, sizeof(decimal), "%lld", (long long)same_payload);

    const char* as_int = rey_typed_val_to_str(REY_VALUE_INT, same_payload);
    CHECK(rey_str_eq(as_int, rey_str_cstr(decimal)), "integer equal to live string pointer formats as integer");
    const char* as_string = rey_typed_val_to_str(REY_VALUE_STRING, same_payload);
    CHECK(as_string == live_string, "typed string returns caller-promised pointer unchanged");
    check_bytes(as_string, bytes, sizeof(bytes), "typed string preserves exact binary bytes");

    // The old one-argument entry point remains for compatibility and remains ambiguous.
    CHECK(rey_val_to_str(same_payload) == live_string, "legacy ambiguity retained");
}

static void test_float_char_and_null(void) {
    double value = 123.5;
    int64_t bits = 0;
    CHECK(sizeof(value) == sizeof(bits), "binary64 payload size");
    memcpy(&bits, &value, sizeof(value));
    CHECK(rey_str_eq(rey_typed_val_to_str(REY_VALUE_FLOAT, bits), rey_str_cstr("123.5")), "float raw-bits conversion");

    const unsigned char nul[] = {0};
    const unsigned char high[] = {0xff};
    check_bytes(rey_typed_val_to_str(REY_VALUE_CHAR, 0), nul, sizeof(nul), "char NUL is one raw byte");
    check_bytes(rey_typed_val_to_str(REY_VALUE_CHAR, 255), high, sizeof(high), "char 0xff is one raw byte");
    CHECK(rey_str_eq(rey_typed_val_to_str(REY_VALUE_NULL, 0), rey_str_cstr("null")), "null spelling");
}

static void test_tagged_enum_payloads(void) {
    int64_t count_a = rey_enum_new(21, 0, 1);
    rey_enum_set(count_a, 0, REY_ENUM_INT, 7);
    int64_t count_b = rey_enum_new(21, 0, 1);
    rey_enum_set(count_b, 0, REY_ENUM_INT, 7);
    int64_t other = rey_enum_new(21, 1, 1);
    rey_enum_set(other, 0, REY_ENUM_INT, 7);
    CHECK(rey_enum_is(count_a, 21, 0), "payload enum variant identity");
    CHECK(!rey_enum_is(count_a, 21, 1), "payload enum discriminant mismatch");
    CHECK(rey_enum_get(count_a, 21, 0, 0, REY_ENUM_INT) == 7, "payload enum checked read");
    CHECK(rey_enum_eq(count_a, count_b, 21), "separate enum boxes compare structurally");
    CHECK(!rey_enum_eq(count_a, other, 21), "enum equality observes discriminants");

    int64_t label_a = rey_enum_new(22, 1, 1);
    rey_enum_set(label_a, 0, REY_ENUM_STRING, (int64_t)(uintptr_t)rey_str_cstr("ready"));
    int64_t label_b = rey_enum_new(22, 1, 1);
    rey_enum_set(label_b, 0, REY_ENUM_STRING, (int64_t)(uintptr_t)rey_str_cstr("ready"));
    CHECK(rey_enum_eq(label_a, label_b, 22), "enum string payload compares by value");

    int64_t nested_a = rey_enum_new(23, 0, 1);
    rey_enum_set(nested_a, 0, REY_ENUM_ENUM, count_a);
    int64_t nested_b = rey_enum_new(23, 0, 1);
    rey_enum_set(nested_b, 0, REY_ENUM_ENUM, count_b);
    CHECK(rey_enum_eq(nested_a, nested_b, 23), "nested enum payload compares structurally");

    int64_t shared = INT64_C(0x1234);
    int64_t box_a = rey_enum_new(24, 0, 1);
    rey_enum_set(box_a, 0, REY_ENUM_STRUCT, shared);
    int64_t box_alias = rey_enum_new(24, 0, 1);
    rey_enum_set(box_alias, 0, REY_ENUM_STRUCT, shared);
    int64_t box_copy = rey_enum_new(24, 0, 1);
    rey_enum_set(box_copy, 0, REY_ENUM_STRUCT, INT64_C(0x5678));
    CHECK(rey_enum_eq(box_a, box_alias, 24), "enum struct payload retains shared reference identity");
    CHECK(!rey_enum_eq(box_a, box_copy, 24), "distinct struct payload references remain unequal");

    int64_t unit_a = rey_enum_new(25, 3, 0);
    int64_t unit_b = rey_enum_new(25, 3, 0);
    CHECK(rey_enum_eq(unit_a, unit_b, 25), "unit enum variants use the same tagged layout");

    int64_t some_a = rey_some_typed(7, REY_ENUM_INT);
    int64_t some_b = rey_some_typed(7, REY_ENUM_INT);
    int64_t option_a = rey_enum_new(26, 0, 1);
    rey_enum_set(option_a, 0, REY_ENUM_OPTION, some_a);
    int64_t option_b = rey_enum_new(26, 0, 1);
    rey_enum_set(option_b, 0, REY_ENUM_OPTION, some_b);
    CHECK(rey_enum_eq(option_a, option_b, 26), "nested Some payload compares by typed value");

    int64_t ok_a = rey_ok_typed(9, REY_ENUM_INT);
    int64_t ok_b = rey_ok_typed(9, REY_ENUM_INT);
    int64_t result_a = rey_enum_new(27, 0, 1);
    rey_enum_set(result_a, 0, REY_ENUM_RESULT, ok_a);
    int64_t result_b = rey_enum_new(27, 0, 1);
    rey_enum_set(result_b, 0, REY_ENUM_RESULT, ok_b);
    CHECK(rey_enum_eq(result_a, result_b, 27), "nested Result payload compares by typed value");
}

static void check_invalid_panics(int64_t kind, int64_t payload, const char* expected_message, const char* test_name) {
    int pipefd[2];
    CHECK(pipe(pipefd) == 0, "invalid-value diagnostic pipe");
    pid_t child = fork();
    CHECK(child >= 0, "fork invalid-value check");
    if (child == 0) {
        close(pipefd[0]);
        if (dup2(pipefd[1], STDERR_FILENO) < 0) _exit(90);
        close(pipefd[1]);
        (void)rey_typed_val_to_str(kind, payload);
        _exit(0);
    }

    close(pipefd[1]);
    char diagnostic[512];
    size_t used = 0;
    for (;;) {
        char chunk[128];
        ssize_t n = read(pipefd[0], chunk, sizeof(chunk));
        if (n == 0) break;
        if (n < 0 && errno == EINTR) continue;
        CHECK(n > 0, "read invalid-value diagnostic");
        size_t copy = (size_t)n;
        if (copy > sizeof(diagnostic) - used) copy = sizeof(diagnostic) - used;
        if (copy) { memcpy(diagnostic + used, chunk, copy); used += copy; }
    }
    close(pipefd[0]);
    int status = 0;
    pid_t waited;
    do { waited = waitpid(child, &status, 0); } while (waited < 0 && errno == EINTR);
    CHECK(waited == child, "wait invalid-value child");
    CHECK(WIFEXITED(status) && WEXITSTATUS(status) == 1, test_name);
    CHECK(contains_bytes(diagnostic, used, expected_message), "invalid value diagnostic is specific");
}

static void test_rejected_kinds_and_ranges(void) {
    check_invalid_panics(INT64_C(0xdead), 1, "unsupported kind", "unknown kind exits with runtime error");
    check_invalid_panics(REY_VALUE_CHAR, -1, "char payload must be in range 0..255", "negative char exits with runtime error");
    check_invalid_panics(REY_VALUE_CHAR, 256, "char payload must be in range 0..255", "oversized char exits with runtime error");
    check_invalid_panics(REY_VALUE_BOOL, 2, "bool payload must be 0 or 1", "invalid bool exits with runtime error");
    check_invalid_panics(REY_VALUE_STRING, 0, "string payload must be non-null", "null string pointer exits with runtime error");
    check_invalid_panics(REY_VALUE_NULL, 1, "null payload must be 0", "nonzero null payload exits with runtime error");
}

int main(void) {
    test_kinds_and_pointer_collision();
    test_float_char_and_null();
    test_tagged_enum_payloads();
    test_rejected_kinds_and_ranges();
    puts("PASS: typed values, tagged enum payloads/identity, binary strings/chars, float bits, and invalid inputs");
    return 0;
}
