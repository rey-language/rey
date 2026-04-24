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

@.str0 = private unnamed_addr constant [6 x i8] c"hello\00", align 1
@.str1 = private unnamed_addr constant [7 x i8] c" world\00", align 1
@.str2 = private unnamed_addr constant [4 x i8] c"hel\00", align 1
@.str3 = private unnamed_addr constant [4 x i8] c"llo\00", align 1
@.str4 = private unnamed_addr constant [6 x i8] c"world\00", align 1
@.str5 = private unnamed_addr constant [7 x i8] c"  hi  \00", align 1
@.str6 = private unnamed_addr constant [6 x i8] c"other\00", align 1
@.str7 = private unnamed_addr constant [3 x i8] c"ab\00", align 1
@.str8 = private unnamed_addr constant [12 x i8] c"hello world\00", align 1
@.str9 = private unnamed_addr constant [4 x i8] c"Rey\00", align 1

define void @_rey_main() {
entry:
%a.addr = alloca i8*, align 8
%b.addr = alloca i8*, align 8
%c.addr = alloca i8*, align 8
%eq.addr = alloca i1, align 8
%neq.addr = alloca i1, align 8
%rep.addr = alloca i8*, align 8
%replaced.addr = alloca i8*, align 8
%t0 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str0, i64 0, i64 0), i64 5)
store i8* %t0, i8** %a.addr
%t1 = call i8* @rey_str_new(i8* getelementptr inbounds ([7 x i8], [7 x i8]* @.str1, i64 0, i64 0), i64 6)
store i8* %t1, i8** %b.addr
%t2 = load i8*, i8** %a.addr
%t3 = load i8*, i8** %b.addr
%t4 = call i8* @rey_str_concat(i8* %t2, i8* %t3)
store i8* %t4, i8** %c.addr
%t5 = load i8*, i8** %c.addr
call void @rey_println(i8* %t5)
%t6 = load i8*, i8** %a.addr
%t7 = call i64 @rey_str_len(i8* %t6)
%t8 = call i8* @rey_val_to_str(i64 %t7)
call void @rey_println(i8* %t8)
%t9 = load i8*, i8** %a.addr
%t10 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str2, i64 0, i64 0), i64 3)
%t11 = call i64 @rey_str_starts_with(i8* %t9, i8* %t10)
%t12 = call i8* @rey_val_to_str(i64 %t11)
call void @rey_println(i8* %t12)
%t13 = load i8*, i8** %a.addr
%t14 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str3, i64 0, i64 0), i64 3)
%t15 = call i64 @rey_str_ends_with(i8* %t13, i8* %t14)
%t16 = call i8* @rey_val_to_str(i64 %t15)
call void @rey_println(i8* %t16)
%t17 = load i8*, i8** %c.addr
%t18 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str4, i64 0, i64 0), i64 5)
%t19 = call i64 @rey_str_index_of(i8* %t17, i8* %t18)
%t20 = call i8* @rey_val_to_str(i64 %t19)
call void @rey_println(i8* %t20)
%t21 = load i8*, i8** %c.addr
%t22 = call i8* @rey_str_slice(i8* %t21, i64 6, i64 11)
call void @rey_println(i8* %t22)
%t23 = load i8*, i8** %a.addr
%t24 = call i8* @rey_str_char_at(i8* %t23, i64 1)
call void @rey_println(i8* %t24)
%t25 = call i8* @rey_str_new(i8* getelementptr inbounds ([7 x i8], [7 x i8]* @.str5, i64 0, i64 0), i64 6)
%t26 = call i8* @rey_str_trim(i8* %t25)
call void @rey_println(i8* %t26)
%t27 = call i8* @rey_val_to_str(i64 42)
call void @rey_println(i8* %t27)
%t29 = zext i1 1 to i64
%t28 = call i8* @rey_bool_to_str(i64 %t29)
call void @rey_println(i8* %t28)
%t31 = zext i1 0 to i64
%t30 = call i8* @rey_bool_to_str(i64 %t31)
call void @rey_println(i8* %t30)
%t32 = load i8*, i8** %a.addr
%t33 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str0, i64 0, i64 0), i64 5)
%t34 = call i64 @rey_str_eq(i8* %t32, i8* %t33)
%t35 = icmp ne i64 %t34, 0
store i1 %t35, i1* %eq.addr
%t36 = load i1, i1* %eq.addr
%t38 = zext i1 %t36 to i64
%t37 = call i8* @rey_bool_to_str(i64 %t38)
call void @rey_println(i8* %t37)
%t39 = load i8*, i8** %a.addr
%t40 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str6, i64 0, i64 0), i64 5)
%t41 = call i64 @rey_str_eq(i8* %t39, i8* %t40)
%t42 = icmp ne i64 %t41, 0
store i1 %t42, i1* %neq.addr
%t43 = load i1, i1* %neq.addr
%t45 = zext i1 %t43 to i64
%t44 = call i8* @rey_bool_to_str(i64 %t45)
call void @rey_println(i8* %t44)
%t46 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str7, i64 0, i64 0), i64 2)
%t47 = call i8* @rey_str_repeat(i8* %t46, i64 3)
store i8* %t47, i8** %rep.addr
%t48 = load i8*, i8** %rep.addr
call void @rey_println(i8* %t48)
%t49 = call i8* @rey_str_new(i8* getelementptr inbounds ([12 x i8], [12 x i8]* @.str8, i64 0, i64 0), i64 11)
%t50 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str4, i64 0, i64 0), i64 5)
%t51 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str9, i64 0, i64 0), i64 3)
%t52 = call i8* @rey_str_replace(i8* %t49, i8* %t50, i8* %t51)
store i8* %t52, i8** %replaced.addr
%t53 = load i8*, i8** %replaced.addr
call void @rey_println(i8* %t53)
ret void
}

define i32 @main(i32 %argc, i8** %argv) {
entry:
  call void @rey_init(i32 %argc, i8** %argv)
  call void @_rey_main()
  ret i32 0
}
