# Tagged AST v0.3 SPI

AST collections use tagged payload enums. Existing concrete payload structs and their fields/layouts remain unchanged; `ExprKind`, `StmtKind`, and `PatternKind` remain compatibility markers and are not base-record inheritance.

```rey
pub enum Expr {
    Error(ErrorExpr), Literal(LiteralExpr), Identifier(IdentExpr),
    Binary(BinaryExpr), Unary(UnaryExpr), Update(UpdateExpr), Call(CallExpr),
    EnumVariant(EnumVariantExpr), Index(IndexExpr), FieldAccess(FieldAccessExpr),
    Lambda(LambdaExpr), Match(MatchExpr), If(IfExpr), Tuple(TupleExpr),
    Array(ArrayExpr), Dict(DictExpr), StructLiteral(StructLiteralExpr),
}

pub enum Stmt {
    VarDecl(VarDeclStmt), ConstDecl(ConstDeclStmt), FuncDecl(FuncDeclStmt),
    StructDecl(StructDeclStmt), EnumDecl(EnumDeclStmt), Import(ImportStmt),
    If(IfStmt), Assign(AssignStmt), Return(ReturnStmt), Break(BreakStmt),
    Continue(ContinueStmt), While(WhileStmt), For(ForStmt), Loop(LoopStmt),
    Block(BlockStmt), ExprStmt(ExprStmt), Match(MatchStmt),
}

pub enum Pattern {
    Error(ErrorPattern), Wildcard(WildcardPattern), Literal(LiteralPattern),
    Variable(VariablePattern), EnumVariant(EnumVariantPattern),
    Struct(StructPattern), Or(OrPattern), Range(RangePattern),
}
```

The payload names above are canonical and constructors/patterns are qualified (`Expr::Literal(...)`, `Stmt::Return(...)`, `Pattern::Wildcard(...)`) wherever names overlap. `Program.statements` and recursive statement/pattern/expression fields keep their declared strong enum types. Public `exprSpan(Expr)`, `stmtSpan(Stmt)`, and `patternSpan(Pattern)` return the wrapped payload's existing span.

Parser entry points and `Program` shape do not change. Parsing wraps concrete payloads at AST boundaries; no fields are widened to `any`, and the tagged enums add no language features.
