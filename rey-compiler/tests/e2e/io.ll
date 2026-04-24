; Module: rey
target datalayout = "e-m:e-i64:64-f80:128-n8:16:32:64-S128"
target triple = "aarch64-apple-darwin"

declare i32 @printf(i8*, ...)
declare i32 @strcmp(i8*, i8*)
declare i8*  @rey_str_new(i8*, i64)
declare i8*  @rey_str_cstr(i8*)
declare i8*  @rey_str_empty()
declare i64  @rey_str_len(i8*)
declare i8*  @rey_str_concat(i8*, i8*)
declare i8*  @rey_str_slice(i8*, i64, i64)
declare i8*  @rey_str_char_at(i8*, i64)
declare i64  @rey_str_index_of(i8*, i8*)
declare i64  @rey_str_starts_with(i8*, i8*)
declare i64  @rey_str_ends_with(i8*, i8*)
declare i8*  @rey_str_trim(i8*)
declare i8*  @rey_str_replace(i8*, i8*, i8*)
declare i8*  @rey_str_repeat(i8*, i64)
declare i8*  @rey_str_to_upper(i8*)
declare i8*  @rey_str_to_lower(i8*)
declare i64  @rey_str_eq(i8*, i8*)
declare i64  @rey_str_split(i8*, i8*)
declare i8*  @rey_int_to_str(i64)
declare i8*  @rey_val_to_str(i64)
declare i8*  @rey_float_to_str(double)
declare i8*  @rey_bool_to_str(i64)
declare void @rey_print(i8*)
declare void @rey_println(i8*)
declare i64  @rey_vec_new()
declare void @rey_vec_push(i64, i64)
declare i64  @rey_vec_pop(i64)
declare i64  @rey_vec_len(i64)
declare i64  @rey_vec_get(i64, i64)
declare void @rey_vec_set(i64, i64, i64)
declare i64  @rey_vec_contains(i64, i64)
declare i64  @rey_vec_str_contains(i64, i8*)
declare i8*  @rey_vec_join(i64, i8*)
declare i64  @rey_vec_slice_v(i64, i64, i64)
declare i64  @rey_vec_reverse(i64)
declare i64  @rey_vec_map(i64, i64)
declare i64  @rey_vec_filter(i64, i64)
declare i64  @rey_hm_new()
declare void @rey_hm_set(i64, i8*, i64)
declare i64  @rey_hm_get(i64, i8*)
declare i64  @rey_hm_has(i64, i8*)
declare void @rey_hm_delete(i64, i8*)
declare i64  @rey_hm_keys(i64)
declare i64  @rey_hm_values(i64)
declare i64  @rey_ok(i64)
declare i64  @rey_err(i64)
declare i64  @rey_ok_str(i8*)
declare i64  @rey_err_str(i8*)
declare i64  @rey_result_is_ok(i64)
declare i64  @rey_result_is_err(i64)
declare i64  @rey_result_unwrap(i64)
declare i64  @rey_result_unwrap_or(i64, i64)
declare i8*  @rey_result_unwrap_str(i64)
declare i8*  @rey_result_unwrap_or_str(i64, i8*)
declare i64  @rey_some(i64)
declare i64  @rey_none()
declare i64  @rey_option_is_some(i64)
declare i64  @rey_option_unwrap(i64)
declare i64  @rey_instanceof_tag(i64, i64)
declare i64  @rey_struct_instanceof(i64, i64)
declare i8*  @rey_read_file(i8*)
declare void @rey_write_file(i8*, i8*)
declare void @rey_append_file(i8*, i8*)
declare i64  @rey_file_exists(i8*)
declare void @rey_delete_file(i8*)
declare void @rey_mkdir(i8*)
declare i64  @rey_list_dir(i8*)
declare i64  @rey_exec_cmd(i8*)
declare void @rey_exit(i64)
declare i8*  @rey_get_env(i8*)
declare void @rey_init(i32, i8**)
declare i64  @rey_args()
declare void @rey_panic(i8*)
declare i8*  @rey_alloc(i64)

@.str0 = private unnamed_addr constant [21 x i8] c"/tmp/rey_io_test.txt\00", align 1
@.str1 = private unnamed_addr constant [16 x i8] c"hello from rey\0A\00", align 1
@.str2 = private unnamed_addr constant [13 x i8] c"overwritten\0A\00", align 1

define void @_rey_main() {
entry:
%path.addr = alloca i8*, align 8
%content.addr = alloca i8*, align 8
%t0 = call i8* @rey_str_new(i8* getelementptr inbounds ([21 x i8], [21 x i8]* @.str0, i64 0, i64 0), i64 20)
store i8* %t0, i8** %path.addr
%t1 = load i8*, i8** %path.addr
%t2 = call i8* @rey_str_new(i8* getelementptr inbounds ([16 x i8], [16 x i8]* @.str1, i64 0, i64 0), i64 15)
call void @rey_write_file(i8* %t1, i8* %t2)
%t3 = load i8*, i8** %path.addr
%t4 = call i64 @rey_file_exists(i8* %t3)
%t5 = call i8* @rey_val_to_str(i64 %t4)
call void @rey_println(i8* %t5)
%t6 = load i8*, i8** %path.addr
%t7 = call i8* @rey_read_file(i8* %t6)
store i8* %t7, i8** %content.addr
%t8 = load i8*, i8** %content.addr
%t9 = call i8* @rey_str_trim(i8* %t8)
call void @rey_println(i8* %t9)
%t10 = load i8*, i8** %path.addr
%t11 = call i8* @rey_str_new(i8* getelementptr inbounds ([13 x i8], [13 x i8]* @.str2, i64 0, i64 0), i64 12)
call void @rey_write_file(i8* %t10, i8* %t11)
%t12 = load i8*, i8** %path.addr
%t13 = call i8* @rey_read_file(i8* %t12)
%t14 = call i8* @rey_str_trim(i8* %t13)
call void @rey_println(i8* %t14)
%t15 = load i8*, i8** %path.addr
call void @rey_delete_file(i8* %t15)
%t16 = load i8*, i8** %path.addr
%t17 = call i64 @rey_file_exists(i8* %t16)
%t18 = call i8* @rey_val_to_str(i64 %t17)
call void @rey_println(i8* %t18)
ret void
}

define i32 @main(i32 %argc, i8** %argv) {
entry:
  call void @rey_init(i32 %argc, i8** %argv)
  call void @_rey_main()
  ret i32 0
}
