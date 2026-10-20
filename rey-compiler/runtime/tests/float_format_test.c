#include "../rey_rt.h"
#include <float.h>
#include <inttypes.h>
#include <locale.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition, message) do { \
    if (!(condition)) { fprintf(stderr, "FAIL: %s\n", message); exit(1); } \
} while (0)

_Static_assert(REY_VALUE_INT == 4097, "int kind ABI changed");
_Static_assert(REY_VALUE_BOOL == 4098, "bool kind ABI changed");
_Static_assert(REY_VALUE_FLOAT == 4099, "float kind ABI changed");
_Static_assert(REY_VALUE_STRING == 4100, "string kind ABI changed");
_Static_assert(REY_VALUE_CHAR == 4101, "char kind ABI changed");
_Static_assert(REY_VALUE_NULL == 4102, "null kind ABI changed");

static uint64_t floatBits(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static double floatFromBits(uint64_t bits) {
    double value;
    memcpy(&value, &bits, sizeof(value));
    return value;
}

static void checkFormat(double value, const char* expected, const char* name) {
    char* actual = rey_float_to_str(value);
    if (strcmp(actual, expected) != 0) {
        fprintf(stderr, "FAIL: %s: expected '%s', got '%s'\n", name, expected, actual);
        exit(1);
    }
}

static void checkRoundTrip(double value, const char* name) {
    char* formatted = rey_float_to_str(value);
    char* end = NULL;
    double parsed = strtod(formatted, &end);
    if (!end || *end != '\0' || floatBits(parsed) != floatBits(value)) {
        fprintf(stderr, "FAIL: %s: round-trip bits %016" PRIx64 " -> '%s' -> %016" PRIx64 "\n",
                name, floatBits(value), formatted, floatBits(parsed));
        exit(1);
    }
    CHECK(strchr(formatted, 'E') == NULL, "exponent is lowercase e");
    CHECK(strstr(formatted, "e+") == NULL, "positive exponent has no plus sign");
    CHECK(strchr(formatted, ',') == NULL, "decimal point is a dot");
}

static void testReferenceSpellings(void) {
    checkFormat(0.0, "0", "zero");
    checkFormat(-0.0, "-0", "negative zero");
    checkFormat(1.0, "1", "one omits .0");
    checkFormat(-1.0, "-1", "negative one omits .0");
    checkFormat(1.5, "1.5", "one and a half");
    checkFormat(0.1, "0.1", "tenth");
    checkFormat(1e-4, "0.0001", "1e-4 stays fixed");
    checkFormat(1e-5, "1e-5", "1e-5 is scientific");
    checkFormat(1e-6, "1e-6", "1e-6 is scientific");
    checkFormat(1e15, "1000000000000000", "1e15 stays fixed");
    checkFormat(1e16, "1e16", "1e16 is scientific");
    checkFormat(1e20, "1e20", "1e20 is scientific");
    checkFormat(1000.0, "1000", "integral power of ten");
    checkFormat(9007199254740992.0, "9007199254740992", "2^53");
    checkFormat(floatFromBits(UINT64_C(0x4450000000000000)), "1.1805916207174113e21", "2^70");
    checkFormat(DBL_MAX, "1.7976931348623157e308", "largest finite");
    checkFormat(floatFromBits(UINT64_C(1)), "5e-324", "minimum positive subnormal");
    checkFormat(floatFromBits(UINT64_C(0x7ff0000000000000)), "inf", "infinity");
    checkFormat(floatFromBits(UINT64_C(0xfff0000000000000)), "-inf", "negative infinity");
    checkFormat(floatFromBits(UINT64_C(0x7ff8000000000001)), "NaN", "nan payload");
    checkFormat(floatFromBits(UINT64_C(0xfff8000000000001)), "NaN", "negative nan");
    checkFormat(1.2345678901234567, "1.2345678901234567", "precision-loss regression");
    checkFormat(1e-3, "0.001", "small decimal power");
    checkFormat(0.3333333333333333, "0.3333333333333333", "repeating fraction");
    checkFormat(floatFromBits(UINT64_C(0x3fefffffffffffff)), "0.9999999999999999", "below one adjacent value");
    checkFormat(floatFromBits(UINT64_C(0x3ff0000000000001)), "1.0000000000000002", "above one adjacent value");
    checkFormat(floatFromBits(UINT64_C(0x431c4d2851425a09)), "1991533726307970.3", "positive decimal midpoint follows Rust tie rule");
    checkFormat(floatFromBits(UINT64_C(0x43113c144eb9d89d)), "1212783130474023.3", "positive midpoint with even upper candidate");
    checkFormat(floatFromBits(UINT64_C(0xc2b3dcbbbec2ca50)), "-21838763573962.313", "negative decimal midpoint rounds away from zero");

    double typed = 1.2345678901234567;
    int64_t payload;
    memcpy(&payload, &typed, sizeof(payload));
    CHECK(strcmp(rey_typed_val_to_str(REY_VALUE_FLOAT, payload), "1.2345678901234567") == 0,
          "typed float uses rey_float_to_str");
    double nanValue = floatFromBits(UINT64_C(0x7ff8000000000001));
    memcpy(&payload, &nanValue, sizeof(payload));
    CHECK(strcmp(rey_typed_val_to_str(REY_VALUE_FLOAT, payload), "NaN") == 0,
          "typed NaN uses the same spelling");
    checkRoundTrip(-0.0, "negative zero round trip");
}

static void testBoundaryRoundTrips(void) {
    const uint64_t cases[] = {
        UINT64_C(0x0000000000000001), // minimum subnormal
        UINT64_C(0x0000000000000002),
        UINT64_C(0x000fffffffffffff), // maximum subnormal
        UINT64_C(0x0010000000000000), // minimum normal
        UINT64_C(0x0010000000000001),
        UINT64_C(0x7feffffffffffffe),
        UINT64_C(0x7fefffffffffffff), // maximum finite
        UINT64_C(0x8000000000000001), // negative minimum subnormal
        UINT64_C(0x3ff0000000000000),
        UINT64_C(0x3ff0000000000001),
        UINT64_C(0x3fefffffffffffff),
        UINT64_C(0x4340000000000000), // 2^53
        UINT64_C(0x4340000000000001),
        UINT64_C(0x4415af1d78b58c40), // 1e20
    };
    for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); i++)
        checkRoundTrip(floatFromBits(cases[i]), "binary64 boundary");

    CHECK(strcmp(rey_float_to_str(floatFromBits(UINT64_C(1))), "5e-324") == 0, "minimum subnormal debug spelling");
    CHECK(strcmp(rey_float_to_str(DBL_MAX), "1.7976931348623157e308") == 0, "largest finite debug spelling");
}

static void testLocaleStability(void) {
    const char* localeNames[] = { "fr_FR.UTF-8", "fr_FR", "de_DE.UTF-8", "de_DE" };
    int changedLocale = 0;
    for (size_t i = 0; i < sizeof(localeNames) / sizeof(localeNames[0]); i++) {
        if (setlocale(LC_NUMERIC, localeNames[i])) {
            changedLocale = 1;
            break;
        }
    }
    if (!changedLocale || strcmp(localeconv()->decimal_point, ".") == 0) {
        CHECK(setlocale(LC_NUMERIC, "C") != NULL, "restore C locale");
        puts("SKIP: no installed comma-decimal locale");
        return;
    }

    checkFormat(1.25, "1.25", "locale-independent decimal point");
    CHECK(setlocale(LC_NUMERIC, "C") != NULL, "restore C locale");
}

static uint64_t nextRandom(uint64_t* state) {
    uint64_t value = *state;
    value ^= value >> 12;
    value ^= value << 25;
    value ^= value >> 27;
    *state = value;
    return value * UINT64_C(2685821657736338717);
}

static void testRandomFiniteRoundTrips(void) {
    uint64_t state = UINT64_C(0x6a09e667f3bcc909);
    size_t checked = 0;
    while (checked < 50000) {
        uint64_t bits = nextRandom(&state);
        if (((bits >> 52) & 0x7ff) == 0x7ff) continue;
        checkRoundTrip(floatFromBits(bits), "random finite binary64");
        checked++;
    }
}

static void testStringAbiUnchanged(void) {
    const unsigned char bytes[] = { 'a', 0, 'b', 0xff };
    char* string = rey_str_new((const char*)bytes, (int64_t)sizeof(bytes));
    const char* typed = rey_typed_val_to_str(REY_VALUE_STRING, (int64_t)(uintptr_t)string);
    CHECK(typed == string, "typed string pointer is returned unchanged");
    CHECK(rey_str_len(typed) == (int64_t)sizeof(bytes), "typed string byte length is unchanged");
    CHECK(memcmp(typed, bytes, sizeof(bytes)) == 0, "typed string preserves embedded and high bytes");
    CHECK(rey_val_to_str((int64_t)(uintptr_t)string) == string, "legacy string-pointer lookup is unchanged");
}

int main(void) {
    CHECK(sizeof(double) == sizeof(uint64_t), "runtime requires binary64 double");
    CHECK(setlocale(LC_NUMERIC, "C") != NULL, "initialize C locale");
    testReferenceSpellings();
    testBoundaryRoundTrips();
    testLocaleStability();
    testRandomFiniteRoundTrips();
    testStringAbiUnchanged();
    puts("PASS: Rust Debug float spelling, locale stability, and 50000 finite round trips");
    return 0;
}
