// Rey Vec runtime. All Vec values are i64 (pointer to ReyVec cast to int).
// Vec items are int64_t. String items are stored as int64_t (i8* cast to int64_t).
// Layout: { uint8_t tag=2; uint8_t _pad[7]; int64_t len; int64_t cap; int64_t* data; int64_t* kinds; }
#include "rey_rt.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    uint8_t  tag;
    uint8_t  _pad[7];
    int64_t  len;
    int64_t  cap;
    int64_t* restrict data;
    int64_t* restrict kinds;
} ReyVec;

static ReyVec* vp(int64_t v) { return (ReyVec*)(uintptr_t)v; }

int64_t rey_vec_new(void) {
    ReyVec* v = (ReyVec*)rey_alloc((int64_t)sizeof(ReyVec));
    v->tag = REY_TAG_VEC;
    return (int64_t)(uintptr_t)v;
}

static void vec_grow(ReyVec* v) {
    uint64_t max_bytes = (uint64_t)INT64_MAX;
    if ((uint64_t)SIZE_MAX < max_bytes) max_bytes = (uint64_t)SIZE_MAX;
    int64_t max_cap = (int64_t)(max_bytes / sizeof(int64_t));
    if (v->cap < 0 || v->cap > max_cap / 2)
        rey_panic_cstr("Vec capacity overflow");
    int64_t nc = v->cap == 0 ? 8 : v->cap * 2;
    if (nc > max_cap) rey_panic_cstr("Vec capacity overflow");

    int64_t* nd = (int64_t*)rey_alloc(nc * (int64_t)sizeof(int64_t));
    int64_t* nk = NULL;
    if (v->kinds) nk = (int64_t*)rey_alloc(nc * (int64_t)sizeof(int64_t));
    if (v->data && v->len > 0) {
        memcpy(nd, v->data, (size_t)v->len * sizeof(int64_t));
        if (v->kinds) memcpy(nk, v->kinds, (size_t)v->len * sizeof(int64_t));
    }
    free(v->data);
    free(v->kinds);
    v->data = nd;
    v->kinds = nk;
    v->cap = nc;
}

// Kinds are only needed once a Vec holds a typed (non-raw) slot. Plain
// Vec<int>/Vec<bool>/Vec<string> values stay on the fast raw path and never
// pay for a second 8-byte metadata array. rey_alloc zeroes, so existing slots
// become REY_SLOT_RAW automatically.
static void vecEnsureKinds(ReyVec* v) {
    if (v->kinds) return;
    v->kinds = (int64_t*)rey_alloc(v->cap * (int64_t)sizeof(int64_t));
}

static int64_t slotKind(ReyVec* vec, int64_t index) {
    return vec->kinds ? vec->kinds[index] : REY_SLOT_RAW;
}

static void vecPush(ReyVec* vec, int64_t kind, int64_t item) {
    if (vec->len >= vec->cap) vec_grow(vec);
    if (kind != REY_SLOT_RAW) vecEnsureKinds(vec);
    vec->data[vec->len] = item;
    if (vec->kinds) vec->kinds[vec->len] = kind;
    vec->len++;
}

void rey_vec_push(int64_t v, int64_t item) {
    vecPush(vp(v), REY_SLOT_RAW, item);
}

void rey_vec_push_typed(int64_t v, int64_t kind, int64_t payload) {
    rey_slot_check(kind, payload);
    vecPush(vp(v), kind, payload);
}

int64_t rey_vec_pop(int64_t v) {
    ReyVec* rv = vp(v);
    if (rv->len == 0) rey_panic_cstr("Vec.pop() on empty vec");
    return rv->data[--rv->len];
}

REY_RUNTIME_ALWAYS_INLINE REY_RUNTIME_LTO_API int64_t rey_vec_len(int64_t v) { return vp(v)->len; }

REY_RUNTIME_ALWAYS_INLINE REY_RUNTIME_LTO_API int64_t rey_vec_get(int64_t v, int64_t i) {
    ReyVec* rv = vp(v);
    int64_t len = rv->len;
    int64_t* restrict data = rv->data;
    if (__builtin_expect((uint64_t)i >= (uint64_t)len, 0))
        rey_panic_cstr("Vec index out of bounds");
    return data[i];
}

REY_RUNTIME_ALWAYS_INLINE REY_RUNTIME_LTO_API void rey_vec_set(int64_t v, int64_t i, int64_t val) {
    ReyVec* rv = vp(v);
    int64_t len = rv->len;
    int64_t* restrict data = rv->data;
    int64_t* restrict kinds = rv->kinds;
    if (__builtin_expect((uint64_t)i >= (uint64_t)len, 0))
        rey_panic_cstr("Vec.set index out of bounds");
    data[i] = val;
    if (kinds) kinds[i] = REY_SLOT_RAW;
}

// Raw-element fast paths. These are only valid for Vec<T> where T is not
// float/any, so the value array is the whole payload and no kinds array can
// exist. Callers that have a statically raw element type may use them to skip
// the per-access kinds test.
REY_RUNTIME_ALWAYS_INLINE REY_RUNTIME_LTO_API int64_t rey_vec_get_raw(int64_t v, int64_t i) {
    ReyVec* rv = vp(v);
    int64_t len = rv->len;
    int64_t* restrict data = rv->data;
    if (__builtin_expect((uint64_t)i >= (uint64_t)len, 0))
        rey_panic_cstr("Vec index out of bounds");
    return data[i];
}

REY_RUNTIME_ALWAYS_INLINE REY_RUNTIME_LTO_API void rey_vec_set_raw(int64_t v, int64_t i, int64_t val) {
    ReyVec* rv = vp(v);
    int64_t len = rv->len;
    int64_t* restrict data = rv->data;
    if (__builtin_expect((uint64_t)i >= (uint64_t)len, 0))
        rey_panic_cstr("Vec.set index out of bounds");
    data[i] = val;
}

// Internal probe/unsafe hooks; not emitted by the default codegen.
REY_RUNTIME_ALWAYS_INLINE REY_RUNTIME_LTO_API int64_t rey_vec_get_unchecked(int64_t v, int64_t i) {
    int64_t* restrict data = vp(v)->data;
    return data[i];
}

REY_RUNTIME_ALWAYS_INLINE REY_RUNTIME_LTO_API void rey_vec_set_unchecked(int64_t v, int64_t i, int64_t val) {
    int64_t* restrict data = vp(v)->data;
    data[i] = val;
}

static void kindMismatch(const char* what, int64_t expected, int64_t actual) {
    char message[128];
    snprintf(message, sizeof(message), "%s kind mismatch: expected %lld, got %lld",
             what, (long long)expected, (long long)actual);
    rey_runtime_error(message);
}

int64_t rey_vec_kind(int64_t v, int64_t index) {
    ReyVec* rv = vp(v);
    if (__builtin_expect((uint64_t)index >= (uint64_t)rv->len, 0))
        rey_panic_cstr("Vec index out of bounds");
    return slotKind(rv, index);
}

int64_t rey_vec_get_typed(int64_t v, int64_t index, int64_t kind) {
    int64_t payload = rey_vec_get(v, index);
    int64_t actual = rey_vec_kind(v, index);
    if (actual != kind) kindMismatch("Vec element", kind, actual);
    return payload;
}

void rey_vec_set_typed(int64_t v, int64_t index, int64_t kind, int64_t payload) {
    rey_slot_check(kind, payload);
    ReyVec* rv = vp(v);
    rey_vec_set(v, index, payload);
    vecEnsureKinds(rv);
    rv->kinds[index] = kind;
}

int64_t rey_vec_contains_typed(int64_t v, int64_t kind, int64_t payload) {
    ReyVec* rv = vp(v);
    for (int64_t i = 0; i < rv->len; i++) {
        if (slotKind(rv, i) != kind) continue;
        if (rey_slot_eq(kind, rv->data[i], payload)) return 1;
    }
    return 0;
}

void rey_vec_push_float(int64_t v, double value) {
    rey_vec_push_typed(v, REY_VALUE_FLOAT, rey_float_bits(value));
}

void rey_vec_set_float(int64_t v, int64_t index, double value) {
    rey_vec_set_typed(v, index, REY_VALUE_FLOAT, rey_float_bits(value));
}

double rey_vec_get_float(int64_t v, int64_t index) {
    return rey_bits_to_float(rey_vec_get_typed(v, index, REY_VALUE_FLOAT));
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
    if (start < 0 || end < 0) rey_runtime_error("Vec.slice() indices must be non-negative");
    if (start > rv->len) start = rv->len;
    if (end > rv->len) end = rv->len;
    int64_t out = rey_vec_new();
    if (end <= start) return out;
    for (int64_t i = start; i < end; i++) rey_vec_push_typed(out, slotKind(rv, i), rv->data[i]);
    return out;
}

int64_t rey_vec_reverse(int64_t v) {
    ReyVec* rv = vp(v);
    int64_t out = rey_vec_new();
    for (int64_t i = rv->len - 1; i >= 0; i--) rey_vec_push_typed(out, slotKind(rv, i), rv->data[i]);
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
        if (fn(rv->data[i])) rey_vec_push_typed(out, slotKind(rv, i), rv->data[i]);
    return out;
}

int64_t rey_vec_index_of(int64_t v, int64_t item) {
    ReyVec* rv = vp(v);
    for (int64_t i = 0; i < rv->len; i++)
        if (rv->data[i] == item) return i;
    return -1;
}

int64_t rey_vec_str_index_of(int64_t v, const char* item) {
    ReyVec* rv = vp(v);
    for (int64_t i = 0; i < rv->len; i++) {
        const char* element = (const char*)(uintptr_t)rv->data[i];
        if (rey_str_eq(element, item)) return i;
    }
    return -1;
}

int64_t rey_vec_index_of_typed(int64_t v, int64_t kind, int64_t payload) {
    ReyVec* rv = vp(v);
    for (int64_t i = 0; i < rv->len; i++) {
        if (slotKind(rv, i) != kind) continue;
        if (rey_slot_eq(kind, rv->data[i], payload)) return i;
    }
    return -1;
}

static void swapSlot(ReyVec* vec, int64_t left, int64_t right) {
    int64_t value = vec->data[left];
    vec->data[left] = vec->data[right];
    vec->data[right] = value;
    if (!vec->kinds) return;
    int64_t kind = vec->kinds[left];
    vec->kinds[left] = vec->kinds[right];
    vec->kinds[right] = kind;
}

void rey_vec_insert_typed(int64_t v, int64_t index, int64_t kind, int64_t payload) {
    if (kind != REY_SLOT_RAW) rey_slot_check(kind, payload);
    ReyVec* rv = vp(v);
    if (index < 0 || index > rv->len) rey_runtime_error("Vec index out of bounds");
    if (rv->len >= rv->cap) vec_grow(rv);
    if (!rv->kinds && rv->cap > 0)
        rv->kinds = (int64_t*)rey_alloc(rv->cap * (int64_t)sizeof(int64_t));
    int64_t tail = rv->len - index;
    if (tail > 0) {
        memmove(rv->data + index + 1, rv->data + index, (size_t)tail * sizeof(int64_t));
        if (rv->kinds)
            memmove(rv->kinds + index + 1, rv->kinds + index, (size_t)tail * sizeof(int64_t));
    }
    rv->data[index] = payload;
    if (rv->kinds) rv->kinds[index] = kind;
    rv->len++;
}

void rey_vec_insert(int64_t v, int64_t index, int64_t item) {
    rey_vec_insert_typed(v, index, REY_SLOT_RAW, item);
}

int64_t rey_vec_remove(int64_t v, int64_t index) {
    ReyVec* rv = vp(v);
    if (index < 0 || index >= rv->len) rey_runtime_error("Vec index out of bounds");
    int64_t item = rv->data[index];
    int64_t tail = rv->len - index - 1;
    if (tail > 0) {
        memmove(rv->data + index, rv->data + index + 1, (size_t)tail * sizeof(int64_t));
        if (rv->kinds)
            memmove(rv->kinds + index, rv->kinds + index + 1, (size_t)tail * sizeof(int64_t));
    }
    rv->len--;
    return item;
}

void rey_vec_clear(int64_t v) {
    vp(v)->len = 0;
}

void rey_vec_reverse_mut(int64_t v) {
    ReyVec* rv = vp(v);
    int64_t left = 0;
    int64_t right = rv->len - 1;
    while (left < right) {
        swapSlot(rv, left, right);
        left++;
        right--;
    }
}

static int sortBefore(int64_t kind, int64_t left, int64_t right) {
    if (kind == REY_VALUE_FLOAT) {
        double a = rey_bits_to_float(left);
        double b = rey_bits_to_float(right);
        return a < b;
    }
    if (kind == REY_VALUE_STRING) {
        const char* a = (const char*)(uintptr_t)left;
        const char* b = (const char*)(uintptr_t)right;
        int64_t aLen = rey_str_len(a);
        int64_t bLen = rey_str_len(b);
        int64_t n = aLen < bLen ? aLen : bLen;
        int cmp = n ? memcmp(a, b, (size_t)n) : 0;
        if (cmp < 0) return 1;
        if (cmp > 0) return 0;
        return aLen < bLen;
    }
    return left < right;
}

static int64_t sortKindOf(int64_t kind) {
    if (kind == REY_SLOT_RAW || kind == REY_VALUE_INT || kind == REY_VALUE_FLOAT ||
        kind == REY_VALUE_STRING || kind == REY_VALUE_BOOL)
        return kind;
    rey_runtime_error("Vec.sort() only supports int, float, String, bool");
    return kind;
}

void rey_vec_sort(int64_t v) {
    ReyVec* rv = vp(v);
    int64_t n = rv->len;
    if (n <= 1) return;
    int64_t kind = sortKindOf(slotKind(rv, 0));
    for (int64_t i = 1; i < n; i++) {
        if (slotKind(rv, i) != kind)
            rey_runtime_error("Vec.sort() requires all elements to be the same comparable type");
    }
    if (n > INT64_MAX / (int64_t)sizeof(int64_t)) rey_panic_cstr("Vec capacity overflow");
    int64_t* aux = (int64_t*)rey_alloc(n * (int64_t)sizeof(int64_t));
    int64_t* auxKinds = rv->kinds ? (int64_t*)rey_alloc(n * (int64_t)sizeof(int64_t)) : NULL;
    int64_t* src = rv->data;
    int64_t* dst = aux;
    int64_t* srcKinds = rv->kinds;
    int64_t* dstKinds = auxKinds;
    for (int64_t width = 1; width < n; ) {
        for (int64_t left = 0; left < n; left += width * 2) {
            int64_t mid = left + width;
            if (mid > n) mid = n;
            int64_t right = mid + width;
            if (right > n) right = n;
            int64_t i = left;
            int64_t j = mid;
            for (int64_t k = left; k < right; k++) {
                int takeRight = j < right && (i >= mid || sortBefore(kind, src[j], src[i]));
                if (takeRight) {
                    dst[k] = src[j];
                    if (dstKinds) dstKinds[k] = srcKinds[j];
                    j++;
                } else {
                    dst[k] = src[i];
                    if (dstKinds) dstKinds[k] = srcKinds[i];
                    i++;
                }
            }
        }
        int64_t* swapData = src;
        src = dst;
        dst = swapData;
        int64_t* swapKinds = srcKinds;
        srcKinds = dstKinds;
        dstKinds = swapKinds;
        if (width > (n - 1) / 2) break;
        width *= 2;
    }
    if (src != rv->data) {
        memcpy(rv->data, src, (size_t)n * sizeof(int64_t));
        if (rv->kinds && srcKinds) memcpy(rv->kinds, srcKinds, (size_t)n * sizeof(int64_t));
    }
    free(aux);
    free(auxKinds);
}

int64_t rey_vec_reduce(int64_t v, int64_t fnPtr, int64_t env, int64_t initial) {
    typedef int64_t (*ReduceFn)(int64_t, int64_t, int64_t);
    ReduceFn fn = (ReduceFn)(uintptr_t)fnPtr;
    ReyVec* rv = vp(v);
    int64_t acc = initial;
    for (int64_t i = 0; i < rv->len; i++) acc = fn(env, acc, rv->data[i]);
    return acc;
}

int64_t rey_pack_vec(int64_t count) {
    if (count < 0) rey_runtime_error("variadic count is negative");
    uint64_t maxBytes = (uint64_t)INT64_MAX;
    if ((uint64_t)SIZE_MAX < maxBytes) maxBytes = (uint64_t)SIZE_MAX;
    if (count > 0 && (uint64_t)count > maxBytes / sizeof(int64_t))
        rey_runtime_error("variadic count overflows");
    int64_t vec = rey_vec_new();
    if (count == 0) return vec;
    ReyVec* object = vp(vec);
    object->data = (int64_t*)rey_alloc(count * (int64_t)sizeof(int64_t));
    object->kinds = (int64_t*)rey_alloc(count * (int64_t)sizeof(int64_t));
    object->len = count;
    object->cap = count;
    return vec;
}

void rey_pack_set(int64_t vec, int64_t index, int64_t kind, int64_t payload) {
    rey_vec_set_typed(vec, index, kind, payload);
}

void rey_pack_set_float(int64_t vec, int64_t index, double value) {
    rey_vec_set_float(vec, index, value);
}
