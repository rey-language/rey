use crate::lexer::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum ParserError {
    Custom {
        message: String,
        span: Span,
    },
}
