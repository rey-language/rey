#![allow(non_snake_case)]

use super::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum LexerError {
    UnexpectedCharacter { found: char, span: Span },

    UnterminatedString { span: Span },
    UnterminatedChar { span: Span },
}
