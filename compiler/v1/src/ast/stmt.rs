use super::{Expr, Type};
use crate::lexer::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub ty: Option<Type>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionVisibility {
    Private,
    Pub,
    ExportPub,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportKind {
    FileSymbols { module: String, symbols: Vec<String> },
    ModuleNamespace { module: String },
    ModuleItems { module: String, items: Vec<String> },
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
        visibility: FunctionVisibility,
        params: Vec<Parameter>,
        return_ty: Option<Type>,
        body: Vec<Stmt>,
    },
    Import {
        kind: ImportKind,
        span: Span,
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
    For {
        variable: String,
        start: Expr,
        end: Expr,
        body: Vec<Stmt>,
    },
    Break,
    Continue,
    Return(Expr),
    ExprStmt(Expr),
}
