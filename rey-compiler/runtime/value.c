#include "rey_rt.h"
#include <math.h>
#include <stdio.h>
#include <string.h>

typedef struct {
    int64_t kind;
    int64_t payload;
} ReySlot;

typedef struct {
    uint8_t tag;
    uint8_t pad[7];
    int64_t count;
    ReySlot slots[];
} ReySlots;

typedef struct {
    uint8_t tag;
    uint8_t pad[7];
    int64_t kind;
    int64_t payload;
    int64_t nominal;
} ReyBox;

typedef struct {
    uint8_t tag;
    uint8_t pad[7];
    int64_t fnPtr;
    int64_t env;
    int64_t arity;
} ReyClosure;

typedef struct {
    uint8_t tagByte;
    uint8_t pad[7];
    int64_t typeId;
} ReyStructHeader;

static int renderDepth;
static int eqDepth;

static int64_t objectTag(int64_t payload) {
    return *(uint8_t*)(uintptr_t)payload;
}

static void requireTag(int64_t payload, int64_t tag, const char* message) {
    if (!payload || objectTag(payload) != tag) rey_runtime_error(message);
}

static int isValueKind(int64_t kind) {
    return kind >= REY_VALUE_INT && kind <= REY_VALUE_ANY;
}

void rey_slot_check(int64_t kind, int64_t payload) {
    switch (kind) {
        case REY_SLOT_RAW:
        case REY_VALUE_INT:
        case REY_VALUE_FLOAT:
            return;
        case REY_VALUE_BOOL:
            if (payload != 0 && payload != 1) rey_runtime_error("bool payload must be 0 or 1");
            return;
        case REY_VALUE_CHAR:
            if (payload < 0 || payload > 255) rey_runtime_error("char payload must be in range 0..255");
            return;
        case REY_VALUE_NULL:
            if (payload != 0) rey_runtime_error("null payload must be 0");
            return;
        case REY_VALUE_STRING:
            if (!payload) rey_runtime_error("string payload must be non-null");
            return;
        case REY_VALUE_STRUCT:
            if (!payload) rey_runtime_error("struct payload must be non-null");
            return;
        case REY_VALUE_ENUM:
            requireTag(payload, REY_TAG_ENUM, "enum payload is invalid");
            return;
        case REY_VALUE_VEC:
            requireTag(payload, REY_TAG_VEC, "Vec payload is invalid");
            return;
        case REY_VALUE_DICT:
            requireTag(payload, REY_TAG_MAP, "HashMap payload is invalid");
            return;
        case REY_VALUE_TUPLE:
            requireTag(payload, REY_TAG_TUPLE, "tuple payload is invalid");
            return;
        case REY_VALUE_OPTION:
            if (!payload) rey_runtime_error("Option payload is invalid");
            if (objectTag(payload) != REY_TAG_SOME && objectTag(payload) != REY_TAG_NONE)
                rey_runtime_error("Option payload is invalid");
            return;
        case REY_VALUE_RESULT:
            if (!payload) rey_runtime_error("Result payload is invalid");
            if (objectTag(payload) != REY_TAG_OK && objectTag(payload) != REY_TAG_ERR)
                rey_runtime_error("Result payload is invalid");
            return;
        case REY_VALUE_FUNC:
            requireTag(payload, REY_TAG_CLOSURE, "function payload is invalid");
            return;
        case REY_VALUE_ANY:
            requireTag(payload, REY_TAG_VALUE, "any payload is invalid");
            return;
        default:
            rey_runtime_error("unsupported value kind");
    }
}

int64_t rey_float_bits(double value) {
    int64_t bits = 0;
    if (sizeof(value) != sizeof(bits)) rey_runtime_error("float payload requires 64-bit double");
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

double rey_bits_to_float(int64_t bits) {
    double value = 0;
    if (sizeof(value) != sizeof(bits)) rey_runtime_error("float payload requires 64-bit double");
    memcpy(&value, &bits, sizeof(value));
    return value;
}

int64_t rey_float_to_int(double value) {
    if (!isfinite(value) || value < -9223372036854775808.0 || value >= 9223372036854775808.0)
        rey_runtime_error("Float value out of int range");
    return (int64_t)value;
}

double rey_int_to_float(int64_t value) {
    return (double)value;
}

int64_t rey_instanceof(int64_t value, int64_t queryKind, int64_t nominal) {
    if (queryKind == REY_VALUE_STRUCT || queryKind == REY_VALUE_ENUM)
        return rey_value_is_nominal(value, queryKind, nominal);
    if (nominal != 0) return 0;
    return rey_value_is(value, queryKind);
}

static ReyBox* valueBox(int64_t value) {
    if (!value) rey_runtime_error("null dynamic value");
    ReyBox* box = (ReyBox*)(uintptr_t)value;
    if (box->tag != REY_TAG_VALUE) rey_runtime_error("invalid dynamic value");
    return box;
}

static ReyBox* valueIfBox(int64_t value) {
    if (!value) return NULL;
    ReyBox* box = (ReyBox*)(uintptr_t)value;
    if (box->tag != REY_TAG_VALUE) return NULL;
    return box;
}

int64_t rey_value_new_nominal(int64_t kind, int64_t payload, int64_t nominal) {
    if (!isValueKind(kind)) rey_runtime_error("unsupported value kind");
    rey_slot_check(kind, payload);
    if (kind != REY_VALUE_STRUCT && kind != REY_VALUE_ENUM && nominal != 0)
        rey_runtime_error("nominal identity is only valid for struct and enum values");
    ReyBox* box = (ReyBox*)rey_alloc((int64_t)sizeof(ReyBox));
    box->tag = REY_TAG_VALUE;
    box->kind = kind;
    box->payload = payload;
    box->nominal = nominal;
    return (int64_t)(uintptr_t)box;
}

int64_t rey_value_new(int64_t kind, int64_t payload) {
    return rey_value_new_nominal(kind, payload, 0);
}

int64_t rey_value_null(void) {
    return rey_value_new(REY_VALUE_NULL, 0);
}

int64_t rey_value_kind(int64_t value) { return valueBox(value)->kind; }
int64_t rey_value_payload(int64_t value) { return valueBox(value)->payload; }
int64_t rey_value_nominal(int64_t value) { return valueBox(value)->nominal; }

int64_t rey_value_is(int64_t value, int64_t kind) {
    ReyBox* box = valueIfBox(value);
    return box && box->kind == kind;
}

int64_t rey_value_is_nominal(int64_t value, int64_t kind, int64_t nominal) {
    ReyBox* box = valueIfBox(value);
    return box && box->kind == kind && box->nominal == nominal;
}

static void kindMismatch(int64_t expectedKind, int64_t actualKind) {
    char message[96];
    snprintf(message, sizeof(message), "value kind mismatch: expected %lld, got %lld",
             (long long)expectedKind, (long long)actualKind);
    rey_runtime_error(message);
}

int64_t rey_value_unbox(int64_t value, int64_t kind) {
    ReyBox* box = valueBox(value);
    if (box->kind != kind) kindMismatch(kind, box->kind);
    return box->payload;
}

int64_t rey_value_unbox_nominal(int64_t value, int64_t kind, int64_t nominal) {
    ReyBox* box = valueBox(value);
    if (box->kind != kind || box->nominal != nominal) {
        char message[160];
        snprintf(message, sizeof(message),
                 "value kind mismatch: expected %lld nominal %lld, got %lld nominal %lld",
                 (long long)kind, (long long)nominal,
                 (long long)box->kind, (long long)box->nominal);
        rey_runtime_error(message);
    }
    return box->payload;
}

static int64_t slotEqBody(int64_t kind, int64_t left, int64_t right) {
    switch (kind) {
        case REY_SLOT_RAW:
        case REY_VALUE_INT:
        case REY_VALUE_BOOL:
        case REY_VALUE_CHAR:
        case REY_VALUE_NULL:
        case REY_VALUE_STRUCT:
        case REY_VALUE_VEC:
        case REY_VALUE_DICT:
            return left == right;
        case REY_VALUE_FLOAT:
            return rey_bits_to_float(left) == rey_bits_to_float(right);
        case REY_VALUE_STRING:
            return rey_str_eq((const char*)(uintptr_t)left, (const char*)(uintptr_t)right);
        case REY_VALUE_ENUM:
            return rey_enum_eq_values(left, right);
        case REY_VALUE_TUPLE:
            return rey_tuple_eq(left, right);
        case REY_VALUE_OPTION:
            return rey_option_eq(left, right);
        case REY_VALUE_RESULT:
            return rey_result_eq(left, right);
        case REY_VALUE_ANY:
            return rey_value_eq(left, right);
        case REY_VALUE_FUNC:
            rey_runtime_error("function values are not comparable");
            return 0;
        default:
            rey_runtime_error("unsupported value kind");
            return 0;
    }
}

int64_t rey_slot_eq(int64_t kind, int64_t left, int64_t right) {
    if (++eqDepth > 64) rey_runtime_error("value comparison is too deep");
    int64_t same = slotEqBody(kind, left, right);
    eqDepth--;
    return same;
}

int64_t rey_value_eq(int64_t left, int64_t right) {
    ReyBox* a = valueBox(left);
    ReyBox* b = valueBox(right);
    if (a->kind != b->kind || a->nominal != b->nominal) return 0;
    return rey_slot_eq(a->kind, a->payload, b->payload);
}

static ReySlots* slotsObject(int64_t value, int64_t tag, const char* message) {
    if (!value) rey_runtime_error(message);
    ReySlots* object = (ReySlots*)(uintptr_t)value;
    if (object->tag != tag) rey_runtime_error(message);
    return object;
}

static int64_t slotsNew(uint8_t tag, int64_t count) {
    if (count < 0) rey_runtime_error("slot count is negative");
    if ((uint64_t)count > ((uint64_t)INT64_MAX - sizeof(ReySlots)) / sizeof(ReySlot))
        rey_runtime_error("slot count overflows");
    int64_t bytes = (int64_t)sizeof(ReySlots) + count * (int64_t)sizeof(ReySlot);
    ReySlots* object = (ReySlots*)rey_alloc(bytes);
    object->tag = tag;
    object->count = count;
    return (int64_t)(uintptr_t)object;
}

static ReySlot* slotAt(ReySlots* object, int64_t index, const char* message) {
    if (index < 0 || index >= object->count) rey_runtime_error(message);
    return &object->slots[index];
}

int64_t rey_tuple_new(int64_t count) { return slotsNew(REY_TAG_TUPLE, count); }

void rey_tuple_set(int64_t tuple, int64_t index, int64_t kind, int64_t payload) {
    rey_slot_check(kind, payload);
    ReySlot* slot = slotAt(slotsObject(tuple, REY_TAG_TUPLE, "invalid tuple"), index, "tuple index out of bounds");
    slot->kind = kind;
    slot->payload = payload;
}

int64_t rey_tuple_get(int64_t tuple, int64_t index) {
    return slotAt(slotsObject(tuple, REY_TAG_TUPLE, "invalid tuple"), index, "tuple index out of bounds")->payload;
}

int64_t rey_tuple_kind(int64_t tuple, int64_t index) {
    return slotAt(slotsObject(tuple, REY_TAG_TUPLE, "invalid tuple"), index, "tuple index out of bounds")->kind;
}

int64_t rey_tuple_len(int64_t tuple) {
    return slotsObject(tuple, REY_TAG_TUPLE, "invalid tuple")->count;
}

int64_t rey_tuple_eq(int64_t left, int64_t right) {
    ReySlots* a = slotsObject(left, REY_TAG_TUPLE, "invalid tuple");
    ReySlots* b = slotsObject(right, REY_TAG_TUPLE, "invalid tuple");
    if (a->count != b->count) return 0;
    for (int64_t i = 0; i < a->count; i++) {
        if (a->slots[i].kind != b->slots[i].kind) return 0;
        if (!rey_slot_eq(a->slots[i].kind, a->slots[i].payload, b->slots[i].payload)) return 0;
    }
    return 1;
}

int64_t rey_env_new(int64_t count) { return slotsNew(REY_TAG_ENV, count); }

void rey_env_set(int64_t env, int64_t index, int64_t kind, int64_t payload) {
    rey_slot_check(kind, payload);
    ReySlot* slot = slotAt(slotsObject(env, REY_TAG_ENV, "invalid closure environment"), index, "env index out of bounds");
    slot->kind = kind;
    slot->payload = payload;
}

int64_t rey_env_get(int64_t env, int64_t index) {
    return slotAt(slotsObject(env, REY_TAG_ENV, "invalid closure environment"), index, "env index out of bounds")->payload;
}

int64_t rey_env_kind(int64_t env, int64_t index) {
    return slotAt(slotsObject(env, REY_TAG_ENV, "invalid closure environment"), index, "env index out of bounds")->kind;
}

int64_t rey_env_len(int64_t env) {
    return slotsObject(env, REY_TAG_ENV, "invalid closure environment")->count;
}

int64_t rey_closure_new(int64_t fnPtr, int64_t env, int64_t arity) {
    if (!fnPtr) rey_runtime_error("closure function is null");
    if (arity < 0) rey_runtime_error("closure arity is negative");
    if (env) slotsObject(env, REY_TAG_ENV, "closure environment is invalid");
    ReyClosure* closure = (ReyClosure*)rey_alloc((int64_t)sizeof(ReyClosure));
    closure->tag = REY_TAG_CLOSURE;
    closure->fnPtr = fnPtr;
    closure->env = env;
    closure->arity = arity;
    return (int64_t)(uintptr_t)closure;
}

static ReyClosure* closureObject(int64_t closure) {
    requireTag(closure, REY_TAG_CLOSURE, "invalid closure");
    return (ReyClosure*)(uintptr_t)closure;
}

int64_t rey_closure_fn(int64_t closure) { return closureObject(closure)->fnPtr; }
int64_t rey_closure_env(int64_t closure) { return closureObject(closure)->env; }
int64_t rey_closure_arity(int64_t closure) { return closureObject(closure)->arity; }

static const char* renderEnumPayload(int64_t kind, int64_t payload) {
    switch (kind) {
        case REY_ENUM_INT: return rey_slot_to_str(REY_VALUE_INT, payload);
        case REY_ENUM_BOOL: return rey_slot_to_str(REY_VALUE_BOOL, payload);
        case REY_ENUM_FLOAT: return rey_slot_to_str(REY_VALUE_FLOAT, payload);
        case REY_ENUM_STRING:
        case REY_ENUM_CHAR: return rey_slot_to_str(REY_VALUE_STRING, payload);
        case REY_ENUM_NULL: return rey_slot_to_str(REY_VALUE_NULL, 0);
        case REY_ENUM_STRUCT: return rey_slot_to_str(REY_VALUE_STRUCT, payload);
        case REY_ENUM_VEC: return rey_slot_to_str(REY_VALUE_VEC, payload);
        case REY_ENUM_DICT: return rey_slot_to_str(REY_VALUE_DICT, payload);
        case REY_ENUM_ENUM: return rey_slot_to_str(REY_VALUE_ENUM, payload);
        case REY_ENUM_OPTION: return rey_slot_to_str(REY_VALUE_OPTION, payload);
        case REY_ENUM_RESULT: return rey_slot_to_str(REY_VALUE_RESULT, payload);
        case REY_ENUM_TUPLE: return rey_slot_to_str(REY_VALUE_TUPLE, payload);
        case REY_ENUM_FUNC: return rey_slot_to_str(REY_VALUE_FUNC, payload);
        case REY_ENUM_ANY: return rey_slot_to_str(REY_VALUE_INT, payload);
        default: rey_runtime_error("invalid enum payload kind"); return NULL;
    }
}

static char* joinItems(const char* openText, const char* closeText, int64_t count,
                       const char* (*itemAt)(int64_t index, int64_t source), int64_t source) {
    char* text = rey_str_cstr(openText);
    for (int64_t i = 0; i < count; i++) {
        if (i > 0) text = rey_str_concat(text, rey_str_cstr(", "));
        text = rey_str_concat(text, itemAt(i, source));
    }
    return rey_str_concat(text, rey_str_cstr(closeText));
}

static const char* tupleItem(int64_t index, int64_t source) {
    ReySlots* tuple = (ReySlots*)(uintptr_t)source;
    return rey_slot_to_str(tuple->slots[index].kind, tuple->slots[index].payload);
}

static const char* formatTuple(int64_t tuple) {
    ReySlots* object = slotsObject(tuple, REY_TAG_TUPLE, "invalid tuple");
    return joinItems("(", ")", object->count, tupleItem, tuple);
}

const char* rey_tuple_to_str(int64_t tuple) { return formatTuple(tuple); }

static const char* vecItem(int64_t index, int64_t source) {
    return rey_slot_to_str(rey_vec_kind(source, index), rey_vec_get(source, index));
}

const char* rey_vec_to_str(int64_t vec) {
    requireTag(vec, REY_TAG_VEC, "invalid Vec");
    return joinItems("[", "]", rey_vec_len(vec), vecItem, vec);
}

const char* rey_enum_to_str(int64_t value) {
    char* text = rey_str_cstr("enum#");
    text = rey_str_concat(text, rey_int_to_str(rey_enum_identity(value)));
    text = rey_str_concat(text, rey_str_cstr("::"));
    text = rey_str_concat(text, rey_int_to_str(rey_enum_discriminant(value)));
    int64_t count = rey_enum_payload_count(value);
    if (count == 0) return text;
    text = rey_str_concat(text, rey_str_cstr("("));
    for (int64_t i = 0; i < count; i++) {
        if (i > 0) text = rey_str_concat(text, rey_str_cstr(", "));
        text = rey_str_concat(text, renderEnumPayload(rey_enum_payload_kind(value, i), rey_enum_payload_at(value, i)));
    }
    return rey_str_concat(text, rey_str_cstr(")"));
}

const char* rey_option_to_str(int64_t option) {
    if (rey_option_tag(option) == REY_TAG_NONE) return rey_str_cstr("None");
    char* text = rey_str_cstr("Some(");
    text = rey_str_concat(text, renderEnumPayload(rey_option_payload_kind(option), rey_option_unwrap(option)));
    return rey_str_concat(text, rey_str_cstr(")"));
}

const char* rey_result_to_str(int64_t result) {
    int64_t tag = rey_result_tag(result);
    char* text = rey_str_cstr(tag == REY_TAG_OK ? "Ok(" : "Err(");
    int64_t payload = tag == REY_TAG_OK ? rey_result_unwrap(result) : rey_result_unwrap_err(result);
    text = rey_str_concat(text, renderEnumPayload(rey_result_payload_kind(result), payload));
    return rey_str_concat(text, rey_str_cstr(")"));
}

static const char* renderSlotBody(int64_t kind, int64_t payload) {
    switch (kind) {
        case REY_SLOT_RAW:
        case REY_VALUE_INT:
            return rey_int_to_str(payload);
        case REY_VALUE_BOOL:
            if (payload != 0 && payload != 1) rey_runtime_error("bool payload must be 0 or 1");
            return rey_bool_to_str(payload);
        case REY_VALUE_FLOAT:
            return rey_float_to_str(rey_bits_to_float(payload));
        case REY_VALUE_STRING:
            if (!payload) rey_runtime_error("string payload must be non-null");
            return (const char*)(uintptr_t)payload;
        case REY_VALUE_CHAR: {
            if (payload < 0 || payload > 255) rey_runtime_error("char payload must be in range 0..255");
            unsigned char byte = (unsigned char)payload;
            return rey_str_new((const char*)&byte, 1);
        }
        case REY_VALUE_NULL:
            if (payload != 0) rey_runtime_error("null payload must be 0");
            return rey_str_cstr("null");
        case REY_VALUE_STRUCT: {
            if (!payload) rey_runtime_error("struct payload must be non-null");
            char* text = rey_str_cstr("struct#");
            return rey_str_concat(text, rey_int_to_str(rey_struct_type_id(payload)));
        }
        case REY_VALUE_ENUM:
            return rey_enum_to_str(payload);
        case REY_VALUE_VEC:
            return rey_vec_to_str(payload);
        case REY_VALUE_DICT:
            return rey_hm_to_str(payload);
        case REY_VALUE_TUPLE:
            return formatTuple(payload);
        case REY_VALUE_OPTION:
            return rey_option_to_str(payload);
        case REY_VALUE_RESULT:
            return rey_result_to_str(payload);
        case REY_VALUE_FUNC:
            requireTag(payload, REY_TAG_CLOSURE, "function payload is invalid");
            return rey_str_cstr("<func>");
        case REY_VALUE_ANY:
            return rey_value_to_str(payload);
        default:
            rey_runtime_error("unsupported value kind");
            return NULL;
    }
}

const char* rey_slot_to_str(int64_t kind, int64_t payload) {
    if (renderDepth >= 32) return rey_str_cstr("...");
    renderDepth++;
    const char* text = renderSlotBody(kind, payload);
    renderDepth--;
    return text;
}

const char* rey_value_to_str(int64_t value) {
    ReyBox* box = valueBox(value);
    return rey_slot_to_str(box->kind, box->payload);
}

void* rey_struct_alloc(int64_t typeId, int64_t dataBytes) {
    if (dataBytes < 0) rey_runtime_error("struct size is negative");
    if ((uint64_t)dataBytes > (uint64_t)INT64_MAX - sizeof(ReyStructHeader))
        rey_runtime_error("struct size overflow");
    ReyStructHeader* header = (ReyStructHeader*)rey_alloc((int64_t)sizeof(ReyStructHeader) + dataBytes);
    header->tagByte = (uint8_t)typeId;
    header->typeId = typeId;
    return (char*)header + sizeof(ReyStructHeader);
}

int64_t rey_struct_type_id(int64_t dataPtr) {
    if (!dataPtr) return 0;
    int64_t typeId = 0;
    memcpy(&typeId, (char*)(uintptr_t)dataPtr - 8, sizeof(typeId));
    return typeId;
}

int64_t rey_struct_is_type(int64_t dataPtr, int64_t typeId) {
    if (!dataPtr) return 0;
    return rey_struct_type_id(dataPtr) == typeId;
}

typedef struct {
    uint8_t tag;
    uint8_t present;
    uint8_t kind;
    uint8_t pad[5];
    int64_t payload;
} ReyNullable;

_Static_assert(sizeof(ReyNullable) == 16, "scalar nullable box is 16 bytes");

enum {
    kNullableInt = 1,
    kNullableBool = 2,
    kNullableFloat = 3,
    kNullableChar = 4
};

static int nullableIsNull(int64_t value) {
    if (!value) return 1;
    if (*(uint8_t*)(uintptr_t)value != REY_TAG_NULLABLE) return 0;
    return !((ReyNullable*)(uintptr_t)value)->present;
}

static ReyNullable* nullablePresent(int64_t value) {
    if (nullableIsNull(value)) rey_runtime_error("null dereference");
    if (*(uint8_t*)(uintptr_t)value != REY_TAG_NULLABLE) rey_runtime_error("invalid nullable");
    return (ReyNullable*)(uintptr_t)value;
}

static int64_t boxScalar(uint8_t kind, int64_t payload) {
    ReyNullable* object = (ReyNullable*)rey_alloc((int64_t)sizeof(ReyNullable));
    object->tag = REY_TAG_NULLABLE;
    object->present = 1;
    object->kind = kind;
    object->payload = payload;
    return (int64_t)(uintptr_t)object;
}

static int64_t unboxScalar(int64_t value, uint8_t kind) {
    ReyNullable* object = nullablePresent(value);
    if (object->kind != kind) rey_runtime_error("nullable kind mismatch");
    return object->payload;
}

int64_t rey_nullable_null(void) {
    static ReyNullable* object;
    if (!object) {
        object = (ReyNullable*)rey_alloc((int64_t)sizeof(ReyNullable));
        object->tag = REY_TAG_NULLABLE;
    }
    return (int64_t)(uintptr_t)object;
}

int64_t rey_nullable_is_null(int64_t value) {
    return nullableIsNull(value);
}

int64_t rey_nullable_box_int(int64_t value) { return boxScalar(kNullableInt, value); }
int64_t rey_nullable_unbox_int(int64_t value) { return unboxScalar(value, kNullableInt); }

int64_t rey_nullable_box_bool(int64_t value) {
    if (value != 0 && value != 1) rey_runtime_error("bool payload must be 0 or 1");
    return boxScalar(kNullableBool, value);
}

int64_t rey_nullable_unbox_bool(int64_t value) { return unboxScalar(value, kNullableBool); }

int64_t rey_nullable_box_char(int64_t value) {
    if (value < 0 || value > 255) rey_runtime_error("char payload must be in range 0..255");
    return boxScalar(kNullableChar, value);
}

int64_t rey_nullable_unbox_char(int64_t value) { return unboxScalar(value, kNullableChar); }

int64_t rey_nullable_box_float(double value) {
    return boxScalar(kNullableFloat, rey_float_bits(value));
}

double rey_nullable_unbox_float(int64_t value) {
    return rey_bits_to_float(unboxScalar(value, kNullableFloat));
}

int64_t rey_nullable_eq(int64_t left, int64_t right) {
    int leftNull = nullableIsNull(left);
    int rightNull = nullableIsNull(right);
    if (leftNull || rightNull) return leftNull && rightNull;
    ReyNullable* a = nullablePresent(left);
    ReyNullable* b = nullablePresent(right);
    if (a->kind != b->kind) return 0;
    if (a->kind == kNullableFloat)
        return rey_bits_to_float(a->payload) == rey_bits_to_float(b->payload);
    return a->payload == b->payload;
}

const char* rey_nullable_to_str(int64_t value) {
    if (nullableIsNull(value)) return rey_str_cstr("null");
    ReyNullable* object = nullablePresent(value);
    if (object->kind == kNullableInt) return rey_int_to_str(object->payload);
    if (object->kind == kNullableBool) return rey_bool_to_str(object->payload);
    if (object->kind == kNullableFloat) return rey_float_to_str(rey_bits_to_float(object->payload));
    if (object->kind == kNullableChar) return rey_typed_val_to_str(REY_VALUE_CHAR, object->payload);
    rey_runtime_error("invalid nullable");
    return NULL;
}
