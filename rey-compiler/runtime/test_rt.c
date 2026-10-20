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
    CHECK(rey_str_hash(a) == 'h', "hash first byte");
    char nulBytes[] = {'a', 0, 'b', 'a', 0, 'b'};
    char* nulText = rey_str_new(nulBytes, 6);
    char nulNeedle[] = {0, 'b'};
    char* nulSub = rey_str_new(nulNeedle, 2);
    CHECK(rey_str_index_of(nulText, nulSub) == 1, "indexOf embedded nul");
    CHECK(rey_str_last_index_of(nulText, nulSub) == 4, "lastIndexOf embedded nul");
    CHECK(rey_str_eq(rey_str_trim_start(rey_str_cstr("  hi")), rey_str_cstr("hi")), "trimStart");
    CHECK(rey_str_eq(rey_str_trim_end(rey_str_cstr("hi  ")), rey_str_cstr("hi")), "trimEnd");
    CHECK(rey_str_eq(rey_str_pad_left(rey_str_cstr("ab"), 4, rey_str_cstr("xy")), rey_str_cstr("xxab")), "padLeft");
    CHECK(rey_str_eq(rey_str_pad_right(rey_str_cstr("ab"), 4, rey_str_cstr("xy")), rey_str_cstr("abxx")), "padRight");
    CHECK(rey_str_eq(rey_str_replace(rey_str_cstr("ab"), rey_str_empty(), rey_str_cstr("-")), rey_str_cstr("-a-b-")), "empty replace");
    CHECK(rey_str_eq(rey_str_replace(rey_str_cstr("hello"), rey_str_cstr("l"), rey_str_cstr("L")), rey_str_cstr("heLLo")), "replace");
    CHECK(rey_abs_int(-3) == 3, "abs int");
    CHECK(rey_abs_float(-1.5) == 1.5, "abs float");
    CHECK(rey_min_int(3, -1) == -1 && rey_max_int(3, -1) == 3, "min max int");
    CHECK(rey_pow_int(2, 10) == 1024, "pow int");
    CHECK(rey_pow_int(-2, 3) == -8, "pow negative base");
    CHECK(rey_pow_int(0, 0) == 1, "pow zero zero");
    CHECK(rey_pow_float(3.0, 2.0) == 9.0, "pow float");
    CHECK(rey_floor(-1.2) == -2 && rey_ceil(-1.2) == -1, "floor ceil");
    CHECK(rey_round(1.2) == 1 && rey_round(1.8) == 2, "round");
    CHECK(rey_round(1.5) == 2 && rey_round(-1.5) == -2, "round halves");
    CHECK(rey_sqrt(4.0) == 2.0, "sqrt");
    CHECK(rey_log(1.0) == 0.0 && rey_sin(0.0) == 0.0 && rey_cos(0.0) == 1.0, "log sin cos");
    CHECK(rey_str_to_int(rey_str_cstr("12.8")) == 12, "toInt truncates");
    CHECK(rey_int_from_char(rey_char_from_int(65)) == 65, "char roundtrip");
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
    CHECK(rey_vec_str_index_of(sv, rey_str_cstr("bar")) == 1, "string indexOf");

    int64_t nums = rey_vec_new();
    rey_vec_push(nums, 3);
    rey_vec_push(nums, 1);
    rey_vec_push(nums, 2);
    CHECK(rey_vec_index_of(nums, 1) == 1, "indexOf");
    rey_vec_sort(nums);
    CHECK(rey_vec_get(nums, 0) == 1 && rey_vec_get(nums, 2) == 3, "sort int");
    rey_vec_insert(nums, 0, 0);
    CHECK(rey_vec_get(nums, 0) == 0 && rey_vec_len(nums) == 4, "insert");
    CHECK(rey_vec_remove(nums, 1) == 1, "remove");
    rey_vec_reverse_mut(nums);
    CHECK(rey_vec_get(nums, 0) == 3, "reverse mut");
    rey_vec_clear(nums);
    CHECK(rey_vec_len(nums) == 0, "clear");

    int64_t words = rey_vec_new();
    rey_vec_push_typed(words, REY_VALUE_STRING, (int64_t)(uintptr_t)rey_str_cstr("b"));
    rey_vec_push_typed(words, REY_VALUE_STRING, (int64_t)(uintptr_t)rey_str_cstr("a"));
    rey_vec_sort(words);
    CHECK(rey_str_eq((char*)(uintptr_t)rey_vec_get(words, 0), rey_str_cstr("a")), "sort string");

    int64_t floats = rey_vec_new();
    rey_vec_push_float(floats, 2.5);
    rey_vec_push_float(floats, -1.0);
    rey_vec_sort(floats);
    CHECK(rey_vec_get_float(floats, 0) == -1.0, "sort float");
    printf("  vec ok\n");
}

static int64_t sumReduce(int64_t env, int64_t acc, int64_t item) {
    return acc + item + env;
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
    CHECK(rey_vec_len(rey_hm_values(hm)) == 2, "values len");
    rey_hm_clear(hm);
    CHECK(rey_hm_len(hm) == 0 && !rey_hm_has(hm, k1), "map clear");
    int64_t reduced = rey_vec_new();
    rey_vec_push(reduced, 2);
    rey_vec_push(reduced, 5);
    CHECK(rey_vec_reduce(reduced, (int64_t)(uintptr_t)sumReduce, 1, 10) == 19, "reduce");
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

static void test_enum(void) {
    int64_t first = rey_enum_new(12, 0, 1);
    rey_enum_set(first, 0, REY_ENUM_INT, 7);
    int64_t same = rey_enum_new(12, 0, 1);
    rey_enum_set(same, 0, REY_ENUM_INT, 7);
    int64_t different = rey_enum_new(12, 1, 1);
    rey_enum_set(different, 0, REY_ENUM_INT, 7);
    CHECK(rey_enum_is(first, 12, 0), "enum variant test");
    CHECK(!rey_enum_is(first, 12, 1), "enum different variant test");
    CHECK(rey_enum_get(first, 12, 0, 0, REY_ENUM_INT) == 7, "enum payload read");
    CHECK(rey_enum_eq(first, same, 12), "enum values compare by discriminant and scalar payload");
    CHECK(!rey_enum_eq(first, different, 12), "enum values compare discriminants");
    CHECK(!rey_enum_eq(first, same, 13), "enum values compare identity");

    int64_t nested_a = rey_enum_new(13, 0, 1);
    rey_enum_set(nested_a, 0, REY_ENUM_ENUM, first);
    int64_t nested_b = rey_enum_new(13, 0, 1);
    rey_enum_set(nested_b, 0, REY_ENUM_ENUM, same);
    CHECK(rey_enum_eq(nested_a, nested_b, 13), "nested enum values compare structurally");

    int64_t shared_box = 0x1234;
    int64_t box_a = rey_enum_new(14, 0, 1);
    rey_enum_set(box_a, 0, REY_ENUM_STRUCT, shared_box);
    int64_t box_alias = rey_enum_new(14, 0, 1);
    rey_enum_set(box_alias, 0, REY_ENUM_STRUCT, shared_box);
    int64_t box_copy = rey_enum_new(14, 0, 1);
    rey_enum_set(box_copy, 0, REY_ENUM_STRUCT, 0x5678);
    CHECK(rey_enum_eq(box_a, box_alias, 14), "enum struct payload compares reference identity");
    CHECK(!rey_enum_eq(box_a, box_copy, 14), "separate struct payload references are unequal");

    int64_t unit_a = rey_enum_new(15, 2, 0);
    int64_t unit_b = rey_enum_new(15, 2, 0);
    CHECK(rey_enum_eq(unit_a, unit_b, 15), "unit enum values compare structurally");
}

static int64_t capturedAdd(int64_t env, int64_t x) {
    return rey_env_get(env, 0) + x;
}

static void test_dynamic(void) {
    int64_t boxed = rey_value_new(REY_VALUE_INT, 42);
    CHECK(rey_value_is(boxed, REY_VALUE_INT), "dynamic int kind");
    CHECK(rey_value_unbox(boxed, REY_VALUE_INT) == 42, "dynamic int unbox");
    CHECK(rey_str_eq(rey_value_to_str(rey_value_null()), rey_str_cstr("null")), "dynamic null text");

    int64_t pair = rey_tuple_new(2);
    rey_tuple_set(pair, 0, REY_VALUE_INT, 1);
    rey_tuple_set(pair, 1, REY_VALUE_BOOL, 1);
    CHECK(rey_str_eq(rey_tuple_to_str(pair), rey_str_cstr("(1, true)")), "tuple text");

    int64_t env = rey_env_new(1);
    rey_env_set(env, 0, REY_VALUE_INT, 7);
    int64_t closure = rey_closure_new((int64_t)(uintptr_t)capturedAdd, env, 1);
    typedef int64_t (*ClosureFn)(int64_t, int64_t);
    ClosureFn fn = (ClosureFn)(uintptr_t)rey_closure_fn(closure);
    CHECK(fn(rey_closure_env(closure), 10) == 17, "closure passes env as first argument");

    int64_t map = rey_hm_new();
    rey_hm_set(map, rey_str_cstr("n"), 3);
    CHECK(rey_hm_len(map) == 1, "map len");
    CHECK(rey_hm_get_checked(map, rey_str_cstr("n")) == 3, "checked map get");

    void* data = rey_struct_alloc(8, 8);
    CHECK(rey_struct_type_id((int64_t)(uintptr_t)data) == 8, "struct type id");
    CHECK(rey_struct_instanceof((int64_t)(uintptr_t)data, 8), "struct byte tag");
    printf("  dynamic ok\n");
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
    test_enum();
    test_dynamic();
    test_io();
    printf("ALL OK\n");
    return 0;
}
