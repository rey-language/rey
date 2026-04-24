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

@.str0 = private unnamed_addr constant [5 x i8] c"name\00", align 1
@.str1 = private unnamed_addr constant [6 x i8] c"Alice\00", align 1
@.str2 = private unnamed_addr constant [5 x i8] c"lang\00", align 1
@.str3 = private unnamed_addr constant [4 x i8] c"Rey\00", align 1
@.str4 = private unnamed_addr constant [8 x i8] c"missing\00", align 1
@.str5 = private unnamed_addr constant [4 x i8] c"Bob\00", align 1
@.str6 = private unnamed_addr constant [5 x i8] c"oops\00", align 1

define void @_rey_main() {
entry:
%hm.addr = alloca i64, align 8
%name.addr = alloca i64, align 8
%ok.addr = alloca i64, align 8
%err.addr = alloca i64, align 8
%v.addr = alloca i64, align 8
%t0 = call i64 @rey_hm_new()
store i64 %t0, i64* %hm.addr
%t1 = load i64, i64* %hm.addr
%t2 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str0, i64 0, i64 0), i64 4)
%t3 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str1, i64 0, i64 0), i64 5)
%t4 = ptrtoint i8* %t3 to i64
call void @rey_hm_set(i64 %t1, i8* %t2, i64 %t4)
%t5 = add i64 0, 0
%t6 = load i64, i64* %hm.addr
%t7 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str2, i64 0, i64 0), i64 4)
%t8 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str3, i64 0, i64 0), i64 3)
%t9 = ptrtoint i8* %t8 to i64
call void @rey_hm_set(i64 %t6, i8* %t7, i64 %t9)
%t10 = add i64 0, 0
%t11 = load i64, i64* %hm.addr
%t12 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str0, i64 0, i64 0), i64 4)
%t13 = call i64 @rey_hm_has(i64 %t11, i8* %t12)
%t14 = call i8* @rey_val_to_str(i64 %t13)
call void @rey_println(i8* %t14)
%t15 = load i64, i64* %hm.addr
%t16 = call i8* @rey_str_new(i8* getelementptr inbounds ([8 x i8], [8 x i8]* @.str4, i64 0, i64 0), i64 7)
%t17 = call i64 @rey_hm_has(i64 %t15, i8* %t16)
%t18 = call i8* @rey_val_to_str(i64 %t17)
call void @rey_println(i8* %t18)
%t19 = load i64, i64* %hm.addr
%t20 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str0, i64 0, i64 0), i64 4)
%t21 = call i64 @rey_hm_get(i64 %t19, i8* %t20)
store i64 %t21, i64* %name.addr
%t22 = load i64, i64* %name.addr
%t23 = call i8* @rey_val_to_str(i64 %t22)
call void @rey_println(i8* %t23)
%t24 = load i64, i64* %hm.addr
%t25 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str0, i64 0, i64 0), i64 4)
%t26 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str5, i64 0, i64 0), i64 3)
%t27 = ptrtoint i8* %t26 to i64
call void @rey_hm_set(i64 %t24, i8* %t25, i64 %t27)
%t28 = add i64 0, 0
%t29 = load i64, i64* %hm.addr
%t30 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str0, i64 0, i64 0), i64 4)
%t31 = call i64 @rey_hm_get(i64 %t29, i8* %t30)
%t32 = call i8* @rey_val_to_str(i64 %t31)
call void @rey_println(i8* %t32)
%t33 = call i64 @rey_ok(i64 42)
store i64 %t33, i64* %ok.addr
%t34 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str6, i64 0, i64 0), i64 4)
%t35 = call i64 @rey_err_str(i8* %t34)
store i64 %t35, i64* %err.addr
%t36 = load i64, i64* %ok.addr
%t37 = call i64 @rey_result_is_ok(i64 %t36)
%t38 = call i8* @rey_val_to_str(i64 %t37)
call void @rey_println(i8* %t38)
%t39 = load i64, i64* %err.addr
%t40 = call i64 @rey_result_is_ok(i64 %t39)
%t41 = call i8* @rey_val_to_str(i64 %t40)
call void @rey_println(i8* %t41)
%t42 = load i64, i64* %err.addr
%t43 = call i64 @rey_result_is_err(i64 %t42)
%t44 = call i8* @rey_val_to_str(i64 %t43)
call void @rey_println(i8* %t44)
%t45 = load i64, i64* %ok.addr
%t46 = call i64 @rey_result_unwrap(i64 %t45)
%t47 = call i8* @rey_val_to_str(i64 %t46)
call void @rey_println(i8* %t47)
%t48 = load i64, i64* %ok.addr
%t49 = call i64 @rey_result_unwrap_or(i64 %t48, i64 99)
%t50 = call i8* @rey_val_to_str(i64 %t49)
call void @rey_println(i8* %t50)
%t51 = load i64, i64* %err.addr
%t52 = call i64 @rey_result_unwrap_or(i64 %t51, i64 99)
%t53 = call i8* @rey_val_to_str(i64 %t52)
call void @rey_println(i8* %t53)
%t54 = call i64 @rey_vec_new()
store i64 %t54, i64* %v.addr
%t55 = load i64, i64* %v.addr
call void @rey_vec_push(i64 %t55, i64 100)
%t56 = add i64 0, 0
%t57 = load i64, i64* %v.addr
call void @rey_vec_push(i64 %t57, i64 200)
%t58 = add i64 0, 0
%t59 = load i64, i64* %v.addr
call void @rey_vec_push(i64 %t59, i64 300)
%t60 = add i64 0, 0
%t61 = load i64, i64* %v.addr
%t62 = call i64 @rey_vec_len(i64 %t61)
%t63 = call i8* @rey_val_to_str(i64 %t62)
call void @rey_println(i8* %t63)
%t64 = load i64, i64* %v.addr
%t65 = call i64 @rey_vec_pop(i64 %t64)
%t66 = call i8* @rey_val_to_str(i64 %t65)
call void @rey_println(i8* %t66)
%t67 = load i64, i64* %v.addr
%t68 = call i64 @rey_vec_len(i64 %t67)
%t69 = call i8* @rey_val_to_str(i64 %t68)
call void @rey_println(i8* %t69)
ret void
}

define i32 @main(i32 %argc, i8** %argv) {
entry:
  call void @rey_init(i32 %argc, i8** %argv)
  call void @_rey_main()
  ret i32 0
}
