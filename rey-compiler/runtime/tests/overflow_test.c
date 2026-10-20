#define _POSIX_C_SOURCE 200809L
#include "../rey_rt.h"
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

static void onTimeout(int signalNumber) {
    static const char message[] = "FAIL: overflow regression test timed out\n";
    (void)signalNumber;
    (void)write(STDERR_FILENO, message, sizeof(message) - 1);
    _Exit(124);
}

static char* gLeft;
static char* gRight;

static char* forgeLen(int64_t fakeLen) {
    char* text = rey_str_new("x", 1);
    memcpy(text - 8, &fakeLen, sizeof(fakeLen));
    return text;
}

static void expectPanic(void (*body)(void), const char* needle, const char* name) {
    fflush(stdout);
    int pipes[2];
    CHECK(pipe(pipes) == 0, "pipe");
    pid_t pid = fork();
    CHECK(pid >= 0, "fork");
    if (pid == 0) {
        alarm(8);
        dup2(pipes[1], STDERR_FILENO);
        close(pipes[0]);
        close(pipes[1]);
        body();
        _Exit(0);
    }
    close(pipes[1]);
    char buffer[512];
    size_t used = 0;
    while (used + 1 < sizeof(buffer)) {
        ssize_t count = read(pipes[0], buffer + used, sizeof(buffer) - 1 - used);
        if (count < 0 && errno == EINTR) continue;
        if (count <= 0) break;
        used += (size_t)count;
    }
    buffer[used] = '\0';
    close(pipes[0]);
    int status = 0;
    pid_t waited;
    do waited = waitpid(pid, &status, 0);
    while (waited < 0 && errno == EINTR);
    CHECK(waited == pid && WIFEXITED(status) && WEXITSTATUS(status) == 1, name);
    CHECK(strstr(buffer, needle) != NULL, name);
}

static void negativeAlloc(void) { (void)rey_alloc(-1); }
static void negativeString(void) { (void)rey_str_new("x", -1); }
static void concatOverflow(void) { (void)rey_str_concat(gLeft, gRight); }
static void repeatOverflow(void) { (void)rey_str_repeat(gLeft, 3); }
static void upperOverflow(void) { (void)rey_str_to_upper(gLeft); }
static void lowerOverflow(void) { (void)rey_str_to_lower(gLeft); }
static void enumOverflow(void) { (void)rey_enum_new(1, 0, INT64_MAX); }
static char* gText;
static void sliceNegStart(void) { (void)rey_str_slice(gText, INT64_MIN, INT64_MAX); }
static void sliceNegEnd(void) { (void)rey_str_slice(gText, 1, INT64_MIN); }

static void testLengthGuards(void) {
    expectPanic(negativeAlloc, "rey panic: negative allocation", "negative allocation exits");
    expectPanic(negativeString, "rey panic: string length overflow", "negative string length exits");
    int64_t huge = INT64_MAX / 2 + 10;
    gLeft = forgeLen(huge);
    gRight = forgeLen(huge);
    expectPanic(concatOverflow, "rey panic: string length overflow", "concat length overflow exits before copying");
    gLeft = forgeLen(huge);
    expectPanic(repeatOverflow, "rey panic: string length overflow", "repeat length overflow exits");
    gLeft = forgeLen(INT64_MAX);
    expectPanic(upperOverflow, "rey panic: string length overflow", "upper length overflow exits before copying");
    gLeft = forgeLen(INT64_MAX);
    expectPanic(lowerOverflow, "rey panic: string length overflow", "lower length overflow exits before copying");
    expectPanic(enumOverflow, "rey panic: invalid enum layout", "enum slot count overflow exits");
    CHECK(rey_enum_is(rey_enum_new(1, 2, 0), 1, 2), "zero-payload enum still constructs");
    CHECK(rey_str_eq(rey_str_to_upper(rey_str_cstr("aB")), rey_str_cstr("AB")), "ordinary upper");
    CHECK(rey_str_eq(rey_str_to_lower(rey_str_cstr("aB")), rey_str_cstr("ab")), "ordinary lower");

    char* empty = rey_str_empty();
    char* repeated = rey_str_repeat(empty, 100000000);
    CHECK(rey_str_len(repeated) == 0, "repeating an empty string does not loop");
    CHECK(rey_str_eq(rey_str_concat(rey_str_cstr("ab"), rey_str_cstr("cd")), rey_str_cstr("abcd")),
          "ordinary concat still copies both sides");
    CHECK(rey_str_eq(rey_str_concat(NULL, rey_str_cstr("z")), rey_str_cstr("z")), "null concat operand is empty");
    printf("PASS: allocation and string length overflow guards\n");
}

static void testBoundsAndIntegers(void) {
    char* text = rey_str_cstr("hello");
    gText = text;
    expectPanic(sliceNegStart, "error: String.slice() indices must be non-negative", "negative slice start exits");
    expectPanic(sliceNegEnd, "error: String.slice() indices must be non-negative", "negative slice end exits");
    CHECK(rey_str_eq(rey_str_slice(text, 0, INT64_MAX), text), "slice clamps oversized end");
    CHECK(rey_str_len(rey_str_slice(text, 4, 2)) == 0, "reversed slice is empty");
    CHECK(rey_str_len(rey_str_char_at(text, INT64_MIN)) == 0, "char_at below zero is empty");
    CHECK(rey_str_len(rey_str_char_at(text, INT64_MAX)) == 0, "char_at above length is empty");
    CHECK(rey_str_eq(rey_str_char_at(text, 1), rey_str_cstr("e")), "in-range char_at");
    CHECK(rey_str_eq(rey_int_to_str(INT64_MIN), rey_str_cstr("-9223372036854775808")), "INT64_MIN text");
    CHECK(rey_str_eq(rey_int_to_str(INT64_MAX), rey_str_cstr("9223372036854775807")), "INT64_MAX text");
    CHECK(rey_str_eq(rey_int_to_str(0), rey_str_cstr("0")), "zero text");

    int64_t vec = rey_vec_new();
    for (int i = 0; i < 1000; i++) rey_vec_push_typed(vec, REY_VALUE_INT, i);
    CHECK(rey_vec_len(vec) == 1000, "typed vec grows");
    CHECK(rey_vec_get_typed(vec, 999, REY_VALUE_INT) == 999, "last typed element survives growth");
    CHECK(rey_vec_get_typed(vec, 0, REY_VALUE_INT) == 0, "first typed element survives growth");
    printf("PASS: substring extremes, integer text, and typed vec growth\n");
}

int main(void) {
    signal(SIGALRM, onTimeout);
    alarm(30);
    testLengthGuards();
    testBoundsAndIntegers();
    alarm(0);
    printf("PASS: overflow guards (%lld assertions)\n", (long long)assertions);
    return 0;
}
