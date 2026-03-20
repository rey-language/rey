use super::{Expr, Literal, Type};

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub ty: Option<Type>,
    pub default: Option<Expr>,
    pub variadic: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub name: String,
    pub ty: Type,
    pub is_pub: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodDecl {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_ty: Option<Type>,
    pub body: Vec<Stmt>,
    pub is_pub: bool,
    pub is_static: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    VarDecl {
        is_const: bool,
        name: String,
        ty: Option<Type>,
        initializer: Expr,
    },
    FuncDecl {
        name: String,
        params: Vec<Parameter>,
        return_ty: Option<Type>,
        body: Vec<Stmt>,
    },
    StructDecl {
        name: String,
        fields: Vec<FieldDecl>,
        methods: Vec<MethodDecl>,
    },
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    Loop {
        body: Vec<Stmt>,
    },
    For {
        variable: String,
        iterator: ForIterator,
        body: Vec<Stmt>,
    },
    EnumDecl {
        name: String,
        variants: Vec<String>,
    },
    Match {
        expr: Expr,
        arms: Vec<MatchArm>,
    },
    Break,
    Continue,
    Return(Expr),
    ExprStmt(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    EnumVariant(String, String), // enum_name, variant_name
    Literal(Literal),
    Variable(String),
    Wildcard,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ForIterator {
    Range { start: Expr, end: Expr },
    Array(Expr),
}
