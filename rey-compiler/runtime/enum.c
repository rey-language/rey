#include "rey_rt.h"
#include <stdlib.h>
#include <string.h>

typedef struct {
    int64_t kind;
    int64_t payload;
} ReyEnumSlot;

typedef struct {
    uint8_t tag;
    uint8_t _pad[7];
    int64_t enum_id;
    int64_t discriminant;
    int64_t payload_count;
    ReyEnumSlot payloads[];
} ReyEnum;

static ReyEnum* enum_value(int64_t value) {
    if (!value) rey_panic_cstr("invalid null enum value");
    ReyEnum* result = (ReyEnum*)(uintptr_t)value;
    if (result->tag != REY_TAG_ENUM) rey_panic_cstr("invalid enum value tag");
    return result;
}

int64_t rey_enum_new(int64_t enum_id, int64_t discriminant, int64_t payload_count) {
    uint64_t maxBytes = (uint64_t)INT64_MAX;
    if ((uint64_t)SIZE_MAX < maxBytes) maxBytes = (uint64_t)SIZE_MAX;
    if (enum_id < 0 || discriminant < 0 || payload_count < 0 ||
        (uint64_t)payload_count > (maxBytes - sizeof(ReyEnum)) / sizeof(ReyEnumSlot))
        rey_panic_cstr("invalid enum layout");
    size_t bytes = sizeof(ReyEnum) + (size_t)payload_count * sizeof(ReyEnumSlot);
    ReyEnum* result = (ReyEnum*)rey_alloc((int64_t)bytes);
    result->tag = REY_TAG_ENUM;
    result->enum_id = enum_id;
    result->discriminant = discriminant;
    result->payload_count = payload_count;
    return (int64_t)(uintptr_t)result;
}

void rey_enum_set(int64_t value, int64_t index, int64_t kind, int64_t payload) {
    ReyEnum* result = enum_value(value);
    if (index < 0 || index >= result->payload_count || kind < REY_ENUM_INT || kind > REY_ENUM_FUNC)
        rey_panic_cstr("invalid enum payload slot");
    result->payloads[index].kind = kind;
    result->payloads[index].payload = payload;
}

int64_t rey_enum_is(int64_t value, int64_t enum_id, int64_t discriminant) {
    if (!value) return 0;
    ReyEnum* candidate = enum_value(value);
    return candidate->enum_id == enum_id && candidate->discriminant == discriminant;
}

int64_t rey_enum_get(int64_t value, int64_t enum_id, int64_t discriminant, int64_t index, int64_t kind) {
    ReyEnum* candidate = enum_value(value);
    if (candidate->enum_id != enum_id || candidate->discriminant != discriminant ||
        index < 0 || index >= candidate->payload_count || candidate->payloads[index].kind != kind)
        rey_panic_cstr("enum payload layout mismatch");
    return candidate->payloads[index].payload;
}

static int enumEqDepth;

static int enum_equal(int64_t left, int64_t right) {
    if (!left || !right) return left == right;
    if (++enumEqDepth > 64) rey_runtime_error("value comparison is too deep");
    int same = 0;
    ReyEnum* a = enum_value(left);
    ReyEnum* b = enum_value(right);
    if (a->enum_id == b->enum_id && a->discriminant == b->discriminant && a->payload_count == b->payload_count) {
        same = 1;
        for (int64_t i = 0; i < a->payload_count; i++) {
            ReyEnumSlot x = a->payloads[i];
            ReyEnumSlot y = b->payloads[i];
            if (x.kind != y.kind || !rey_value_eq_typed(x.kind, x.payload, y.payload)) {
                same = 0;
                break;
            }
        }
    }
    enumEqDepth--;
    return same;
}

int64_t rey_value_eq_typed(int64_t kind, int64_t left, int64_t right) {
    switch (kind) {
        case REY_ENUM_ANY:
        case REY_ENUM_INT:
        case REY_ENUM_BOOL:
        case REY_ENUM_NULL:
        case REY_ENUM_STRUCT:
        case REY_ENUM_VEC:
        case REY_ENUM_DICT:
            return left == right;
        case REY_ENUM_FLOAT: {
            double a, b;
            memcpy(&a, &left, sizeof(a));
            memcpy(&b, &right, sizeof(b));
            return a == b;
        }
        case REY_ENUM_STRING:
        case REY_ENUM_CHAR:
            return rey_str_eq((const char*)(uintptr_t)left, (const char*)(uintptr_t)right);
        case REY_ENUM_ENUM:
            return enum_equal(left, right);
        case REY_ENUM_OPTION:
            return rey_option_eq(left, right);
        case REY_ENUM_RESULT:
            return rey_result_eq(left, right);
        case REY_ENUM_TUPLE:
            return rey_tuple_eq(left, right);
        case REY_ENUM_FUNC:
            rey_runtime_error("function values are not comparable");
            return 0;
        default:
            rey_panic_cstr("invalid enum payload kind");
            return 0;
    }
}

int64_t rey_enum_eq(int64_t left, int64_t right, int64_t enum_id) {
    if (!left || !right) return left == right;
    ReyEnum* a = enum_value(left);
    ReyEnum* b = enum_value(right);
    if (a->enum_id != enum_id || b->enum_id != enum_id) return 0;
    return enum_equal(left, right);
}

int64_t rey_enum_eq_values(int64_t left, int64_t right) {
    return enum_equal(left, right);
}

static ReyEnum* enumChecked(int64_t value) {
    if (!value) rey_runtime_error("invalid null enum value");
    ReyEnum* result = (ReyEnum*)(uintptr_t)value;
    if (result->tag != REY_TAG_ENUM) rey_runtime_error("invalid enum value tag");
    return result;
}

int64_t rey_enum_identity(int64_t value) { return enumChecked(value)->enum_id; }
int64_t rey_enum_discriminant(int64_t value) { return enumChecked(value)->discriminant; }

int64_t rey_enum_is_type(int64_t value, int64_t enumId) {
    if (!value) return 0;
    ReyEnum* candidate = (ReyEnum*)(uintptr_t)value;
    if (candidate->tag != REY_TAG_ENUM) return 0;
    return candidate->enum_id == enumId;
}

int64_t rey_enum_payload_count(int64_t value) { return enumChecked(value)->payload_count; }

int64_t rey_enum_payload_kind(int64_t value, int64_t index) {
    ReyEnum* result = enumChecked(value);
    if (index < 0 || index >= result->payload_count) rey_runtime_error("enum payload index out of bounds");
    return result->payloads[index].kind;
}

int64_t rey_enum_payload_at(int64_t value, int64_t index) {
    ReyEnum* result = enumChecked(value);
    if (index < 0 || index >= result->payload_count) rey_runtime_error("enum payload index out of bounds");
    return result->payloads[index].payload;
}
