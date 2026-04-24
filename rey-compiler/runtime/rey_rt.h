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
#define REY_TAG_STRUCT 8

// ReyStr on-heap layout:
//   byte 0:   tag = REY_TAG_STR
//   bytes 1-7:  padding
//   bytes 8-15: int64_t len
//   bytes 16+:  char data[len+1]  (null-terminated)
// String functions accept/return char* pointing to data (offset 16).
// To reach the header: hdr = data - 16.
#define REY_STR_HDR 16

// --- mem ---
void* rey_alloc(int64_t size);
void  rey_panic_cstr(const char* msg);
void  rey_panic(const char* msg);  // msg is a data pointer (i8*)

// --- string (i8* ABI) ---
char* rey_str_new(const char* data, int64_t len);
char* rey_str_cstr(const char* cstr);
char* rey_str_empty(void);
int64_t rey_str_len(const char* s);
char* rey_str_concat(const char* a, const char* b);
char* rey_str_slice(const char* s, int64_t start, int64_t end);
char* rey_str_char_at(const char* s, int64_t i);
int64_t rey_str_index_of(const char* s, const char* sub);
int64_t rey_str_starts_with(const char* s, const char* prefix);
int64_t rey_str_ends_with(const char* s, const char* suffix);
char* rey_str_trim(const char* s);
char* rey_str_replace(const char* s, const char* from, const char* to);
char* rey_str_repeat(const char* s, int64_t n);
char* rey_str_to_upper(const char* s);
char* rey_str_to_lower(const char* s);
int64_t rey_str_eq(const char* a, const char* b);
int64_t rey_str_split(const char* s, const char* delim);  // returns Vec (i64)

// int/bool → string
char* rey_int_to_str(int64_t n);
// smart val→str: inspects tag byte to handle opaque i64 from Vec/HashMap
char* rey_val_to_str(int64_t val);
char* rey_float_to_str(double f);
char* rey_bool_to_str(int64_t b);

// print (i8* ABI)
void rey_print(const char* s);
void rey_println(const char* s);

// --- vec (i64 ABI) ---
int64_t rey_vec_new(void);
void    rey_vec_push(int64_t v, int64_t item);
int64_t rey_vec_pop(int64_t v);
int64_t rey_vec_len(int64_t v);
int64_t rey_vec_get(int64_t v, int64_t i);
void    rey_vec_set(int64_t v, int64_t i, int64_t val);
int64_t rey_vec_contains(int64_t v, int64_t item);  // for string items: rey_str_eq
int64_t rey_vec_str_contains(int64_t v, const char* item); // Vec<String>.contains(str)
char*   rey_vec_join(int64_t v, const char* sep);    // Vec<String>.join -> i8*
int64_t rey_vec_slice_v(int64_t v, int64_t start, int64_t end);
int64_t rey_vec_reverse(int64_t v);
int64_t rey_vec_map(int64_t v, int64_t fn_ptr);
int64_t rey_vec_filter(int64_t v, int64_t fn_ptr);

// --- hashmap (i64 ABI, string keys as i8*) ---
int64_t rey_hm_new(void);
void    rey_hm_set(int64_t hm, const char* key, int64_t val);
int64_t rey_hm_get(int64_t hm, const char* key);
int64_t rey_hm_has(int64_t hm, const char* key);
void    rey_hm_delete(int64_t hm, const char* key);
int64_t rey_hm_keys(int64_t hm);    // returns Vec<String> (items are i8*)
int64_t rey_hm_values(int64_t hm);  // returns Vec<i64>

// --- result / option (i64 ABI) ---
int64_t rey_ok(int64_t val);
int64_t rey_err(int64_t val);        // val can be string (i64 holding i8* cast)
int64_t rey_ok_str(const char* val); // convenience: wrap i8* string in Ok
int64_t rey_err_str(const char* val);
int64_t rey_result_is_ok(int64_t r);
int64_t rey_result_is_err(int64_t r);
int64_t rey_result_unwrap(int64_t r);
int64_t rey_result_unwrap_or(int64_t r, int64_t def);
// when payload is a string (i8*)
const char* rey_result_unwrap_str(int64_t r);
const char* rey_result_unwrap_or_str(int64_t r, const char* def);

int64_t rey_some(int64_t val);
int64_t rey_none(void);
int64_t rey_option_is_some(int64_t o);
int64_t rey_option_unwrap(int64_t o);

// instanceof — check tag of a Vec/Map/Result pointer
int64_t rey_instanceof_tag(int64_t ptr, int64_t tag);
// instanceof for user structs — tag is at ptr-16 (past 16-byte header)
int64_t rey_struct_instanceof(int64_t ptr, int64_t tag);

// --- io ---
char*   rey_read_file(const char* path);   // returns i8* string
void    rey_write_file(const char* path, const char* content);
void    rey_append_file(const char* path, const char* content);
int64_t rey_file_exists(const char* path);
void    rey_delete_file(const char* path);
void    rey_mkdir(const char* path);
int64_t rey_list_dir(const char* path);  // returns Vec<String> (i64)

// --- process ---
void    rey_init(int argc, char** argv);
int64_t rey_args(void);            // returns Vec<String> (items are i8*)
int64_t rey_exec_cmd(const char* cmd);  // returns Result<i8*,i8*> as i64
void    rey_exit(int64_t code);
char*   rey_get_env(const char* name);  // returns i8*
