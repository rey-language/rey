use super::Literal;
use crate::lexer::TokenKind;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal),
    Binary {
        left: Box<Expr>,
        op: TokenKind,
        right: Box<Expr>,
    },
    Variable(String),
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    ArrayLiteral {
        elements: Vec<Expr>,
    },
    DictLiteral {
        entries: Vec<(String, Expr)>,
    },
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    Get {
        object: Box<Expr>,
        name: String,
    },
    MethodCall {
        receiver: Box<Expr>,
        name: String,
        args: Vec<Expr>,
    },
    Unary {
        op: TokenKind,
        right: Box<Expr>,
    },

    Assign {
        name: String,
        value: Box<Expr>,
    },

    Update {
        name: String,
        op: TokenKind,
        prefix: bool,
    },
}
