// Rey Result<T,E> and Option<T>. All i64 ABI.
// Layout: { uint8_t tag; uint8_t _pad[7]; int64_t payload; }
// Payload is int64_t; for string payloads it stores i8* cast to int64_t.
#include "rey_rt.h"

typedef struct {
    uint8_t tag;
    uint8_t _pad[7];
    int64_t payload;
} ReyTagged;

static int64_t tagged_new(uint8_t tag, int64_t payload) {
    ReyTagged* r = (ReyTagged*)rey_alloc((int64_t)sizeof(ReyTagged));
    r->tag = tag;
    r->payload = payload;
    return (int64_t)(uintptr_t)r;
}

int64_t rey_ok(int64_t val)  { return tagged_new(REY_TAG_OK,  val); }
int64_t rey_err(int64_t val) { return tagged_new(REY_TAG_ERR, val); }

int64_t rey_ok_str(const char* val) {
    return tagged_new(REY_TAG_OK, (int64_t)(uintptr_t)val);
}
int64_t rey_err_str(const char* val) {
    return tagged_new(REY_TAG_ERR, (int64_t)(uintptr_t)val);
}

static ReyTagged* tp(int64_t r) { return (ReyTagged*)(uintptr_t)r; }

int64_t rey_result_is_ok(int64_t r)  { return r && tp(r)->tag == REY_TAG_OK  ? 1 : 0; }
int64_t rey_result_is_err(int64_t r) { return !r || tp(r)->tag == REY_TAG_ERR ? 1 : 0; }

int64_t rey_result_unwrap(int64_t r) {
    if (!r || tp(r)->tag != REY_TAG_OK) rey_panic_cstr("Result.unwrap() on Err");
    return tp(r)->payload;
}
int64_t rey_result_unwrap_or(int64_t r, int64_t def) {
    if (!r || tp(r)->tag != REY_TAG_OK) return def;
    return tp(r)->payload;
}

const char* rey_result_unwrap_str(int64_t r) {
    if (!r || tp(r)->tag != REY_TAG_OK) rey_panic_cstr("Result.unwrap() on Err");
    return (const char*)(uintptr_t)tp(r)->payload;
}
const char* rey_result_unwrap_or_str(int64_t r, const char* def) {
    if (!r || tp(r)->tag != REY_TAG_OK) return def;
    return (const char*)(uintptr_t)tp(r)->payload;
}

int64_t rey_some(int64_t val) { return tagged_new(REY_TAG_SOME, val); }
int64_t rey_none(void)        { return tagged_new(REY_TAG_NONE, 0); }

static ReyTagged* op(int64_t o) { return (ReyTagged*)(uintptr_t)o; }

int64_t rey_option_is_some(int64_t o) { return o && op(o)->tag == REY_TAG_SOME ? 1 : 0; }
int64_t rey_option_unwrap(int64_t o) {
    if (!o || op(o)->tag != REY_TAG_SOME) rey_panic_cstr("Option.unwrap() on None");
    return op(o)->payload;
}

// instanceof for Vec/Map/Result/Option: check the tag byte at the pointer
int64_t rey_instanceof_tag(int64_t ptr, int64_t tag) {
    if (!ptr) return 0;
    return *(uint8_t*)(uintptr_t)ptr == (uint8_t)tag ? 1 : 0;
}

// instanceof for user structs: tag is at ptr-16 (data ptr is past the 16-byte header)
int64_t rey_struct_instanceof(int64_t ptr, int64_t tag) {
    if (!ptr) return 0;
    return *(uint8_t*)((uintptr_t)ptr - 16) == (uint8_t)tag ? 1 : 0;
}
