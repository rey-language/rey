// Rey string runtime.
// Strings are represented as i8* pointing to ReyStr.data (16 bytes past header).
// Layout: { uint8_t tag=1; uint8_t _pad[7]; int64_t len; char data[len+1]; }
#include "rey_rt.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include <errno.h>
#include <locale.h>
#include <math.h>

typedef struct {
    uint8_t  tag;
    uint8_t  _pad[7];
    int64_t  len;
    char     data[];
} ReyStr;

// Exact data-pointer membership avoids dereferencing arbitrary i64 values.
static uintptr_t* stringPointers;
static size_t stringPointerCount;
static size_t stringPointerCapacity;

static size_t stringPointerHash(uintptr_t ptr) {
    uint64_t x = (uint64_t)ptr;
    x ^= x >> 33;
    x *= UINT64_C(0xff51afd7ed558ccd);
    x ^= x >> 33;
    return (size_t)x;
}

static void stringPointerInsert(uintptr_t* slots, size_t capacity, uintptr_t ptr) {
    size_t i = stringPointerHash(ptr) & (capacity - 1);
    while (slots[i]) i = (i + 1) & (capacity - 1);
    slots[i] = ptr;
}

static void growStringPointers(void) {
    size_t capacity = stringPointerCapacity ? stringPointerCapacity * 2 : 256;
    if (capacity < stringPointerCapacity || capacity > SIZE_MAX / sizeof(uintptr_t)) {
        fprintf(stderr, "rey: out of memory\n");
        exit(1);
    }
    uintptr_t* slots = (uintptr_t*)calloc(capacity, sizeof(uintptr_t));
    if (!slots) {
        fprintf(stderr, "rey: out of memory\n");
        exit(1);
    }
    for (size_t i = 0; i < stringPointerCapacity; i++) {
        if (stringPointers[i]) stringPointerInsert(slots, capacity, stringPointers[i]);
    }
    free(stringPointers);
    stringPointers = slots;
    stringPointerCapacity = capacity;
}

static void registerStringPointer(const char* data) {
    uintptr_t ptr = (uintptr_t)data;
    if (!ptr) return;
    if (!stringPointerCapacity || stringPointerCount >= stringPointerCapacity - stringPointerCapacity / 4)
        growStringPointers();
    stringPointerInsert(stringPointers, stringPointerCapacity, ptr);
    stringPointerCount++;
}

static int isRegisteredStringPointer(uintptr_t ptr) {
    if (!ptr || !stringPointerCapacity) return 0;
    size_t i = stringPointerHash(ptr) & (stringPointerCapacity - 1);
    while (stringPointers[i]) {
        if (stringPointers[i] == ptr) return 1;
        i = (i + 1) & (stringPointerCapacity - 1);
    }
    return 0;
}

// data pointer → header pointer
static ReyStr* data2hdr(const char* data) {
    return (ReyStr*)(data - REY_STR_HDR);
}

static int64_t stringLen(const char* data) {
    if (!data) return 0;
    int64_t len = data2hdr(data)->len;
    if (len < 0) rey_panic_cstr("string length overflow");
    return len;
}

static int64_t stringBytes(int64_t len) {
    if (len < 0 || len > INT64_MAX - (int64_t)sizeof(ReyStr) - 1)
        rey_panic_cstr("string length overflow");
    return (int64_t)sizeof(ReyStr) + len + 1;
}

char* rey_str_new(const char* data, int64_t len) {
    ReyStr* s = (ReyStr*)rey_alloc(stringBytes(len));
    s->tag = REY_TAG_STR;
    s->len = len;
    if (data && len > 0) memcpy(s->data, data, (size_t)len);
    s->data[len] = '\0';
    registerStringPointer(s->data);
    return s->data;
}

char* rey_str_cstr(const char* cstr) {
    if (!cstr) return rey_str_new(NULL, 0);
    size_t rawLen = strlen(cstr);
    if (rawLen > (size_t)INT64_MAX) rey_panic_cstr("string length overflow");
    return rey_str_new(cstr, (int64_t)rawLen);
}

char* rey_str_empty(void) {
    return rey_str_new(NULL, 0);
}

int64_t rey_str_len(const char* s) {
    return stringLen(s);
}

char* rey_str_concat(const char* a, const char* b) {
    int64_t la = stringLen(a);
    int64_t lb = stringLen(b);
    if (la > INT64_MAX - lb) rey_panic_cstr("string length overflow");
    int64_t total = la + lb;
    ReyStr* out = (ReyStr*)rey_alloc(stringBytes(total));
    out->tag = REY_TAG_STR;
    out->len = total;
    if (la) memcpy(out->data, a, (size_t)la);
    if (lb) memcpy(out->data + la, b, (size_t)lb);
    out->data[total] = '\0';
    registerStringPointer(out->data);
    return out->data;
}

int64_t rey_str_eq(const char* a, const char* b) {
    if (a == b) return 1;
    if (!a || !b) return 0;
    int64_t la = stringLen(a);
    int64_t lb = stringLen(b);
    if (la != lb) return 0;
    return memcmp(a, b, (size_t)la) == 0 ? 1 : 0;
}

char* rey_str_slice(const char* s, int64_t start, int64_t end) {
    if (!s) return rey_str_empty();
    if (start < 0 || end < 0) rey_runtime_error("String.slice() indices must be non-negative");
    int64_t len = stringLen(s);
    if (start > len) start = len;
    if (end > len) end = len;
    if (start >= end) return rey_str_empty();
    return rey_str_new(s + start, end - start);
}

char* rey_str_char_at(const char* s, int64_t i) {
    if (!s) return rey_str_empty();
    int64_t len = stringLen(s);
    if (i < 0 || i >= len) return rey_str_empty();
    return rey_str_new(s + i, 1);
}

int64_t rey_str_index_of(const char* s, const char* sub) {
    if (!s || !sub) return -1;
    int64_t ls = stringLen(s);
    int64_t lsub = stringLen(sub);
    if (lsub == 0) return 0;
    if (lsub > ls) return -1;
    for (int64_t i = 0; i <= ls - lsub; i++) {
        if (memcmp(s + i, sub, (size_t)lsub) == 0) return i;
    }
    return -1;
}

int64_t rey_str_contains(const char* s, const char* needle) {
    return rey_str_index_of(s, needle) >= 0 ? 1 : 0;
}

int64_t rey_str_starts_with(const char* s, const char* prefix) {
    if (!s || !prefix) return 0;
    int64_t ls = stringLen(s);
    int64_t lp = stringLen(prefix);
    if (lp > ls) return 0;
    return memcmp(s, prefix, (size_t)lp) == 0 ? 1 : 0;
}

int64_t rey_str_ends_with(const char* s, const char* suffix) {
    if (!s || !suffix) return 0;
    int64_t ls = stringLen(s);
    int64_t lx = stringLen(suffix);
    if (lx > ls) return 0;
    return memcmp(s + ls - lx, suffix, (size_t)lx) == 0 ? 1 : 0;
}

static int asciiSpace(unsigned char byte) {
    return byte == ' ' || byte == '\t' || byte == '\n' || byte == '\v' || byte == '\f' || byte == '\r';
}

char* rey_str_trim(const char* s) {
    if (!s) return rey_str_empty();
    int64_t len = stringLen(s);
    int64_t lo = 0, hi = len;
    while (lo < hi && asciiSpace((unsigned char)s[lo])) lo++;
    while (hi > lo && asciiSpace((unsigned char)s[hi - 1])) hi--;
    return rey_str_new(s + lo, hi - lo);
}

static int64_t addLength(int64_t left, int64_t right) {
    if (left < 0 || right < 0 || left > INT64_MAX - right) rey_panic_cstr("string length overflow");
    return left + right;
}

static int64_t mulLength(int64_t count, int64_t size) {
    if (count < 0 || size < 0) rey_panic_cstr("string length overflow");
    if (count != 0 && size > INT64_MAX / count) rey_panic_cstr("string length overflow");
    return count * size;
}

static char* fillReplace(const char* bytes, int64_t total) {
    return rey_str_new(bytes, total);
}

char* rey_str_replace(const char* s, const char* from, const char* to) {
    if (!s) return rey_str_empty();
    int64_t ls = stringLen(s);
    int64_t lf = from ? stringLen(from) : 0;
    int64_t lt = to ? stringLen(to) : 0;
    if (lf == 0) {
        int64_t total = addLength(mulLength(lt, addLength(ls, 1)), ls);
        char* out = rey_str_new(NULL, total);
        int64_t cursor = 0;
        if (lt) {
            memcpy(out + cursor, to, (size_t)lt);
            cursor += lt;
        }
        for (int64_t i = 0; i < ls; i++) {
            out[cursor++] = s[i];
            if (lt) {
                memcpy(out + cursor, to, (size_t)lt);
                cursor += lt;
            }
        }
        return out;
    }
    int64_t matches = 0;
    for (int64_t pos = 0; pos <= ls - lf; ) {
        if (memcmp(s + pos, from, (size_t)lf) == 0) {
            matches++;
            pos += lf;
        } else {
            pos++;
        }
    }
    int64_t total = addLength(ls - mulLength(matches, lf), mulLength(matches, lt));
    char* raw = (char*)malloc((size_t)total + 1);
    if (!raw) rey_panic_cstr("string length overflow");
    int64_t cursor = 0;
    int64_t pos = 0;
    while (pos <= ls - lf) {
        if (memcmp(s + pos, from, (size_t)lf) == 0) {
            if (lt) {
                memcpy(raw + cursor, to, (size_t)lt);
                cursor += lt;
            }
            pos += lf;
        } else {
            raw[cursor++] = s[pos++];
        }
    }
    if (pos < ls) {
        memcpy(raw + cursor, s + pos, (size_t)(ls - pos));
        cursor += ls - pos;
    }
    raw[cursor] = '\0';
    char* out = fillReplace(raw, total);
    free(raw);
    return out;
}

char* rey_str_repeat(const char* s, int64_t n) {
    if (n < 0) rey_runtime_error("String.repeat() count must be non-negative");
    if (!s || n == 0) return rey_str_empty();
    int64_t ls = stringLen(s);
    if (ls == 0) return rey_str_empty();
    if (n > INT64_MAX / ls) rey_panic_cstr("string length overflow");
    int64_t total = ls * n;
    ReyStr* out = (ReyStr*)rey_alloc(stringBytes(total));
    out->tag = REY_TAG_STR;
    out->len = total;
    for (int64_t i = 0; i < n; i++) memcpy(out->data + i * ls, s, (size_t)ls);
    out->data[total] = '\0';
    registerStringPointer(out->data);
    return out->data;
}

char* rey_str_to_upper(const char* s) {
    if (!s) return rey_str_empty();
    int64_t len = stringLen(s);
    ReyStr* out = (ReyStr*)rey_alloc(stringBytes(len));
    out->tag = REY_TAG_STR;
    out->len = len;
    for (int64_t i = 0; i < len; i++) {
        unsigned char c = (unsigned char)s[i];
        out->data[i] = (char)(c >= 'a' && c <= 'z' ? c - ('a' - 'A') : c);
    }
    out->data[len] = '\0';
    registerStringPointer(out->data);
    return out->data;
}

char* rey_str_to_lower(const char* s) {
    if (!s) return rey_str_empty();
    int64_t len = stringLen(s);
    ReyStr* out = (ReyStr*)rey_alloc(stringBytes(len));
    out->tag = REY_TAG_STR;
    out->len = len;
    for (int64_t i = 0; i < len; i++) {
        unsigned char c = (unsigned char)s[i];
        out->data[i] = (char)(c >= 'A' && c <= 'Z' ? c + ('a' - 'A') : c);
    }
    out->data[len] = '\0';
    registerStringPointer(out->data);
    return out->data;
}

char* rey_int_to_str(int64_t n) {
    char buf[32];
    int len = snprintf(buf, sizeof(buf), "%lld", (long long)n);
    if (len < 0 || (size_t)len >= sizeof(buf)) rey_panic_cstr("int to string failed");
    return rey_str_new(buf, len);
}

static int asciiDigit(unsigned char byte) {
    return byte >= '0' && byte <= '9';
}

static double parseAsciiDecimal(const char* text) {
    int64_t length = stringLen(text);
    if (length <= 0) rey_runtime_error("Invalid ASCII decimal text");
    const unsigned char* bytes = (const unsigned char*)text;
    char* normalized = (char*)malloc((size_t)length + 1);
    if (!normalized) rey_runtime_error("Invalid ASCII decimal text");
    int64_t written = 0;
    int64_t index = 0;
    int64_t digitCount = 0;
    if (bytes[0] == '+' || bytes[0] == '-') {
        normalized[written++] = (char)bytes[0];
        index = 1;
    }
    while (index < length) {
        unsigned char byte = bytes[index];
        if (asciiDigit(byte)) {
            normalized[written++] = (char)byte;
            digitCount++;
            index++;
        } else if (byte == '_' && index > 0 && asciiDigit(bytes[index - 1]) &&
                   index + 1 < length && asciiDigit(bytes[index + 1])) {
            index++;
        } else {
            break;
        }
    }
    if (digitCount == 0) {
        free(normalized);
        rey_runtime_error("Invalid ASCII decimal text");
    }
    if (index < length && bytes[index] == '.') {
        normalized[written++] = '.';
        index++;
        while (index < length) {
            unsigned char byte = bytes[index];
            if (asciiDigit(byte)) {
                normalized[written++] = (char)byte;
                index++;
            } else if (byte == '_' && index > 0 && asciiDigit(bytes[index - 1]) &&
                       index + 1 < length && asciiDigit(bytes[index + 1])) {
                index++;
            } else {
                break;
            }
        }
    }
    if (index != length) {
        free(normalized);
        rey_runtime_error("Invalid ASCII decimal text");
    }
    normalized[written] = '\0';
    char savedLocale[128];
    const char* currentLocale = setlocale(LC_NUMERIC, NULL);
    if (!currentLocale || strlen(currentLocale) >= sizeof(savedLocale)) {
        free(normalized);
        rey_runtime_error("Invalid ASCII decimal text");
    }
    memcpy(savedLocale, currentLocale, strlen(currentLocale) + 1);
    if (!setlocale(LC_NUMERIC, "C")) {
        free(normalized);
        rey_runtime_error("Invalid ASCII decimal text");
    }
    errno = 0;
    char* end = NULL;
    double value = strtod(normalized, &end);
    int rejected = end != normalized + written || errno == ERANGE || !isfinite(value);
    setlocale(LC_NUMERIC, savedLocale);
    free(normalized);
    if (rejected) rey_runtime_error("Decimal value out of float range");
    return value;
}

int64_t rey_str_to_int(const char* text) {
    return rey_float_to_int(parseAsciiDecimal(text));
}

double rey_str_to_float(const char* text) {
    return parseAsciiDecimal(text);
}

const char* rey_char_from_int(int64_t value) {
    if (value < 0 || value > 255) {
        char message[80];
        snprintf(message, sizeof(message), "char value out of range 0..=255: %lld", (long long)value);
        rey_runtime_error(message);
    }
    unsigned char byte = (unsigned char)value;
    return rey_str_new((const char*)&byte, 1);
}

int64_t rey_int_from_char(const char* value) {
    if (!value) rey_runtime_error("char value is null");
    return (unsigned char)*value;
}

static int parseFloatParts(const char* number, const char* decimalPoint, char* digits, size_t capacity,
                           size_t* digitCount, int* decimalExponent, int* negative) {
    size_t count = 0;
    int decimalPosition = 0;
    int sawDecimalPoint = 0;
    int exponent = 0;
    const char* cursor = number;
    const char* exponentMark;
    size_t decimalPointLength = strlen(decimalPoint);

    *negative = 0;
    if (*cursor == '-' || *cursor == '+') {
        *negative = *cursor == '-';
        cursor++;
    }
    exponentMark = strchr(cursor, 'e');
    if (!exponentMark) exponentMark = strchr(cursor, 'E');
    const char* end = exponentMark ? exponentMark : cursor + strlen(cursor);

    while (cursor < end) {
        if (decimalPointLength && strncmp(cursor, decimalPoint, decimalPointLength) == 0) {
            if (sawDecimalPoint) return -1;
            decimalPosition = (int)count;
            sawDecimalPoint = 1;
            cursor += decimalPointLength;
        } else if (*cursor >= '0' && *cursor <= '9') {
            if (count == capacity) return -1;
            digits[count++] = *cursor++;
        } else {
            return -1;
        }
    }
    if (!sawDecimalPoint) decimalPosition = (int)count;
    if (count == 0) return -1;

    if (exponentMark) {
        const char* p = exponentMark + 1;
        int exponentSign = 1;
        if (*p == '-' || *p == '+') {
            if (*p == '-') exponentSign = -1;
            p++;
        }
        if (!*p) return -1;
        while (*p) {
            if (*p < '0' || *p > '9' || exponent > 10000) return -1;
            exponent = exponent * 10 + (*p++ - '0');
        }
        exponent *= exponentSign;
    }

    size_t firstSignificant = 0;
    while (firstSignificant < count && digits[firstSignificant] == '0') firstSignificant++;
    if (firstSignificant == count) return -1;
    if (firstSignificant) {
        memmove(digits, digits + firstSignificant, count - firstSignificant);
        count -= firstSignificant;
    }
    *digitCount = count;
    *decimalExponent = decimalPosition + exponent - (int)firstSignificant - 1;
    return 0;
}

static int writeFixedFloat(const char* digits, size_t digitCount, int decimalExponent, int negative,
                           char* output, size_t capacity) {
    int decimalPosition = decimalExponent + 1;
    size_t used = 0;
    if (negative) {
        if (used + 1 >= capacity) return -1;
        output[used++] = '-';
    }
    if (decimalPosition <= 0) {
        if (used + 2 + (size_t)(-decimalPosition) + digitCount >= capacity) return -1;
        output[used++] = '0';
        output[used++] = '.';
        for (int i = 0; i < -decimalPosition; i++) output[used++] = '0';
        memcpy(output + used, digits, digitCount);
        used += digitCount;
    } else if ((size_t)decimalPosition >= digitCount) {
        size_t zeroCount = (size_t)decimalPosition - digitCount;
        if (used + digitCount + zeroCount >= capacity) return -1;
        memcpy(output + used, digits, digitCount);
        used += digitCount;
        memset(output + used, '0', zeroCount);
        used += zeroCount;
    } else {
        if (used + digitCount + 1 >= capacity) return -1;
        memcpy(output + used, digits, (size_t)decimalPosition);
        used += (size_t)decimalPosition;
        output[used++] = '.';
        memcpy(output + used, digits + decimalPosition, digitCount - (size_t)decimalPosition);
        used += digitCount - (size_t)decimalPosition;
    }
    output[used] = '\0';
    return (int)used;
}

static void trimTrailingZeros(char* digits, size_t* digitCount) {
    while (*digitCount > 1 && digits[*digitCount - 1] == '0') (*digitCount)--;
}

static int writeScientificFloat(const char* digits, size_t digitCount, int decimalExponent, int negative,
                                char* output, size_t capacity) {
    if (digitCount == 0) return -1;
    int length = snprintf(output, capacity, "%s%.*s%s%.*se%d",
                          negative ? "-" : "",
                          1, digits,
                          digitCount > 1 ? "." : "",
                          digitCount > 1 ? (int)(digitCount - 1) : 0,
                          digits + 1,
                          decimalExponent);
    if (length < 0 || (size_t)length >= capacity) return -1;
    return length;
}

static int formatRustFloat(const char* digits, size_t digitCount, int decimalExponent, int negative,
                           char* output, size_t capacity) {
    int length;
    if (decimalExponent < -4 || decimalExponent >= 16)
        length = writeScientificFloat(digits, digitCount, decimalExponent, negative, output, capacity);
    else
        length = writeFixedFloat(digits, digitCount, decimalExponent, negative, output, capacity);
    if (length < 3) return length;
    if (output[length - 2] != '.' || output[length - 1] != '0' || strchr(output, 'e') != NULL) return length;
    int start = output[0] == '-' ? 1 : 0;
    if (length - 2 <= start) return length;
    for (int i = start; i < length - 2; i++)
        if (output[i] < '0' || output[i] > '9') return length;
    length -= 2;
    output[length] = '\0';
    return length;
}

// libc %g breaks exact decimal ties toward even. Rust Debug breaks them away from zero.
static int repairTieAwayFromZero(double value, int precision, const char* candidate,
                                 char* digits, size_t capacity, size_t* digitCount,
                                 int* decimalExponent, int* negative) {
    char exact[1200];
    char exactDigits[1100];
    char candidateDigits[32];
    size_t exactDigitCount;
    size_t candidateDigitCount;
    int exactExponent;
    int candidateExponent;
    int exactNegative;
    int candidateNegative;
    int exactLength = snprintf(exact, sizeof(exact), "%.1074g", value);
    if (exactLength < 0 || (size_t)exactLength >= sizeof(exact)) return -1;
    if (parseFloatParts(exact, ".", exactDigits, sizeof(exactDigits),
                        &exactDigitCount, &exactExponent, &exactNegative) < 0 ||
        exactDigitCount <= (size_t)precision || exactDigits[precision] != '5') return 0;
    for (size_t i = (size_t)precision + 1; i < exactDigitCount; i++)
        if (exactDigits[i] != '0') return 0;

    if (parseFloatParts(candidate, ".", candidateDigits, sizeof(candidateDigits),
                        &candidateDigitCount, &candidateExponent, &candidateNegative) < 0 ||
        candidateExponent != exactExponent || candidateNegative != exactNegative) return 0;
    uint64_t lower = 0;
    uint64_t rounded = 0;
    for (int i = 0; i < precision; i++) {
        lower = lower * 10 + (uint64_t)(exactDigits[i] - '0');
        rounded = rounded * 10 + (uint64_t)(i < (int)candidateDigitCount ? candidateDigits[i] - '0' : 0);
    }
    if (rounded != lower) return 0;

    char upperDigits[20];
    if ((size_t)precision > sizeof(upperDigits) || (size_t)precision > capacity) return -1;
    memcpy(upperDigits, exactDigits, (size_t)precision);
    int carry = 1;
    for (int i = precision - 1; i >= 0 && carry; i--) {
        if (upperDigits[i] == '9') upperDigits[i] = '0';
        else {
            upperDigits[i]++;
            carry = 0;
        }
    }
    if (carry) {
        upperDigits[0] = '1';
        for (int i = 1; i < precision; i++) upperDigits[i] = '0';
        exactExponent++;
    }
    size_t count = (size_t)precision;
    trimTrailingZeros(upperDigits, &count);
    memcpy(digits, upperDigits, count);
    *digitCount = count;
    *decimalExponent = exactExponent;
    *negative = exactNegative;
    return 1;
}

char* rey_float_to_str(double f) {
    uint64_t bits;
    memcpy(&bits, &f, sizeof(bits));
    unsigned int exponent = (unsigned int)((bits >> 52) & 0x7ff);
    if (exponent == 0x7ff) {
        if ((bits & UINT64_C(0x000fffffffffffff)) == 0)
            return rey_str_cstr((bits >> 63) ? "-inf" : "inf");
        return rey_str_cstr("NaN");
    }
    if ((bits & UINT64_C(0x7fffffffffffffff)) == 0)
        return rey_str_cstr((bits >> 63) ? "-0" : "0");

    char savedLocale[128];
    const char* currentLocale = setlocale(LC_NUMERIC, NULL);
    if (!currentLocale || strlen(currentLocale) >= sizeof(savedLocale)) {
        rey_panic_cstr("rey_float_to_str: failed to format finite float");
        return NULL;
    }
    memcpy(savedLocale, currentLocale, strlen(currentLocale) + 1);
    if (!setlocale(LC_NUMERIC, "C")) {
        rey_panic_cstr("rey_float_to_str: failed to format finite float");
        return NULL;
    }

    char digits[24];
    char candidate[64];
    size_t digitCount = 0;
    int decimalExponent = 0;
    int negative = 0;
    int formatted = 0;
    for (int precision = 1; precision <= 17 && !formatted; precision++) {
        int candidateLength = snprintf(candidate, sizeof(candidate), "%.*g", precision, f);
        if (candidateLength < 0 || (size_t)candidateLength >= sizeof(candidate)) break;
        char* end = NULL;
        double parsed = strtod(candidate, &end);
        uint64_t parsedBits;
        memcpy(&parsedBits, &parsed, sizeof(parsedBits));
        if (!end || *end != '\0' || parsedBits != bits) continue;
        if (parseFloatParts(candidate, ".", digits, sizeof(digits), &digitCount, &decimalExponent, &negative) < 0)
            break;
        int repaired = repairTieAwayFromZero(f, precision, candidate, digits, sizeof(digits),
                                              &digitCount, &decimalExponent, &negative);
        if (repaired < 0) break;
        if (!repaired) trimTrailingZeros(digits, &digitCount);
        formatted = 1;
    }
    setlocale(LC_NUMERIC, savedLocale);
    char buf[64];
    int length = formatted
        ? formatRustFloat(digits, digitCount, decimalExponent, negative, buf, sizeof(buf))
        : -1;
    if (length < 0) {
        rey_panic_cstr("rey_float_to_str: failed to format finite float");
        return NULL;
    }
    return rey_str_new(buf, length);
}

char* rey_bool_to_str(int64_t b) {
    return b ? rey_str_cstr("true") : rey_str_cstr("false");
}

const char* rey_typed_val_to_str(int64_t kind, int64_t payload) {
    switch (kind) {
        case REY_VALUE_INT:
            return rey_int_to_str(payload);
        case REY_VALUE_BOOL:
            if (payload != 0 && payload != 1) {
                rey_panic_cstr("rey_typed_val_to_str: bool payload must be 0 or 1");
                return NULL;
            }
            return rey_bool_to_str(payload);
        case REY_VALUE_FLOAT: {
            if (sizeof(double) != sizeof(payload)) {
                rey_panic_cstr("rey_typed_val_to_str: float payload requires 64-bit double");
                return NULL;
            }
            double value;
            memcpy(&value, &payload, sizeof(value));
            return rey_float_to_str(value);
        }
        case REY_VALUE_STRING:
            if (payload == 0) {
                rey_panic_cstr("rey_typed_val_to_str: string payload must be non-null");
                return NULL;
            }
            return (const char*)(uintptr_t)payload;
        case REY_VALUE_CHAR: {
            if (payload < 0 || payload > 255) {
                rey_panic_cstr("rey_typed_val_to_str: char payload must be in range 0..255");
                return NULL;
            }
            unsigned char byte = (unsigned char)payload;
            return rey_str_new((const char*)&byte, 1);
        }
        case REY_VALUE_NULL:
            if (payload != 0) {
                rey_panic_cstr("rey_typed_val_to_str: null payload must be 0");
                return NULL;
            }
            return rey_str_cstr("null");
        default: {
            char message[96];
            snprintf(message, sizeof(message), "rey_typed_val_to_str: unsupported kind %lld", (long long)kind);
            rey_panic_cstr(message);
            return NULL;
        }
    }
}

int64_t rey_str_last_index_of(const char* s, const char* sub) {
    if (!s || !sub) return -1;
    int64_t ls = stringLen(s);
    int64_t lsub = stringLen(sub);
    if (lsub == 0) return ls;
    if (lsub > ls) return -1;
    for (int64_t i = ls - lsub; i >= 0; i--) {
        if (memcmp(s + i, sub, (size_t)lsub) == 0) return i;
    }
    return -1;
}

char* rey_str_trim_start(const char* s) {
    if (!s) return rey_str_empty();
    int64_t len = stringLen(s);
    int64_t lo = 0;
    while (lo < len && asciiSpace((unsigned char)s[lo])) lo++;
    return rey_str_new(s + lo, len - lo);
}

char* rey_str_trim_end(const char* s) {
    if (!s) return rey_str_empty();
    int64_t len = stringLen(s);
    int64_t hi = len;
    while (hi > 0 && asciiSpace((unsigned char)s[hi - 1])) hi--;
    return rey_str_new(s, hi);
}

int64_t rey_str_hash(const char* s) {
    if (!s || stringLen(s) == 0) rey_runtime_error("hash() called on empty string");
    return (unsigned char)s[0];
}

static char* padString(const char* s, int64_t width, const char* pad, int onLeft,
                       const char* widthMessage, const char* padMessage) {
    if (width < 0) rey_runtime_error(widthMessage);
    int64_t padLen = pad ? stringLen(pad) : 0;
    if (padLen == 0) rey_runtime_error(padMessage);
    int64_t len = stringLen(s);
    if (len >= width) return rey_str_new(s, len);
    int64_t missing = width - len;
    char* out = rey_str_new(NULL, width);
    unsigned char byte = (unsigned char)pad[0];
    if (onLeft) {
        memset(out, byte, (size_t)missing);
        if (len) memcpy(out + missing, s, (size_t)len);
    } else {
        if (len) memcpy(out, s, (size_t)len);
        memset(out + len, byte, (size_t)missing);
    }
    return out;
}

char* rey_str_pad_left(const char* s, int64_t width, const char* pad) {
    return padString(s, width, pad, 1,
                     "String.padLeft() width must be non-negative",
                     "String.padLeft() expects non-empty string pad char");
}

char* rey_str_pad_right(const char* s, int64_t width, const char* pad) {
    return padString(s, width, pad, 0,
                     "String.padRight() width must be non-negative",
                     "String.padRight() expects non-empty string pad char");
}

// split string; returns Vec<String> where items are i8* string data pointers
int64_t rey_str_split(const char* s, const char* delim) {
    extern int64_t rey_vec_new(void);
    extern void    rey_vec_push(int64_t, int64_t);

    int64_t vec = rey_vec_new();
    if (!s) return vec;
    int64_t ls = stringLen(s);
    int64_t ld = delim ? stringLen(delim) : 0;

    if (ld == 0) {
        for (int64_t i = 0; i < ls; i++)
            rey_vec_push(vec, (int64_t)(uintptr_t)rey_str_new(s + i, 1));
        return vec;
    }

    int64_t pos = 0;
    while (pos <= ls) {
        int64_t found = -1;
        for (int64_t i = pos; i <= ls - ld; i++) {
            if (memcmp(s + i, delim, (size_t)ld) == 0) { found = i; break; }
        }
        if (found < 0) {
            rey_vec_push(vec, (int64_t)(uintptr_t)rey_str_new(s + pos, ls - pos));
            break;
        }
        rey_vec_push(vec, (int64_t)(uintptr_t)rey_str_new(s + pos, found - pos));
        pos = found + ld;
    }
    return vec;
}

void rey_print(const char* s) {
    if (!s) return;
    int64_t len = stringLen(s);
    fwrite(s, 1, (size_t)len, stdout);
}

void rey_println(const char* s) {
    rey_print(s);
    putchar('\n');
    fflush(stdout);
}

// Legacy untagged ABI; integer/pointer collisions remain ambiguous.
char* rey_val_to_str(int64_t val) {
    if (isRegisteredStringPointer((uintptr_t)val)) return (char*)(uintptr_t)val;
    return rey_int_to_str(val);
}

REY_NORETURN void rey_panic(const char* msg) {
    fprintf(stderr, "rey panic: ");
    if (msg) {
        int64_t len = stringLen(msg);
        fwrite(msg, 1, (size_t)len, stderr);
    } else {
        fprintf(stderr, "(null)");
    }
    fprintf(stderr, "\n");
    exit(1);
}
