// Sanity tests for the Rey runtime library.
#include "rey_rt.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>

#define CHECK(cond, msg) do { \
    if (!(cond)) { fprintf(stderr, "FAIL: %s\n", msg); exit(1); } \
} while(0)

static void test_strings(void) {
    char* a = rey_str_cstr("hello");
    char* b = rey_str_cstr(" world");
    char* c = rey_str_concat(a, b);
    CHECK(rey_str_eq(c, rey_str_cstr("hello world")), "concat");
    CHECK(rey_str_len(a) == 5, "len");
    CHECK(rey_str_starts_with(a, rey_str_cstr("hel")), "startsWith");
    CHECK(rey_str_ends_with(a, rey_str_cstr("llo")), "endsWith");
    CHECK(rey_str_index_of(c, rey_str_cstr("world")) == 6, "indexOf");
    CHECK(rey_str_eq(rey_str_slice(c, 6, 11), rey_str_cstr("world")), "slice");
    CHECK(rey_str_eq(rey_str_char_at(a, 1), rey_str_cstr("e")), "charAt");
    CHECK(rey_str_eq(rey_str_trim(rey_str_cstr("  hi  ")), rey_str_cstr("hi")), "trim");
    CHECK(rey_str_eq(rey_int_to_str(42), rey_str_cstr("42")), "intToStr");
    CHECK(rey_str_eq(rey_bool_to_str(1), rey_str_cstr("true")), "boolToStr true");
    CHECK(rey_str_eq(rey_bool_to_str(0), rey_str_cstr("false")), "boolToStr false");
    CHECK(!rey_str_eq(a, b), "neq");
    // printf compat: plain printf should print the data directly
    printf("  strings concat: %s\n", c);
    printf("  strings ok\n");
}

static void test_vec(void) {
    int64_t v = rey_vec_new();
    CHECK(rey_vec_len(v) == 0, "empty len");
    rey_vec_push(v, 10);
    rey_vec_push(v, 20);
    rey_vec_push(v, 30);
    CHECK(rey_vec_len(v) == 3, "len after push");
    CHECK(rey_vec_get(v, 0) == 10, "get 0");
    CHECK(rey_vec_get(v, 2) == 30, "get 2");
    CHECK(rey_vec_pop(v) == 30, "pop");
    CHECK(rey_vec_len(v) == 2, "len after pop");
    rey_vec_set(v, 0, 99);
    CHECK(rey_vec_get(v, 0) == 99, "set");

    // string vec
    int64_t sv = rey_vec_new();
    rey_vec_push(sv, (int64_t)(uintptr_t)rey_str_cstr("foo"));
    rey_vec_push(sv, (int64_t)(uintptr_t)rey_str_cstr("bar"));
    CHECK(rey_vec_str_contains(sv, rey_str_cstr("foo")), "contains found");
    CHECK(!rey_vec_str_contains(sv, rey_str_cstr("baz")), "contains not found");
    char* joined = rey_vec_join(sv, rey_str_cstr(", "));
    CHECK(rey_str_eq(joined, rey_str_cstr("foo, bar")), "join");
    printf("  vec ok\n");
}

static void test_hashmap(void) {
    int64_t hm = rey_hm_new();
    char* k1 = rey_str_cstr("name");
    char* k2 = rey_str_cstr("age");
    rey_hm_set(hm, k1, (int64_t)(uintptr_t)rey_str_cstr("Alice"));
    rey_hm_set(hm, k2, 30);
    CHECK(rey_hm_has(hm, k1), "has k1");
    CHECK(!rey_hm_has(hm, rey_str_cstr("missing")), "no missing");
    CHECK(rey_str_eq((char*)(uintptr_t)rey_hm_get(hm, k1), rey_str_cstr("Alice")), "get k1");
    CHECK(rey_hm_get(hm, k2) == 30, "get k2");
    rey_hm_set(hm, k1, (int64_t)(uintptr_t)rey_str_cstr("Bob"));
    CHECK(rey_str_eq((char*)(uintptr_t)rey_hm_get(hm, k1), rey_str_cstr("Bob")), "overwrite");
    int64_t keys = rey_hm_keys(hm);
    CHECK(rey_vec_len(keys) == 2, "keys len");
    printf("  hashmap ok\n");
}

static void test_result(void) {
    int64_t ok = rey_ok(42);
    int64_t err = rey_err_str(rey_str_cstr("oops"));
    CHECK(rey_result_is_ok(ok), "is_ok");
    CHECK(!rey_result_is_ok(err), "err is not ok");
    CHECK(rey_result_is_err(err), "is_err");
    CHECK(rey_result_unwrap(ok) == 42, "unwrap ok");
    CHECK(rey_result_unwrap_or(err, 99) == 99, "unwrap_or");
    CHECK(rey_instanceof_tag(ok, REY_TAG_OK), "instanceof ok");
    CHECK(rey_instanceof_tag(err, REY_TAG_ERR), "instanceof err");
    printf("  result ok\n");
}

static void test_io(void) {
    rey_init(0, NULL);
    int64_t args = rey_args();
    CHECK(rey_vec_len(args) == 0, "args empty");

    char* path = rey_str_cstr("/tmp/rey_rt_test.txt");
    rey_write_file(path, rey_str_cstr("hello rt\n"));
    CHECK(rey_file_exists(path), "file exists");
    char* content = rey_read_file(path);
    CHECK(rey_str_eq(content, rey_str_cstr("hello rt\n")), "read back");
    rey_delete_file(path);
    CHECK(!rey_file_exists(path), "file gone");
    printf("  io ok\n");
}

int main(void) {
    printf("rey_rt tests:\n");
    test_strings();
    test_vec();
    test_hashmap();
    test_result();
    test_io();
    printf("ALL OK\n");
    return 0;
}
