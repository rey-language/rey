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

@.str0 = private unnamed_addr constant [4 x i8] c"foo\00", align 1
@.str1 = private unnamed_addr constant [4 x i8] c"bar\00", align 1
@.str2 = private unnamed_addr constant [4 x i8] c"baz\00", align 1
@.str3 = private unnamed_addr constant [3 x i8] c", \00", align 1
@.str4 = private unnamed_addr constant [4 x i8] c"qux\00", align 1

define void @_rey_main() {
entry:
%v.addr = alloca i64, align 8
%sv.addr = alloca i64, align 8
%arr.addr = alloca i64, align 8
%t0 = call i64 @rey_vec_new()
store i64 %t0, i64* %v.addr
%t1 = load i64, i64* %v.addr
call void @rey_vec_push(i64 %t1, i64 10)
%t2 = add i64 0, 0
%t3 = load i64, i64* %v.addr
call void @rey_vec_push(i64 %t3, i64 20)
%t4 = add i64 0, 0
%t5 = load i64, i64* %v.addr
call void @rey_vec_push(i64 %t5, i64 30)
%t6 = add i64 0, 0
%t7 = load i64, i64* %v.addr
%t8 = call i64 @rey_vec_len(i64 %t7)
%t9 = call i8* @rey_val_to_str(i64 %t8)
call void @rey_println(i8* %t9)
%t10 = load i64, i64* %v.addr
%t11 = call i64 @rey_vec_get(i64 %t10, i64 0)
%t12 = call i8* @rey_val_to_str(i64 %t11)
call void @rey_println(i8* %t12)
%t13 = load i64, i64* %v.addr
%t14 = call i64 @rey_vec_get(i64 %t13, i64 2)
%t15 = call i8* @rey_val_to_str(i64 %t14)
call void @rey_println(i8* %t15)
%t16 = load i64, i64* %v.addr
%t17 = call i64 @rey_vec_pop(i64 %t16)
%t18 = call i8* @rey_val_to_str(i64 %t17)
call void @rey_println(i8* %t18)
%t19 = load i64, i64* %v.addr
%t20 = call i64 @rey_vec_len(i64 %t19)
%t21 = call i8* @rey_val_to_str(i64 %t20)
call void @rey_println(i8* %t21)
%t22 = load i64, i64* %v.addr
call void @rey_vec_set(i64 %t22, i64 0, i64 99)
%t23 = add i64 0, 0
%t24 = load i64, i64* %v.addr
%t25 = call i64 @rey_vec_get(i64 %t24, i64 0)
%t26 = call i8* @rey_val_to_str(i64 %t25)
call void @rey_println(i8* %t26)
%t27 = call i64 @rey_vec_new()
store i64 %t27, i64* %sv.addr
%t28 = load i64, i64* %sv.addr
%t29 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str0, i64 0, i64 0), i64 3)
%t30 = ptrtoint i8* %t29 to i64
call void @rey_vec_push(i64 %t28, i64 %t30)
%t31 = add i64 0, 0
%t32 = load i64, i64* %sv.addr
%t33 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str1, i64 0, i64 0), i64 3)
%t34 = ptrtoint i8* %t33 to i64
call void @rey_vec_push(i64 %t32, i64 %t34)
%t35 = add i64 0, 0
%t36 = load i64, i64* %sv.addr
%t37 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str2, i64 0, i64 0), i64 3)
%t38 = ptrtoint i8* %t37 to i64
call void @rey_vec_push(i64 %t36, i64 %t38)
%t39 = add i64 0, 0
%t40 = load i64, i64* %sv.addr
%t41 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str3, i64 0, i64 0), i64 2)
%t42 = call i8* @rey_vec_join(i64 %t40, i8* %t41)
call void @rey_println(i8* %t42)
%t43 = load i64, i64* %sv.addr
%t44 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str1, i64 0, i64 0), i64 3)
%t45 = call i64 @rey_vec_str_contains(i64 %t43, i8* %t44)
%t46 = call i8* @rey_val_to_str(i64 %t45)
call void @rey_println(i8* %t46)
%t47 = load i64, i64* %sv.addr
%t48 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str4, i64 0, i64 0), i64 3)
%t49 = call i64 @rey_vec_str_contains(i64 %t47, i8* %t48)
%t50 = call i8* @rey_val_to_str(i64 %t49)
call void @rey_println(i8* %t50)
%t51 = load i64, i64* %sv.addr
%t52 = call i64 @rey_vec_len(i64 %t51)
%t53 = call i8* @rey_val_to_str(i64 %t52)
call void @rey_println(i8* %t53)
%t54 = call i64 @rey_vec_new()
call void @rey_vec_push(i64 %t54, i64 1)
call void @rey_vec_push(i64 %t54, i64 2)
call void @rey_vec_push(i64 %t54, i64 3)
call void @rey_vec_push(i64 %t54, i64 4)
call void @rey_vec_push(i64 %t54, i64 5)
store i64 %t54, i64* %arr.addr
%t55 = load i64, i64* %arr.addr
%t56 = call i64 @rey_vec_len(i64 %t55)
%t57 = call i8* @rey_val_to_str(i64 %t56)
call void @rey_println(i8* %t57)
%t58 = load i64, i64* %arr.addr
%t59 = call i64 @rey_vec_get(i64 %t58, i64 4)
%t60 = call i8* @rey_val_to_str(i64 %t59)
call void @rey_println(i8* %t60)
ret void
}

define i32 @main(i32 %argc, i8** %argv) {
entry:
  call void @rey_init(i32 %argc, i8** %argv)
  call void @_rey_main()
  ret i32 0
}
