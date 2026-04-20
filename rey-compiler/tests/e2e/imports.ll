; Module: rey
target datalayout = "e-m:e-i64:64-f80:128-n8:16:32:64-S128"
target triple = "aarch64-apple-darwin"

declare i32 @printf(i8*, ...)

@.str0 = private unnamed_addr constant [5 x i8] c"%ld\0A\00", align 1

define i64 @add(i64 %a, i64 %b) {
entry:
%a.addr = alloca i64, align 8
store i64 %a, i64* %a.addr
%b.addr = alloca i64, align 8
store i64 %b, i64* %b.addr
%t0 = load i64, i64* %a.addr
%t1 = load i64, i64* %b.addr
%t2 = add i64 %t0, %t1
ret i64 %t2
ret i64 0
}

define i32 @main() {
entry:
%t0 = call i64 @add(i64 2, i64 3)
call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str0, i64 0, i64 0), i64 %t0)
ret i32 0
}

