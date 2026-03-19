use super::Literal;
use crate::lexer::TokenKind;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal),
    Binary {
        left: Box<Expr>,
        op: TokenKind,
        right: Box<Expr>,
        span: Span,
    },
    Variable(String),
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    ArrayLiteral {
        elements: Vec<Expr>,
        span: Span,
    },
    DictLiteral {
        entries: Vec<(String, Expr)>,
        span: Span,
    },
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    Get {
        object: Box<Expr>,
        name: String,
        span: Span,
    },
    MethodCall {
        receiver: Box<Expr>,
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    StructLiteral {
        name: String,
        fields: Vec<(String, Expr)>,
        span: Span,
    },
    StaticCall {
        struct_name: String,
        method: String,
        args: Vec<Expr>,
        span: Span,
    },
    Unary {
        op: TokenKind,
        right: Box<Expr>,
        span: Span,
    },

    Assign {
        name: String,
        value: Box<Expr>,
        span: Span,
    },

    Update {
        name: String,
        op: TokenKind,
        prefix: bool,
        span: Span,
    },
}
