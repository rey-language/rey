#![allow(non_snake_case)]

use super::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum LexerError {
    UnexpectedCharacter { found: char, span: Span },

    UnterminatedString { span: Span },
    UnterminatedChar { span: Span },
}

impl LexerError {
    pub fn span(&self) -> &Span {
        match self {
            LexerError::UnexpectedCharacter { span, .. } => span,
            LexerError::UnterminatedString { span } => span,
            LexerError::UnterminatedChar { span } => span,
        }
    }

    pub fn message(&self) -> String {
        match self {
            LexerError::UnexpectedCharacter { found, .. } => {
                format!("Unexpected character '{}'", found)
            }
            LexerError::UnterminatedString { .. } => "Unterminated string literal".to_string(),
            LexerError::UnterminatedChar { .. } => "Unterminated character literal".to_string(),
        }
    }
}
