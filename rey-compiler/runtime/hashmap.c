// Rey HashMap runtime. Keys are string data pointers (i8*); values are i64.
// HashMap itself is an i64 (pointer to ReyMap cast to int).
// Layout: { uint8_t tag=3; uint8_t _pad[7]; int64_t cap; int64_t len; HmEntry* entries; }
#include "rey_rt.h"
#include <stdlib.h>
#include <string.h>

typedef struct {
    const char* key;   // i8* (ReyStr data pointer)
    int64_t     val;
    uint8_t     occupied;
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

static uint64_t hm_hash(const char* key) {
    uint64_t h = 14695981039346656037ULL;
    while (*key) { h ^= (uint8_t)*key++; h *= 1099511628211ULL; }
    return h;
}

int64_t rey_hm_new(void) {
    ReyMap* m = (ReyMap*)rey_alloc((int64_t)sizeof(ReyMap));
    m->tag = REY_TAG_MAP;
    m->cap = HM_INIT_CAP;
    m->entries = (HmEntry*)rey_alloc(HM_INIT_CAP * (int64_t)sizeof(HmEntry));
    return (int64_t)(uintptr_t)m;
}

static void hm_insert_raw(ReyMap* m, const char* key, int64_t val) {
    uint64_t h = hm_hash(key) % (uint64_t)m->cap;
    while (m->entries[h].occupied) {
        if (rey_str_eq(m->entries[h].key, key)) {
            m->entries[h].val = val;
            return;
        }
        h = (h + 1) % (uint64_t)m->cap;
    }
    m->entries[h].key = key;
    m->entries[h].val = val;
    m->entries[h].occupied = 1;
    m->len++;
}

static void hm_grow(ReyMap* m) {
    int64_t oldcap = m->cap;
    HmEntry* old = m->entries;
    m->cap = oldcap * 2;
    m->len = 0;
    m->entries = (HmEntry*)rey_alloc(m->cap * (int64_t)sizeof(HmEntry));
    for (int64_t i = 0; i < oldcap; i++)
        if (old[i].occupied) hm_insert_raw(m, old[i].key, old[i].val);
    free(old);
}

void rey_hm_set(int64_t hm, const char* key, int64_t val) {
    ReyMap* m = mp(hm);
    if (m->len * 2 >= m->cap) hm_grow(m);
    hm_insert_raw(m, key, val);
}

static HmEntry* hm_find(ReyMap* m, const char* key) {
    uint64_t h = hm_hash(key) % (uint64_t)m->cap;
    int64_t tries = 0;
    while (m->entries[h].occupied && tries < m->cap) {
        if (rey_str_eq(m->entries[h].key, key)) return &m->entries[h];
        h = (h + 1) % (uint64_t)m->cap;
        tries++;
    }
    return NULL;
}

int64_t rey_hm_get(int64_t hm, const char* key) {
    HmEntry* e = hm_find(mp(hm), key);
    return e ? e->val : 0;
}

int64_t rey_hm_has(int64_t hm, const char* key) {
    return hm_find(mp(hm), key) ? 1 : 0;
}

void rey_hm_delete(int64_t hm, const char* key) {
    HmEntry* e = hm_find(mp(hm), key);
    if (e) { e->occupied = 0; mp(hm)->len--; }
}

// returns Vec where items are i8* (cast to int64_t)
int64_t rey_hm_keys(int64_t hm) {
    ReyMap* m = mp(hm);
    int64_t v = rey_vec_new();
    for (int64_t i = 0; i < m->cap; i++)
        if (m->entries[i].occupied)
            rey_vec_push(v, (int64_t)(uintptr_t)m->entries[i].key);
    return v;
}

int64_t rey_hm_values(int64_t hm) {
    ReyMap* m = mp(hm);
    int64_t v = rey_vec_new();
    for (int64_t i = 0; i < m->cap; i++)
        if (m->entries[i].occupied) rey_vec_push(v, m->entries[i].val);
    return v;
}
