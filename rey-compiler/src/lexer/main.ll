; Module: rey
target datalayout = "e-m:e-i64:64-f80:128-n8:16:32:64-S128"
target triple = "aarch64-apple-darwin"

declare i32 @printf(i8*, ...)
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

%struct.Lexer = type { i8*, i64, i64, i64, i64, i64, i64 }
%struct.LexerResult = type { i64, i64 }
%struct.LexError = type { i8*, i8*, %struct.Span* }
%struct.Span = type { i64, i64, i64, i64 }
%struct.Token = type { i8*, i8*, %struct.Span* }

@.str0 = private unnamed_addr constant [2 x i8] c"a\00", align 1
@.str1 = private unnamed_addr constant [2 x i8] c"z\00", align 1
@.str2 = private unnamed_addr constant [2 x i8] c"A\00", align 1
@.str3 = private unnamed_addr constant [2 x i8] c"Z\00", align 1
@.str4 = private unnamed_addr constant [2 x i8] c"_\00", align 1
@.str5 = private unnamed_addr constant [2 x i8] c"0\00", align 1
@.str6 = private unnamed_addr constant [2 x i8] c"9\00", align 1
@.str7 = private unnamed_addr constant [2 x i8] c" \00", align 1
@.str8 = private unnamed_addr constant [2 x i8] c"\0A\00", align 1
@.str9 = private unnamed_addr constant [2 x i8] c"\09\00", align 1
@.str10 = private unnamed_addr constant [2 x i8] c"\0D\00", align 1
@.str11 = private unnamed_addr constant [2 x i8] c"(\00", align 1
@.str12 = private unnamed_addr constant [2 x i8] c")\00", align 1
@.str13 = private unnamed_addr constant [2 x i8] c"{\00", align 1
@.str14 = private unnamed_addr constant [2 x i8] c"}\00", align 1
@.str15 = private unnamed_addr constant [2 x i8] c"[\00", align 1
@.str16 = private unnamed_addr constant [2 x i8] c"]\00", align 1
@.str17 = private unnamed_addr constant [2 x i8] c",\00", align 1
@.str18 = private unnamed_addr constant [2 x i8] c".\00", align 1
@.str19 = private unnamed_addr constant [2 x i8] c";\00", align 1
@.str20 = private unnamed_addr constant [2 x i8] c":\00", align 1
@.str21 = private unnamed_addr constant [2 x i8] c"+\00", align 1
@.str22 = private unnamed_addr constant [2 x i8] c"-\00", align 1
@.str23 = private unnamed_addr constant [2 x i8] c"*\00", align 1
@.str24 = private unnamed_addr constant [2 x i8] c"/\00", align 1
@.str25 = private unnamed_addr constant [2 x i8] c"%\00", align 1
@.str26 = private unnamed_addr constant [2 x i8] c"^\00", align 1
@.str27 = private unnamed_addr constant [2 x i8] c"&\00", align 1
@.str28 = private unnamed_addr constant [2 x i8] c"|\00", align 1
@.str29 = private unnamed_addr constant [2 x i8] c"!\00", align 1
@.str30 = private unnamed_addr constant [2 x i8] c"=\00", align 1
@.str31 = private unnamed_addr constant [2 x i8] c"<\00", align 1
@.str32 = private unnamed_addr constant [2 x i8] c">\00", align 1
@.str33 = private unnamed_addr constant [2 x i8] c"?\00", align 1
@.str34 = private unnamed_addr constant [1 x i8] c"\00", align 1
@.str35 = private unnamed_addr constant [20 x i8] c"UnexpectedCharacter\00", align 1
@.str36 = private unnamed_addr constant [27 x i8] c"Unterminated block comment\00", align 1
@.str37 = private unnamed_addr constant [19 x i8] c"UnterminatedString\00", align 1
@.str38 = private unnamed_addr constant [28 x i8] c"Unterminated string literal\00", align 1
@.str39 = private unnamed_addr constant [2 x i8] c"\22\00", align 1
@.str40 = private unnamed_addr constant [2 x i8] c"\5C\00", align 1
@.str41 = private unnamed_addr constant [2 x i8] c"n\00", align 1
@.str42 = private unnamed_addr constant [2 x i8] c"t\00", align 1
@.str43 = private unnamed_addr constant [2 x i8] c"r\00", align 1
@.str44 = private unnamed_addr constant [17 x i8] c"UnterminatedChar\00", align 1
@.str45 = private unnamed_addr constant [26 x i8] c"Unterminated char literal\00", align 1
@.str46 = private unnamed_addr constant [2 x i8] c"'\00", align 1
@.str47 = private unnamed_addr constant [2 x i8] c"e\00", align 1
@.str48 = private unnamed_addr constant [2 x i8] c"E\00", align 1
@.str49 = private unnamed_addr constant [4 x i8] c"var\00", align 1
@.str50 = private unnamed_addr constant [4 x i8] c"Var\00", align 1
@.str51 = private unnamed_addr constant [6 x i8] c"const\00", align 1
@.str52 = private unnamed_addr constant [6 x i8] c"Const\00", align 1
@.str53 = private unnamed_addr constant [5 x i8] c"func\00", align 1
@.str54 = private unnamed_addr constant [5 x i8] c"Func\00", align 1
@.str55 = private unnamed_addr constant [7 x i8] c"struct\00", align 1
@.str56 = private unnamed_addr constant [7 x i8] c"Struct\00", align 1
@.str57 = private unnamed_addr constant [5 x i8] c"enum\00", align 1
@.str58 = private unnamed_addr constant [5 x i8] c"Enum\00", align 1
@.str59 = private unnamed_addr constant [7 x i8] c"import\00", align 1
@.str60 = private unnamed_addr constant [7 x i8] c"Import\00", align 1
@.str61 = private unnamed_addr constant [7 x i8] c"export\00", align 1
@.str62 = private unnamed_addr constant [7 x i8] c"Export\00", align 1
@.str63 = private unnamed_addr constant [4 x i8] c"pub\00", align 1
@.str64 = private unnamed_addr constant [4 x i8] c"Pub\00", align 1
@.str65 = private unnamed_addr constant [7 x i8] c"return\00", align 1
@.str66 = private unnamed_addr constant [7 x i8] c"Return\00", align 1
@.str67 = private unnamed_addr constant [3 x i8] c"if\00", align 1
@.str68 = private unnamed_addr constant [3 x i8] c"If\00", align 1
@.str69 = private unnamed_addr constant [5 x i8] c"else\00", align 1
@.str70 = private unnamed_addr constant [5 x i8] c"Else\00", align 1
@.str71 = private unnamed_addr constant [6 x i8] c"while\00", align 1
@.str72 = private unnamed_addr constant [6 x i8] c"While\00", align 1
@.str73 = private unnamed_addr constant [4 x i8] c"for\00", align 1
@.str74 = private unnamed_addr constant [4 x i8] c"For\00", align 1
@.str75 = private unnamed_addr constant [3 x i8] c"in\00", align 1
@.str76 = private unnamed_addr constant [3 x i8] c"In\00", align 1
@.str77 = private unnamed_addr constant [5 x i8] c"loop\00", align 1
@.str78 = private unnamed_addr constant [5 x i8] c"Loop\00", align 1
@.str79 = private unnamed_addr constant [6 x i8] c"break\00", align 1
@.str80 = private unnamed_addr constant [6 x i8] c"Break\00", align 1
@.str81 = private unnamed_addr constant [9 x i8] c"continue\00", align 1
@.str82 = private unnamed_addr constant [9 x i8] c"Continue\00", align 1
@.str83 = private unnamed_addr constant [6 x i8] c"match\00", align 1
@.str84 = private unnamed_addr constant [6 x i8] c"Match\00", align 1
@.str85 = private unnamed_addr constant [11 x i8] c"instanceof\00", align 1
@.str86 = private unnamed_addr constant [11 x i8] c"InstanceOf\00", align 1
@.str87 = private unnamed_addr constant [5 x i8] c"true\00", align 1
@.str88 = private unnamed_addr constant [5 x i8] c"True\00", align 1
@.str89 = private unnamed_addr constant [6 x i8] c"false\00", align 1
@.str90 = private unnamed_addr constant [6 x i8] c"False\00", align 1
@.str91 = private unnamed_addr constant [5 x i8] c"null\00", align 1
@.str92 = private unnamed_addr constant [5 x i8] c"Null\00", align 1
@.str93 = private unnamed_addr constant [11 x i8] c"Identifier\00", align 1
@.str94 = private unnamed_addr constant [4 x i8] c"Eof\00", align 1
@.str95 = private unnamed_addr constant [10 x i8] c"LeftParen\00", align 1
@.str96 = private unnamed_addr constant [11 x i8] c"RightParen\00", align 1
@.str97 = private unnamed_addr constant [10 x i8] c"LeftBrace\00", align 1
@.str98 = private unnamed_addr constant [11 x i8] c"RightBrace\00", align 1
@.str99 = private unnamed_addr constant [12 x i8] c"LeftBracket\00", align 1
@.str100 = private unnamed_addr constant [13 x i8] c"RightBracket\00", align 1
@.str101 = private unnamed_addr constant [6 x i8] c"Comma\00", align 1
@.str102 = private unnamed_addr constant [4 x i8] c"Dot\00", align 1
@.str103 = private unnamed_addr constant [10 x i8] c"Semicolon\00", align 1
@.str104 = private unnamed_addr constant [9 x i8] c"PlusPlus\00", align 1
@.str105 = private unnamed_addr constant [3 x i8] c"++\00", align 1
@.str106 = private unnamed_addr constant [10 x i8] c"PlusEqual\00", align 1
@.str107 = private unnamed_addr constant [3 x i8] c"+=\00", align 1
@.str108 = private unnamed_addr constant [5 x i8] c"Plus\00", align 1
@.str109 = private unnamed_addr constant [11 x i8] c"MinusMinus\00", align 1
@.str110 = private unnamed_addr constant [3 x i8] c"--\00", align 1
@.str111 = private unnamed_addr constant [11 x i8] c"MinusEqual\00", align 1
@.str112 = private unnamed_addr constant [3 x i8] c"-=\00", align 1
@.str113 = private unnamed_addr constant [6 x i8] c"Arrow\00", align 1
@.str114 = private unnamed_addr constant [3 x i8] c"->\00", align 1
@.str115 = private unnamed_addr constant [6 x i8] c"Minus\00", align 1
@.str116 = private unnamed_addr constant [10 x i8] c"StarEqual\00", align 1
@.str117 = private unnamed_addr constant [3 x i8] c"*=\00", align 1
@.str118 = private unnamed_addr constant [5 x i8] c"Star\00", align 1
@.str119 = private unnamed_addr constant [11 x i8] c"SlashEqual\00", align 1
@.str120 = private unnamed_addr constant [3 x i8] c"/=\00", align 1
@.str121 = private unnamed_addr constant [6 x i8] c"Slash\00", align 1
@.str122 = private unnamed_addr constant [13 x i8] c"PercentEqual\00", align 1
@.str123 = private unnamed_addr constant [3 x i8] c"%=\00", align 1
@.str124 = private unnamed_addr constant [8 x i8] c"Percent\00", align 1
@.str125 = private unnamed_addr constant [11 x i8] c"EqualEqual\00", align 1
@.str126 = private unnamed_addr constant [3 x i8] c"==\00", align 1
@.str127 = private unnamed_addr constant [6 x i8] c"Equal\00", align 1
@.str128 = private unnamed_addr constant [9 x i8] c"NotEqual\00", align 1
@.str129 = private unnamed_addr constant [3 x i8] c"!=\00", align 1
@.str130 = private unnamed_addr constant [4 x i8] c"Not\00", align 1
@.str131 = private unnamed_addr constant [10 x i8] c"LessEqual\00", align 1
@.str132 = private unnamed_addr constant [3 x i8] c"<=\00", align 1
@.str133 = private unnamed_addr constant [5 x i8] c"Less\00", align 1
@.str134 = private unnamed_addr constant [13 x i8] c"GreaterEqual\00", align 1
@.str135 = private unnamed_addr constant [3 x i8] c">=\00", align 1
@.str136 = private unnamed_addr constant [8 x i8] c"Greater\00", align 1
@.str137 = private unnamed_addr constant [11 x i8] c"ColonColon\00", align 1
@.str138 = private unnamed_addr constant [3 x i8] c"::\00", align 1
@.str139 = private unnamed_addr constant [6 x i8] c"Colon\00", align 1
@.str140 = private unnamed_addr constant [7 x i8] c"AndAnd\00", align 1
@.str141 = private unnamed_addr constant [3 x i8] c"&&\00", align 1
@.str142 = private unnamed_addr constant [5 x i8] c"OrOr\00", align 1
@.str143 = private unnamed_addr constant [3 x i8] c"||\00", align 1
@.str144 = private unnamed_addr constant [14 x i8] c"StringLiteral\00", align 1
@.str145 = private unnamed_addr constant [12 x i8] c"CharLiteral\00", align 1
@.str146 = private unnamed_addr constant [14 x i8] c"NumberLiteral\00", align 1
@.str147 = private unnamed_addr constant [23 x i8] c"Unexpected character: \00", align 1

define i1 @isAlpha(i8* %c) {
entry:
%c.addr = alloca i8*, align 8
store i8* %c, i8** %c.addr
%t0 = load i8*, i8** %c.addr
%t1 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str0, i64 0, i64 0), i64 1)
%t2 = icmp sge i64 %t0, %t1
%t3 = load i8*, i8** %c.addr
%t4 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str1, i64 0, i64 0), i64 1)
%t5 = icmp sle i64 %t3, %t4
%t6 = and i1 %t2, %t5
%t7 = load i8*, i8** %c.addr
%t8 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str2, i64 0, i64 0), i64 1)
%t9 = icmp sge i64 %t7, %t8
%t10 = load i8*, i8** %c.addr
%t11 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str3, i64 0, i64 0), i64 1)
%t12 = icmp sle i64 %t10, %t11
%t13 = and i1 %t9, %t12
%t14 = or i1 %t6, %t13
%t15 = load i8*, i8** %c.addr
%t16 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str4, i64 0, i64 0), i64 1)
%t17 = call i64 @rey_str_eq(i8* %t15, i8* %t16)
%t18 = icmp ne i64 %t17, 0
%t19 = or i1 %t14, %t18
ret i1 %t19
ret i64 0
}

define i1 @isDigit(i8* %c) {
entry:
%c.addr = alloca i8*, align 8
store i8* %c, i8** %c.addr
%t0 = load i8*, i8** %c.addr
%t1 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str5, i64 0, i64 0), i64 1)
%t2 = icmp sge i64 %t0, %t1
%t3 = load i8*, i8** %c.addr
%t4 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str6, i64 0, i64 0), i64 1)
%t5 = icmp sle i64 %t3, %t4
%t6 = and i1 %t2, %t5
ret i1 %t6
ret i64 0
}

define i1 @isAlphaNumeric(i8* %c) {
entry:
%c.addr = alloca i8*, align 8
store i8* %c, i8** %c.addr
%t0 = load i8*, i8** %c.addr
%t1 = call i64 @isAlpha(i8* %t0)
%t2 = load i8*, i8** %c.addr
%t3 = call i64 @isDigit(i8* %t2)
%t4 = icmp ne i64 %t1, 0
%t5 = icmp ne i64 %t3, 0
%t6 = or i1 %t4, %t5
ret i1 %t6
ret i64 0
}

define i1 @isWhitespace(i8* %c) {
entry:
%c.addr = alloca i8*, align 8
store i8* %c, i8** %c.addr
%t0 = load i8*, i8** %c.addr
%t1 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str7, i64 0, i64 0), i64 1)
%t2 = call i64 @rey_str_eq(i8* %t0, i8* %t1)
%t3 = icmp ne i64 %t2, 0
%t4 = load i8*, i8** %c.addr
%t5 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str8, i64 0, i64 0), i64 1)
%t6 = call i64 @rey_str_eq(i8* %t4, i8* %t5)
%t7 = icmp ne i64 %t6, 0
%t8 = or i1 %t3, %t7
%t9 = load i8*, i8** %c.addr
%t10 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str9, i64 0, i64 0), i64 1)
%t11 = call i64 @rey_str_eq(i8* %t9, i8* %t10)
%t12 = icmp ne i64 %t11, 0
%t13 = or i1 %t8, %t12
%t14 = load i8*, i8** %c.addr
%t15 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str10, i64 0, i64 0), i64 1)
%t16 = call i64 @rey_str_eq(i8* %t14, i8* %t15)
%t17 = icmp ne i64 %t16, 0
%t18 = or i1 %t13, %t17
ret i1 %t18
ret i64 0
}

define i1 @isPunctuation(i8* %c) {
entry:
%c.addr = alloca i8*, align 8
store i8* %c, i8** %c.addr
%t0 = load i8*, i8** %c.addr
%t1 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str11, i64 0, i64 0), i64 1)
%t2 = call i64 @rey_str_eq(i8* %t0, i8* %t1)
%t3 = icmp ne i64 %t2, 0
%t4 = load i8*, i8** %c.addr
%t5 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str12, i64 0, i64 0), i64 1)
%t6 = call i64 @rey_str_eq(i8* %t4, i8* %t5)
%t7 = icmp ne i64 %t6, 0
%t8 = or i1 %t3, %t7
%t9 = load i8*, i8** %c.addr
%t10 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str13, i64 0, i64 0), i64 1)
%t11 = call i64 @rey_str_eq(i8* %t9, i8* %t10)
%t12 = icmp ne i64 %t11, 0
%t13 = or i1 %t8, %t12
%t14 = load i8*, i8** %c.addr
%t15 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str14, i64 0, i64 0), i64 1)
%t16 = call i64 @rey_str_eq(i8* %t14, i8* %t15)
%t17 = icmp ne i64 %t16, 0
%t18 = or i1 %t13, %t17
%t19 = load i8*, i8** %c.addr
%t20 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str15, i64 0, i64 0), i64 1)
%t21 = call i64 @rey_str_eq(i8* %t19, i8* %t20)
%t22 = icmp ne i64 %t21, 0
%t23 = or i1 %t18, %t22
%t24 = load i8*, i8** %c.addr
%t25 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str16, i64 0, i64 0), i64 1)
%t26 = call i64 @rey_str_eq(i8* %t24, i8* %t25)
%t27 = icmp ne i64 %t26, 0
%t28 = or i1 %t23, %t27
%t29 = load i8*, i8** %c.addr
%t30 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str17, i64 0, i64 0), i64 1)
%t31 = call i64 @rey_str_eq(i8* %t29, i8* %t30)
%t32 = icmp ne i64 %t31, 0
%t33 = or i1 %t28, %t32
%t34 = load i8*, i8** %c.addr
%t35 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str18, i64 0, i64 0), i64 1)
%t36 = call i64 @rey_str_eq(i8* %t34, i8* %t35)
%t37 = icmp ne i64 %t36, 0
%t38 = or i1 %t33, %t37
%t39 = load i8*, i8** %c.addr
%t40 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str19, i64 0, i64 0), i64 1)
%t41 = call i64 @rey_str_eq(i8* %t39, i8* %t40)
%t42 = icmp ne i64 %t41, 0
%t43 = or i1 %t38, %t42
%t44 = load i8*, i8** %c.addr
%t45 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str20, i64 0, i64 0), i64 1)
%t46 = call i64 @rey_str_eq(i8* %t44, i8* %t45)
%t47 = icmp ne i64 %t46, 0
%t48 = or i1 %t43, %t47
%t49 = load i8*, i8** %c.addr
%t50 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str21, i64 0, i64 0), i64 1)
%t51 = call i64 @rey_str_eq(i8* %t49, i8* %t50)
%t52 = icmp ne i64 %t51, 0
%t53 = or i1 %t48, %t52
%t54 = load i8*, i8** %c.addr
%t55 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str22, i64 0, i64 0), i64 1)
%t56 = call i64 @rey_str_eq(i8* %t54, i8* %t55)
%t57 = icmp ne i64 %t56, 0
%t58 = or i1 %t53, %t57
%t59 = load i8*, i8** %c.addr
%t60 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str23, i64 0, i64 0), i64 1)
%t61 = call i64 @rey_str_eq(i8* %t59, i8* %t60)
%t62 = icmp ne i64 %t61, 0
%t63 = or i1 %t58, %t62
%t64 = load i8*, i8** %c.addr
%t65 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str24, i64 0, i64 0), i64 1)
%t66 = call i64 @rey_str_eq(i8* %t64, i8* %t65)
%t67 = icmp ne i64 %t66, 0
%t68 = or i1 %t63, %t67
%t69 = load i8*, i8** %c.addr
%t70 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str25, i64 0, i64 0), i64 1)
%t71 = call i64 @rey_str_eq(i8* %t69, i8* %t70)
%t72 = icmp ne i64 %t71, 0
%t73 = or i1 %t68, %t72
%t74 = load i8*, i8** %c.addr
%t75 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str26, i64 0, i64 0), i64 1)
%t76 = call i64 @rey_str_eq(i8* %t74, i8* %t75)
%t77 = icmp ne i64 %t76, 0
%t78 = or i1 %t73, %t77
%t79 = load i8*, i8** %c.addr
%t80 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str27, i64 0, i64 0), i64 1)
%t81 = call i64 @rey_str_eq(i8* %t79, i8* %t80)
%t82 = icmp ne i64 %t81, 0
%t83 = or i1 %t78, %t82
%t84 = load i8*, i8** %c.addr
%t85 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str28, i64 0, i64 0), i64 1)
%t86 = call i64 @rey_str_eq(i8* %t84, i8* %t85)
%t87 = icmp ne i64 %t86, 0
%t88 = or i1 %t83, %t87
%t89 = load i8*, i8** %c.addr
%t90 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str29, i64 0, i64 0), i64 1)
%t91 = call i64 @rey_str_eq(i8* %t89, i8* %t90)
%t92 = icmp ne i64 %t91, 0
%t93 = or i1 %t88, %t92
%t94 = load i8*, i8** %c.addr
%t95 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str30, i64 0, i64 0), i64 1)
%t96 = call i64 @rey_str_eq(i8* %t94, i8* %t95)
%t97 = icmp ne i64 %t96, 0
%t98 = or i1 %t93, %t97
%t99 = load i8*, i8** %c.addr
%t100 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str31, i64 0, i64 0), i64 1)
%t101 = call i64 @rey_str_eq(i8* %t99, i8* %t100)
%t102 = icmp ne i64 %t101, 0
%t103 = or i1 %t98, %t102
%t104 = load i8*, i8** %c.addr
%t105 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str32, i64 0, i64 0), i64 1)
%t106 = call i64 @rey_str_eq(i8* %t104, i8* %t105)
%t107 = icmp ne i64 %t106, 0
%t108 = or i1 %t103, %t107
%t109 = load i8*, i8** %c.addr
%t110 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str33, i64 0, i64 0), i64 1)
%t111 = call i64 @rey_str_eq(i8* %t109, i8* %t110)
%t112 = icmp ne i64 %t111, 0
%t113 = or i1 %t108, %t112
ret i1 %t113
ret i64 0
}

define i64 @newSpan(i64 %start, i64 %end, i64 %line, i64 %column) {
entry:
%start.addr = alloca i64, align 8
store i64 %start, i64* %start.addr
%end.addr = alloca i64, align 8
store i64 %end, i64* %end.addr
%line.addr = alloca i64, align 8
store i64 %line, i64* %line.addr
%column.addr = alloca i64, align 8
store i64 %column, i64* %column.addr
%t0 = alloca %struct.Span, align 8
%t1 = getelementptr inbounds %struct.Span, %struct.Span* %t0, i32 0, i32 0
store i64 0, i64* %t1
%t2 = getelementptr inbounds %struct.Span, %struct.Span* %t0, i32 0, i32 1
store i64 0, i64* %t2
%t3 = getelementptr inbounds %struct.Span, %struct.Span* %t0, i32 0, i32 2
store i64 0, i64* %t3
%t4 = getelementptr inbounds %struct.Span, %struct.Span* %t0, i32 0, i32 3
store i64 0, i64* %t4
%t5 = load i64, i64* %start.addr
%t6 = getelementptr inbounds %struct.Span, %struct.Span* %t0, i32 0, i32 0
store i64 %t5, i64* %t6
%t7 = load i64, i64* %end.addr
%t8 = getelementptr inbounds %struct.Span, %struct.Span* %t0, i32 0, i32 1
store i64 %t7, i64* %t8
%t9 = load i64, i64* %line.addr
%t10 = getelementptr inbounds %struct.Span, %struct.Span* %t0, i32 0, i32 2
store i64 %t9, i64* %t10
%t11 = load i64, i64* %column.addr
%t12 = getelementptr inbounds %struct.Span, %struct.Span* %t0, i32 0, i32 3
store i64 %t11, i64* %t12
ret i64 %t0
ret i64 0
}

define i64 @makeToken(i64 %kind, i8* %lexeme, i64 %span) {
entry:
%kind.addr = alloca i64, align 8
store i64 %kind, i64* %kind.addr
%lexeme.addr = alloca i8*, align 8
store i8* %lexeme, i8** %lexeme.addr
%span.addr = alloca i64, align 8
store i64 %span, i64* %span.addr
%t0 = alloca %struct.Token, align 8
%t1 = getelementptr inbounds %struct.Token, %struct.Token* %t0, i32 0, i32 0
%t2 = getelementptr inbounds %struct.Token, %struct.Token* %t0, i32 0, i32 1
%t3 = getelementptr inbounds %struct.Token, %struct.Token* %t0, i32 0, i32 2
%t4 = load i64, i64* %kind.addr
%t5 = getelementptr inbounds %struct.Token, %struct.Token* %t0, i32 0, i32 0
store i8* %t4, i8** %t5
%t6 = load i8*, i8** %lexeme.addr
%t7 = getelementptr inbounds %struct.Token, %struct.Token* %t0, i32 0, i32 1
store i8* %t6, i8** %t7
%t8 = load i64, i64* %span.addr
%t9 = getelementptr inbounds %struct.Token, %struct.Token* %t0, i32 0, i32 2
store %struct.Span* %t8, %struct.Span** %t9
ret i64 %t0
ret i64 0
}

define i64 @makeError(i64 %kind, i8* %message, i64 %span) {
entry:
%kind.addr = alloca i64, align 8
store i64 %kind, i64* %kind.addr
%message.addr = alloca i8*, align 8
store i8* %message, i8** %message.addr
%span.addr = alloca i64, align 8
store i64 %span, i64* %span.addr
%t0 = alloca %struct.LexError, align 8
%t1 = getelementptr inbounds %struct.LexError, %struct.LexError* %t0, i32 0, i32 0
%t2 = getelementptr inbounds %struct.LexError, %struct.LexError* %t0, i32 0, i32 1
%t3 = getelementptr inbounds %struct.LexError, %struct.LexError* %t0, i32 0, i32 2
%t4 = load i64, i64* %kind.addr
%t5 = getelementptr inbounds %struct.LexError, %struct.LexError* %t0, i32 0, i32 0
store i8* %t4, i8** %t5
%t6 = load i8*, i8** %message.addr
%t7 = getelementptr inbounds %struct.LexError, %struct.LexError* %t0, i32 0, i32 1
store i8* %t6, i8** %t7
%t8 = load i64, i64* %span.addr
%t9 = getelementptr inbounds %struct.LexError, %struct.LexError* %t0, i32 0, i32 2
store %struct.Span* %t8, %struct.Span** %t9
ret i64 %t0
ret i64 0
}

define i64 @addToken(i64 %lexer, i64 %kind, i8* %lexeme) {
entry:
%lexer.addr = alloca i64, align 8
store i64 %lexer, i64* %lexer.addr
%kind.addr = alloca i64, align 8
store i64 %kind, i64* %kind.addr
%lexeme.addr = alloca i8*, align 8
store i8* %lexeme, i8** %lexeme.addr
%t0 = load i64, i64* %lexer.addr
%t1 = load i64, i64* %lexer.addr
%t2 = load i64, i64* %lexer.addr
%t3 = load i64, i64* %lexer.addr
%t4 = call i64 @newSpan(i64 0, i64 0, i64 0, i64 0)
%span.addr = alloca i64, align 8
store i64 %t4, i64* %span.addr
%t5 = load i64, i64* %kind.addr
%t6 = load i8*, i8** %lexeme.addr
%t7 = load i64, i64* %span.addr
%t8 = call i64 @makeToken(i64 %t5, i8* %t6, i64 %t7)
%token.addr = alloca i64, align 8
store i64 %t8, i64* %token.addr
%t9 = load i64, i64* %lexer.addr
%t10 = load i64, i64* %token.addr
call void @rey_vec_push(i64 0, i64 %t10)
%t11 = add i64 0, 0
ret i64 0
}

define i64 @addError(i64 %lexer, i64 %kind, i8* %message) {
entry:
%lexer.addr = alloca i64, align 8
store i64 %lexer, i64* %lexer.addr
%kind.addr = alloca i64, align 8
store i64 %kind, i64* %kind.addr
%message.addr = alloca i8*, align 8
store i8* %message, i8** %message.addr
%t0 = load i64, i64* %lexer.addr
%t1 = load i64, i64* %lexer.addr
%t2 = add i64 0, 1
%t3 = load i64, i64* %lexer.addr
%t4 = load i64, i64* %lexer.addr
%t5 = call i64 @newSpan(i64 0, i64 %t2, i64 0, i64 0)
%span.addr = alloca i64, align 8
store i64 %t5, i64* %span.addr
%t6 = load i64, i64* %kind.addr
%t7 = load i8*, i8** %message.addr
%t8 = load i64, i64* %span.addr
%t9 = call i64 @makeError(i64 %t6, i8* %t7, i64 %t8)
%error.addr = alloca i64, align 8
store i64 %t9, i64* %error.addr
%t10 = load i64, i64* %lexer.addr
%t11 = load i64, i64* %error.addr
call void @rey_vec_push(i64 0, i64 %t11)
%t12 = add i64 0, 0
ret i64 0
}

define i8* @lexAdvance(i64 %lexer) {
entry:
%lexer.addr = alloca i64, align 8
store i64 %lexer, i64* %lexer.addr
%t0 = load i64, i64* %lexer.addr
%t1 = load i64, i64* %lexer.addr
%t2 = call i64 @rey_vec_len(i64 0)
%t3 = icmp sge i64 0, %t2
br i1 %t3, label %then4, label %endif6
then4:
%t7 = call i8* @rey_str_new(i8* getelementptr inbounds ([1 x i8], [1 x i8]* @.str34, i64 0, i64 0), i64 0)
ret i8* %t7
endif6:
%t8 = load i64, i64* %lexer.addr
%t9 = load i64, i64* %lexer.addr
%t10 = call i64 @rey_vec_get(i64 0, i64 0)
%c.addr = alloca i64, align 8
store i64 %t10, i64* %c.addr
%t11 = load i64, i64* %c.addr
%t12 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str8, i64 0, i64 0), i64 1)
%t13 = call i64 @rey_str_eq(i8* %t11, i8* %t12)
%t14 = icmp ne i64 %t13, 0
br i1 %t14, label %then15, label %else16
then15:
br label %endif17
else16:
br label %endif17
endif17:
%t18 = load i64, i64* %c.addr
ret i8* %t18
ret i64 0
}

define i8* @lexPeek(i64 %lexer) {
entry:
%lexer.addr = alloca i64, align 8
store i64 %lexer, i64* %lexer.addr
%t0 = load i64, i64* %lexer.addr
%t1 = load i64, i64* %lexer.addr
%t2 = call i64 @rey_vec_len(i64 0)
%t3 = icmp sge i64 0, %t2
br i1 %t3, label %then4, label %endif6
then4:
%t7 = call i8* @rey_str_new(i8* getelementptr inbounds ([1 x i8], [1 x i8]* @.str34, i64 0, i64 0), i64 0)
ret i8* %t7
endif6:
%t8 = load i64, i64* %lexer.addr
%t9 = load i64, i64* %lexer.addr
%t10 = call i64 @rey_vec_get(i64 0, i64 0)
ret i8* %t10
ret i64 0
}

define i8* @lexPeekNext(i64 %lexer) {
entry:
%lexer.addr = alloca i64, align 8
store i64 %lexer, i64* %lexer.addr
%t0 = load i64, i64* %lexer.addr
%t1 = add i64 0, 1
%t2 = load i64, i64* %lexer.addr
%t3 = call i64 @rey_vec_len(i64 0)
%t4 = icmp sge i64 %t1, %t3
br i1 %t4, label %then5, label %endif7
then5:
%t8 = call i8* @rey_str_new(i8* getelementptr inbounds ([1 x i8], [1 x i8]* @.str34, i64 0, i64 0), i64 0)
ret i8* %t8
endif7:
%t9 = load i64, i64* %lexer.addr
%t10 = load i64, i64* %lexer.addr
%t11 = add i64 0, 1
%t12 = call i64 @rey_vec_get(i64 0, i64 %t11)
ret i8* %t12
ret i64 0
}

define i1 @lexMatchChar(i64 %lexer, i8* %expected) {
entry:
%lexer.addr = alloca i64, align 8
store i64 %lexer, i64* %lexer.addr
%expected.addr = alloca i8*, align 8
store i8* %expected, i8** %expected.addr
%t0 = load i64, i64* %lexer.addr
%t1 = load i64, i64* %lexer.addr
%t2 = call i64 @rey_vec_len(i64 0)
%t3 = icmp sge i64 0, %t2
br i1 %t3, label %then4, label %endif6
then4:
ret i1 0
endif6:
%t7 = load i64, i64* %lexer.addr
%t8 = load i64, i64* %lexer.addr
%t9 = call i64 @rey_vec_get(i64 0, i64 0)
%t10 = load i8*, i8** %expected.addr
%t11 = call i64 @rey_str_eq(i8* %t9, i8* %t10)
%t12 = icmp eq i64 %t11, 0
br i1 %t12, label %then13, label %endif15
then13:
ret i1 0
endif15:
ret i1 1
ret i64 0
}

define i64 @skipWhitespace(i64 %lexer) {
entry:
%lexer.addr = alloca i64, align 8
store i64 %lexer, i64* %lexer.addr
br label %loop.body0
loop.body0:
%t2 = load i64, i64* %lexer.addr
%t3 = load i64, i64* %lexer.addr
%t4 = call i64 @rey_vec_len(i64 0)
%t5 = icmp sge i64 0, %t4
br i1 %t5, label %then6, label %endif8
then6:
br label %loop.end1
endif8:
%t9 = load i64, i64* %lexer.addr
%t10 = call i64 @lexPeek(i64 %t9)
%c.addr = alloca i64, align 8
store i64 %t10, i64* %c.addr
%t11 = load i64, i64* %c.addr
%t12 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str7, i64 0, i64 0), i64 1)
%t13 = call i64 @rey_str_eq(i8* %t11, i8* %t12)
%t14 = icmp ne i64 %t13, 0
%t15 = load i64, i64* %c.addr
%t16 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str9, i64 0, i64 0), i64 1)
%t17 = call i64 @rey_str_eq(i8* %t15, i8* %t16)
%t18 = icmp ne i64 %t17, 0
%t19 = or i1 %t14, %t18
%t20 = load i64, i64* %c.addr
%t21 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str10, i64 0, i64 0), i64 1)
%t22 = call i64 @rey_str_eq(i8* %t20, i8* %t21)
%t23 = icmp ne i64 %t22, 0
%t24 = or i1 %t19, %t23
br i1 %t24, label %then25, label %else26
then25:
%t28 = load i64, i64* %lexer.addr
%t29 = call i64 @lexAdvance(i64 %t28)
br label %endif27
else26:
%t30 = load i64, i64* %c.addr
%t31 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str8, i64 0, i64 0), i64 1)
%t32 = call i64 @rey_str_eq(i8* %t30, i8* %t31)
%t33 = icmp ne i64 %t32, 0
br i1 %t33, label %then34, label %else35
then34:
%t37 = load i64, i64* %lexer.addr
%t38 = call i64 @lexAdvance(i64 %t37)
br label %endif36
else35:
%t39 = load i64, i64* %c.addr
%t40 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str24, i64 0, i64 0), i64 1)
%t41 = call i64 @rey_str_eq(i8* %t39, i8* %t40)
%t42 = icmp ne i64 %t41, 0
br i1 %t42, label %then43, label %else44
then43:
%t46 = load i64, i64* %lexer.addr
%t47 = call i64 @lexPeekNext(i64 %t46)
%next.addr = alloca i64, align 8
store i64 %t47, i64* %next.addr
%t48 = load i64, i64* %next.addr
%t49 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str24, i64 0, i64 0), i64 1)
%t50 = call i64 @rey_str_eq(i8* %t48, i8* %t49)
%t51 = icmp ne i64 %t50, 0
br i1 %t51, label %then52, label %else53
then52:
br label %loop.body55
loop.body55:
%t57 = load i64, i64* %lexer.addr
%t58 = load i64, i64* %lexer.addr
%t59 = call i64 @rey_vec_len(i64 0)
%t60 = icmp sge i64 0, %t59
br i1 %t60, label %then61, label %endif63
then61:
br label %loop.end56
endif63:
%t64 = load i64, i64* %lexer.addr
%t65 = call i64 @lexPeek(i64 %t64)
%t66 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str8, i64 0, i64 0), i64 1)
%t67 = call i64 @rey_str_eq(i8* %t65, i8* %t66)
%t68 = icmp ne i64 %t67, 0
br i1 %t68, label %then69, label %endif71
then69:
br label %loop.end56
endif71:
%t72 = load i64, i64* %lexer.addr
%t73 = call i64 @lexAdvance(i64 %t72)
br label %loop.body55
loop.end56:
br label %endif54
else53:
%t74 = load i64, i64* %next.addr
%t75 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str23, i64 0, i64 0), i64 1)
%t76 = call i64 @rey_str_eq(i8* %t74, i8* %t75)
%t77 = icmp ne i64 %t76, 0
br i1 %t77, label %then78, label %else79
then78:
%t81 = load i64, i64* %lexer.addr
%t82 = call i64 @lexAdvance(i64 %t81)
%t83 = load i64, i64* %lexer.addr
%t84 = call i64 @lexAdvance(i64 %t83)
br label %loop.body85
loop.body85:
%t87 = load i64, i64* %lexer.addr
%t88 = load i64, i64* %lexer.addr
%t89 = call i64 @rey_vec_len(i64 0)
%t90 = icmp sge i64 0, %t89
br i1 %t90, label %then91, label %endif93
then91:
%t94 = load i64, i64* %lexer.addr
%t95 = call i8* @rey_str_new(i8* getelementptr inbounds ([20 x i8], [20 x i8]* @.str35, i64 0, i64 0), i64 19)
%t96 = call i8* @rey_str_new(i8* getelementptr inbounds ([27 x i8], [27 x i8]* @.str36, i64 0, i64 0), i64 26)
%t97 = call i64 @addError(i64 %t94, i8* %t95, i8* %t96)
br label %loop.end86
endif93:
%t98 = load i64, i64* %lexer.addr
%t99 = call i64 @lexPeek(i64 %t98)
%t100 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str23, i64 0, i64 0), i64 1)
%t101 = call i64 @rey_str_eq(i8* %t99, i8* %t100)
%t102 = icmp ne i64 %t101, 0
%t103 = load i64, i64* %lexer.addr
%t104 = call i64 @lexPeekNext(i64 %t103)
%t105 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str24, i64 0, i64 0), i64 1)
%t106 = call i64 @rey_str_eq(i8* %t104, i8* %t105)
%t107 = icmp ne i64 %t106, 0
%t108 = and i1 %t102, %t107
br i1 %t108, label %then109, label %endif111
then109:
%t112 = load i64, i64* %lexer.addr
%t113 = call i64 @lexAdvance(i64 %t112)
%t114 = load i64, i64* %lexer.addr
%t115 = call i64 @lexAdvance(i64 %t114)
br label %loop.end86
endif111:
%t116 = load i64, i64* %lexer.addr
%t117 = call i64 @lexAdvance(i64 %t116)
br label %loop.body85
loop.end86:
br label %endif80
else79:
br label %loop.end1
endif80:
br label %endif54
endif54:
br label %endif45
else44:
br label %loop.end1
endif45:
br label %endif36
endif36:
br label %endif27
endif27:
br label %loop.body0
loop.end1:
ret i64 0
}

define i8* @scanString(i64 %lexer) {
entry:
%lexer.addr = alloca i64, align 8
store i64 %lexer, i64* %lexer.addr
%t0 = call i8* @rey_str_new(i8* getelementptr inbounds ([1 x i8], [1 x i8]* @.str34, i64 0, i64 0), i64 0)
%result.addr = alloca i8*, align 8
store i8* %t0, i8** %result.addr
br label %loop.body1
loop.body1:
%t3 = load i64, i64* %lexer.addr
%t4 = load i64, i64* %lexer.addr
%t5 = call i64 @rey_vec_len(i64 0)
%t6 = icmp sge i64 0, %t5
br i1 %t6, label %then7, label %endif9
then7:
%t10 = load i64, i64* %lexer.addr
%t11 = call i8* @rey_str_new(i8* getelementptr inbounds ([19 x i8], [19 x i8]* @.str37, i64 0, i64 0), i64 18)
%t12 = call i8* @rey_str_new(i8* getelementptr inbounds ([28 x i8], [28 x i8]* @.str38, i64 0, i64 0), i64 27)
%t13 = call i64 @addError(i64 %t10, i8* %t11, i8* %t12)
br label %loop.end2
endif9:
%t14 = load i64, i64* %lexer.addr
%t15 = call i64 @lexAdvance(i64 %t14)
%c.addr = alloca i64, align 8
store i64 %t15, i64* %c.addr
%t16 = load i64, i64* %c.addr
%t17 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str39, i64 0, i64 0), i64 1)
%t18 = call i64 @rey_str_eq(i8* %t16, i8* %t17)
%t19 = icmp ne i64 %t18, 0
br i1 %t19, label %then20, label %endif22
then20:
br label %loop.end2
endif22:
%t23 = load i64, i64* %c.addr
%t24 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str40, i64 0, i64 0), i64 1)
%t25 = call i64 @rey_str_eq(i8* %t23, i8* %t24)
%t26 = icmp ne i64 %t25, 0
br i1 %t26, label %then27, label %else28
then27:
%t30 = load i64, i64* %lexer.addr
%t31 = call i64 @lexPeek(i64 %t30)
%next.addr = alloca i64, align 8
store i64 %t31, i64* %next.addr
%t32 = load i64, i64* %next.addr
%t33 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str41, i64 0, i64 0), i64 1)
%t34 = call i64 @rey_str_eq(i8* %t32, i8* %t33)
%t35 = icmp ne i64 %t34, 0
br i1 %t35, label %then36, label %else37
then36:
%t39 = load i8*, i8** %result.addr
%t40 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str8, i64 0, i64 0), i64 1)
%t41 = call i8* @rey_str_concat(i8* %t39, i8* %t40)
store i8* %t41, i8** %result.addr
%t42 = load i64, i64* %lexer.addr
%t43 = call i64 @lexAdvance(i64 %t42)
br label %endif38
else37:
%t44 = load i64, i64* %next.addr
%t45 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str42, i64 0, i64 0), i64 1)
%t46 = call i64 @rey_str_eq(i8* %t44, i8* %t45)
%t47 = icmp ne i64 %t46, 0
br i1 %t47, label %then48, label %else49
then48:
%t51 = load i8*, i8** %result.addr
%t52 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str9, i64 0, i64 0), i64 1)
%t53 = call i8* @rey_str_concat(i8* %t51, i8* %t52)
store i8* %t53, i8** %result.addr
%t54 = load i64, i64* %lexer.addr
%t55 = call i64 @lexAdvance(i64 %t54)
br label %endif50
else49:
%t56 = load i64, i64* %next.addr
%t57 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str43, i64 0, i64 0), i64 1)
%t58 = call i64 @rey_str_eq(i8* %t56, i8* %t57)
%t59 = icmp ne i64 %t58, 0
br i1 %t59, label %then60, label %else61
then60:
%t63 = load i8*, i8** %result.addr
%t64 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str10, i64 0, i64 0), i64 1)
%t65 = call i8* @rey_str_concat(i8* %t63, i8* %t64)
store i8* %t65, i8** %result.addr
%t66 = load i64, i64* %lexer.addr
%t67 = call i64 @lexAdvance(i64 %t66)
br label %endif62
else61:
%t68 = load i64, i64* %next.addr
%t69 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str40, i64 0, i64 0), i64 1)
%t70 = call i64 @rey_str_eq(i8* %t68, i8* %t69)
%t71 = icmp ne i64 %t70, 0
br i1 %t71, label %then72, label %else73
then72:
%t75 = load i8*, i8** %result.addr
%t76 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str40, i64 0, i64 0), i64 1)
%t77 = call i8* @rey_str_concat(i8* %t75, i8* %t76)
store i8* %t77, i8** %result.addr
%t78 = load i64, i64* %lexer.addr
%t79 = call i64 @lexAdvance(i64 %t78)
br label %endif74
else73:
%t80 = load i64, i64* %next.addr
%t81 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str39, i64 0, i64 0), i64 1)
%t82 = call i64 @rey_str_eq(i8* %t80, i8* %t81)
%t83 = icmp ne i64 %t82, 0
br i1 %t83, label %then84, label %else85
then84:
%t87 = load i8*, i8** %result.addr
%t88 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str39, i64 0, i64 0), i64 1)
%t89 = call i8* @rey_str_concat(i8* %t87, i8* %t88)
store i8* %t89, i8** %result.addr
%t90 = load i64, i64* %lexer.addr
%t91 = call i64 @lexAdvance(i64 %t90)
br label %endif86
else85:
%t92 = load i8*, i8** %result.addr
%t93 = load i64, i64* %next.addr
%t95 = call i8* @rey_int_to_str(i64 %t93)
%t94 = call i8* @rey_str_concat(i8* %t92, i8* %t95)
store i8* %t94, i8** %result.addr
%t96 = load i64, i64* %lexer.addr
%t97 = call i64 @lexAdvance(i64 %t96)
br label %endif86
endif86:
br label %endif74
endif74:
br label %endif62
endif62:
br label %endif50
endif50:
br label %endif38
endif38:
br label %endif29
else28:
%t98 = load i8*, i8** %result.addr
%t99 = load i64, i64* %c.addr
%t101 = call i8* @rey_int_to_str(i64 %t99)
%t100 = call i8* @rey_str_concat(i8* %t98, i8* %t101)
store i8* %t100, i8** %result.addr
br label %endif29
endif29:
br label %loop.body1
loop.end2:
%t102 = load i8*, i8** %result.addr
ret i8* %t102
ret i64 0
}

define i8* @scanChar(i64 %lexer) {
entry:
%lexer.addr = alloca i64, align 8
store i64 %lexer, i64* %lexer.addr
%t0 = call i8* @rey_str_new(i8* getelementptr inbounds ([1 x i8], [1 x i8]* @.str34, i64 0, i64 0), i64 0)
%result.addr = alloca i8*, align 8
store i8* %t0, i8** %result.addr
br label %loop.body1
loop.body1:
%t3 = load i64, i64* %lexer.addr
%t4 = load i64, i64* %lexer.addr
%t5 = call i64 @rey_vec_len(i64 0)
%t6 = icmp sge i64 0, %t5
br i1 %t6, label %then7, label %endif9
then7:
%t10 = load i64, i64* %lexer.addr
%t11 = call i8* @rey_str_new(i8* getelementptr inbounds ([17 x i8], [17 x i8]* @.str44, i64 0, i64 0), i64 16)
%t12 = call i8* @rey_str_new(i8* getelementptr inbounds ([26 x i8], [26 x i8]* @.str45, i64 0, i64 0), i64 25)
%t13 = call i64 @addError(i64 %t10, i8* %t11, i8* %t12)
br label %loop.end2
endif9:
%t14 = load i64, i64* %lexer.addr
%t15 = call i64 @lexAdvance(i64 %t14)
%c.addr = alloca i64, align 8
store i64 %t15, i64* %c.addr
%t16 = load i64, i64* %c.addr
%t17 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str46, i64 0, i64 0), i64 1)
%t18 = call i64 @rey_str_eq(i8* %t16, i8* %t17)
%t19 = icmp ne i64 %t18, 0
br i1 %t19, label %then20, label %endif22
then20:
br label %loop.end2
endif22:
%t23 = load i64, i64* %c.addr
%t24 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str40, i64 0, i64 0), i64 1)
%t25 = call i64 @rey_str_eq(i8* %t23, i8* %t24)
%t26 = icmp ne i64 %t25, 0
br i1 %t26, label %then27, label %else28
then27:
%t30 = load i64, i64* %lexer.addr
%t31 = call i64 @lexPeek(i64 %t30)
%next.addr = alloca i64, align 8
store i64 %t31, i64* %next.addr
%t32 = load i64, i64* %next.addr
%t33 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str41, i64 0, i64 0), i64 1)
%t34 = call i64 @rey_str_eq(i8* %t32, i8* %t33)
%t35 = icmp ne i64 %t34, 0
br i1 %t35, label %then36, label %else37
then36:
%t39 = load i8*, i8** %result.addr
%t40 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str8, i64 0, i64 0), i64 1)
%t41 = call i8* @rey_str_concat(i8* %t39, i8* %t40)
store i8* %t41, i8** %result.addr
%t42 = load i64, i64* %lexer.addr
%t43 = call i64 @lexAdvance(i64 %t42)
br label %endif38
else37:
%t44 = load i64, i64* %next.addr
%t45 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str42, i64 0, i64 0), i64 1)
%t46 = call i64 @rey_str_eq(i8* %t44, i8* %t45)
%t47 = icmp ne i64 %t46, 0
br i1 %t47, label %then48, label %else49
then48:
%t51 = load i8*, i8** %result.addr
%t52 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str9, i64 0, i64 0), i64 1)
%t53 = call i8* @rey_str_concat(i8* %t51, i8* %t52)
store i8* %t53, i8** %result.addr
%t54 = load i64, i64* %lexer.addr
%t55 = call i64 @lexAdvance(i64 %t54)
br label %endif50
else49:
%t56 = load i64, i64* %next.addr
%t57 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str40, i64 0, i64 0), i64 1)
%t58 = call i64 @rey_str_eq(i8* %t56, i8* %t57)
%t59 = icmp ne i64 %t58, 0
br i1 %t59, label %then60, label %else61
then60:
%t63 = load i8*, i8** %result.addr
%t64 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str40, i64 0, i64 0), i64 1)
%t65 = call i8* @rey_str_concat(i8* %t63, i8* %t64)
store i8* %t65, i8** %result.addr
%t66 = load i64, i64* %lexer.addr
%t67 = call i64 @lexAdvance(i64 %t66)
br label %endif62
else61:
%t68 = load i64, i64* %next.addr
%t69 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str46, i64 0, i64 0), i64 1)
%t70 = call i64 @rey_str_eq(i8* %t68, i8* %t69)
%t71 = icmp ne i64 %t70, 0
br i1 %t71, label %then72, label %endif74
then72:
%t75 = load i8*, i8** %result.addr
%t76 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str46, i64 0, i64 0), i64 1)
%t77 = call i8* @rey_str_concat(i8* %t75, i8* %t76)
store i8* %t77, i8** %result.addr
%t78 = load i64, i64* %lexer.addr
%t79 = call i64 @lexAdvance(i64 %t78)
br label %endif74
endif74:
br label %endif62
endif62:
br label %endif50
endif50:
br label %endif38
endif38:
br label %endif29
else28:
%t80 = load i8*, i8** %result.addr
%t81 = load i64, i64* %c.addr
%t83 = call i8* @rey_int_to_str(i64 %t81)
%t82 = call i8* @rey_str_concat(i8* %t80, i8* %t83)
store i8* %t82, i8** %result.addr
br label %endif29
endif29:
br label %loop.body1
loop.end2:
%t84 = load i8*, i8** %result.addr
ret i8* %t84
ret i64 0
}

define i8* @scanNumber(i64 %lexer) {
entry:
%lexer.addr = alloca i64, align 8
store i64 %lexer, i64* %lexer.addr
%t0 = call i8* @rey_str_new(i8* getelementptr inbounds ([1 x i8], [1 x i8]* @.str34, i64 0, i64 0), i64 0)
%result.addr = alloca i8*, align 8
store i8* %t0, i8** %result.addr
%hasDot.addr = alloca i1, align 8
store i1 0, i1* %hasDot.addr
br label %loop.body1
loop.body1:
%t3 = load i64, i64* %lexer.addr
%t4 = load i64, i64* %lexer.addr
%t5 = call i64 @rey_vec_len(i64 0)
%t6 = icmp sge i64 0, %t5
br i1 %t6, label %then7, label %endif9
then7:
br label %loop.end2
endif9:
%t10 = load i64, i64* %lexer.addr
%t11 = call i64 @lexPeek(i64 %t10)
%c.addr = alloca i64, align 8
store i64 %t11, i64* %c.addr
%t12 = load i64, i64* %c.addr
%t13 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str18, i64 0, i64 0), i64 1)
%t14 = call i64 @rey_str_eq(i8* %t12, i8* %t13)
%t15 = icmp ne i64 %t14, 0
br i1 %t15, label %then16, label %else17
then16:
%t19 = load i64, i64* %lexer.addr
%t20 = call i64 @lexPeekNext(i64 %t19)
%next.addr = alloca i64, align 8
store i64 %t20, i64* %next.addr
%t21 = load i64, i64* %next.addr
%t22 = call i64 @isDigit(i64 %t21)
%t23 = load i1, i1* %hasDot.addr
%t24 = xor i1 %t23, true
%t25 = icmp ne i64 %t22, 0
%t26 = and i1 %t25, %t24
br i1 %t26, label %then27, label %else28
then27:
%t30 = load i8*, i8** %result.addr
%t31 = load i64, i64* %c.addr
%t33 = call i8* @rey_int_to_str(i64 %t31)
%t32 = call i8* @rey_str_concat(i8* %t30, i8* %t33)
store i8* %t32, i8** %result.addr
%t34 = load i64, i64* %lexer.addr
%t35 = call i64 @lexAdvance(i64 %t34)
store i1 1, i1* %hasDot.addr
br label %endif29
else28:
br label %loop.end2
endif29:
br label %endif18
else17:
%t36 = load i64, i64* %c.addr
%t37 = call i64 @isDigit(i64 %t36)
%t38 = icmp ne i64 %t37, 0
br i1 %t38, label %then39, label %else40
then39:
%t42 = load i8*, i8** %result.addr
%t43 = load i64, i64* %c.addr
%t45 = call i8* @rey_int_to_str(i64 %t43)
%t44 = call i8* @rey_str_concat(i8* %t42, i8* %t45)
store i8* %t44, i8** %result.addr
%t46 = load i64, i64* %lexer.addr
%t47 = call i64 @lexAdvance(i64 %t46)
br label %endif41
else40:
%t48 = load i64, i64* %c.addr
%t49 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str47, i64 0, i64 0), i64 1)
%t50 = call i64 @rey_str_eq(i8* %t48, i8* %t49)
%t51 = icmp ne i64 %t50, 0
%t52 = load i64, i64* %c.addr
%t53 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str48, i64 0, i64 0), i64 1)
%t54 = call i64 @rey_str_eq(i8* %t52, i8* %t53)
%t55 = icmp ne i64 %t54, 0
%t56 = or i1 %t51, %t55
br i1 %t56, label %then57, label %else58
then57:
%t60 = load i8*, i8** %result.addr
%t61 = load i64, i64* %c.addr
%t63 = call i8* @rey_int_to_str(i64 %t61)
%t62 = call i8* @rey_str_concat(i8* %t60, i8* %t63)
store i8* %t62, i8** %result.addr
%t64 = load i64, i64* %lexer.addr
%t65 = call i64 @lexAdvance(i64 %t64)
%t66 = load i64, i64* %lexer.addr
%t67 = call i64 @lexPeek(i64 %t66)
%next.addr = alloca i64, align 8
store i64 %t67, i64* %next.addr
%t68 = load i64, i64* %next.addr
%t69 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str21, i64 0, i64 0), i64 1)
%t70 = call i64 @rey_str_eq(i8* %t68, i8* %t69)
%t71 = icmp ne i64 %t70, 0
%t72 = load i64, i64* %next.addr
%t73 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str22, i64 0, i64 0), i64 1)
%t74 = call i64 @rey_str_eq(i8* %t72, i8* %t73)
%t75 = icmp ne i64 %t74, 0
%t76 = or i1 %t71, %t75
br i1 %t76, label %then77, label %endif79
then77:
%t80 = load i8*, i8** %result.addr
%t81 = load i64, i64* %next.addr
%t83 = call i8* @rey_int_to_str(i64 %t81)
%t82 = call i8* @rey_str_concat(i8* %t80, i8* %t83)
store i8* %t82, i8** %result.addr
%t84 = load i64, i64* %lexer.addr
%t85 = call i64 @lexAdvance(i64 %t84)
br label %endif79
endif79:
br label %endif59
else58:
br label %loop.end2
endif59:
br label %endif41
endif41:
br label %endif18
endif18:
br label %loop.body1
loop.end2:
%t86 = load i8*, i8** %result.addr
ret i8* %t86
ret i64 0
}

define i8* @scanIdentifier(i64 %lexer) {
entry:
%lexer.addr = alloca i64, align 8
store i64 %lexer, i64* %lexer.addr
%t0 = call i8* @rey_str_new(i8* getelementptr inbounds ([1 x i8], [1 x i8]* @.str34, i64 0, i64 0), i64 0)
%result.addr = alloca i8*, align 8
store i8* %t0, i8** %result.addr
br label %loop.body1
loop.body1:
%t3 = load i64, i64* %lexer.addr
%t4 = load i64, i64* %lexer.addr
%t5 = call i64 @rey_vec_len(i64 0)
%t6 = icmp sge i64 0, %t5
br i1 %t6, label %then7, label %endif9
then7:
br label %loop.end2
endif9:
%t10 = load i64, i64* %lexer.addr
%t11 = call i64 @lexPeek(i64 %t10)
%c.addr = alloca i64, align 8
store i64 %t11, i64* %c.addr
%t12 = load i64, i64* %c.addr
%t13 = call i64 @isAlphaNumeric(i64 %t12)
%t14 = icmp ne i64 %t13, 0
br i1 %t14, label %then15, label %else16
then15:
%t18 = load i8*, i8** %result.addr
%t19 = load i64, i64* %c.addr
%t21 = call i8* @rey_int_to_str(i64 %t19)
%t20 = call i8* @rey_str_concat(i8* %t18, i8* %t21)
store i8* %t20, i8** %result.addr
%t22 = load i64, i64* %lexer.addr
%t23 = call i64 @lexAdvance(i64 %t22)
br label %endif17
else16:
br label %loop.end2
endif17:
br label %loop.body1
loop.end2:
%t24 = load i8*, i8** %result.addr
ret i8* %t24
ret i64 0
}

define i64 @getKeyword(i8* %kind) {
entry:
%kind.addr = alloca i8*, align 8
store i8* %kind, i8** %kind.addr
%t0 = load i8*, i8** %kind.addr
%t1 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str49, i64 0, i64 0), i64 3)
%t2 = call i64 @rey_str_eq(i8* %t0, i8* %t1)
%t3 = icmp ne i64 %t2, 0
br i1 %t3, label %then4, label %endif6
then4:
%t7 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str50, i64 0, i64 0), i64 3)
ret i64 %t7
endif6:
%t8 = load i8*, i8** %kind.addr
%t9 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str51, i64 0, i64 0), i64 5)
%t10 = call i64 @rey_str_eq(i8* %t8, i8* %t9)
%t11 = icmp ne i64 %t10, 0
br i1 %t11, label %then12, label %endif14
then12:
%t15 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str52, i64 0, i64 0), i64 5)
ret i64 %t15
endif14:
%t16 = load i8*, i8** %kind.addr
%t17 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str53, i64 0, i64 0), i64 4)
%t18 = call i64 @rey_str_eq(i8* %t16, i8* %t17)
%t19 = icmp ne i64 %t18, 0
br i1 %t19, label %then20, label %endif22
then20:
%t23 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str54, i64 0, i64 0), i64 4)
ret i64 %t23
endif22:
%t24 = load i8*, i8** %kind.addr
%t25 = call i8* @rey_str_new(i8* getelementptr inbounds ([7 x i8], [7 x i8]* @.str55, i64 0, i64 0), i64 6)
%t26 = call i64 @rey_str_eq(i8* %t24, i8* %t25)
%t27 = icmp ne i64 %t26, 0
br i1 %t27, label %then28, label %endif30
then28:
%t31 = call i8* @rey_str_new(i8* getelementptr inbounds ([7 x i8], [7 x i8]* @.str56, i64 0, i64 0), i64 6)
ret i64 %t31
endif30:
%t32 = load i8*, i8** %kind.addr
%t33 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str57, i64 0, i64 0), i64 4)
%t34 = call i64 @rey_str_eq(i8* %t32, i8* %t33)
%t35 = icmp ne i64 %t34, 0
br i1 %t35, label %then36, label %endif38
then36:
%t39 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str58, i64 0, i64 0), i64 4)
ret i64 %t39
endif38:
%t40 = load i8*, i8** %kind.addr
%t41 = call i8* @rey_str_new(i8* getelementptr inbounds ([7 x i8], [7 x i8]* @.str59, i64 0, i64 0), i64 6)
%t42 = call i64 @rey_str_eq(i8* %t40, i8* %t41)
%t43 = icmp ne i64 %t42, 0
br i1 %t43, label %then44, label %endif46
then44:
%t47 = call i8* @rey_str_new(i8* getelementptr inbounds ([7 x i8], [7 x i8]* @.str60, i64 0, i64 0), i64 6)
ret i64 %t47
endif46:
%t48 = load i8*, i8** %kind.addr
%t49 = call i8* @rey_str_new(i8* getelementptr inbounds ([7 x i8], [7 x i8]* @.str61, i64 0, i64 0), i64 6)
%t50 = call i64 @rey_str_eq(i8* %t48, i8* %t49)
%t51 = icmp ne i64 %t50, 0
br i1 %t51, label %then52, label %endif54
then52:
%t55 = call i8* @rey_str_new(i8* getelementptr inbounds ([7 x i8], [7 x i8]* @.str62, i64 0, i64 0), i64 6)
ret i64 %t55
endif54:
%t56 = load i8*, i8** %kind.addr
%t57 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str63, i64 0, i64 0), i64 3)
%t58 = call i64 @rey_str_eq(i8* %t56, i8* %t57)
%t59 = icmp ne i64 %t58, 0
br i1 %t59, label %then60, label %endif62
then60:
%t63 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str64, i64 0, i64 0), i64 3)
ret i64 %t63
endif62:
%t64 = load i8*, i8** %kind.addr
%t65 = call i8* @rey_str_new(i8* getelementptr inbounds ([7 x i8], [7 x i8]* @.str65, i64 0, i64 0), i64 6)
%t66 = call i64 @rey_str_eq(i8* %t64, i8* %t65)
%t67 = icmp ne i64 %t66, 0
br i1 %t67, label %then68, label %endif70
then68:
%t71 = call i8* @rey_str_new(i8* getelementptr inbounds ([7 x i8], [7 x i8]* @.str66, i64 0, i64 0), i64 6)
ret i64 %t71
endif70:
%t72 = load i8*, i8** %kind.addr
%t73 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str67, i64 0, i64 0), i64 2)
%t74 = call i64 @rey_str_eq(i8* %t72, i8* %t73)
%t75 = icmp ne i64 %t74, 0
br i1 %t75, label %then76, label %endif78
then76:
%t79 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str68, i64 0, i64 0), i64 2)
ret i64 %t79
endif78:
%t80 = load i8*, i8** %kind.addr
%t81 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str69, i64 0, i64 0), i64 4)
%t82 = call i64 @rey_str_eq(i8* %t80, i8* %t81)
%t83 = icmp ne i64 %t82, 0
br i1 %t83, label %then84, label %endif86
then84:
%t87 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str70, i64 0, i64 0), i64 4)
ret i64 %t87
endif86:
%t88 = load i8*, i8** %kind.addr
%t89 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str71, i64 0, i64 0), i64 5)
%t90 = call i64 @rey_str_eq(i8* %t88, i8* %t89)
%t91 = icmp ne i64 %t90, 0
br i1 %t91, label %then92, label %endif94
then92:
%t95 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str72, i64 0, i64 0), i64 5)
ret i64 %t95
endif94:
%t96 = load i8*, i8** %kind.addr
%t97 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str73, i64 0, i64 0), i64 3)
%t98 = call i64 @rey_str_eq(i8* %t96, i8* %t97)
%t99 = icmp ne i64 %t98, 0
br i1 %t99, label %then100, label %endif102
then100:
%t103 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str74, i64 0, i64 0), i64 3)
ret i64 %t103
endif102:
%t104 = load i8*, i8** %kind.addr
%t105 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str75, i64 0, i64 0), i64 2)
%t106 = call i64 @rey_str_eq(i8* %t104, i8* %t105)
%t107 = icmp ne i64 %t106, 0
br i1 %t107, label %then108, label %endif110
then108:
%t111 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str76, i64 0, i64 0), i64 2)
ret i64 %t111
endif110:
%t112 = load i8*, i8** %kind.addr
%t113 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str77, i64 0, i64 0), i64 4)
%t114 = call i64 @rey_str_eq(i8* %t112, i8* %t113)
%t115 = icmp ne i64 %t114, 0
br i1 %t115, label %then116, label %endif118
then116:
%t119 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str78, i64 0, i64 0), i64 4)
ret i64 %t119
endif118:
%t120 = load i8*, i8** %kind.addr
%t121 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str79, i64 0, i64 0), i64 5)
%t122 = call i64 @rey_str_eq(i8* %t120, i8* %t121)
%t123 = icmp ne i64 %t122, 0
br i1 %t123, label %then124, label %endif126
then124:
%t127 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str80, i64 0, i64 0), i64 5)
ret i64 %t127
endif126:
%t128 = load i8*, i8** %kind.addr
%t129 = call i8* @rey_str_new(i8* getelementptr inbounds ([9 x i8], [9 x i8]* @.str81, i64 0, i64 0), i64 8)
%t130 = call i64 @rey_str_eq(i8* %t128, i8* %t129)
%t131 = icmp ne i64 %t130, 0
br i1 %t131, label %then132, label %endif134
then132:
%t135 = call i8* @rey_str_new(i8* getelementptr inbounds ([9 x i8], [9 x i8]* @.str82, i64 0, i64 0), i64 8)
ret i64 %t135
endif134:
%t136 = load i8*, i8** %kind.addr
%t137 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str83, i64 0, i64 0), i64 5)
%t138 = call i64 @rey_str_eq(i8* %t136, i8* %t137)
%t139 = icmp ne i64 %t138, 0
br i1 %t139, label %then140, label %endif142
then140:
%t143 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str84, i64 0, i64 0), i64 5)
ret i64 %t143
endif142:
%t144 = load i8*, i8** %kind.addr
%t145 = call i8* @rey_str_new(i8* getelementptr inbounds ([11 x i8], [11 x i8]* @.str85, i64 0, i64 0), i64 10)
%t146 = call i64 @rey_str_eq(i8* %t144, i8* %t145)
%t147 = icmp ne i64 %t146, 0
br i1 %t147, label %then148, label %endif150
then148:
%t151 = call i8* @rey_str_new(i8* getelementptr inbounds ([11 x i8], [11 x i8]* @.str86, i64 0, i64 0), i64 10)
ret i64 %t151
endif150:
%t152 = load i8*, i8** %kind.addr
%t153 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str87, i64 0, i64 0), i64 4)
%t154 = call i64 @rey_str_eq(i8* %t152, i8* %t153)
%t155 = icmp ne i64 %t154, 0
br i1 %t155, label %then156, label %endif158
then156:
%t159 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str88, i64 0, i64 0), i64 4)
ret i64 %t159
endif158:
%t160 = load i8*, i8** %kind.addr
%t161 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str89, i64 0, i64 0), i64 5)
%t162 = call i64 @rey_str_eq(i8* %t160, i8* %t161)
%t163 = icmp ne i64 %t162, 0
br i1 %t163, label %then164, label %endif166
then164:
%t167 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str90, i64 0, i64 0), i64 5)
ret i64 %t167
endif166:
%t168 = load i8*, i8** %kind.addr
%t169 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str91, i64 0, i64 0), i64 4)
%t170 = call i64 @rey_str_eq(i8* %t168, i8* %t169)
%t171 = icmp ne i64 %t170, 0
br i1 %t171, label %then172, label %endif174
then172:
%t175 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str92, i64 0, i64 0), i64 4)
ret i64 %t175
endif174:
%t176 = call i8* @rey_str_new(i8* getelementptr inbounds ([11 x i8], [11 x i8]* @.str93, i64 0, i64 0), i64 10)
ret i64 %t176
ret i64 0
}

define i64 @scanToken(i64 %lexer) {
entry:
%lexer.addr = alloca i64, align 8
store i64 %lexer, i64* %lexer.addr
%t0 = load i64, i64* %lexer.addr
%t1 = call i64 @lexAdvance(i64 %t0)
%c.addr = alloca i64, align 8
store i64 %t1, i64* %c.addr
%t2 = load i64, i64* %c.addr
%t3 = call i8* @rey_str_new(i8* getelementptr inbounds ([1 x i8], [1 x i8]* @.str34, i64 0, i64 0), i64 0)
%t4 = call i64 @rey_str_eq(i8* %t2, i8* %t3)
%t5 = icmp ne i64 %t4, 0
br i1 %t5, label %then6, label %endif8
then6:
%t9 = load i64, i64* %lexer.addr
%t10 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str94, i64 0, i64 0), i64 3)
%t11 = call i8* @rey_str_new(i8* getelementptr inbounds ([1 x i8], [1 x i8]* @.str34, i64 0, i64 0), i64 0)
%t12 = call i64 @addToken(i64 %t9, i8* %t10, i8* %t11)
ret i64 0
endif8:
%t13 = load i64, i64* %c.addr
%t14 = call i64 @isWhitespace(i64 %t13)
%t15 = icmp ne i64 %t14, 0
br i1 %t15, label %then16, label %endif18
then16:
ret i64 0
endif18:
%t19 = load i64, i64* %c.addr
%t20 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str11, i64 0, i64 0), i64 1)
%t21 = call i64 @rey_str_eq(i8* %t19, i8* %t20)
%t22 = icmp ne i64 %t21, 0
br i1 %t22, label %then23, label %endif25
then23:
%t26 = load i64, i64* %lexer.addr
%t27 = call i8* @rey_str_new(i8* getelementptr inbounds ([10 x i8], [10 x i8]* @.str95, i64 0, i64 0), i64 9)
%t28 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str11, i64 0, i64 0), i64 1)
%t29 = call i64 @addToken(i64 %t26, i8* %t27, i8* %t28)
ret i64 0
endif25:
%t30 = load i64, i64* %c.addr
%t31 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str12, i64 0, i64 0), i64 1)
%t32 = call i64 @rey_str_eq(i8* %t30, i8* %t31)
%t33 = icmp ne i64 %t32, 0
br i1 %t33, label %then34, label %endif36
then34:
%t37 = load i64, i64* %lexer.addr
%t38 = call i8* @rey_str_new(i8* getelementptr inbounds ([11 x i8], [11 x i8]* @.str96, i64 0, i64 0), i64 10)
%t39 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str12, i64 0, i64 0), i64 1)
%t40 = call i64 @addToken(i64 %t37, i8* %t38, i8* %t39)
ret i64 0
endif36:
%t41 = load i64, i64* %c.addr
%t42 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str13, i64 0, i64 0), i64 1)
%t43 = call i64 @rey_str_eq(i8* %t41, i8* %t42)
%t44 = icmp ne i64 %t43, 0
br i1 %t44, label %then45, label %endif47
then45:
%t48 = load i64, i64* %lexer.addr
%t49 = call i8* @rey_str_new(i8* getelementptr inbounds ([10 x i8], [10 x i8]* @.str97, i64 0, i64 0), i64 9)
%t50 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str13, i64 0, i64 0), i64 1)
%t51 = call i64 @addToken(i64 %t48, i8* %t49, i8* %t50)
ret i64 0
endif47:
%t52 = load i64, i64* %c.addr
%t53 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str14, i64 0, i64 0), i64 1)
%t54 = call i64 @rey_str_eq(i8* %t52, i8* %t53)
%t55 = icmp ne i64 %t54, 0
br i1 %t55, label %then56, label %endif58
then56:
%t59 = load i64, i64* %lexer.addr
%t60 = call i8* @rey_str_new(i8* getelementptr inbounds ([11 x i8], [11 x i8]* @.str98, i64 0, i64 0), i64 10)
%t61 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str14, i64 0, i64 0), i64 1)
%t62 = call i64 @addToken(i64 %t59, i8* %t60, i8* %t61)
ret i64 0
endif58:
%t63 = load i64, i64* %c.addr
%t64 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str15, i64 0, i64 0), i64 1)
%t65 = call i64 @rey_str_eq(i8* %t63, i8* %t64)
%t66 = icmp ne i64 %t65, 0
br i1 %t66, label %then67, label %endif69
then67:
%t70 = load i64, i64* %lexer.addr
%t71 = call i8* @rey_str_new(i8* getelementptr inbounds ([12 x i8], [12 x i8]* @.str99, i64 0, i64 0), i64 11)
%t72 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str15, i64 0, i64 0), i64 1)
%t73 = call i64 @addToken(i64 %t70, i8* %t71, i8* %t72)
ret i64 0
endif69:
%t74 = load i64, i64* %c.addr
%t75 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str16, i64 0, i64 0), i64 1)
%t76 = call i64 @rey_str_eq(i8* %t74, i8* %t75)
%t77 = icmp ne i64 %t76, 0
br i1 %t77, label %then78, label %endif80
then78:
%t81 = load i64, i64* %lexer.addr
%t82 = call i8* @rey_str_new(i8* getelementptr inbounds ([13 x i8], [13 x i8]* @.str100, i64 0, i64 0), i64 12)
%t83 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str16, i64 0, i64 0), i64 1)
%t84 = call i64 @addToken(i64 %t81, i8* %t82, i8* %t83)
ret i64 0
endif80:
%t85 = load i64, i64* %c.addr
%t86 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str17, i64 0, i64 0), i64 1)
%t87 = call i64 @rey_str_eq(i8* %t85, i8* %t86)
%t88 = icmp ne i64 %t87, 0
br i1 %t88, label %then89, label %endif91
then89:
%t92 = load i64, i64* %lexer.addr
%t93 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str101, i64 0, i64 0), i64 5)
%t94 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str17, i64 0, i64 0), i64 1)
%t95 = call i64 @addToken(i64 %t92, i8* %t93, i8* %t94)
ret i64 0
endif91:
%t96 = load i64, i64* %c.addr
%t97 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str18, i64 0, i64 0), i64 1)
%t98 = call i64 @rey_str_eq(i8* %t96, i8* %t97)
%t99 = icmp ne i64 %t98, 0
br i1 %t99, label %then100, label %endif102
then100:
%t103 = load i64, i64* %lexer.addr
%t104 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str102, i64 0, i64 0), i64 3)
%t105 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str18, i64 0, i64 0), i64 1)
%t106 = call i64 @addToken(i64 %t103, i8* %t104, i8* %t105)
ret i64 0
endif102:
%t107 = load i64, i64* %c.addr
%t108 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str19, i64 0, i64 0), i64 1)
%t109 = call i64 @rey_str_eq(i8* %t107, i8* %t108)
%t110 = icmp ne i64 %t109, 0
br i1 %t110, label %then111, label %endif113
then111:
%t114 = load i64, i64* %lexer.addr
%t115 = call i8* @rey_str_new(i8* getelementptr inbounds ([10 x i8], [10 x i8]* @.str103, i64 0, i64 0), i64 9)
%t116 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str19, i64 0, i64 0), i64 1)
%t117 = call i64 @addToken(i64 %t114, i8* %t115, i8* %t116)
ret i64 0
endif113:
%t118 = load i64, i64* %c.addr
%t119 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str21, i64 0, i64 0), i64 1)
%t120 = call i64 @rey_str_eq(i8* %t118, i8* %t119)
%t121 = icmp ne i64 %t120, 0
br i1 %t121, label %then122, label %endif124
then122:
%t125 = load i64, i64* %lexer.addr
%t126 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str21, i64 0, i64 0), i64 1)
%t127 = call i64 @lexMatchChar(i64 %t125, i8* %t126)
%t128 = icmp ne i64 %t127, 0
br i1 %t128, label %then129, label %endif131
then129:
%t132 = load i64, i64* %lexer.addr
%t133 = call i8* @rey_str_new(i8* getelementptr inbounds ([9 x i8], [9 x i8]* @.str104, i64 0, i64 0), i64 8)
%t134 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str105, i64 0, i64 0), i64 2)
%t135 = call i64 @addToken(i64 %t132, i8* %t133, i8* %t134)
ret i64 0
endif131:
%t136 = load i64, i64* %lexer.addr
%t137 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str30, i64 0, i64 0), i64 1)
%t138 = call i64 @lexMatchChar(i64 %t136, i8* %t137)
%t139 = icmp ne i64 %t138, 0
br i1 %t139, label %then140, label %endif142
then140:
%t143 = load i64, i64* %lexer.addr
%t144 = call i8* @rey_str_new(i8* getelementptr inbounds ([10 x i8], [10 x i8]* @.str106, i64 0, i64 0), i64 9)
%t145 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str107, i64 0, i64 0), i64 2)
%t146 = call i64 @addToken(i64 %t143, i8* %t144, i8* %t145)
ret i64 0
endif142:
%t147 = load i64, i64* %lexer.addr
%t148 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str108, i64 0, i64 0), i64 4)
%t149 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str21, i64 0, i64 0), i64 1)
%t150 = call i64 @addToken(i64 %t147, i8* %t148, i8* %t149)
ret i64 0
endif124:
%t151 = load i64, i64* %c.addr
%t152 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str22, i64 0, i64 0), i64 1)
%t153 = call i64 @rey_str_eq(i8* %t151, i8* %t152)
%t154 = icmp ne i64 %t153, 0
br i1 %t154, label %then155, label %endif157
then155:
%t158 = load i64, i64* %lexer.addr
%t159 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str22, i64 0, i64 0), i64 1)
%t160 = call i64 @lexMatchChar(i64 %t158, i8* %t159)
%t161 = icmp ne i64 %t160, 0
br i1 %t161, label %then162, label %endif164
then162:
%t165 = load i64, i64* %lexer.addr
%t166 = call i8* @rey_str_new(i8* getelementptr inbounds ([11 x i8], [11 x i8]* @.str109, i64 0, i64 0), i64 10)
%t167 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str110, i64 0, i64 0), i64 2)
%t168 = call i64 @addToken(i64 %t165, i8* %t166, i8* %t167)
ret i64 0
endif164:
%t169 = load i64, i64* %lexer.addr
%t170 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str30, i64 0, i64 0), i64 1)
%t171 = call i64 @lexMatchChar(i64 %t169, i8* %t170)
%t172 = icmp ne i64 %t171, 0
br i1 %t172, label %then173, label %endif175
then173:
%t176 = load i64, i64* %lexer.addr
%t177 = call i8* @rey_str_new(i8* getelementptr inbounds ([11 x i8], [11 x i8]* @.str111, i64 0, i64 0), i64 10)
%t178 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str112, i64 0, i64 0), i64 2)
%t179 = call i64 @addToken(i64 %t176, i8* %t177, i8* %t178)
ret i64 0
endif175:
%t180 = load i64, i64* %lexer.addr
%t181 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str32, i64 0, i64 0), i64 1)
%t182 = call i64 @lexMatchChar(i64 %t180, i8* %t181)
%t183 = icmp ne i64 %t182, 0
br i1 %t183, label %then184, label %endif186
then184:
%t187 = load i64, i64* %lexer.addr
%t188 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str113, i64 0, i64 0), i64 5)
%t189 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str114, i64 0, i64 0), i64 2)
%t190 = call i64 @addToken(i64 %t187, i8* %t188, i8* %t189)
ret i64 0
endif186:
%t191 = load i64, i64* %lexer.addr
%t192 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str115, i64 0, i64 0), i64 5)
%t193 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str22, i64 0, i64 0), i64 1)
%t194 = call i64 @addToken(i64 %t191, i8* %t192, i8* %t193)
ret i64 0
endif157:
%t195 = load i64, i64* %c.addr
%t196 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str23, i64 0, i64 0), i64 1)
%t197 = call i64 @rey_str_eq(i8* %t195, i8* %t196)
%t198 = icmp ne i64 %t197, 0
br i1 %t198, label %then199, label %endif201
then199:
%t202 = load i64, i64* %lexer.addr
%t203 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str30, i64 0, i64 0), i64 1)
%t204 = call i64 @lexMatchChar(i64 %t202, i8* %t203)
%t205 = icmp ne i64 %t204, 0
br i1 %t205, label %then206, label %endif208
then206:
%t209 = load i64, i64* %lexer.addr
%t210 = call i8* @rey_str_new(i8* getelementptr inbounds ([10 x i8], [10 x i8]* @.str116, i64 0, i64 0), i64 9)
%t211 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str117, i64 0, i64 0), i64 2)
%t212 = call i64 @addToken(i64 %t209, i8* %t210, i8* %t211)
ret i64 0
endif208:
%t213 = load i64, i64* %lexer.addr
%t214 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str118, i64 0, i64 0), i64 4)
%t215 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str23, i64 0, i64 0), i64 1)
%t216 = call i64 @addToken(i64 %t213, i8* %t214, i8* %t215)
ret i64 0
endif201:
%t217 = load i64, i64* %c.addr
%t218 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str24, i64 0, i64 0), i64 1)
%t219 = call i64 @rey_str_eq(i8* %t217, i8* %t218)
%t220 = icmp ne i64 %t219, 0
br i1 %t220, label %then221, label %endif223
then221:
%t224 = load i64, i64* %lexer.addr
%t225 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str30, i64 0, i64 0), i64 1)
%t226 = call i64 @lexMatchChar(i64 %t224, i8* %t225)
%t227 = icmp ne i64 %t226, 0
br i1 %t227, label %then228, label %endif230
then228:
%t231 = load i64, i64* %lexer.addr
%t232 = call i8* @rey_str_new(i8* getelementptr inbounds ([11 x i8], [11 x i8]* @.str119, i64 0, i64 0), i64 10)
%t233 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str120, i64 0, i64 0), i64 2)
%t234 = call i64 @addToken(i64 %t231, i8* %t232, i8* %t233)
ret i64 0
endif230:
%t235 = load i64, i64* %lexer.addr
%t236 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str121, i64 0, i64 0), i64 5)
%t237 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str24, i64 0, i64 0), i64 1)
%t238 = call i64 @addToken(i64 %t235, i8* %t236, i8* %t237)
ret i64 0
endif223:
%t239 = load i64, i64* %c.addr
%t240 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str25, i64 0, i64 0), i64 1)
%t241 = call i64 @rey_str_eq(i8* %t239, i8* %t240)
%t242 = icmp ne i64 %t241, 0
br i1 %t242, label %then243, label %endif245
then243:
%t246 = load i64, i64* %lexer.addr
%t247 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str30, i64 0, i64 0), i64 1)
%t248 = call i64 @lexMatchChar(i64 %t246, i8* %t247)
%t249 = icmp ne i64 %t248, 0
br i1 %t249, label %then250, label %endif252
then250:
%t253 = load i64, i64* %lexer.addr
%t254 = call i8* @rey_str_new(i8* getelementptr inbounds ([13 x i8], [13 x i8]* @.str122, i64 0, i64 0), i64 12)
%t255 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str123, i64 0, i64 0), i64 2)
%t256 = call i64 @addToken(i64 %t253, i8* %t254, i8* %t255)
ret i64 0
endif252:
%t257 = load i64, i64* %lexer.addr
%t258 = call i8* @rey_str_new(i8* getelementptr inbounds ([8 x i8], [8 x i8]* @.str124, i64 0, i64 0), i64 7)
%t259 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str25, i64 0, i64 0), i64 1)
%t260 = call i64 @addToken(i64 %t257, i8* %t258, i8* %t259)
ret i64 0
endif245:
%t261 = load i64, i64* %c.addr
%t262 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str30, i64 0, i64 0), i64 1)
%t263 = call i64 @rey_str_eq(i8* %t261, i8* %t262)
%t264 = icmp ne i64 %t263, 0
br i1 %t264, label %then265, label %endif267
then265:
%t268 = load i64, i64* %lexer.addr
%t269 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str30, i64 0, i64 0), i64 1)
%t270 = call i64 @lexMatchChar(i64 %t268, i8* %t269)
%t271 = icmp ne i64 %t270, 0
br i1 %t271, label %then272, label %endif274
then272:
%t275 = load i64, i64* %lexer.addr
%t276 = call i8* @rey_str_new(i8* getelementptr inbounds ([11 x i8], [11 x i8]* @.str125, i64 0, i64 0), i64 10)
%t277 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str126, i64 0, i64 0), i64 2)
%t278 = call i64 @addToken(i64 %t275, i8* %t276, i8* %t277)
ret i64 0
endif274:
%t279 = load i64, i64* %lexer.addr
%t280 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str127, i64 0, i64 0), i64 5)
%t281 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str30, i64 0, i64 0), i64 1)
%t282 = call i64 @addToken(i64 %t279, i8* %t280, i8* %t281)
ret i64 0
endif267:
%t283 = load i64, i64* %c.addr
%t284 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str29, i64 0, i64 0), i64 1)
%t285 = call i64 @rey_str_eq(i8* %t283, i8* %t284)
%t286 = icmp ne i64 %t285, 0
br i1 %t286, label %then287, label %endif289
then287:
%t290 = load i64, i64* %lexer.addr
%t291 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str30, i64 0, i64 0), i64 1)
%t292 = call i64 @lexMatchChar(i64 %t290, i8* %t291)
%t293 = icmp ne i64 %t292, 0
br i1 %t293, label %then294, label %endif296
then294:
%t297 = load i64, i64* %lexer.addr
%t298 = call i8* @rey_str_new(i8* getelementptr inbounds ([9 x i8], [9 x i8]* @.str128, i64 0, i64 0), i64 8)
%t299 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str129, i64 0, i64 0), i64 2)
%t300 = call i64 @addToken(i64 %t297, i8* %t298, i8* %t299)
ret i64 0
endif296:
%t301 = load i64, i64* %lexer.addr
%t302 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str130, i64 0, i64 0), i64 3)
%t303 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str29, i64 0, i64 0), i64 1)
%t304 = call i64 @addToken(i64 %t301, i8* %t302, i8* %t303)
ret i64 0
endif289:
%t305 = load i64, i64* %c.addr
%t306 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str31, i64 0, i64 0), i64 1)
%t307 = call i64 @rey_str_eq(i8* %t305, i8* %t306)
%t308 = icmp ne i64 %t307, 0
br i1 %t308, label %then309, label %endif311
then309:
%t312 = load i64, i64* %lexer.addr
%t313 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str30, i64 0, i64 0), i64 1)
%t314 = call i64 @lexMatchChar(i64 %t312, i8* %t313)
%t315 = icmp ne i64 %t314, 0
br i1 %t315, label %then316, label %endif318
then316:
%t319 = load i64, i64* %lexer.addr
%t320 = call i8* @rey_str_new(i8* getelementptr inbounds ([10 x i8], [10 x i8]* @.str131, i64 0, i64 0), i64 9)
%t321 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str132, i64 0, i64 0), i64 2)
%t322 = call i64 @addToken(i64 %t319, i8* %t320, i8* %t321)
ret i64 0
endif318:
%t323 = load i64, i64* %lexer.addr
%t324 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str133, i64 0, i64 0), i64 4)
%t325 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str31, i64 0, i64 0), i64 1)
%t326 = call i64 @addToken(i64 %t323, i8* %t324, i8* %t325)
ret i64 0
endif311:
%t327 = load i64, i64* %c.addr
%t328 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str32, i64 0, i64 0), i64 1)
%t329 = call i64 @rey_str_eq(i8* %t327, i8* %t328)
%t330 = icmp ne i64 %t329, 0
br i1 %t330, label %then331, label %endif333
then331:
%t334 = load i64, i64* %lexer.addr
%t335 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str30, i64 0, i64 0), i64 1)
%t336 = call i64 @lexMatchChar(i64 %t334, i8* %t335)
%t337 = icmp ne i64 %t336, 0
br i1 %t337, label %then338, label %endif340
then338:
%t341 = load i64, i64* %lexer.addr
%t342 = call i8* @rey_str_new(i8* getelementptr inbounds ([13 x i8], [13 x i8]* @.str134, i64 0, i64 0), i64 12)
%t343 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str135, i64 0, i64 0), i64 2)
%t344 = call i64 @addToken(i64 %t341, i8* %t342, i8* %t343)
ret i64 0
endif340:
%t345 = load i64, i64* %lexer.addr
%t346 = call i8* @rey_str_new(i8* getelementptr inbounds ([8 x i8], [8 x i8]* @.str136, i64 0, i64 0), i64 7)
%t347 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str32, i64 0, i64 0), i64 1)
%t348 = call i64 @addToken(i64 %t345, i8* %t346, i8* %t347)
ret i64 0
endif333:
%t349 = load i64, i64* %c.addr
%t350 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str20, i64 0, i64 0), i64 1)
%t351 = call i64 @rey_str_eq(i8* %t349, i8* %t350)
%t352 = icmp ne i64 %t351, 0
br i1 %t352, label %then353, label %endif355
then353:
%t356 = load i64, i64* %lexer.addr
%t357 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str20, i64 0, i64 0), i64 1)
%t358 = call i64 @lexMatchChar(i64 %t356, i8* %t357)
%t359 = icmp ne i64 %t358, 0
br i1 %t359, label %then360, label %endif362
then360:
%t363 = load i64, i64* %lexer.addr
%t364 = call i8* @rey_str_new(i8* getelementptr inbounds ([11 x i8], [11 x i8]* @.str137, i64 0, i64 0), i64 10)
%t365 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str138, i64 0, i64 0), i64 2)
%t366 = call i64 @addToken(i64 %t363, i8* %t364, i8* %t365)
ret i64 0
endif362:
%t367 = load i64, i64* %lexer.addr
%t368 = call i8* @rey_str_new(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str139, i64 0, i64 0), i64 5)
%t369 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str20, i64 0, i64 0), i64 1)
%t370 = call i64 @addToken(i64 %t367, i8* %t368, i8* %t369)
ret i64 0
endif355:
%t371 = load i64, i64* %c.addr
%t372 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str27, i64 0, i64 0), i64 1)
%t373 = call i64 @rey_str_eq(i8* %t371, i8* %t372)
%t374 = icmp ne i64 %t373, 0
br i1 %t374, label %then375, label %endif377
then375:
%t378 = load i64, i64* %lexer.addr
%t379 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str27, i64 0, i64 0), i64 1)
%t380 = call i64 @lexMatchChar(i64 %t378, i8* %t379)
%t381 = icmp ne i64 %t380, 0
br i1 %t381, label %then382, label %endif384
then382:
%t385 = load i64, i64* %lexer.addr
%t386 = call i8* @rey_str_new(i8* getelementptr inbounds ([7 x i8], [7 x i8]* @.str140, i64 0, i64 0), i64 6)
%t387 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str141, i64 0, i64 0), i64 2)
%t388 = call i64 @addToken(i64 %t385, i8* %t386, i8* %t387)
ret i64 0
endif384:
%t389 = load i64, i64* %lexer.addr
%t390 = call i8* @rey_str_new(i8* getelementptr inbounds ([11 x i8], [11 x i8]* @.str93, i64 0, i64 0), i64 10)
%t391 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str27, i64 0, i64 0), i64 1)
%t392 = call i64 @addToken(i64 %t389, i8* %t390, i8* %t391)
ret i64 0
endif377:
%t393 = load i64, i64* %c.addr
%t394 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str28, i64 0, i64 0), i64 1)
%t395 = call i64 @rey_str_eq(i8* %t393, i8* %t394)
%t396 = icmp ne i64 %t395, 0
br i1 %t396, label %then397, label %endif399
then397:
%t400 = load i64, i64* %lexer.addr
%t401 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str28, i64 0, i64 0), i64 1)
%t402 = call i64 @lexMatchChar(i64 %t400, i8* %t401)
%t403 = icmp ne i64 %t402, 0
br i1 %t403, label %then404, label %endif406
then404:
%t407 = load i64, i64* %lexer.addr
%t408 = call i8* @rey_str_new(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str142, i64 0, i64 0), i64 4)
%t409 = call i8* @rey_str_new(i8* getelementptr inbounds ([3 x i8], [3 x i8]* @.str143, i64 0, i64 0), i64 2)
%t410 = call i64 @addToken(i64 %t407, i8* %t408, i8* %t409)
ret i64 0
endif406:
%t411 = load i64, i64* %lexer.addr
%t412 = call i8* @rey_str_new(i8* getelementptr inbounds ([11 x i8], [11 x i8]* @.str93, i64 0, i64 0), i64 10)
%t413 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str28, i64 0, i64 0), i64 1)
%t414 = call i64 @addToken(i64 %t411, i8* %t412, i8* %t413)
ret i64 0
endif399:
%t415 = load i64, i64* %c.addr
%t416 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str39, i64 0, i64 0), i64 1)
%t417 = call i64 @rey_str_eq(i8* %t415, i8* %t416)
%t418 = icmp ne i64 %t417, 0
br i1 %t418, label %then419, label %endif421
then419:
%t422 = load i64, i64* %lexer.addr
%t423 = call i64 @scanString(i64 %t422)
%str.addr = alloca i64, align 8
store i64 %t423, i64* %str.addr
%t424 = load i64, i64* %lexer.addr
%t425 = call i8* @rey_str_new(i8* getelementptr inbounds ([14 x i8], [14 x i8]* @.str144, i64 0, i64 0), i64 13)
%t426 = load i64, i64* %str.addr
%t427 = call i64 @addToken(i64 %t424, i8* %t425, i64 %t426)
ret i64 0
endif421:
%t428 = load i64, i64* %c.addr
%t429 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str46, i64 0, i64 0), i64 1)
%t430 = call i64 @rey_str_eq(i8* %t428, i8* %t429)
%t431 = icmp ne i64 %t430, 0
br i1 %t431, label %then432, label %endif434
then432:
%t435 = load i64, i64* %lexer.addr
%t436 = call i64 @scanChar(i64 %t435)
%ch.addr = alloca i64, align 8
store i64 %t436, i64* %ch.addr
%t437 = load i64, i64* %lexer.addr
%t438 = call i8* @rey_str_new(i8* getelementptr inbounds ([12 x i8], [12 x i8]* @.str145, i64 0, i64 0), i64 11)
%t439 = load i64, i64* %ch.addr
%t440 = call i64 @addToken(i64 %t437, i8* %t438, i64 %t439)
ret i64 0
endif434:
%t441 = load i64, i64* %c.addr
%t442 = call i64 @isDigit(i64 %t441)
%t443 = icmp ne i64 %t442, 0
br i1 %t443, label %then444, label %endif446
then444:
%t447 = load i64, i64* %lexer.addr
%t448 = call i64 @scanNumber(i64 %t447)
%num.addr = alloca i64, align 8
store i64 %t448, i64* %num.addr
%t449 = load i64, i64* %lexer.addr
%t450 = call i8* @rey_str_new(i8* getelementptr inbounds ([14 x i8], [14 x i8]* @.str146, i64 0, i64 0), i64 13)
%t451 = load i64, i64* %num.addr
%t452 = call i64 @addToken(i64 %t449, i8* %t450, i64 %t451)
ret i64 0
endif446:
%t453 = load i64, i64* %c.addr
%t454 = call i64 @isAlpha(i64 %t453)
%t455 = icmp ne i64 %t454, 0
br i1 %t455, label %then456, label %endif458
then456:
%t459 = load i64, i64* %lexer.addr
%t460 = call i64 @scanIdentifier(i64 %t459)
%ident.addr = alloca i64, align 8
store i64 %t460, i64* %ident.addr
%t461 = load i64, i64* %ident.addr
%t462 = call i64 @getKeyword(i64 %t461)
%kind.addr = alloca i64, align 8
store i64 %t462, i64* %kind.addr
%t463 = load i64, i64* %lexer.addr
%t464 = load i64, i64* %kind.addr
%t465 = load i64, i64* %ident.addr
%t466 = call i64 @addToken(i64 %t463, i64 %t464, i64 %t465)
ret i64 0
endif458:
%t467 = load i64, i64* %lexer.addr
%t468 = call i8* @rey_str_new(i8* getelementptr inbounds ([20 x i8], [20 x i8]* @.str35, i64 0, i64 0), i64 19)
%t469 = call i8* @rey_str_new(i8* getelementptr inbounds ([23 x i8], [23 x i8]* @.str147, i64 0, i64 0), i64 22)
%t470 = load i64, i64* %c.addr
%t472 = call i8* @rey_int_to_str(i64 %t470)
%t471 = call i8* @rey_str_concat(i8* %t469, i8* %t472)
%t473 = call i64 @addError(i64 %t467, i8* %t468, i8* %t471)
ret i64 0
}

define i64 @newLexer(i8* %source) {
entry:
%source.addr = alloca i8*, align 8
store i8* %source, i8** %source.addr
%t0 = alloca %struct.Lexer, align 8
%t1 = getelementptr inbounds %struct.Lexer, %struct.Lexer* %t0, i32 0, i32 0
%t2 = getelementptr inbounds %struct.Lexer, %struct.Lexer* %t0, i32 0, i32 1
store i64 0, i64* %t2
%t3 = getelementptr inbounds %struct.Lexer, %struct.Lexer* %t0, i32 0, i32 2
store i64 0, i64* %t3
%t4 = getelementptr inbounds %struct.Lexer, %struct.Lexer* %t0, i32 0, i32 3
store i64 0, i64* %t4
%t5 = getelementptr inbounds %struct.Lexer, %struct.Lexer* %t0, i32 0, i32 4
store i64 0, i64* %t5
%t6 = getelementptr inbounds %struct.Lexer, %struct.Lexer* %t0, i32 0, i32 5
store i64 0, i64* %t6
%t7 = getelementptr inbounds %struct.Lexer, %struct.Lexer* %t0, i32 0, i32 6
store i64 0, i64* %t7
%t8 = load i8*, i8** %source.addr
%t9 = getelementptr inbounds %struct.Lexer, %struct.Lexer* %t0, i32 0, i32 0
store i8* %t8, i8** %t9
%t10 = getelementptr inbounds %struct.Lexer, %struct.Lexer* %t0, i32 0, i32 1
store i64 0, i64* %t10
%t11 = getelementptr inbounds %struct.Lexer, %struct.Lexer* %t0, i32 0, i32 2
store i64 0, i64* %t11
%t12 = getelementptr inbounds %struct.Lexer, %struct.Lexer* %t0, i32 0, i32 3
store i64 1, i64* %t12
%t13 = getelementptr inbounds %struct.Lexer, %struct.Lexer* %t0, i32 0, i32 4
store i64 0, i64* %t13
%t14 = call i64 @rey_vec_new()
%t15 = getelementptr inbounds %struct.Lexer, %struct.Lexer* %t0, i32 0, i32 5
store i64 %t14, i64* %t15
%t16 = call i64 @rey_vec_new()
%t17 = getelementptr inbounds %struct.Lexer, %struct.Lexer* %t0, i32 0, i32 6
store i64 %t16, i64* %t17
ret i64 %t0
ret i64 0
}

define i64 @tokenize(i8* %source, i8* %path) {
entry:
%source.addr = alloca i8*, align 8
store i8* %source, i8** %source.addr
%path.addr = alloca i8*, align 8
store i8* %path, i8** %path.addr
%t0 = load i8*, i8** %source.addr
%t1 = call i64 @newLexer(i8* %t0)
%lexer.addr = alloca i64, align 8
store i64 %t1, i64* %lexer.addr
br label %loop.body2
loop.body2:
%t4 = load i64, i64* %lexer.addr
%t5 = load i64, i64* %lexer.addr
%t6 = call i64 @rey_vec_len(i64 0)
%t7 = icmp sge i64 0, %t6
br i1 %t7, label %then8, label %endif10
then8:
br label %loop.end3
endif10:
%t11 = load i64, i64* %lexer.addr
%t12 = call i64 @skipWhitespace(i64 %t11)
%t13 = load i64, i64* %lexer.addr
%t14 = load i64, i64* %lexer.addr
%t15 = call i64 @rey_vec_len(i64 0)
%t16 = icmp sge i64 0, %t15
br i1 %t16, label %then17, label %endif19
then17:
br label %loop.end3
endif19:
%t20 = load i64, i64* %lexer.addr
%t21 = call i64 @lexPeek(i64 %t20)
%c.addr = alloca i64, align 8
store i64 %t21, i64* %c.addr
%t22 = load i64, i64* %c.addr
%t23 = call i8* @rey_str_new(i8* getelementptr inbounds ([1 x i8], [1 x i8]* @.str34, i64 0, i64 0), i64 0)
%t24 = call i64 @rey_str_eq(i8* %t22, i8* %t23)
%t25 = icmp ne i64 %t24, 0
br i1 %t25, label %then26, label %endif28
then26:
br label %loop.end3
endif28:
%t29 = load i64, i64* %c.addr
%t30 = call i8* @rey_str_new(i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str8, i64 0, i64 0), i64 1)
%t31 = call i64 @rey_str_eq(i8* %t29, i8* %t30)
%t32 = icmp ne i64 %t31, 0
br i1 %t32, label %then33, label %endif35
then33:
%t36 = load i64, i64* %lexer.addr
%t37 = call i64 @lexAdvance(i64 %t36)
br label %loop.body2
endif35:
%t38 = load i64, i64* %lexer.addr
%t39 = call i64 @scanToken(i64 %t38)
br label %loop.body2
loop.end3:
%t40 = load i64, i64* %lexer.addr
%t41 = call i8* @rey_str_new(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str94, i64 0, i64 0), i64 3)
%t42 = call i8* @rey_str_new(i8* getelementptr inbounds ([1 x i8], [1 x i8]* @.str34, i64 0, i64 0), i64 0)
%t43 = call i64 @addToken(i64 %t40, i8* %t41, i8* %t42)
%t44 = alloca %struct.LexerResult, align 8
%t45 = getelementptr inbounds %struct.LexerResult, %struct.LexerResult* %t44, i32 0, i32 0
store i64 0, i64* %t45
%t46 = getelementptr inbounds %struct.LexerResult, %struct.LexerResult* %t44, i32 0, i32 1
store i64 0, i64* %t46
%t47 = load i64, i64* %lexer.addr
%t48 = getelementptr inbounds %struct.LexerResult, %struct.LexerResult* %t44, i32 0, i32 0
store i64 0, i64* %t48
%t49 = load i64, i64* %lexer.addr
%t50 = getelementptr inbounds %struct.LexerResult, %struct.LexerResult* %t44, i32 0, i32 1
store i64 0, i64* %t50
ret i64 %t44
ret i64 0
}

