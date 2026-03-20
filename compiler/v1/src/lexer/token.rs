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
    Ellipsis,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    Percent,
    Colon,
    Question,
    Not,
    AndAnd,
    OrOr,
    Pipe,

    //multi-char operators
    PlusPlus,
    MinusMinus,
    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    PercentEqual,

    //keywords
    Var,
    Const,
    Func,
    Return,
    If,
    Else,
    While,
    Loop,
    Break,
    Continue,
    For,
    In,
    InstanceOf,
    True,
    False,
    Null,
    Struct,
    Pub,
    SelfKw,

    //literals
    Identifier(String),
    StringLiteral(String),
    CharLiteral(char),
    NumberLiteral(f64),

    //operators
    Equal,
    EqualEqual,
    Arrow,
    Greater,
    GreaterEqual,
    NotEqual,
    Less,
    LessEqual,

    //special
    Eof,
}
