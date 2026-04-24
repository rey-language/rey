// Rey Vec runtime. All Vec values are i64 (pointer to ReyVec cast to int).
// Vec items are int64_t. String items are stored as int64_t (i8* cast to int64_t).
// Layout: { uint8_t tag=2; uint8_t _pad[7]; int64_t len; int64_t cap; int64_t* data; }
#include "rey_rt.h"
#include <stdlib.h>
#include <string.h>

typedef struct {
    uint8_t  tag;
    uint8_t  _pad[7];
    int64_t  len;
    int64_t  cap;
    int64_t* data;
} ReyVec;

static ReyVec* vp(int64_t v) { return (ReyVec*)(uintptr_t)v; }

int64_t rey_vec_new(void) {
    ReyVec* v = (ReyVec*)rey_alloc((int64_t)sizeof(ReyVec));
    v->tag = REY_TAG_VEC;
    return (int64_t)(uintptr_t)v;
}

static void vec_grow(ReyVec* v) {
    int64_t nc = v->cap == 0 ? 8 : v->cap * 2;
    int64_t* nd = (int64_t*)rey_alloc(nc * (int64_t)sizeof(int64_t));
    if (v->data) memcpy(nd, v->data, (size_t)(v->len * (int64_t)sizeof(int64_t)));
    free(v->data);
    v->data = nd;
    v->cap = nc;
}

void rey_vec_push(int64_t v, int64_t item) {
    ReyVec* rv = vp(v);
    if (rv->len >= rv->cap) vec_grow(rv);
    rv->data[rv->len++] = item;
}

int64_t rey_vec_pop(int64_t v) {
    ReyVec* rv = vp(v);
    if (rv->len == 0) rey_panic_cstr("Vec.pop() on empty vec");
    return rv->data[--rv->len];
}

int64_t rey_vec_len(int64_t v) { return vp(v)->len; }

int64_t rey_vec_get(int64_t v, int64_t i) {
    ReyVec* rv = vp(v);
    if (i < 0 || i >= rv->len) rey_panic_cstr("Vec index out of bounds");
    return rv->data[i];
}

void rey_vec_set(int64_t v, int64_t i, int64_t val) {
    ReyVec* rv = vp(v);
    if (i < 0 || i >= rv->len) rey_panic_cstr("Vec.set index out of bounds");
    rv->data[i] = val;
}

// For integer items: equality by value. Not used for string-typed Vecs.
int64_t rey_vec_contains(int64_t v, int64_t item) {
    ReyVec* rv = vp(v);
    for (int64_t i = 0; i < rv->len; i++)
        if (rv->data[i] == item) return 1;
    return 0;
}

// Vec<String>.contains(str): compare using rey_str_eq
int64_t rey_vec_str_contains(int64_t v, const char* item) {
    ReyVec* rv = vp(v);
    for (int64_t i = 0; i < rv->len; i++) {
        const char* el = (const char*)(uintptr_t)rv->data[i];
        if (rey_str_eq(el, item)) return 1;
    }
    return 0;
}

// Vec<String>.join(sep) — items are i8* cast to int64_t
char* rey_vec_join(int64_t v, const char* sep) {
    ReyVec* rv = vp(v);
    char* result = rey_str_empty();
    for (int64_t i = 0; i < rv->len; i++) {
        if (i > 0) result = rey_str_concat(result, sep);
        const char* el = (const char*)(uintptr_t)rv->data[i];
        result = rey_str_concat(result, el);
    }
    return result;
}

int64_t rey_vec_slice_v(int64_t v, int64_t start, int64_t end) {
    ReyVec* rv = vp(v);
    if (start < 0) start = 0;
    if (end > rv->len) end = rv->len;
    int64_t out = rey_vec_new();
    for (int64_t i = start; i < end; i++) rey_vec_push(out, rv->data[i]);
    return out;
}

int64_t rey_vec_reverse(int64_t v) {
    ReyVec* rv = vp(v);
    int64_t out = rey_vec_new();
    for (int64_t i = rv->len - 1; i >= 0; i--) rey_vec_push(out, rv->data[i]);
    return out;
}

int64_t rey_vec_map(int64_t v, int64_t fn_ptr) {
    typedef int64_t (*MapFn)(int64_t);
    MapFn fn = (MapFn)(uintptr_t)fn_ptr;
    ReyVec* rv = vp(v);
    int64_t out = rey_vec_new();
    for (int64_t i = 0; i < rv->len; i++) rey_vec_push(out, fn(rv->data[i]));
    return out;
}

int64_t rey_vec_filter(int64_t v, int64_t fn_ptr) {
    typedef int64_t (*FilterFn)(int64_t);
    FilterFn fn = (FilterFn)(uintptr_t)fn_ptr;
    ReyVec* rv = vp(v);
    int64_t out = rey_vec_new();
    for (int64_t i = 0; i < rv->len; i++)
        if (fn(rv->data[i])) rey_vec_push(out, rv->data[i]);
    return out;
}
