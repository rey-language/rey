use crate::lexer::span::Span;
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

//this enum will grow over time
// for v0, keeping it simple
//contains all tokens supported for v0
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    //single char tokens
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    Percent,
    Colon,
    Not,
    AndAnd,
    OrOr,

    //keywords
    Var,
    Func,
    Return,
    If,
    Else,
    While,
    Break,
    Continue,
    For,
    In,
    True,
    False,
    Null,

    //literals
    Identifier(String),
    StringLiteral(String),
    NumberLiteral(f64),

    //operators
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    NotEqual,
    Less,
    LessEqual,

    //special
    Eof,
}
