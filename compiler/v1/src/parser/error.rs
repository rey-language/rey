use crate::lexer::span::Span;
use crate::lexer::TokenKind;

#[derive(Debug, Clone, PartialEq)]
pub enum ParserError {
    UnexpectedToken {
        expected: Vec<TokenKind>,
        found: TokenKind,
        span: Span,
    },

    UnexpectedEOF {
        expected: Vec<TokenKind>,
        span: Span,
    },

    Custom {
        message: String,
        span: Span,
    },
}
impl ParserError {
    pub fn new(message: String, span: Span) -> Self {
        Self::Custom { message, span }
    }
}