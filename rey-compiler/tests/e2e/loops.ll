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

define void @_rey_main() {
entry:
%i.addr = alloca i64, align 8
%j.addr = alloca i64, align 8
%x.addr = alloca i64, align 8
store i64 0, i64* %i.addr
br label %while.cond0
while.cond0:
%t3 = load i64, i64* %i.addr
%t4 = icmp slt i64 %t3, 3
br i1 %t4, label %while.body1, label %while.end2
while.body1:
%t5 = load i64, i64* %i.addr
%t6 = call i8* @rey_int_to_str(i64 %t5)
call void @rey_println(i8* %t6)
%t7 = load i64, i64* %i.addr
%t8 = add i64 %t7, 1
store i64 %t8, i64* %i.addr
br label %while.cond0
while.end2:
store i64 0, i64* %j.addr
br label %loop.body9
loop.body9:
%t11 = load i64, i64* %j.addr
%t12 = add i64 %t11, 1
store i64 %t12, i64* %j.addr
%t13 = load i64, i64* %j.addr
%t14 = icmp eq i64 %t13, 2
br i1 %t14, label %then15, label %endif17
then15:
br label %loop.body9
endif17:
%t18 = load i64, i64* %j.addr
%t19 = call i8* @rey_int_to_str(i64 %t18)
call void @rey_println(i8* %t19)
%t20 = load i64, i64* %j.addr
%t21 = icmp eq i64 %t20, 3
br i1 %t21, label %then22, label %endif24
then22:
br label %loop.end10
endif24:
br label %loop.body9
loop.end10:
store i64 10, i64* %x.addr
%t25 = load i64, i64* %x.addr
%t26 = call i8* @rey_int_to_str(i64 %t25)
call void @rey_println(i8* %t26)
store i64 20, i64* %x.addr
%t27 = load i64, i64* %x.addr
%t28 = call i8* @rey_int_to_str(i64 %t27)
call void @rey_println(i8* %t28)
store i64 30, i64* %x.addr
%t29 = load i64, i64* %x.addr
%t30 = call i8* @rey_int_to_str(i64 %t29)
call void @rey_println(i8* %t30)
ret void
}

define i32 @main(i32 %argc, i8** %argv) {
entry:
  call void @rey_init(i32 %argc, i8** %argv)
  call void @_rey_main()
  ret i32 0
}
