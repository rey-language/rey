// Rey Result<T,E> and Option<T>. All i64 ABI.
// Layout: { uint8_t tag; uint8_t _pad[7]; int64_t payload; }
// Payload is int64_t; for string payloads it stores i8* cast to int64_t.
#include "rey_rt.h"

typedef struct {
    uint8_t tag;
    uint8_t _pad[7];
    int64_t payload_kind;
    int64_t payload;
} ReyTagged;

static int64_t tagged_new(uint8_t tag, int64_t payload_kind, int64_t payload) {
    ReyTagged* r = (ReyTagged*)rey_alloc((int64_t)sizeof(ReyTagged));
    r->tag = tag;
    r->payload_kind = payload_kind;
    r->payload = payload;
    return (int64_t)(uintptr_t)r;
}

static void validate_payload_kind(int64_t kind) {
    if (kind < REY_ENUM_ANY || kind > REY_ENUM_FUNC)
        rey_panic_cstr("invalid tagged payload kind");
}

int64_t rey_ok(int64_t val)  { return tagged_new(REY_TAG_OK, REY_ENUM_ANY, val); }
int64_t rey_err(int64_t val) { return tagged_new(REY_TAG_ERR, REY_ENUM_ANY, val); }
int64_t rey_ok_typed(int64_t val, int64_t kind) {
    validate_payload_kind(kind);
    return tagged_new(REY_TAG_OK, kind, val);
}
int64_t rey_err_typed(int64_t val, int64_t kind) {
    validate_payload_kind(kind);
    return tagged_new(REY_TAG_ERR, kind, val);
}

int64_t rey_ok_str(const char* val) {
    return tagged_new(REY_TAG_OK, REY_ENUM_STRING, (int64_t)(uintptr_t)val);
}
int64_t rey_err_str(const char* val) {
    return tagged_new(REY_TAG_ERR, REY_ENUM_STRING, (int64_t)(uintptr_t)val);
}

static ReyTagged* tp(int64_t r) { return (ReyTagged*)(uintptr_t)r; }

int64_t rey_result_is_ok(int64_t r)  { return r && tp(r)->tag == REY_TAG_OK  ? 1 : 0; }
int64_t rey_result_is_err(int64_t r) { return !r || tp(r)->tag == REY_TAG_ERR ? 1 : 0; }

int64_t rey_result_unwrap(int64_t r) {
    if (!r || tp(r)->tag != REY_TAG_OK) rey_panic_cstr("Result.unwrap() on Err");
    return tp(r)->payload;
}
int64_t rey_result_unwrap_err(int64_t r) {
    if (!r || tp(r)->tag != REY_TAG_ERR) rey_panic_cstr("Result.unwrapErr() on Ok");
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

int64_t rey_some(int64_t val) { return tagged_new(REY_TAG_SOME, REY_ENUM_ANY, val); }
int64_t rey_some_typed(int64_t val, int64_t kind) {
    validate_payload_kind(kind);
    return tagged_new(REY_TAG_SOME, kind, val);
}
int64_t rey_none(void)        { return tagged_new(REY_TAG_NONE, REY_ENUM_ANY, 0); }

static ReyTagged* op(int64_t o) { return (ReyTagged*)(uintptr_t)o; }

int64_t rey_option_is_some(int64_t o) { return o && op(o)->tag == REY_TAG_SOME ? 1 : 0; }
int64_t rey_option_unwrap(int64_t o) {
    if (!o || op(o)->tag != REY_TAG_SOME) rey_panic_cstr("Option.unwrap() on None");
    return op(o)->payload;
}

int64_t rey_option_eq(int64_t left, int64_t right) {
    if (!left || !right) return left == right;
    ReyTagged* a = tp(left);
    ReyTagged* b = tp(right);
    if (a->tag != REY_TAG_SOME && a->tag != REY_TAG_NONE) rey_panic_cstr("invalid Option value tag");
    if (b->tag != REY_TAG_SOME && b->tag != REY_TAG_NONE) rey_panic_cstr("invalid Option value tag");
    if (a->tag != b->tag) return 0;
    if (a->tag == REY_TAG_NONE) return 1;
    if (a->payload_kind != b->payload_kind) return 0;
    return rey_value_eq_typed(a->payload_kind, a->payload, b->payload);
}

int64_t rey_result_eq(int64_t left, int64_t right) {
    if (!left || !right) return left == right;
    ReyTagged* a = tp(left);
    ReyTagged* b = tp(right);
    if (a->tag != REY_TAG_OK && a->tag != REY_TAG_ERR) rey_panic_cstr("invalid Result value tag");
    if (b->tag != REY_TAG_OK && b->tag != REY_TAG_ERR) rey_panic_cstr("invalid Result value tag");
    if (a->tag != b->tag || a->payload_kind != b->payload_kind) return 0;
    return rey_value_eq_typed(a->payload_kind, a->payload, b->payload);
}

static ReyTagged* requireTagged(int64_t value, uint8_t first, uint8_t second, const char* message) {
    if (!value) rey_runtime_error(message);
    ReyTagged* tagged = tp(value);
    if (tagged->tag != first && tagged->tag != second) rey_runtime_error(message);
    return tagged;
}

int64_t rey_option_tag(int64_t option) {
    return requireTagged(option, REY_TAG_SOME, REY_TAG_NONE, "invalid Option value")->tag;
}

int64_t rey_option_payload_kind(int64_t option) {
    return requireTagged(option, REY_TAG_SOME, REY_TAG_NONE, "invalid Option value")->payload_kind;
}

int64_t rey_result_tag(int64_t result) {
    return requireTagged(result, REY_TAG_OK, REY_TAG_ERR, "invalid Result value")->tag;
}

int64_t rey_result_payload_kind(int64_t result) {
    return requireTagged(result, REY_TAG_OK, REY_TAG_ERR, "invalid Result value")->payload_kind;
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
