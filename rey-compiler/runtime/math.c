// Numeric builtins. Domain and overflow failures print "error: ..." and exit 1.
#include "rey_rt.h"
#include <math.h>
#include <stddef.h>
#include <stdint.h>
#include <sys/time.h>

int64_t rey_abs_int(int64_t value) {
    if (value == INT64_MIN) rey_runtime_error("Integer overflow in abs");
    return value < 0 ? -value : value;
}

double rey_abs_float(double value) {
    return fabs(value);
}

int64_t rey_min_int(int64_t a, int64_t b) {
    return a < b ? a : b;
}

int64_t rey_max_int(int64_t a, int64_t b) {
    return a > b ? a : b;
}

double rey_min_float(double a, double b) {
    return a < b ? a : b;
}

double rey_max_float(double a, double b) {
    return a > b ? a : b;
}

static int64_t mulChecked(int64_t a, int64_t b) {
    int64_t out = 0;
    if (__builtin_mul_overflow(a, b, &out)) rey_runtime_error("Integer overflow in pow");
    return out;
}

int64_t rey_pow_int(int64_t base, int64_t exp) {
    if (exp < 0) rey_runtime_error("Integer overflow in pow");
    int64_t result = 1;
    int64_t factor = base;
    int64_t remaining = exp;
    while (remaining > 0) {
        if (remaining & 1) result = mulChecked(result, factor);
        remaining >>= 1;
        if (remaining > 0) factor = mulChecked(factor, factor);
    }
    return result;
}

double rey_pow_float(double base, double exp) {
    return pow(base, exp);
}

int64_t rey_floor(double value) {
    return rey_float_to_int(floor(value));
}

int64_t rey_ceil(double value) {
    return rey_float_to_int(ceil(value));
}

int64_t rey_round(double value) {
    return rey_float_to_int(round(value));
}


double rey_exp(double value) {
    return exp(value);
}

double rey_sqrt(double value) {
    if (value < 0.0) rey_runtime_error("sqrt expects non-negative input");
    return sqrt(value);
}

double rey_log(double value) {
    if (value <= 0.0) rey_runtime_error("log expects positive input");
    return log(value);
}

double rey_sin(double value) {
    return sin(value);
}

double rey_cos(double value) {
    return cos(value);
}

double rey_tan(double value) {
    return tan(value);
}

double rey_random(void) {
    struct timeval now;
    gettimeofday(&now, NULL);
    uint64_t micros = (uint64_t)now.tv_sec * 1000000ULL + (uint64_t)now.tv_usec;
    uint64_t mixed = micros * 1103515245ULL + 12345ULL;
    return (double)(mixed % 10000ULL) / 10000.0;
}

int64_t rey_bit_xor(int64_t a, int64_t b) {
    return a ^ b;
}

int64_t rey_bit_and(int64_t a, int64_t b) {
    return a & b;
}

int64_t rey_bit_or(int64_t a, int64_t b) {
    return a | b;
}

int64_t rey_bit_shl(int64_t a, int64_t b) {
    return a << b;
}

int64_t rey_bit_shr(int64_t a, int64_t b) {
    return (int64_t)(((uint64_t)a) >> b);
}

int64_t rey_bit_not(int64_t a) {
    return ~a;
}

int64_t rey_wrap_add(int64_t a, int64_t b) {
    return (int64_t)((uint64_t)a + (uint64_t)b);
}

int64_t rey_wrap_mul(int64_t a, int64_t b) {
    return (int64_t)((uint64_t)a * (uint64_t)b);
}

int64_t rey_wrap_sub(int64_t a, int64_t b) {
    return (int64_t)((uint64_t)a - (uint64_t)b);
}
