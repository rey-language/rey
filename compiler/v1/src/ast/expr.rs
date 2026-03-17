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
    Unary {
        op: TokenKind,
        right: Box<Expr>,
    },

    Assign {
        name: String,
        value: Box<Expr>,
    },
    Get {
        object: Box<Expr>,
        name: String,
    },
}
