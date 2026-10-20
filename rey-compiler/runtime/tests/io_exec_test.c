#define _POSIX_C_SOURCE 200809L
#include "../rey_rt.h"
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define CHECK(cond, name) do { \
    if (!(cond)) { fprintf(stderr, "FAIL: %s\n", name); exit(1); } \
} while (0)

static const char* result_error(int64_t result) {
    CHECK(rey_result_is_err(result), "expected Result.Err");
    return (const char*)(uintptr_t)rey_result_unwrap_err(result);
}

static const char* result_value(int64_t result) {
    CHECK(rey_result_is_ok(result), "expected Result.Ok");
    return rey_result_unwrap_str(result);
}

static int contains_bytes(const char* haystack, size_t haystack_len, const char* needle) {
    size_t needle_len = strlen(needle);
    if (needle_len > haystack_len) return 0;
    for (size_t i = 0; i <= haystack_len - needle_len; i++)
        if (memcmp(haystack + i, needle, needle_len) == 0) return 1;
    return 0;
}

static void test_success(void) {
    int64_t result = rey_exec_cmd(rey_str_cstr("value='shell mode'; printf '<%s>\\n' \"$value\""));
    const char* out = result_value(result);
    CHECK(rey_str_eq(out, rey_str_cstr("<shell mode>\n")), "success stdout and shell syntax preserved");

    result = rey_exec_cmd(rey_str_cstr(":"));
    out = result_value(result);
    CHECK(rey_str_len(out) == 0, "empty success stdout");

    result = rey_exec_cmd(rey_str_cstr("printf 'a\\000b'"));
    out = result_value(result);
    const unsigned char binary[] = {'a', 0, 'b'};
    CHECK(rey_str_len(out) == (int64_t)sizeof(binary), "success captures NUL by length");
    CHECK(memcmp(out, binary, sizeof(binary)) == 0, "success preserves stdout bytes");
}

static void test_success_stderr_forwarding(void) {
    FILE* capture = tmpfile();
    CHECK(capture != NULL, "stderr tmpfile");
    int saved_stderr = dup(STDERR_FILENO);
    CHECK(saved_stderr >= 0, "save stderr");
    fflush(stderr);
    CHECK(dup2(fileno(capture), STDERR_FILENO) >= 0, "redirect stderr");
    int64_t result = rey_exec_cmd(rey_str_cstr("printf 'stdout-ok'; printf 'stderr-visible' >&2"));
    fflush(stderr);
    CHECK(dup2(saved_stderr, STDERR_FILENO) >= 0, "restore stderr");
    close(saved_stderr);

    CHECK(rey_str_eq(result_value(result), rey_str_cstr("stdout-ok")), "success stdout remains separate");
    CHECK(fseek(capture, 0, SEEK_SET) == 0, "rewind forwarded stderr");
    char actual[32] = {0};
    const char expected[] = "stderr-visible";
    CHECK(fread(actual, 1, sizeof(expected) - 1, capture) == sizeof(expected) - 1, "forwarded stderr length");
    CHECK(memcmp(actual, expected, sizeof(expected) - 1) == 0, "successful child stderr remains inherited");
    CHECK(fgetc(capture) == EOF, "no extra forwarded stderr");
    fclose(capture);
}

static void test_failure_status_and_stderr(void) {
    int64_t result = rey_exec_cmd(rey_str_cstr("printf 'stdout-fragment'; printf 'stderr-specific\\n' >&2; exit 23"));
    const char* error = result_error(result);
    size_t len = (size_t)rey_str_len(error);
    CHECK(contains_bytes(error, len, "command failed (exit status 23)"), "nonzero exit status reported");
    CHECK(contains_bytes(error, len, "stdout:\nstdout-fragment"), "failure stdout retained");
    CHECK(contains_bytes(error, len, "stderr:\nstderr-specific\n"), "child stderr included in Err");

    result = rey_exec_cmd(rey_str_cstr("printf 'stderr-only' >&2; exit 9"));
    error = result_error(result);
    len = (size_t)rey_str_len(error);
    CHECK(contains_bytes(error, len, "command failed (exit status 9)\nstdout:\n\nstderr:\nstderr-only"), "stderr-only failure distinguished from stdout");
}

static void test_spawn_and_signal_errors(void) {
    int64_t result = rey_exec_cmd(NULL);
    const char* error = result_error(result);
    CHECK(contains_bytes(error, (size_t)rey_str_len(error), "exec failed: null command"), "invalid spawn request is informative");

    result = rey_exec_cmd(rey_str_cstr("__rey_io_test_missing_command_9f23c7__"));
    error = result_error(result);
    size_t len = (size_t)rey_str_len(error);
    CHECK(contains_bytes(error, len, "command failed (exit status 127)"), "missing command status reported");
    CHECK(contains_bytes(error, len, "__rey_io_test_missing_command_9f23c7__"), "missing command stderr included");

    result = rey_exec_cmd(rey_str_cstr("kill -TERM \"$$\""));
    error = result_error(result);
    char signal_text[64];
    snprintf(signal_text, sizeof(signal_text), "command terminated by signal %d", SIGTERM);
    CHECK(contains_bytes(error, (size_t)rey_str_len(error), signal_text), "signal termination reported");
}

static void test_large_simultaneous_streams(void) {
    const size_t repetitions = 8192;
    const char out_piece[] = "0123456789abcdef";
    const char err_piece[] = "fedcba9876543210";
    const size_t stream_len = repetitions * (sizeof(out_piece) - 1);
    char* expected_out = (char*)malloc(stream_len);
    char* expected_err = (char*)malloc(stream_len);
    CHECK(expected_out && expected_err, "allocate expected large streams");
    for (size_t i = 0; i < repetitions; i++) {
        memcpy(expected_out + i * (sizeof(out_piece) - 1), out_piece, sizeof(out_piece) - 1);
        memcpy(expected_err + i * (sizeof(err_piece) - 1), err_piece, sizeof(err_piece) - 1);
    }

    int64_t result = rey_exec_cmd(rey_str_cstr(
        "i=0; while [ \"$i\" -lt 8192 ]; do "
        "printf '0123456789abcdef'; printf 'fedcba9876543210' >&2; "
        "i=$((i + 1)); done; exit 3"));
    const char* error = result_error(result);
    const char* status = "command failed (exit status 3)\nstdout:\n";
    const char* middle = "\nstderr:\n";
    size_t status_len = strlen(status), middle_len = strlen(middle);
    size_t expected_len = status_len + stream_len + middle_len + stream_len;
    CHECK((size_t)rey_str_len(error) == expected_len, "both large streams captured without truncation");
    CHECK(memcmp(error, status, status_len) == 0, "large stream status/header");
    CHECK(memcmp(error + status_len, expected_out, stream_len) == 0, "large stdout byte-for-byte");
    CHECK(memcmp(error + status_len + stream_len, middle, middle_len) == 0, "large stream separator");
    CHECK(memcmp(error + status_len + stream_len + middle_len, expected_err, stream_len) == 0, "large stderr byte-for-byte");
    free(expected_out);
    free(expected_err);
}

int main(void) {
    test_success();
    test_success_stderr_forwarding();
    test_failure_status_and_stderr();
    test_spawn_and_signal_errors();
    test_large_simultaneous_streams();
    puts("PASS: exec stdout, Result status/stderr, spawn errors, signals, and simultaneous large pipes");
    return 0;
}
