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

static void test_string_payload(void) {
    const unsigned char bytes[] = {'e', 0, 'r', 0xff, '!'};
    char* payload = rey_str_new((const char*)bytes, (int64_t)sizeof(bytes));
    int64_t result = rey_err_str(payload);
    CHECK(rey_result_is_err(result), "string error is Err");
    int64_t extracted = rey_result_unwrap_err(result);
    CHECK(extracted == (int64_t)(uintptr_t)payload, "string payload returned unchanged");
    const char* error = (const char*)(uintptr_t)extracted;
    CHECK(rey_str_len(error) == (int64_t)sizeof(bytes), "embedded NUL length preserved");
    CHECK(memcmp(error, bytes, sizeof(bytes)) == 0, "embedded NUL/non-ASCII bytes preserved");
}

static void test_integer_payload(void) {
    const int64_t payload = INT64_MIN + 12345;
    int64_t result = rey_err(payload);
    CHECK(rey_result_is_err(result), "integer error is Err");
    CHECK(rey_result_unwrap_err(result) == payload, "integer payload returned unchanged");
}

static void check_unwrap_err_panics(int64_t result, const char* test_name) {
    int pipefd[2];
    CHECK(pipe(pipefd) == 0, "panic output pipe");
    pid_t child = fork();
    CHECK(child >= 0, "fork panic check");
    if (child == 0) {
        close(pipefd[0]);
        if (dup2(pipefd[1], STDERR_FILENO) < 0) _exit(90);
        close(pipefd[1]);
        (void)rey_result_unwrap_err(result);
        _exit(0);
    }

    close(pipefd[1]);
    char diagnostic[1024];
    size_t used = 0;
    for (;;) {
        char chunk[128];
        ssize_t n = read(pipefd[0], chunk, sizeof(chunk));
        if (n == 0) break;
        if (n < 0 && errno == EINTR) continue;
        CHECK(n > 0, "read panic diagnostic");
        size_t copy = (size_t)n;
        if (copy > sizeof(diagnostic) - used) copy = sizeof(diagnostic) - used;
        if (copy) { memcpy(diagnostic + used, chunk, copy); used += copy; }
    }
    close(pipefd[0]);
    int status = 0;
    pid_t waited;
    do { waited = waitpid(child, &status, 0); } while (waited < 0 && errno == EINTR);
    CHECK(waited == child, "wait for misuse child");
    CHECK(WIFEXITED(status) && WEXITSTATUS(status) == 1, test_name);
    CHECK(contains_bytes(diagnostic, used, "Result.unwrapErr() on Ok"), "unwrapErr misuse diagnostic");
}

static void test_wrong_variant_and_null(void) {
    int64_t ok = rey_ok_str(rey_str_cstr("ok payload"));
    check_unwrap_err_panics(ok, "unwrapErr on Ok exits with runtime error");

    // Null follows the existing unwrap panic policy; arbitrary forged nonzero handles are not probed.
    check_unwrap_err_panics(0, "unwrapErr on null exits with runtime error");
}

static void test_exec_error_payload(void) {
    int64_t result = rey_exec_cmd(rey_str_cstr("printf 'exec-stdout'; printf 'exec-stderr\\n' >&2; exit 17"));
    CHECK(rey_result_is_err(result), "failed exec is Err");
    int64_t payload = rey_result_unwrap_err(result);
    const char* error = (const char*)(uintptr_t)payload;
    size_t len = (size_t)rey_str_len(error);
    CHECK(contains_bytes(error, len, "command failed (exit status 17)"), "exec status available through Err accessor");
    CHECK(contains_bytes(error, len, "stdout:\nexec-stdout"), "exec stdout retained in Err");
    CHECK(contains_bytes(error, len, "stderr:\nexec-stderr\n"), "exec stderr available through Err accessor");
}

int main(void) {
    test_string_payload();
    test_integer_payload();
    test_wrong_variant_and_null();
    test_exec_error_payload();
    puts("PASS: Err payload extraction, variant misuse panic, null policy, exec diagnostics");
    return 0;
}
