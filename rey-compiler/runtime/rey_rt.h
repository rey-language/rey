// Rey runtime — public ABI.
// Strings:    i8*  in LLVM (pointer to data field inside ReyStr; header is 16 bytes before)
// Vec/Map:    i64  in LLVM (pointer to heap struct cast to integer)
// Result/Option: i64 in LLVM (pointer to tagged struct cast to integer)
#pragma once
#include <stdint.h>

// --- type tags (stored in byte 0 of every heap object) ---
#define REY_TAG_STR    1
#define REY_TAG_VEC    2
#define REY_TAG_MAP    3
#define REY_TAG_OK     4
#define REY_TAG_ERR    5
#define REY_TAG_SOME   6
#define REY_TAG_NONE   7
#define REY_TAG_STRUCT  8
#define REY_TAG_ENUM    9
#define REY_TAG_VALUE   10
#define REY_TAG_TUPLE   11
#define REY_TAG_CLOSURE 12
#define REY_TAG_ENV     13
#define REY_TAG_NULLABLE 14

// Keep Vec accessors link-visible across archive members but out of dynamic interposition.
#if defined(__GNUC__) || defined(__clang__)
#define REY_RUNTIME_LTO_API __attribute__((visibility("hidden")))
#define REY_RUNTIME_ALWAYS_INLINE __attribute__((always_inline))
#else
#define REY_RUNTIME_LTO_API
#define REY_RUNTIME_ALWAYS_INLINE
#endif

// ReyStr on-heap layout:
//   byte 0:   tag = REY_TAG_STR
//   bytes 1-7:  padding
//   bytes 8-15: int64_t len
//   bytes 16+:  char data[len+1]  (null-terminated)
// String functions accept/return char* pointing to data (offset 16).
// To reach the header: hdr = data - 16.
#define REY_STR_HDR 16

// --- mem ---
#if defined(__GNUC__) || defined(__clang__)
#define REY_NORETURN __attribute__((noreturn, cold))
#else
#define REY_NORETURN
#endif
void* rey_alloc(int64_t size);
REY_NORETURN void rey_panic_cstr(const char* msg);
REY_NORETURN void rey_panic(const char* msg);  // msg is a data pointer (i8*)
// New semantic failures: prints "error: <msg>" to stderr and exits 1.
REY_NORETURN void rey_runtime_error(const char* msg);

// --- string (i8* ABI) ---
char* rey_str_new(const char* data, int64_t len);
char* rey_str_cstr(const char* cstr);
char* rey_str_empty(void);
int64_t rey_str_len(const char* s);
char* rey_str_concat(const char* a, const char* b);
char* rey_str_slice(const char* s, int64_t start, int64_t end);
char* rey_str_char_at(const char* s, int64_t i);
int64_t rey_str_index_of(const char* s, const char* sub);
int64_t rey_str_contains(const char* s, const char* needle);
int64_t rey_str_starts_with(const char* s, const char* prefix);
int64_t rey_str_ends_with(const char* s, const char* suffix);
char* rey_str_trim(const char* s);
char* rey_str_replace(const char* s, const char* from, const char* to);
char* rey_str_repeat(const char* s, int64_t n);
char* rey_str_to_upper(const char* s);
char* rey_str_to_lower(const char* s);
int64_t rey_str_eq(const char* a, const char* b);
int64_t rey_str_split(const char* s, const char* delim);  // returns Vec (i64)
// slice: negative index exits 1 ("String.slice() indices must be non-negative");
// ends clamp to length; end <= start yields empty. repeat: negative count exits 1.
// replace: empty `from` inserts `to` at every byte boundary, including the ends.
int64_t rey_str_last_index_of(const char* s, const char* sub); // byte index or -1; empty needle => len
char* rey_str_trim_start(const char* s);
char* rey_str_trim_end(const char* s);
// First byte as 0..255. Empty string exits 1: "hash() called on empty string".
int64_t rey_str_hash(const char* s);
// Width is a byte count. Pad text must be nonempty; its first byte is repeated.
char* rey_str_pad_left(const char* s, int64_t width, const char* pad);
char* rey_str_pad_right(const char* s, int64_t width, const char* pad);

// int/bool → string
char* rey_int_to_str(int64_t n);
// SPEC §9.1 conversions. toString for scalars is rey_int_to_str / rey_float_to_str /
// rey_bool_to_str / rey_typed_val_to_str. Parsing accepts ASCII decimal text:
// optional leading + or -, digits, underscores only between digits, and one
// optional fraction. No exponent and no whitespace. string.toInt truncates
// toward zero. Malformed text, a non-finite parse, and an out-of-range
// float-to-int exit 1 with "error: ...".
int64_t rey_str_to_int(const char* text);
double  rey_str_to_float(const char* text);
int64_t rey_float_to_int(double value);
double  rey_int_to_float(int64_t value);
// char(int) -> one-byte char. The returned i8* is Rey string data, so the
// first byte is the value and toString can use the pointer directly.
// Out of range 0..255 exits 1. int(char) reads that first byte.
const char* rey_char_from_int(int64_t value);
int64_t rey_int_from_char(const char* value);
// Legacy untagged ABI; a registered string pointer can collide with an integer.
char* rey_val_to_str(int64_t val);
// Rust 1.98 f64 Debug, then drop a trailing ".0" that has no exponent.
// Non-finite text is "inf", "-inf", or "NaN". Decimal point is always '.'.
char* rey_float_to_str(double f);
char* rey_bool_to_str(int64_t b);

// Explicit scalar kinds; these values are separate from REY_TAG_* object tags.
#define REY_VALUE_INT    0x1001
#define REY_VALUE_BOOL   0x1002
#define REY_VALUE_FLOAT  0x1003
#define REY_VALUE_STRING 0x1004
#define REY_VALUE_CHAR   0x1005
#define REY_VALUE_NULL   0x1006
#define REY_VALUE_STRUCT 0x1007
#define REY_VALUE_ENUM   0x1008
#define REY_VALUE_VEC    0x1009
#define REY_VALUE_DICT   0x100A
#define REY_VALUE_TUPLE  0x100B
#define REY_VALUE_OPTION 0x100C
#define REY_VALUE_RESULT 0x100D
#define REY_VALUE_FUNC   0x100E
#define REY_VALUE_ANY    0x100F
// Untyped Vec/HashMap slots store raw i64 bits and use this kind, not a REY_VALUE_* kind.
#define REY_SLOT_RAW     0
// FLOAT payload is the raw binary64 bit pattern; STRING payload is a valid ReyStr data pointer.
// CHAR payload is a byte 0..255. REY_ENUM_CHAR is different: that payload is a Rey string pointer.
const char* rey_typed_val_to_str(int64_t kind, int64_t payload);

// print (i8* ABI)
void rey_print(const char* s);
void rey_println(const char* s);
void rey_eprint(const char* s);
void rey_eprintln(const char* s);

// --- vec (i64 ABI) ---
// Iteration is index order: for i in 0 .. rey_vec_len(v)-1, rey_vec_get(v, i).
// Typed for-in uses the same loop. Float elements are rey_vec_get_float;
// every other element type is rey_vec_get_typed(v, i, REY_VALUE_*).
// rey_vec_get and rey_vec_set reject out-of-range indexes (exit 1). That is the
// indexed-write and for-in lowering; there is no separate iterator object.
// Float Vec literals and push/set of doubles use rey_vec_push_float / rey_vec_set_float
// (binary64 bits, kind REY_VALUE_FLOAT). Do not pass a raw double to rey_vec_push.
int64_t rey_vec_new(void);
void    rey_vec_push(int64_t v, int64_t item);
int64_t rey_vec_pop(int64_t v);
REY_RUNTIME_LTO_API int64_t rey_vec_len(int64_t v);
REY_RUNTIME_LTO_API int64_t rey_vec_get(int64_t v, int64_t i);
REY_RUNTIME_LTO_API void    rey_vec_set(int64_t v, int64_t i, int64_t val);
REY_RUNTIME_LTO_API int64_t rey_vec_get_raw(int64_t v, int64_t i);
REY_RUNTIME_LTO_API void    rey_vec_set_raw(int64_t v, int64_t i, int64_t val);
REY_RUNTIME_LTO_API int64_t rey_vec_get_unchecked(int64_t v, int64_t i);
REY_RUNTIME_LTO_API void    rey_vec_set_unchecked(int64_t v, int64_t i, int64_t val);
int64_t rey_vec_contains(int64_t v, int64_t item);  // for string items: rey_str_eq
int64_t rey_vec_str_contains(int64_t v, const char* item); // Vec<String>.contains(str)
char*   rey_vec_join(int64_t v, const char* sep);    // Vec<String>.join -> i8*
int64_t rey_vec_slice_v(int64_t v, int64_t start, int64_t end); // negative index exits 1
int64_t rey_vec_reverse(int64_t v); // new Vec, original unchanged
void    rey_vec_reverse_mut(int64_t v); // in-place; source reverse() returns void
int64_t rey_vec_index_of(int64_t v, int64_t item); // raw equality, -1 if absent
int64_t rey_vec_str_index_of(int64_t v, const char* item);
int64_t rey_vec_index_of_typed(int64_t v, int64_t kind, int64_t payload);
void    rey_vec_insert(int64_t v, int64_t index, int64_t item); // index == len appends
void    rey_vec_insert_typed(int64_t v, int64_t index, int64_t kind, int64_t payload);
int64_t rey_vec_remove(int64_t v, int64_t index); // returns the removed slot
void    rey_vec_clear(int64_t v);
// In-place stable sort. Homogeneous raw/int, float bits, string, or bool.
// Mixed or unsupported kinds exit 1 with the interpreter's Vec.sort() text.
void    rey_vec_sort(int64_t v);
// fnPtr is i64 (i64 env, i64 acc, i64 item). Empty vec returns initial.
int64_t rey_vec_reduce(int64_t v, int64_t fnPtr, int64_t env, int64_t initial);
int64_t rey_vec_map(int64_t v, int64_t fn_ptr);
int64_t rey_vec_filter(int64_t v, int64_t fn_ptr);
// Typed slots keep a REY_VALUE_* / REY_SLOT_RAW kind beside each payload.
// rey_vec_push/set record REY_SLOT_RAW. Float helpers store binary64 bits.
void    rey_vec_push_typed(int64_t v, int64_t kind, int64_t payload);
void    rey_vec_set_typed(int64_t v, int64_t index, int64_t kind, int64_t payload);
int64_t rey_vec_get_typed(int64_t v, int64_t index, int64_t kind);
int64_t rey_vec_kind(int64_t v, int64_t index);
int64_t rey_vec_contains_typed(int64_t v, int64_t kind, int64_t payload);
void    rey_vec_push_float(int64_t v, double value);
void    rey_vec_set_float(int64_t v, int64_t index, double value);
double  rey_vec_get_float(int64_t v, int64_t index);
const char* rey_vec_to_str(int64_t v);
// Variadic extras and default-filled argument lists are a Vec of `count` slots.
// Build it, then rey_pack_set / rey_pack_set_float at each index. count 0 is the
// empty tail. Defaults are evaluated by codegen and stored here; the runtime
// does not know default expressions.
int64_t rey_pack_vec(int64_t count);
void    rey_pack_set(int64_t vec, int64_t index, int64_t kind, int64_t payload);
void    rey_pack_set_float(int64_t vec, int64_t index, double value);

// --- hashmap (i64 ABI, string keys as i8*) ---
// Keys are Rey string data pointers, compared by bytes (embedded NUL is significant).
// rey_hm_get returns 0 when the key is absent (ambiguous with a stored 0).
// Indexed `map[key]` is rey_hm_get_checked, rey_hm_get_typed, or rey_hm_get_float:
// a missing key is a runtime error. Dictionary literals are rey_hm_new plus
// rey_hm_set / rey_hm_set_typed / rey_hm_set_float. Float values use the float
// helpers, not a raw double bit-cast through rey_hm_set. len is rey_hm_len.
int64_t rey_hm_new(void);
void    rey_hm_set(int64_t hm, const char* key, int64_t val);
int64_t rey_hm_get(int64_t hm, const char* key);
int64_t rey_hm_has(int64_t hm, const char* key);
void    rey_hm_delete(int64_t hm, const char* key);
int64_t rey_hm_keys(int64_t hm);    // returns Vec<String> (items are i8*)
int64_t rey_hm_values(int64_t hm);  // returns Vec<i64>
void    rey_hm_clear(int64_t hm);
int64_t rey_hm_len(int64_t hm);
int64_t rey_hm_get_checked(int64_t hm, const char* key);
void    rey_hm_set_typed(int64_t hm, const char* key, int64_t kind, int64_t val);
int64_t rey_hm_get_typed(int64_t hm, const char* key, int64_t kind);
int64_t rey_hm_value_kind(int64_t hm, const char* key);
// Vec of 2-element Vecs in the same order as keys()/values().
// Slot 0 is REY_VALUE_STRING. Untyped values are reported as REY_VALUE_INT.
int64_t rey_hm_entries(int64_t hm);
void    rey_hm_set_float(int64_t hm, const char* key, double value);
double  rey_hm_get_float(int64_t hm, const char* key);
const char* rey_hm_to_str(int64_t hm);

// --- result / option (i64 ABI) ---
int64_t rey_ok(int64_t val);
int64_t rey_err(int64_t val);        // val can be string (i64 holding i8* cast)
int64_t rey_ok_typed(int64_t val, int64_t kind);
int64_t rey_err_typed(int64_t val, int64_t kind);
int64_t rey_ok_str(const char* val); // convenience: wrap i8* string in Ok
int64_t rey_err_str(const char* val);
int64_t rey_result_is_ok(int64_t r);
int64_t rey_result_is_err(int64_t r);
int64_t rey_result_unwrap(int64_t r);
int64_t rey_result_unwrap_err(int64_t r); // requires a valid nonzero Result; panics unless Err
int64_t rey_result_unwrap_or(int64_t r, int64_t def);
// when payload is a string (i8*)
const char* rey_result_unwrap_str(int64_t r);
const char* rey_result_unwrap_or_str(int64_t r, const char* def);

int64_t rey_some(int64_t val);
int64_t rey_some_typed(int64_t val, int64_t kind);
int64_t rey_none(void);
int64_t rey_option_is_some(int64_t o);
int64_t rey_option_unwrap(int64_t o);
int64_t rey_option_eq(int64_t left, int64_t right);
int64_t rey_result_eq(int64_t left, int64_t right);
int64_t rey_option_tag(int64_t option);
int64_t rey_option_payload_kind(int64_t option);
int64_t rey_result_tag(int64_t result);
int64_t rey_result_payload_kind(int64_t result);
const char* rey_option_to_str(int64_t option);
const char* rey_result_to_str(int64_t result);

// --- user enum (i64 ABI) ---
// Layout: { tag, enum identity, discriminant, slot count, typed payload slots }.
// Enum identities are deterministic declaration indices within one compiled unit.
#define REY_ENUM_INT    1
#define REY_ENUM_BOOL   2
#define REY_ENUM_FLOAT  3
#define REY_ENUM_STRING 4
#define REY_ENUM_CHAR   5
#define REY_ENUM_NULL   6
#define REY_ENUM_STRUCT 7
#define REY_ENUM_VEC    8
#define REY_ENUM_DICT   9
#define REY_ENUM_ENUM   10
#define REY_ENUM_OPTION 11
#define REY_ENUM_RESULT 12
#define REY_ENUM_TUPLE  13
#define REY_ENUM_FUNC   14
#define REY_ENUM_ANY    0
int64_t rey_enum_new(int64_t enum_id, int64_t discriminant, int64_t payload_count);
void    rey_enum_set(int64_t value, int64_t index, int64_t kind, int64_t payload);
int64_t rey_enum_is(int64_t value, int64_t enum_id, int64_t discriminant);
int64_t rey_enum_get(int64_t value, int64_t enum_id, int64_t discriminant, int64_t index, int64_t kind);
int64_t rey_enum_eq(int64_t left, int64_t right, int64_t enum_id);
int64_t rey_value_eq_typed(int64_t kind, int64_t left, int64_t right);
// instanceof for a nominal enum ignores the discriminant. A non-enum returns 0.
int64_t rey_enum_identity(int64_t value);
int64_t rey_enum_discriminant(int64_t value);
int64_t rey_enum_is_type(int64_t value, int64_t enum_id);
int64_t rey_enum_payload_count(int64_t value);
int64_t rey_enum_payload_kind(int64_t value, int64_t index);
int64_t rey_enum_payload_at(int64_t value, int64_t index);
int64_t rey_enum_eq_values(int64_t left, int64_t right);
const char* rey_enum_to_str(int64_t value);

// instanceof — check tag of a Vec/Map/Result pointer
int64_t rey_instanceof_tag(int64_t ptr, int64_t tag);
// instanceof for user structs — tag byte is at ptr-16 (past 16-byte header)
int64_t rey_struct_instanceof(int64_t ptr, int64_t tag);
// rey_struct_alloc returns the data pointer (header + 16). Byte 0 of the header
// stores (uint8_t)type_id so rey_struct_instanceof keeps working for ids 0..255.
// The full int64 type id is stored at data_ptr-8. data_bytes is the payload size.
void*   rey_struct_alloc(int64_t type_id, int64_t data_bytes);
int64_t rey_struct_type_id(int64_t data_ptr);
int64_t rey_struct_is_type(int64_t data_ptr, int64_t type_id);

// --- tagged dynamic values (nullable / union / any) ---
// Heap box {tag=REY_TAG_VALUE, kind, payload, nominal}. kind is a REY_VALUE_*.
// nominal is the struct or enum identity for REY_VALUE_STRUCT / REY_VALUE_ENUM
// and 0 for every other kind. A null literal is a box from rey_value_null(),
// not the integer 0. Equality follows SPEC: scalars and strings by value, enums
// and tuples structurally, structs/Vecs/dicts by reference. Function values are
// not comparable and rey_value_eq / rey_slot_eq exit 1 on REY_VALUE_FUNC.
// to-string borrows STRING payloads and allocates every other rendering.
// Forms: null; true/false; decimal ints; rey_float_to_str; one char byte;
// (a, b); [a, b]; {k: v, ...} in keys() order; Some(v)/None; Ok(v)/Err(v);
// enum#<id>::<disc> or enum#<id>::<disc>(...); struct#<type id>; <func>.
int64_t rey_value_new(int64_t kind, int64_t payload);
int64_t rey_value_new_nominal(int64_t kind, int64_t payload, int64_t nominal);
int64_t rey_value_null(void);
int64_t rey_value_kind(int64_t value);
int64_t rey_value_payload(int64_t value);
int64_t rey_value_nominal(int64_t value);
int64_t rey_value_is(int64_t value, int64_t kind);
int64_t rey_value_is_nominal(int64_t value, int64_t kind, int64_t nominal);
// instanceof on a nullable/union box. Returns 0 when value is null, not a box,
// or a different kind. queryKind is REY_VALUE_*. nominal is the struct/enum id
// for REY_VALUE_STRUCT / REY_VALUE_ENUM and must be 0 for every other kind
// (a nonzero nominal then returns 0). Unboxed structs use rey_struct_is_type.
// Unboxed enums use rey_enum_is_type. Both of those also return 0 on a mismatch.
int64_t rey_instanceof(int64_t value, int64_t queryKind, int64_t nominal);
int64_t rey_value_unbox(int64_t value, int64_t kind);
int64_t rey_value_unbox_nominal(int64_t value, int64_t kind, int64_t nominal);
int64_t rey_value_eq(int64_t left, int64_t right);
const char* rey_value_to_str(int64_t value);
int64_t rey_slot_eq(int64_t kind, int64_t left, int64_t right);
const char* rey_slot_to_str(int64_t kind, int64_t payload);
// Rejects an unknown kind or a payload that cannot represent that kind.
void    rey_slot_check(int64_t kind, int64_t payload);
int64_t rey_float_bits(double value);
double  rey_bits_to_float(int64_t bits);

// --- tuples ---
// Heap object, N slots of {REY_VALUE_* kind, payload}. Fresh slots are raw 0.
// Elements compare and render structurally. One-element text is "(v)", not "(v,)".
int64_t rey_tuple_new(int64_t count);
void    rey_tuple_set(int64_t tuple, int64_t index, int64_t kind, int64_t payload);
int64_t rey_tuple_get(int64_t tuple, int64_t index);
int64_t rey_tuple_kind(int64_t tuple, int64_t index);
int64_t rey_tuple_len(int64_t tuple);
int64_t rey_tuple_eq(int64_t left, int64_t right);
const char* rey_tuple_to_str(int64_t tuple);

// --- closures ---
// Calling convention: the lifted function is
//   <return> fn(int64_t env, <user arguments...>)
// env is always the first argument, even when the capture count is 0 (env may be 0).
// A closure value is {fn_ptr, env, arity}. Call sites load rey_closure_fn and
// rey_closure_env and call the function pointer. Do not pass the closure object
// to rey_vec_map or rey_vec_filter; those still take a raw int64_t (*)(int64_t).
// Captured slots use the same kinds as tuple slots. Scalars are snapshots;
// aggregate payloads are shared references stored by the caller.
int64_t rey_env_new(int64_t count);
void    rey_env_set(int64_t env, int64_t index, int64_t kind, int64_t payload);
int64_t rey_env_get(int64_t env, int64_t index);
int64_t rey_env_kind(int64_t env, int64_t index);
int64_t rey_env_len(int64_t env);
int64_t rey_closure_new(int64_t fn_ptr, int64_t env, int64_t arity);
int64_t rey_closure_fn(int64_t closure);
int64_t rey_closure_env(int64_t closure);
int64_t rey_closure_arity(int64_t closure);

// Scalar nullables (int?, bool?, char?, float?). Pointer nullables (string?,
// struct?, Vec?, ...) stay raw pointers: null is integer 0 and these helpers
// are not used. A scalar null is either 0 or the shared rey_nullable_null()
// box. A present value is a 16-byte heap tag plus the payload, so int 0 and
// char 0 are not null. Unboxing null exits 1. Equality treats every null as
// equal only to null. toString of null is "null".
int64_t rey_nullable_null(void);
int64_t rey_nullable_is_null(int64_t value);
int64_t rey_nullable_box_int(int64_t value);
int64_t rey_nullable_unbox_int(int64_t value);
int64_t rey_nullable_box_bool(int64_t value);
int64_t rey_nullable_unbox_bool(int64_t value);
int64_t rey_nullable_box_char(int64_t value);
int64_t rey_nullable_unbox_char(int64_t value);
int64_t rey_nullable_box_float(double value);
double  rey_nullable_unbox_float(int64_t value);
int64_t rey_nullable_eq(int64_t left, int64_t right);
const char* rey_nullable_to_str(int64_t value);

// --- io ---
char*   rey_read_file(const char* path);   // i8*; missing path, directory, or short read exits 1
void    rey_write_file(const char* path, const char* content);
void    rey_append_file(const char* path, const char* content);
int64_t rey_file_exists(const char* path);
void    rey_delete_file(const char* path);
void    rey_mkdir(const char* path);
int64_t rey_list_dir(const char* path);  // Vec<String>. A missing directory is an empty Vec.

// --- process ---
void    rey_init(int argc, char** argv);
int64_t rey_args(void);            // returns Vec<String> (items are i8*)
int64_t rey_exec_cmd(const char* cmd);  // returns Result<i8*,i8*> as i64
void    rey_exit(int64_t code);
char*   rey_get_env(const char* name);  // returns i8*

// --- numeric builtins ---
// int abs/pow exit 1 on overflow ("Integer overflow in abs" / "Integer overflow in pow").
// INT64_MIN abs overflows. pow int rejects a negative exponent with the same overflow text.
// 0^0 is 1. floor/ceil/round return int; out of int range uses "Float value out of int range".
// round is half away from zero. sqrt/log/trig/random return float.
// sqrt(negative) => "sqrt expects non-negative input". log(non-positive) => "log expects positive input".
int64_t rey_abs_int(int64_t value);
double  rey_abs_float(double value);
int64_t rey_min_int(int64_t a, int64_t b);
int64_t rey_max_int(int64_t a, int64_t b);
double  rey_min_float(double a, double b);
double  rey_max_float(double a, double b);
int64_t rey_pow_int(int64_t base, int64_t exp);
double  rey_pow_float(double base, double exp);
int64_t rey_floor(double value);
int64_t rey_ceil(double value);
int64_t rey_round(double value);
double  rey_sqrt(double value);
double  rey_log(double value);
double  rey_sin(double value);
double  rey_cos(double value);
double  rey_tan(double value);
double  rey_random(void);

// bitwise & wrapping math
int64_t rey_bit_xor(int64_t a, int64_t b);
int64_t rey_bit_and(int64_t a, int64_t b);
int64_t rey_bit_or(int64_t a, int64_t b);
int64_t rey_bit_shl(int64_t a, int64_t b);
int64_t rey_bit_shr(int64_t a, int64_t b);
int64_t rey_bit_not(int64_t a);
int64_t rey_wrap_add(int64_t a, int64_t b);
int64_t rey_wrap_mul(int64_t a, int64_t b);
int64_t rey_wrap_sub(int64_t a, int64_t b);
