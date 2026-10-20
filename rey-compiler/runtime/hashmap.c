// Rey HashMap runtime. Keys are string data pointers (i8*); values are i64.
// HashMap itself is an i64 (pointer to ReyMap cast to int).
// Layout: { uint8_t tag=3; uint8_t _pad[7]; int64_t cap; int64_t len; HmEntry* entries; }
#include "rey_rt.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    const char* key;   // i8* (ReyStr data pointer)
    int64_t     val;
    int64_t     kind;
    uint8_t     occupied; // 0 empty, 1 live, 2 tombstone
} HmEntry;

typedef struct {
    uint8_t   tag;
    uint8_t   _pad[7];
    int64_t   cap;
    int64_t   len;
    HmEntry*  entries;
} ReyMap;

static ReyMap* mp(int64_t hm) { return (ReyMap*)(uintptr_t)hm; }

#define HM_INIT_CAP 16

static const char* requireKey(const char* key) {
    if (!key) rey_runtime_error("HashMap key is null");
    return key;
}

static uint64_t hm_hash(const char* key) {
    uint64_t hash = 14695981039346656037ULL;
    int64_t len = rey_str_len(key);
    for (int64_t i = 0; i < len; i++) {
        hash ^= (uint8_t)key[i];
        hash *= 1099511628211ULL;
    }
    return hash;
}

int64_t rey_hm_new(void) {
    ReyMap* m = (ReyMap*)rey_alloc((int64_t)sizeof(ReyMap));
    m->tag = REY_TAG_MAP;
    m->cap = HM_INIT_CAP;
    m->entries = (HmEntry*)rey_alloc(HM_INIT_CAP * (int64_t)sizeof(HmEntry));
    return (int64_t)(uintptr_t)m;
}

static void hm_insert_raw(ReyMap* m, const char* key, int64_t kind, int64_t val) {
    if (m->cap <= 0) rey_panic_cstr("HashMap capacity overflow");
    uint64_t h = hm_hash(key) % (uint64_t)m->cap;
    // Tombstones preserve probe chains after deletion.
    HmEntry* tombstone = NULL;
    for (int64_t tries = 0; tries < m->cap; tries++) {
        HmEntry* e = &m->entries[h];
        if (e->occupied == 1) {
            if (rey_str_eq(e->key, key)) {
                e->val = val;
                e->kind = kind;
                return;
            }
        } else if (e->occupied == 2) {
            if (!tombstone) tombstone = e;
        } else {
            HmEntry* target = tombstone ? tombstone : e;
            target->key = key;
            target->val = val;
            target->kind = kind;
            target->occupied = 1;
            m->len++;
            return;
        }
        h = (h + 1) % (uint64_t)m->cap;
    }
    if (tombstone) {
        tombstone->key = key;
        tombstone->val = val;
        tombstone->kind = kind;
        tombstone->occupied = 1;
        m->len++;
    }
}

static void hm_grow(ReyMap* m) {
    if (m->cap <= 0 || (uint64_t)m->cap > (uint64_t)INT64_MAX / sizeof(HmEntry) / 2)
        rey_panic_cstr("HashMap capacity overflow");
    int64_t nextCap = m->cap * 2;
    int64_t oldcap = m->cap;
    HmEntry* old = m->entries;
    m->cap = nextCap;
    m->len = 0;
    m->entries = (HmEntry*)rey_alloc(nextCap * (int64_t)sizeof(HmEntry));
    for (int64_t i = 0; i < oldcap; i++)
        if (old[i].occupied == 1) hm_insert_raw(m, old[i].key, old[i].kind, old[i].val);
    free(old);
}

static void mapSet(int64_t hm, const char* key, int64_t kind, int64_t val) {
    key = requireKey(key);
    rey_slot_check(kind, val);
    ReyMap* m = mp(hm);
    if (m->len < 0 || m->cap <= 1 || m->len >= m->cap / 2) hm_grow(m);
    hm_insert_raw(m, key, kind, val);
}

void rey_hm_set(int64_t hm, const char* key, int64_t val) {
    mapSet(hm, key, REY_SLOT_RAW, val);
}

void rey_hm_set_typed(int64_t hm, const char* key, int64_t kind, int64_t val) {
    mapSet(hm, key, kind, val);
}

static HmEntry* hm_find(ReyMap* m, const char* key) {
    if (m->cap <= 0) rey_panic_cstr("HashMap capacity overflow");
    uint64_t h = hm_hash(key) % (uint64_t)m->cap;
    for (int64_t tries = 0; tries < m->cap; tries++) {
        HmEntry* e = &m->entries[h];
        if (e->occupied == 0) return NULL;
        if (e->occupied == 1 && rey_str_eq(e->key, key)) return e;
        h = (h + 1) % (uint64_t)m->cap;
    }
    return NULL;
}

int64_t rey_hm_get(int64_t hm, const char* key) {
    HmEntry* e = hm_find(mp(hm), requireKey(key));
    return e ? e->val : 0;
}

int64_t rey_hm_has(int64_t hm, const char* key) {
    return hm_find(mp(hm), requireKey(key)) ? 1 : 0;
}

void rey_hm_delete(int64_t hm, const char* key) {
    HmEntry* e = hm_find(mp(hm), requireKey(key));
    if (e) { e->occupied = 2; mp(hm)->len--; }
}

int64_t rey_hm_len(int64_t hm) { return mp(hm)->len; }

static HmEntry* requireEntry(int64_t hm, const char* key) {
    HmEntry* entry = hm_find(mp(hm), requireKey(key));
    if (!entry) rey_runtime_error("HashMap key not found");
    return entry;
}

int64_t rey_hm_get_checked(int64_t hm, const char* key) {
    return requireEntry(hm, key)->val;
}

int64_t rey_hm_value_kind(int64_t hm, const char* key) {
    return requireEntry(hm, key)->kind;
}

int64_t rey_hm_get_typed(int64_t hm, const char* key, int64_t kind) {
    HmEntry* entry = requireEntry(hm, key);
    if (entry->kind != kind) {
        char message[128];
        snprintf(message, sizeof(message), "HashMap value kind mismatch: expected %lld, got %lld",
                 (long long)kind, (long long)entry->kind);
        rey_runtime_error(message);
    }
    return entry->val;
}

void rey_hm_set_float(int64_t hm, const char* key, double value) {
    rey_hm_set_typed(hm, key, REY_VALUE_FLOAT, rey_float_bits(value));
}

double rey_hm_get_float(int64_t hm, const char* key) {
    return rey_bits_to_float(rey_hm_get_typed(hm, key, REY_VALUE_FLOAT));
}

// returns Vec where items are i8* (cast to int64_t)
int64_t rey_hm_keys(int64_t hm) {
    ReyMap* m = mp(hm);
    int64_t v = rey_vec_new();
    for (int64_t i = 0; i < m->cap; i++)
        if (m->entries[i].occupied == 1)
            rey_vec_push(v, (int64_t)(uintptr_t)m->entries[i].key);
    return v;
}

int64_t rey_hm_values(int64_t hm) {
    ReyMap* m = mp(hm);
    int64_t v = rey_vec_new();
    for (int64_t i = 0; i < m->cap; i++)
        if (m->entries[i].occupied == 1) rey_vec_push(v, m->entries[i].val);
    return v;
}

void rey_hm_clear(int64_t hm) {
    ReyMap* m = mp(hm);
    for (int64_t i = 0; i < m->cap; i++) m->entries[i].occupied = 0;
    m->len = 0;
}

static int64_t storedKind(int64_t kind) {
    return kind == REY_SLOT_RAW ? REY_VALUE_INT : kind;
}

int64_t rey_hm_entries(int64_t hm) {
    ReyMap* map = mp(hm);
    int64_t entries = rey_vec_new();
    for (int64_t i = 0; i < map->cap; i++) {
        if (map->entries[i].occupied != 1) continue;
        int64_t pair = rey_vec_new();
        rey_vec_push_typed(pair, REY_VALUE_STRING, (int64_t)(uintptr_t)map->entries[i].key);
        rey_vec_push_typed(pair, storedKind(map->entries[i].kind), map->entries[i].val);
        rey_vec_push_typed(entries, REY_VALUE_VEC, pair);
    }
    return entries;
}

const char* rey_hm_to_str(int64_t hm) {
    ReyMap* map = mp(hm);
    char* text = rey_str_cstr("{");
    int emitted = 0;
    for (int64_t i = 0; i < map->cap; i++) {
        if (map->entries[i].occupied != 1) continue;
        if (emitted) text = rey_str_concat(text, rey_str_cstr(", "));
        emitted = 1;
        text = rey_str_concat(text, map->entries[i].key);
        text = rey_str_concat(text, rey_str_cstr(": "));
        text = rey_str_concat(text, rey_slot_to_str(storedKind(map->entries[i].kind), map->entries[i].val));
    }
    return rey_str_concat(text, rey_str_cstr("}"));
}
