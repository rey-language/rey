// Rey string runtime.
// Strings are represented as i8* pointing to ReyStr.data (16 bytes past header).
// Layout: { uint8_t tag=1; uint8_t _pad[7]; int64_t len; char data[len+1]; }
#include "rey_rt.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

typedef struct {
    uint8_t  tag;
    uint8_t  _pad[7];
    int64_t  len;
    char     data[];
} ReyStr;

// data pointer → header pointer
static ReyStr* data2hdr(const char* data) {
    return (ReyStr*)(data - REY_STR_HDR);
}

char* rey_str_new(const char* data, int64_t len) {
    ReyStr* s = (ReyStr*)rey_alloc((int64_t)sizeof(ReyStr) + len + 1);
    s->tag = REY_TAG_STR;
    s->len = len;
    if (data && len > 0) memcpy(s->data, data, (size_t)len);
    s->data[len] = '\0';
    return s->data;
}

char* rey_str_cstr(const char* cstr) {
    int64_t len = cstr ? (int64_t)strlen(cstr) : 0;
    return rey_str_new(cstr, len);
}

char* rey_str_empty(void) {
    return rey_str_new(NULL, 0);
}

int64_t rey_str_len(const char* s) {
    if (!s) return 0;
    return data2hdr(s)->len;
}

char* rey_str_concat(const char* a, const char* b) {
    int64_t la = a ? data2hdr(a)->len : 0;
    int64_t lb = b ? data2hdr(b)->len : 0;
    ReyStr* out = (ReyStr*)rey_alloc((int64_t)sizeof(ReyStr) + la + lb + 1);
    out->tag = REY_TAG_STR;
    out->len = la + lb;
    if (la) memcpy(out->data, a, (size_t)la);
    if (lb) memcpy(out->data + la, b, (size_t)lb);
    out->data[la + lb] = '\0';
    return out->data;
}

int64_t rey_str_eq(const char* a, const char* b) {
    if (a == b) return 1;
    if (!a || !b) return 0;
    int64_t la = data2hdr(a)->len;
    int64_t lb = data2hdr(b)->len;
    if (la != lb) return 0;
    return memcmp(a, b, (size_t)la) == 0 ? 1 : 0;
}

char* rey_str_slice(const char* s, int64_t start, int64_t end) {
    if (!s) return rey_str_empty();
    int64_t len = data2hdr(s)->len;
    if (start < 0) start = 0;
    if (end > len) end = len;
    if (start >= end) return rey_str_empty();
    return rey_str_new(s + start, end - start);
}

char* rey_str_char_at(const char* s, int64_t i) {
    if (!s) return rey_str_empty();
    int64_t len = data2hdr(s)->len;
    if (i < 0 || i >= len) return rey_str_empty();
    return rey_str_new(s + i, 1);
}

int64_t rey_str_index_of(const char* s, const char* sub) {
    if (!s || !sub) return -1;
    int64_t ls = data2hdr(s)->len;
    int64_t lsub = data2hdr(sub)->len;
    if (lsub == 0) return 0;
    if (lsub > ls) return -1;
    for (int64_t i = 0; i <= ls - lsub; i++) {
        if (memcmp(s + i, sub, (size_t)lsub) == 0) return i;
    }
    return -1;
}

int64_t rey_str_starts_with(const char* s, const char* prefix) {
    if (!s || !prefix) return 0;
    int64_t ls = data2hdr(s)->len;
    int64_t lp = data2hdr(prefix)->len;
    if (lp > ls) return 0;
    return memcmp(s, prefix, (size_t)lp) == 0 ? 1 : 0;
}

int64_t rey_str_ends_with(const char* s, const char* suffix) {
    if (!s || !suffix) return 0;
    int64_t ls = data2hdr(s)->len;
    int64_t lx = data2hdr(suffix)->len;
    if (lx > ls) return 0;
    return memcmp(s + ls - lx, suffix, (size_t)lx) == 0 ? 1 : 0;
}

char* rey_str_trim(const char* s) {
    if (!s) return rey_str_empty();
    int64_t len = data2hdr(s)->len;
    int64_t lo = 0, hi = len;
    while (lo < hi && isspace((unsigned char)s[lo])) lo++;
    while (hi > lo && isspace((unsigned char)s[hi-1])) hi--;
    return rey_str_new(s + lo, hi - lo);
}

char* rey_str_replace(const char* s, const char* from, const char* to) {
    if (!s) return rey_str_empty();
    if (!from || data2hdr(from)->len == 0) return rey_str_cstr(s);
    int64_t ls = data2hdr(s)->len;
    int64_t lf = data2hdr(from)->len;
    char* result = rey_str_empty();
    int64_t pos = 0;
    while (pos <= ls - lf) {
        if (memcmp(s + pos, from, (size_t)lf) == 0) {
            result = rey_str_concat(result, to);
            pos += lf;
        } else {
            char ch[2] = { s[pos], '\0' };
            char* cs = rey_str_new(ch, 1);
            result = rey_str_concat(result, cs);
            pos++;
        }
    }
    if (pos < ls) {
        char* tail = rey_str_new(s + pos, ls - pos);
        result = rey_str_concat(result, tail);
    }
    return result;
}

char* rey_str_repeat(const char* s, int64_t n) {
    if (!s || n <= 0) return rey_str_empty();
    int64_t ls = data2hdr(s)->len;
    int64_t total = ls * n;
    ReyStr* out = (ReyStr*)rey_alloc((int64_t)sizeof(ReyStr) + total + 1);
    out->tag = REY_TAG_STR;
    out->len = total;
    for (int64_t i = 0; i < n; i++) memcpy(out->data + i * ls, s, (size_t)ls);
    out->data[total] = '\0';
    return out->data;
}

char* rey_str_to_upper(const char* s) {
    if (!s) return rey_str_empty();
    int64_t len = data2hdr(s)->len;
    ReyStr* out = (ReyStr*)rey_alloc((int64_t)sizeof(ReyStr) + len + 1);
    out->tag = REY_TAG_STR;
    out->len = len;
    for (int64_t i = 0; i < len; i++) out->data[i] = (char)toupper((unsigned char)s[i]);
    out->data[len] = '\0';
    return out->data;
}

char* rey_str_to_lower(const char* s) {
    if (!s) return rey_str_empty();
    int64_t len = data2hdr(s)->len;
    ReyStr* out = (ReyStr*)rey_alloc((int64_t)sizeof(ReyStr) + len + 1);
    out->tag = REY_TAG_STR;
    out->len = len;
    for (int64_t i = 0; i < len; i++) out->data[i] = (char)tolower((unsigned char)s[i]);
    out->data[len] = '\0';
    return out->data;
}

char* rey_int_to_str(int64_t n) {
    char buf[32];
    int len = snprintf(buf, sizeof(buf), "%lld", (long long)n);
    return rey_str_new(buf, len);
}

char* rey_float_to_str(double f) {
    char buf[64];
    int len = snprintf(buf, sizeof(buf), "%g", f);
    return rey_str_new(buf, len);
}

char* rey_bool_to_str(int64_t b) {
    return b ? rey_str_cstr("true") : rey_str_cstr("false");
}

// split string; returns Vec<String> where items are i8* string data pointers
int64_t rey_str_split(const char* s, const char* delim) {
    extern int64_t rey_vec_new(void);
    extern void    rey_vec_push(int64_t, int64_t);

    int64_t vec = rey_vec_new();
    if (!s) return vec;
    int64_t ls = data2hdr(s)->len;
    int64_t ld = delim ? data2hdr(delim)->len : 0;

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
    int64_t len = data2hdr(s)->len;
    fwrite(s, 1, (size_t)len, stdout);
}

void rey_println(const char* s) {
    rey_print(s);
    putchar('\n');
    fflush(stdout);
}

// smart toString: inspects tag byte to handle opaque i64 values from Vec/HashMap
// val is an i8* data pointer (ReyStr ABI: header is 16 bytes before data)
// integers are < 65536; heap pointers are always much larger
char* rey_val_to_str(int64_t val) {
    if (val > 65536) {
        // val = data ptr = header + 16; tag byte is at header = val - 16
        uint8_t tag = *(uint8_t*)((uintptr_t)val - REY_STR_HDR);
        if (tag == REY_TAG_STR) {
            return (char*)(uintptr_t)val;
        }
    }
    return rey_int_to_str(val);
}

void rey_panic(const char* msg) {
    fprintf(stderr, "rey panic: ");
    if (msg) {
        int64_t len = data2hdr(msg)->len;
        fwrite(msg, 1, (size_t)len, stderr);
    } else {
        fprintf(stderr, "(null)");
    }
    fprintf(stderr, "\n");
    exit(1);
}
