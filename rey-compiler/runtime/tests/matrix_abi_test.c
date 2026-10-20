#define _POSIX_C_SOURCE 200809L
#include "../rey_rt.h"
#include <errno.h>
#include <math.h>
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
    static const char message[] = "FAIL: matrix ABI test timed out\n";
    (void)signalNumber;
    (void)write(STDERR_FILENO, message, sizeof(message) - 1);
    _Exit(124);
}

static const char* gText;
static int64_t gMap;

static void expectError(void (*body)(void), const char* needle, const char* name) {
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
    CHECK(strncmp(buffer, "error: ", 7) == 0 && strstr(buffer, needle) != NULL, name);
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
    CHECK(strncmp(buffer, "rey panic: ", 11) == 0 && strstr(buffer, needle) != NULL, name);
}

static void parseInt(void) { (void)rey_str_to_int(gText); }
static void parseFloat(void) { (void)rey_str_to_float(gText); }
static void convertFloat(void) { (void)rey_float_to_int(rey_bits_to_float((int64_t)UINT64_C(0x7ff0000000000000))); }
static void convertNan(void) { (void)rey_float_to_int(rey_bits_to_float((int64_t)UINT64_C(0x7ff8000000000001))); }
static void convertUpper(void) { (void)rey_float_to_int(9223372036854775808.0); }
static void negativePack(void) { (void)rey_pack_vec(-1); }
static void missingKey(void) { (void)rey_hm_get_checked(gMap, gText); }
static void missingFloat(void) { (void)rey_hm_get_float(gMap, gText); }
static void charHigh(void) { (void)rey_char_from_int(256); }
static void charLow(void) { (void)rey_char_from_int(-1); }
static void charNull(void) { (void)rey_int_from_char(NULL); }
static void missingFile(void) { (void)rey_read_file(rey_str_cstr("/tmp/rey-rt-missing-file-9f23c7")); }
static void directoryFile(void) { (void)rey_read_file(rey_str_cstr("/tmp")); }
static void writeDirectory(void) { rey_write_file(rey_str_cstr("/tmp"), rey_str_cstr("x")); }
static void unboxNull(void) { (void)rey_nullable_unbox_int(0); }
static void unboxKind(void) { (void)rey_nullable_unbox_int(rey_nullable_box_bool(1)); }

static void testConversions(void) {
    CHECK(rey_str_to_int(rey_str_cstr("12.8")) == 12, "fraction truncates toward zero");
    CHECK(rey_str_to_int(rey_str_cstr("1_2.5")) == 12, "underscore digits parse");
    CHECK(rey_str_to_int(rey_str_cstr("-3.9")) == -3, "negative truncation");
    CHECK(rey_str_to_int(rey_str_cstr("+8")) == 8, "explicit plus");
    CHECK(rey_str_to_int(rey_str_cstr("12.")) == 12, "trailing dot");
    CHECK(rey_str_to_float(rey_str_cstr("1_2.5")) == 12.5, "float text");
    CHECK(rey_float_to_int(-9223372036854775808.0) == INT64_MIN, "int minimum float converts");
    CHECK(rey_float_to_int(1.9) == 1 && rey_float_to_int(-1.9) == -1, "float truncation");
    CHECK(rey_int_to_float(3) == 3.0, "small int to float");
    CHECK(rey_int_to_float(INT64_MIN) == -9223372036854775808.0, "int minimum to float");
    CHECK(rey_int_to_float(INT64_C(9007199254740993)) == 9007199254740992.0, "int to binary64 rounds");
    CHECK(rey_str_eq(rey_int_to_str(12), rey_str_cstr("12")), "int toString");
    CHECK(rey_str_eq(rey_float_to_str(12.0), rey_str_cstr("12")), "integral float toString");
    CHECK(rey_str_eq(rey_bool_to_str(0), rey_str_cstr("false")), "bool toString");
    CHECK(rey_str_eq(rey_typed_val_to_str(REY_VALUE_NULL, 0), rey_str_cstr("null")), "null toString");

    const char* bad[] = {"x", "12oops", "1__2", ".5", "1e2", " 12", "12 "};
    for (size_t i = 0; i < sizeof(bad) / sizeof(bad[0]); i++) {
        gText = rey_str_cstr(bad[i]);
        expectError(parseInt, "Invalid ASCII decimal text", "malformed int text");
        expectError(parseFloat, "Invalid ASCII decimal text", "malformed float text");
    }
    gText = rey_str_empty();
    expectError(parseInt, "Invalid ASCII decimal text", "empty int text");
    char huge[402];
    huge[0] = '1';
    memset(huge + 1, '0', 400);
    huge[401] = '\0';
    gText = rey_str_cstr(huge);
    expectError(parseFloat, "Decimal value out of float range", "oversized decimal");
    gText = rey_str_cstr("9223372036854775808");
    expectError(parseInt, "Float value out of int range", "string above int range");
    expectError(convertFloat, "Float value out of int range", "infinity to int");
    expectError(convertNan, "Float value out of int range", "nan to int");
    expectError(convertUpper, "Float value out of int range", "2^63 to int");
    printf("PASS: int/float/string conversions\n");
}

static void testPackAndCollections(void) {
    CHECK(rey_vec_len(rey_pack_vec(0)) == 0, "empty variadic tail");
    expectError(negativePack, "variadic count is negative", "negative variadic count");
    int64_t packed = rey_pack_vec(3);
    rey_pack_set(packed, 0, REY_VALUE_INT, 4);
    rey_pack_set(packed, 1, REY_VALUE_STRING, (int64_t)(uintptr_t)rey_str_cstr("a"));
    rey_pack_set_float(packed, 2, 1.25);
    CHECK(rey_vec_len(packed) == 3, "packed length");
    CHECK(rey_vec_get_typed(packed, 0, REY_VALUE_INT) == 4, "packed int");
    CHECK(rey_str_eq((char*)(uintptr_t)rey_vec_get_typed(packed, 1, REY_VALUE_STRING), rey_str_cstr("a")),
          "packed string");
    CHECK(rey_vec_get_float(packed, 2) == 1.25, "packed float");

    int64_t numbers = rey_vec_new();
    rey_vec_push_float(numbers, 1.5);
    rey_vec_push_float(numbers, -2.25);
    double sum = 0;
    for (int64_t index = 0; index < rey_vec_len(numbers); index++)
        sum += rey_vec_get_float(numbers, index);
    CHECK(sum == -0.75, "typed float iteration");
    rey_vec_set_float(numbers, 1, 0.25);
    CHECK(rey_vec_get_float(numbers, 1) == 0.25, "float indexed set");

    int64_t map = rey_hm_new();
    rey_hm_set_float(map, rey_str_cstr("pi"), 1.5);
    rey_hm_set_typed(map, rey_str_cstr("name"), REY_VALUE_STRING, (int64_t)(uintptr_t)rey_str_cstr("ada"));
    rey_hm_set(map, rey_str_cstr("zero"), 0);
    CHECK(rey_hm_len(map) == 3, "dict literal length");
    CHECK(rey_hm_get_float(map, rey_str_cstr("pi")) == 1.5, "dict float index");
    CHECK(rey_hm_get_checked(map, rey_str_cstr("zero")) == 0, "stored zero is indexed");
    gMap = map;
    gText = rey_str_cstr("missing");
    expectError(missingKey, "HashMap key not found", "missing dict index");
    expectError(missingFloat, "HashMap key not found", "missing float dict index");
    printf("PASS: variadic pack, float Vec iteration, and dict index\n");
}

static void testCompilerBuiltins(void) {
    const char* letter = rey_char_from_int(65);
    CHECK(rey_str_len(letter) == 1 && letter[0] == 'A', "char 65 is one byte A");
    CHECK(rey_str_eq(letter, rey_str_cstr("A")), "char toString is the byte string");
    CHECK(rey_int_from_char(letter) == 65, "int(char) reads the first byte");
    const char* zero = rey_char_from_int(0);
    CHECK(rey_str_len(zero) == 1 && zero[0] == '\0', "char 0 keeps a NUL byte");
    CHECK(rey_int_from_char(zero) == 0, "int of NUL char");
    CHECK(rey_int_from_char(rey_char_from_int(255)) == 255, "char 255 round trip");
    expectError(charHigh, "char value out of range 0..=255", "char 256");
    expectError(charLow, "char value out of range 0..=255", "char negative");
    expectError(charNull, "char value is null", "null char");
    expectError(missingFile, "readFile: cannot open", "missing readFile");
    expectError(directoryFile, "readFile: cannot open", "directory readFile");
    expectPanic(writeDirectory, "writeFile: cannot open for writing", "directory writeFile");
    const char rawBytes[] = {'a', '\0', 'b'};
    const char* rawPath = rey_str_cstr("/tmp/rey-rt-nul-file-9f23c7");
    rey_write_file(rawPath, rey_str_new(rawBytes, 3));
    const char* rawBack = rey_read_file(rawPath);
    CHECK(rey_str_len(rawBack) == 3 && memcmp(rawBack, rawBytes, 3) == 0, "readFile keeps embedded NUL");
    rey_delete_file(rawPath);
    CHECK(rey_vec_len(rey_list_dir("/tmp/rey-rt-missing-dir-9f23c7")) == 0, "missing listDir is empty");

    char programName[] = "rey-compiler";
    char commandName[] = "check";
    char sourceName[] = "file with spaces.rey";
    char* argv[] = {programName, commandName, sourceName};
    rey_init(3, argv);
    int64_t cliArgs = rey_args();
    CHECK(rey_vec_len(cliArgs) == 3, "args includes the program name");
    CHECK(rey_str_eq((const char*)(uintptr_t)rey_vec_get(cliArgs, 0), rey_str_cstr("rey-compiler")), "argv0");
    CHECK(rey_str_eq((const char*)(uintptr_t)rey_vec_get(cliArgs, 2), rey_str_cstr("file with spaces.rey")), "arg with spaces");
    sourceName[0] = 'X';
    CHECK(rey_str_eq((const char*)(uintptr_t)rey_vec_get(cliArgs, 2), rey_str_cstr("file with spaces.rey")), "args copies its strings");
    rey_init(0, NULL);

    const char* path = rey_str_cstr("std/vec.rey");
    CHECK(rey_str_contains(path, rey_str_cstr("/")) == 1, "string contains slash");
    CHECK(rey_str_contains(path, rey_str_cstr("::")) == 0, "string misses colon pair");
    CHECK(rey_str_index_of(rey_str_cstr("a\nb"), rey_str_cstr("\n")) == 1, "index of newline");
    const char* padded = rey_str_new(" \t hi\n\v\f\r ", 10);
    CHECK(rey_str_eq(rey_str_trim(padded), rey_str_cstr("hi")), "trim keeps only ASCII whitespace");
    unsigned char high[] = {(unsigned char)0xA0, 'x', (unsigned char)0xA0};
    char* kept = rey_str_new((const char*)high, 3);
    CHECK(rey_str_len(rey_str_trim(kept)) == 3, "trim leaves non-ASCII bytes");

    int64_t stack = rey_vec_new();
    char* first = rey_str_cstr("a.rey");
    char* again = rey_str_cstr("a.rey");
    rey_vec_push(stack, (int64_t)(uintptr_t)first);
    CHECK(rey_vec_str_contains(stack, again) == 1, "Vec string contains is by bytes");
    CHECK(rey_str_eq(rey_vec_join(stack, rey_str_cstr(" -> ")), rey_str_cstr("a.rey")), "Vec join");
    printf("PASS: compiler char, string search, trim, and Vec contains\n");
}

static void testScalarNullable(void) {
    int64_t shared = rey_nullable_null();
    CHECK(rey_nullable_is_null(0) == 1, "pointer null is null");
    CHECK(rey_nullable_is_null(shared) == 1, "shared scalar null");
    CHECK(rey_nullable_eq(0, shared) == 1, "pointer null equals scalar null");
    CHECK(rey_str_eq(rey_nullable_to_str(0), rey_str_cstr("null")), "null text");
    CHECK(rey_str_eq(rey_nullable_to_str(shared), rey_str_cstr("null")), "shared null text");

    int64_t zero = rey_nullable_box_int(0);
    CHECK(rey_nullable_is_null(zero) == 0, "boxed int zero is present");
    CHECK(rey_nullable_unbox_int(zero) == 0, "unbox int zero");
    CHECK(rey_nullable_eq(zero, 0) == 0, "int zero is not null");
    CHECK(rey_str_eq(rey_nullable_to_str(rey_nullable_box_int(7)), rey_str_cstr("7")), "int text");
    CHECK(rey_nullable_eq(rey_nullable_box_int(7), rey_nullable_box_int(7)) == 1, "int equality");
    CHECK(rey_nullable_eq(rey_nullable_box_int(7), rey_nullable_box_int(8)) == 0, "int inequality");

    CHECK(rey_nullable_is_null(rey_nullable_box_bool(0)) == 0, "false is present");
    CHECK(rey_nullable_unbox_bool(rey_nullable_box_bool(0)) == 0, "unbox false");
    CHECK(rey_str_eq(rey_nullable_to_str(rey_nullable_box_bool(0)), rey_str_cstr("false")), "false text");
    CHECK(rey_nullable_eq(rey_nullable_box_bool(1), rey_nullable_box_int(1)) == 0, "bool is not int");

    int64_t letter = rey_nullable_box_char(65);
    int64_t nul = rey_nullable_box_char(0);
    CHECK(rey_nullable_is_null(nul) == 0, "char zero is present");
    CHECK(rey_nullable_unbox_char(letter) == 65, "unbox char");
    CHECK(rey_str_eq(rey_nullable_to_str(letter), rey_str_cstr("A")), "char text");
    CHECK(rey_str_len(rey_nullable_to_str(nul)) == 1 && rey_nullable_to_str(nul)[0] == '\0', "NUL char text");

    CHECK(rey_nullable_unbox_float(rey_nullable_box_float(1.5)) == 1.5, "unbox float");
    CHECK(rey_nullable_eq(rey_nullable_box_float(-0.0), rey_nullable_box_float(0.0)) == 1, "float negative zero");
    int64_t nanBits = (int64_t)UINT64_C(0x7ff8000000000001);
    int64_t nanBox = rey_nullable_box_float(rey_bits_to_float(nanBits));
    CHECK(rey_float_bits(rey_nullable_unbox_float(nanBox)) == nanBits, "NaN bits survive unbox");
    CHECK(rey_nullable_eq(nanBox, nanBox) == 0, "NaN is not equal to itself");
    expectError(unboxNull, "null dereference", "unbox null");
    expectError(unboxKind, "nullable kind mismatch", "unbox wrong kind");
    printf("PASS: scalar nullable box, null equality, and text\n");
}

static void testInstanceof(void) {
    int64_t boxed = rey_value_new(REY_VALUE_INT, 3);
    CHECK(rey_instanceof(boxed, REY_VALUE_INT, 0) == 1, "boxed int");
    CHECK(rey_instanceof(boxed, REY_VALUE_BOOL, 0) == 0, "int is not bool");
    CHECK(rey_instanceof(boxed, REY_VALUE_INT, 9) == 0, "primitive nominal is rejected");
    CHECK(rey_instanceof(0, REY_VALUE_INT, 0) == 0, "unboxed zero is not a dynamic int");
    int64_t nullBox = rey_value_null();
    CHECK(rey_instanceof(nullBox, REY_VALUE_NULL, 0) == 1, "null kind");
    CHECK(rey_instanceof(nullBox, REY_VALUE_INT, 0) == 0, "null is not int");
    int64_t data = (int64_t)(uintptr_t)rey_struct_alloc(300, 8);
    int64_t structBox = rey_value_new_nominal(REY_VALUE_STRUCT, data, 300);
    CHECK(rey_instanceof(structBox, REY_VALUE_STRUCT, 300) == 1, "struct nominal");
    CHECK(rey_instanceof(structBox, REY_VALUE_STRUCT, 8) == 0, "other struct id");
    CHECK(rey_struct_is_type(data, 300) == 1, "unboxed struct id");
    int64_t enumeration = rey_enum_new(4, 1, 0);
    int64_t enumBox = rey_value_new_nominal(REY_VALUE_ENUM, enumeration, 4);
    CHECK(rey_instanceof(enumBox, REY_VALUE_ENUM, 4) == 1, "enum nominal");
    CHECK(rey_enum_is_type(enumeration, 4) == 1 && rey_enum_is_type(enumeration, 5) == 0, "unboxed enum id");
    printf("PASS: instanceof kind and nominal queries\n");
}

int main(void) {
    signal(SIGALRM, onTimeout);
    alarm(30);
    testConversions();
    testPackAndCollections();
    testCompilerBuiltins();
    testScalarNullable();
    testInstanceof();
    alarm(0);
    printf("PASS: matrix ABI (%lld assertions)\n", (long long)assertions);
    return 0;
}
