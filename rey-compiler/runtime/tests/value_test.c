#define _POSIX_C_SOURCE 200809L
#include "../rey_rt.h"
#include <errno.h>
#include <signal.h>
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
    static const char message[] = "FAIL: value regression test timed out\n";
    (void)signalNumber;
    (void)write(STDERR_FILENO, message, sizeof(message) - 1);
    _Exit(124);
}

static int64_t gLeft;
static int64_t gRight;
static int64_t gKind;
static const char* gText;

static void runChild(void (*body)(void), const char* needle, const char* name) {
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
    char buffer[1024];
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
    CHECK(strstr(buffer, "error: ") == buffer, name);
    CHECK(strstr(buffer, needle) != NULL, name);
}

static void unboxMismatch(void) { (void)rey_value_unbox(gLeft, gKind); }
static void nominalMismatch(void) { (void)rey_value_unbox_nominal(gLeft, gKind, gRight); }
static void compareFunctions(void) { (void)rey_value_eq(gLeft, gRight); }
static void tupleOutOfRange(void) { (void)rey_tuple_get(gLeft, gRight); }
static void envOutOfRange(void) { (void)rey_env_get(gLeft, gRight); }
static void missingKey(void) { (void)rey_hm_get_checked(gLeft, gText); }
static void vecKindMismatch(void) { (void)rey_vec_get_typed(gLeft, gRight, gKind); }
static void deepEqual(void) { (void)rey_tuple_eq(gLeft, gLeft); }

static int64_t capturedAdd(int64_t env, int64_t value) {
    return rey_env_get(env, 0) + value;
}

static int64_t ignoreEnv(int64_t env, int64_t value) {
    (void)env;
    return value + 1;
}

static int64_t floatBits(double value) {
    int64_t bits = 0;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static void testScalars(void) {
    int64_t number = rey_value_new(REY_VALUE_INT, -42);
    int64_t other = rey_value_new(REY_VALUE_INT, -42);
    int64_t flag = rey_value_new(REY_VALUE_BOOL, 0);
    int64_t nothing = rey_value_null();
    CHECK(rey_value_kind(number) == REY_VALUE_INT, "int kind");
    CHECK(rey_value_payload(number) == -42, "int payload");
    CHECK(rey_value_nominal(number) == 0, "scalar nominal is zero");
    CHECK(rey_value_eq(number, other), "equal ints");
    CHECK(!rey_value_eq(number, flag), "int is not bool");
    CHECK(!rey_value_is(0, REY_VALUE_NULL), "integer zero is not a null box");
    CHECK(rey_value_is(nothing, REY_VALUE_NULL), "null box kind");
    CHECK(!rey_value_is(nothing, REY_VALUE_INT), "null is not int");
    CHECK(rey_value_unbox(nothing, REY_VALUE_NULL) == 0, "null unbox");
    CHECK(rey_str_eq(rey_value_to_str(nothing), rey_str_cstr("null")), "null text");
    CHECK(rey_str_eq(rey_value_to_str(flag), rey_str_cstr("false")), "bool text");
    CHECK(rey_str_eq(rey_slot_to_str(REY_VALUE_CHAR, 'A'), rey_str_cstr("A")), "char text");

    double numberValue = 1.5;
    int64_t bits = rey_float_bits(numberValue);
    CHECK(rey_bits_to_float(bits) == numberValue, "float bits round trip");
    int64_t floatBox = rey_value_new(REY_VALUE_FLOAT, bits);
    int64_t negativeZero = rey_value_new(REY_VALUE_FLOAT, floatBits(-0.0));
    int64_t positiveZero = rey_value_new(REY_VALUE_FLOAT, floatBits(0.0));
    CHECK(rey_value_eq(negativeZero, positiveZero), "float equality uses IEEE ==");
    int64_t nanBits = (int64_t)UINT64_C(0x7ff8000000000001);
    CHECK(!rey_value_eq(rey_value_new(REY_VALUE_FLOAT, nanBits),
                        rey_value_new(REY_VALUE_FLOAT, nanBits)),
          "NaN is not equal to NaN");
    CHECK(rey_str_eq(rey_value_to_str(floatBox), rey_float_to_str(numberValue)), "float text");

    char* left = rey_str_cstr("same");
    char* right = rey_str_cstr("same");
    CHECK(rey_value_eq(rey_value_new(REY_VALUE_STRING, (int64_t)(uintptr_t)left),
                       rey_value_new(REY_VALUE_STRING, (int64_t)(uintptr_t)right)),
          "strings compare by bytes");
    CHECK(rey_value_to_str(rey_value_new(REY_VALUE_STRING, (int64_t)(uintptr_t)left)) == left,
          "string to-string borrows the payload");

    gLeft = number;
    gKind = REY_VALUE_BOOL;
    runChild(unboxMismatch, "value kind mismatch", "unbox mismatch diagnostic");
    printf("PASS: scalar boxes, equality, to-string, and checked unbox\n");
}

static void testStructsAndEnums(void) {
    void* small = rey_struct_alloc(8, 16);
    void* wide = rey_struct_alloc(300, 0);
    int64_t smallPtr = (int64_t)(uintptr_t)small;
    int64_t widePtr = (int64_t)(uintptr_t)wide;
    CHECK(rey_struct_type_id(smallPtr) == 8, "full type id 8");
    CHECK(rey_struct_is_type(smallPtr, 8), "struct identity 8");
    CHECK(rey_struct_instanceof(smallPtr, 8), "byte tag still matches id 8");
    CHECK(rey_struct_type_id(widePtr) == 300, "type id above 255 is preserved");
    CHECK(rey_struct_is_type(widePtr, 300), "full identity distinguishes 300");
    CHECK(!rey_struct_is_type(widePtr, 44), "truncated id is not the full id");
    CHECK(rey_struct_instanceof(widePtr, 300), "legacy instanceof compares the low byte");
    CHECK(!rey_struct_is_type(0, 8), "null struct is not a type");

    int64_t left = rey_value_new_nominal(REY_VALUE_STRUCT, smallPtr, 8);
    int64_t alias = rey_value_new_nominal(REY_VALUE_STRUCT, smallPtr, 8);
    int64_t distinct = rey_value_new_nominal(REY_VALUE_STRUCT, widePtr, 300);
    CHECK(rey_value_eq(left, alias), "struct equality is reference identity");
    CHECK(!rey_value_eq(left, distinct), "distinct struct references are unequal");
    CHECK(rey_value_is_nominal(left, REY_VALUE_STRUCT, 8), "struct instanceof uses nominal id");
    CHECK(!rey_value_is_nominal(left, REY_VALUE_STRUCT, 300), "wrong struct nominal fails");
    CHECK(rey_str_eq(rey_value_to_str(left), rey_str_cstr("struct#8")), "struct text");
    gLeft = left;
    gKind = REY_VALUE_STRUCT;
    gRight = 9;
    runChild(nominalMismatch, "nominal", "nominal unbox diagnostic");

    int64_t countA = rey_enum_new(21, 0, 1);
    rey_enum_set(countA, 0, REY_ENUM_INT, 7);
    int64_t countB = rey_enum_new(21, 0, 1);
    rey_enum_set(countB, 0, REY_ENUM_INT, 7);
    int64_t other = rey_enum_new(22, 0, 0);
    CHECK(rey_enum_identity(countA) == 21, "enum identity");
    CHECK(rey_enum_discriminant(countA) == 0, "enum discriminant");
    CHECK(rey_enum_is_type(countA, 21), "enum type test ignores discriminant");
    CHECK(!rey_enum_is_type(countA, 22), "enum type mismatch");
    CHECK(!rey_enum_is_type(smallPtr, 21), "struct pointer is not an enum");
    CHECK(!rey_enum_is_type(0, 21), "null is not an enum");
    int64_t boxedA = rey_value_new_nominal(REY_VALUE_ENUM, countA, 21);
    int64_t boxedB = rey_value_new_nominal(REY_VALUE_ENUM, countB, 21);
    int64_t boxedOther = rey_value_new_nominal(REY_VALUE_ENUM, other, 22);
    CHECK(rey_value_eq(boxedA, boxedB), "enum boxes compare structurally");
    CHECK(!rey_value_eq(boxedA, boxedOther), "different enum nominals are unequal");
    CHECK(rey_value_is_nominal(boxedA, REY_VALUE_ENUM, 21), "enum instanceof");
    CHECK(rey_str_eq(rey_enum_to_str(countA), rey_str_cstr("enum#21::0(7)")), "payload enum text");
    CHECK(rey_str_eq(rey_enum_to_str(other), rey_str_cstr("enum#22::0")), "unit enum text");
    printf("PASS: struct identity and enum nominal instanceof\n");
}

static void testTuples(void) {
    int64_t pair = rey_tuple_new(3);
    rey_tuple_set(pair, 0, REY_VALUE_INT, 1);
    rey_tuple_set(pair, 1, REY_VALUE_BOOL, 1);
    rey_tuple_set(pair, 2, REY_VALUE_FLOAT, floatBits(1.5));
    CHECK(rey_tuple_len(pair) == 3, "tuple length");
    CHECK(rey_tuple_get(pair, 0) == 1, "tuple get");
    CHECK(rey_tuple_kind(pair, 1) == REY_VALUE_BOOL, "tuple slot kind");
    char* expected = rey_str_cstr("(1, true, ");
    expected = rey_str_concat(expected, rey_float_to_str(1.5));
    expected = rey_str_concat(expected, rey_str_cstr(")"));
    CHECK(rey_str_eq(rey_tuple_to_str(pair), expected), "tuple text");

    int64_t copy = rey_tuple_new(3);
    rey_tuple_set(copy, 0, REY_VALUE_INT, 1);
    rey_tuple_set(copy, 1, REY_VALUE_BOOL, 1);
    rey_tuple_set(copy, 2, REY_VALUE_FLOAT, floatBits(1.5));
    CHECK(rey_tuple_eq(pair, copy), "tuple structural equality");
    rey_tuple_set(copy, 2, REY_VALUE_FLOAT, floatBits(-0.0));
    int64_t zeros = rey_tuple_new(1);
    rey_tuple_set(zeros, 0, REY_VALUE_FLOAT, floatBits(0.0));
    int64_t negZeros = rey_tuple_new(1);
    rey_tuple_set(negZeros, 0, REY_VALUE_FLOAT, floatBits(-0.0));
    CHECK(rey_tuple_eq(zeros, negZeros), "tuple float equality is numeric");

    int64_t one = rey_tuple_new(1);
    rey_tuple_set(one, 0, REY_VALUE_CHAR, 'Q');
    CHECK(rey_str_eq(rey_tuple_to_str(one), rey_str_cstr("(Q)")), "one-element tuple text");
    CHECK(rey_str_eq(rey_tuple_to_str(rey_tuple_new(0)), rey_str_cstr("()")), "empty tuple text");

    int64_t inner = rey_tuple_new(2);
    rey_tuple_set(inner, 0, REY_VALUE_INT, 1);
    rey_tuple_set(inner, 1, REY_VALUE_INT, 2);
    int64_t outer = rey_tuple_new(2);
    rey_tuple_set(outer, 0, REY_VALUE_TUPLE, inner);
    rey_tuple_set(outer, 1, REY_VALUE_NULL, 0);
    CHECK(rey_str_eq(rey_tuple_to_str(outer), rey_str_cstr("((1, 2), null)")), "nested tuple text");

    int64_t wrapped = rey_enum_new(30, 1, 1);
    rey_enum_set(wrapped, 0, REY_ENUM_TUPLE, inner);
    int64_t wrappedCopy = rey_enum_new(30, 1, 1);
    int64_t innerCopy = rey_tuple_new(2);
    rey_tuple_set(innerCopy, 0, REY_VALUE_INT, 1);
    rey_tuple_set(innerCopy, 1, REY_VALUE_INT, 2);
    rey_enum_set(wrappedCopy, 0, REY_ENUM_TUPLE, innerCopy);
    CHECK(rey_enum_eq(wrapped, wrappedCopy, 30), "enum tuple payload compares structurally");

    gLeft = pair;
    gRight = 3;
    runChild(tupleOutOfRange, "tuple index out of bounds", "tuple bounds diagnostic");
    int64_t cycle = rey_tuple_new(1);
    rey_tuple_set(cycle, 0, REY_VALUE_TUPLE, cycle);
    CHECK(strstr(rey_tuple_to_str(cycle), "...") != NULL, "cyclic tuple text stops");
    gLeft = cycle;
    runChild(deepEqual, "too deep", "cyclic tuple equality stops");
    printf("PASS: tuple get/set, structural equality, and text\n");
}

static void testClosures(void) {
    int64_t env = rey_env_new(2);
    rey_env_set(env, 0, REY_VALUE_INT, 7);
    rey_env_set(env, 1, REY_VALUE_STRING, (int64_t)(uintptr_t)rey_str_cstr("kept"));
    CHECK(rey_env_len(env) == 2, "env length");
    CHECK(rey_env_kind(env, 1) == REY_VALUE_STRING, "env slot kind");
    int64_t closure = rey_closure_new((int64_t)(uintptr_t)capturedAdd, env, 1);
    CHECK(rey_closure_arity(closure) == 1, "closure arity");
    CHECK(rey_closure_env(closure) == env, "closure retains env");
    typedef int64_t (*ClosureFn)(int64_t, int64_t);
    ClosureFn fn = (ClosureFn)(uintptr_t)rey_closure_fn(closure);
    CHECK(fn(rey_closure_env(closure), 10) == 17, "env is the first call argument");
    int64_t bare = rey_closure_new((int64_t)(uintptr_t)ignoreEnv, 0, 1);
    ClosureFn bareFn = (ClosureFn)(uintptr_t)rey_closure_fn(bare);
    CHECK(bareFn(0, 4) == 5, "closure with no captures uses a null env");
    int64_t left = rey_value_new(REY_VALUE_FUNC, closure);
    int64_t right = rey_value_new(REY_VALUE_FUNC, bare);
    CHECK(rey_str_eq(rey_value_to_str(left), rey_str_cstr("<func>")), "function text");
    gLeft = left;
    gRight = right;
    runChild(compareFunctions, "function values are not comparable", "function equality diagnostic");
    gLeft = env;
    gRight = 2;
    runChild(envOutOfRange, "env index out of bounds", "env bounds diagnostic");
    printf("PASS: closure environment and calling convention\n");
}

static void testCollections(void) {
    int64_t vec = rey_vec_new();
    rey_vec_push_float(vec, 1.5);
    rey_vec_push_float(vec, -2.0);
    rey_vec_push_typed(vec, REY_VALUE_STRING, (int64_t)(uintptr_t)rey_str_cstr("hi"));
    CHECK(rey_vec_len(vec) == 3, "typed vec length");
    CHECK(rey_vec_get_float(vec, 0) == 1.5, "float unbox");
    CHECK(rey_vec_kind(vec, 2) == REY_VALUE_STRING, "vec slot kind");
    rey_vec_set_float(vec, 1, 4.0);
    CHECK(rey_vec_get_float(vec, 1) == 4.0, "bounds-checked float set");
    int64_t sliced = rey_vec_slice_v(vec, 0, 2);
    CHECK(rey_vec_kind(sliced, 0) == REY_VALUE_FLOAT, "slice keeps slot kinds");
    CHECK(rey_vec_get_float(sliced, 1) == 4.0, "slice keeps float payload");
    int64_t nanBits = (int64_t)UINT64_C(0x7ff8000000000001);
    int64_t numbers = rey_vec_new();
    rey_vec_push_typed(numbers, REY_VALUE_FLOAT, nanBits);
    CHECK(!rey_vec_contains_typed(numbers, REY_VALUE_FLOAT, nanBits), "float contains uses numeric equality");
    CHECK(rey_vec_contains(numbers, nanBits), "raw contains compares stored bits");

    int64_t raw = rey_vec_new();
    for (int64_t i = 0; i < 3; i++) rey_vec_push(raw, i + 1);
    CHECK(rey_str_eq(rey_vec_to_str(raw), rey_str_cstr("[1, 2, 3]")), "vec text");
    int64_t seen = 0;
    for (int64_t i = 0; i < rey_vec_len(raw); i++) seen += rey_vec_get(raw, i);
    CHECK(seen == 6, "iteration is len plus bounds-checked get");
    gLeft = raw;
    gRight = 0;
    gKind = REY_VALUE_FLOAT;
    runChild(vecKindMismatch, "Vec element kind mismatch", "vec kind diagnostic");

    int64_t map = rey_hm_new();
    CHECK(rey_hm_len(map) == 0, "empty dict length");
    CHECK(rey_hm_get(map, rey_str_cstr("missing")) == 0, "legacy missing get returns zero");
    gLeft = map;
    gText = rey_str_cstr("missing");
    runChild(missingKey, "HashMap key not found", "missing key diagnostic");
    rey_hm_set(map, rey_str_cstr("zero"), 0);
    CHECK(rey_hm_has(map, rey_str_cstr("zero")), "stored zero is present");
    CHECK(rey_hm_get_checked(map, rey_str_cstr("zero")) == 0, "checked get returns stored zero");
    rey_hm_set_typed(map, rey_str_cstr("name"), REY_VALUE_STRING, (int64_t)(uintptr_t)rey_str_cstr("ada"));
    rey_hm_set_float(map, rey_str_cstr("pi"), 1.5);
    CHECK(rey_hm_len(map) == 3, "dict length counts live keys");
    CHECK(rey_hm_get_float(map, rey_str_cstr("pi")) == 1.5, "dict float unbox");
    CHECK(rey_str_eq((char*)(uintptr_t)rey_hm_get_typed(map, rey_str_cstr("name"), REY_VALUE_STRING),
                     rey_str_cstr("ada")),
          "dict string payload");
    rey_hm_set_float(map, rey_str_cstr("pi"), 2.5);
    CHECK(rey_hm_len(map) == 3, "overwrite keeps length");
    CHECK(rey_hm_get_float(map, rey_str_cstr("pi")) == 2.5, "overwrite updates float");

    const unsigned char leftBytes[] = {'a', 0, 'b'};
    const unsigned char rightBytes[] = {'a', 0, 'c'};
    char* leftKey = rey_str_new((const char*)leftBytes, 3);
    char* rightKey = rey_str_new((const char*)rightBytes, 3);
    char* shortKey = rey_str_new("a", 1);
    int64_t bytes = rey_hm_new();
    rey_hm_set(bytes, leftKey, 1);
    rey_hm_set(bytes, rightKey, 2);
    rey_hm_set(bytes, shortKey, 3);
    CHECK(rey_hm_len(bytes) == 3, "embedded NUL keys are distinct");
    CHECK(rey_hm_get(bytes, leftKey) == 1 && rey_hm_get(bytes, rightKey) == 2, "NUL key lookup");

    int64_t grown = rey_hm_new();
    for (int i = 0; i < 64; i++) {
        char buf[16];
        snprintf(buf, sizeof(buf), "k%d", i);
        rey_hm_set_float(grown, rey_str_cstr(buf), (double)i + 0.5);
    }
    CHECK(rey_hm_len(grown) == 64, "dict grows past the initial capacity");
    CHECK(rey_hm_get_float(grown, rey_str_cstr("k0")) == 0.5, "first float survives resize");
    CHECK(rey_hm_get_float(grown, rey_str_cstr("k63")) == 63.5, "last float survives resize");

    int64_t entries = rey_hm_entries(map);
    int64_t keys = rey_hm_keys(map);
    int64_t values = rey_hm_values(map);
    CHECK(rey_vec_len(entries) == rey_hm_len(map), "entries length");
    CHECK(rey_vec_len(keys) == rey_vec_len(entries), "entries follow keys order");
    for (int64_t i = 0; i < rey_vec_len(keys); i++) {
        int64_t pair = rey_vec_get(entries, i);
        char* key = (char*)(uintptr_t)rey_vec_get(keys, i);
        CHECK(rey_vec_kind(entries, i) == REY_VALUE_VEC, "entry element kind");
        CHECK(rey_vec_len(pair) == 2, "entry pair length");
        CHECK(rey_vec_kind(pair, 0) == REY_VALUE_STRING, "entry key kind");
        CHECK(rey_str_eq((char*)(uintptr_t)rey_vec_get(pair, 0), key), "entry key matches keys()");
        CHECK(rey_vec_get(pair, 1) == rey_vec_get(values, i), "entry value matches values()");
        int64_t valueKind = REY_VALUE_INT;
        if (rey_str_eq(key, rey_str_cstr("name"))) valueKind = REY_VALUE_STRING;
        else if (rey_str_eq(key, rey_str_cstr("pi"))) valueKind = REY_VALUE_FLOAT;
        CHECK(rey_vec_kind(pair, 1) == valueKind, "entry value kind");
    }
    int64_t single = rey_hm_new();
    rey_hm_set(single, rey_str_cstr("name"), 7);
    CHECK(rey_str_eq(rey_hm_to_str(single), rey_str_cstr("{name: 7}")), "dict text");
    printf("PASS: vec/dict element kinds, missing-key error, and growth\n");
}

static void testDynamicNesting(void) {
    int64_t inner = rey_value_new(REY_VALUE_INT, 9);
    int64_t any = rey_value_new(REY_VALUE_ANY, inner);
    CHECK(rey_value_eq(any, rey_value_new(REY_VALUE_ANY, rey_value_new(REY_VALUE_INT, 9))),
          "any compares the boxed value");
    CHECK(rey_str_eq(rey_value_to_str(any), rey_str_cstr("9")), "any text unwraps one level");
    int64_t some = rey_some_typed(9, REY_ENUM_INT);
    int64_t none = rey_none();
    CHECK(rey_str_eq(rey_option_to_str(some), rey_str_cstr("Some(9)")), "Some text");
    CHECK(rey_str_eq(rey_option_to_str(none), rey_str_cstr("None")), "None text");
    CHECK(rey_str_eq(rey_result_to_str(rey_ok_typed(4, REY_ENUM_INT)), rey_str_cstr("Ok(4)")), "Ok text");
    CHECK(rey_str_eq(rey_result_to_str(rey_err_str(rey_str_cstr("no"))), rey_str_cstr("Err(no)")), "Err text");
    int64_t nullable = rey_value_new(REY_VALUE_OPTION, some);
    CHECK(rey_value_is(nullable, REY_VALUE_OPTION), "nullable option kind");
    CHECK(rey_option_unwrap(rey_value_unbox(nullable, REY_VALUE_OPTION)) == 9, "unbox option payload");
    printf("PASS: any, option, and result dynamic values\n");
}

int main(void) {
    signal(SIGALRM, onTimeout);
    alarm(60);
    testScalars();
    testStructsAndEnums();
    testTuples();
    testClosures();
    testCollections();
    testDynamicNesting();
    alarm(0);
    printf("PASS: value ABI tests (%lld assertions)\n", (long long)assertions);
    return 0;
}
