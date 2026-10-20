#define _POSIX_C_SOURCE 200809L
#include "../rey_rt.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define CHECK(cond, name) do { \
    if (!(cond)) { fprintf(stderr, "FAIL: %s\n", name); exit(1); } \
} while (0)

static void check_printed_bytes(const char* s, const unsigned char* expected, size_t len) {
    FILE* capture = tmpfile();
    CHECK(capture != NULL, "tmpfile");
    int saved_stdout = dup(STDOUT_FILENO);
    CHECK(saved_stdout >= 0, "save stdout");
    fflush(stdout);
    CHECK(dup2(fileno(capture), STDOUT_FILENO) >= 0, "redirect stdout");
    rey_print(s);
    fflush(stdout);
    CHECK(dup2(saved_stdout, STDOUT_FILENO) >= 0, "restore stdout");
    close(saved_stdout);

    CHECK(fseek(capture, 0, SEEK_SET) == 0, "rewind printed bytes");
    unsigned char* actual = (unsigned char*)malloc(len ? len : 1);
    CHECK(actual != NULL, "allocate print comparison buffer");
    CHECK(fread(actual, 1, len, capture) == len, "printed byte count");
    CHECK(memcmp(actual, expected, len) == 0, "printed bytes match");
    CHECK(fgetc(capture) == EOF, "printed output has no extra bytes");
    free(actual);
    fclose(capture);
}

static void test_integer_to_string(void) {
    const int64_t values[] = {
        -INT64_C(200000), -65537, -65536, -65535, -1,
        0, 1, 65535, 65536, 65537, 200000, INT64_MIN, INT64_MAX
    };
    for (size_t i = 0; i < sizeof(values) / sizeof(values[0]); i++) {
        char expected[32];
        snprintf(expected, sizeof(expected), "%lld", (long long)values[i]);
        CHECK(rey_str_eq(rey_val_to_str(values[i]), rey_str_cstr(expected)), "integer toString value");
    }
    CHECK(rey_str_eq(rey_val_to_str(INT64_C(200000)), rey_str_cstr("200000")), "threshold integer 200000");
}

static void test_string_sizes_and_print(void) {
    const int64_t sizes[] = {0, 1, 65535, 65536, 65537, 200000};
    for (size_t j = 0; j < sizeof(sizes) / sizeof(sizes[0]); j++) {
        int64_t len = sizes[j];
        unsigned char* bytes = (unsigned char*)malloc(len ? (size_t)len : 1);
        CHECK(bytes != NULL, "allocate size-boundary source");
        for (int64_t i = 0; i < len; i++) bytes[i] = (unsigned char)(i * 73 + 0x80);

        char* s = rey_str_new((const char*)bytes, len);
        char* converted = rey_val_to_str((int64_t)(uintptr_t)s);
        CHECK(rey_str_len(s) == len, "string len is exact byte count");
        CHECK(converted == s, "runtime string toString identity");
        CHECK(rey_str_len(converted) == len, "toString retains stored length");
        CHECK(memcmp(converted, bytes, (size_t)len) == 0, "string content round-trip");
        CHECK(rey_str_eq(s, rey_str_new((const char*)bytes, len)), "equal content at separate allocations");

        if (len == 200000) check_printed_bytes(converted, bytes, (size_t)len);
        free(bytes);
    }
}

static void test_escapes_nul_and_ascii_case(void) {
    // These are the bytes after lexical escape processing, including \0 and one \xNN byte.
    const unsigned char escaped[] = {'\n', '\t', '\r', '\\', '"', '\'', 0, 0xa5};
    const unsigned char joined[] = {'p', 'r', 'e', ':', '\n', '\t', '\r', '\\', '"', '\'', 0, 0xa5};
    char* raw = rey_str_new((const char*)escaped, (int64_t)sizeof(escaped));
    char* prefix = rey_str_cstr("pre:");
    char* combined = rey_str_concat(prefix, raw);
    CHECK(rey_str_len(raw) == (int64_t)sizeof(escaped), "embedded NUL counts toward byte length");
    CHECK(rey_str_len(combined) == (int64_t)sizeof(joined), "concat byte length with NUL");
    CHECK(memcmp(combined, joined, sizeof(joined)) == 0, "concat preserves unescaped bytes");
    CHECK(rey_val_to_str((int64_t)(uintptr_t)combined) == combined, "concat result registered for conversion");
    check_printed_bytes(combined, joined, sizeof(joined));
    CHECK(rey_str_eq(raw, rey_str_new((const char*)escaped, (int64_t)sizeof(escaped))), "content equality across allocations");
    CHECK(!rey_str_eq(raw, rey_str_new((const char*)escaped, (int64_t)sizeof(escaped) - 1)), "equality compares length and NUL-containing content");

    const unsigned char mixed[] = {'a', 'z', 'A', 'Z', '-', 0xc0, 0xe9, 0xff, 0, 'b'};
    const unsigned char upper[] = {'A', 'Z', 'A', 'Z', '-', 0xc0, 0xe9, 0xff, 0, 'B'};
    const unsigned char lower[] = {'a', 'z', 'a', 'z', '-', 0xc0, 0xe9, 0xff, 0, 'b'};
    char* original = rey_str_new((const char*)mixed, (int64_t)sizeof(mixed));
    char* up = rey_str_to_upper(original);
    char* down = rey_str_to_lower(original);
    CHECK(rey_str_len(original) == (int64_t)sizeof(mixed), "len counts bytes, not terminator or characters");
    CHECK(rey_str_len(up) == (int64_t)sizeof(upper) && memcmp(up, upper, sizeof(upper)) == 0, "uppercase changes ASCII only");
    CHECK(rey_str_len(down) == (int64_t)sizeof(lower) && memcmp(down, lower, sizeof(lower)) == 0, "lowercase changes ASCII only");
    CHECK(rey_val_to_str((int64_t)(uintptr_t)up) == up, "uppercase output registered");
    CHECK(rey_val_to_str((int64_t)(uintptr_t)down) == down, "lowercase output registered");
}

static void test_constructor_registration_and_growth(void) {
    char* empty = rey_str_empty();
    char* base = rey_str_cstr("Rey");
    char* concatenated = rey_str_concat(base, rey_str_cstr(" runtime"));
    char* repeated = rey_str_repeat(base, 3);
    char* upper = rey_str_to_upper(base);
    char* lower = rey_str_to_lower(base);
    char* numeric = rey_int_to_str(INT64_C(200000));
    char* boolean = rey_bool_to_str(1);
    char* outputs[] = {empty, base, concatenated, repeated, upper, lower, numeric, boolean};
    for (size_t i = 0; i < sizeof(outputs) / sizeof(outputs[0]); i++) {
        CHECK(rey_val_to_str((int64_t)(uintptr_t)outputs[i]) == outputs[i], "all runtime constructors register data pointers");
    }

    // Force multiple registry table growths while continuing to mix strings and integers.
    for (int64_t i = 0; i < 1500; i++) {
        char text[32];
        snprintf(text, sizeof(text), "registry-%lld", (long long)i);
        char* s = rey_str_cstr(text);
        CHECK(rey_val_to_str((int64_t)(uintptr_t)s) == s, "registered string survives registry growth");
        snprintf(text, sizeof(text), "%lld", (long long)i);
        CHECK(rey_str_eq(rey_val_to_str(i), rey_str_cstr(text)), "integer remains integer through registry growth");
    }
}

int main(void) {
    test_integer_to_string();
    test_string_sizes_and_print();
    test_escapes_nul_and_ascii_case();
    test_constructor_registration_and_growth();
    puts("PASS: value conversion, size boundaries, binary strings/printing, ASCII case, equality, registry growth");
    return 0;
}
