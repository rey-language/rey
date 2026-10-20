#![allow(non_snake_case)]

use super::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum LexerError {
    UnexpectedCharacter { found: char, span: Span },

    UnterminatedString { span: Span },
    UnterminatedChar { span: Span },
    InvalidEscape { escape: String, span: Span },
    InvalidCharacterValue { value: u32, span: Span },
    UnterminatedInterpolation { span: Span },
    UnmatchedStringBrace { span: Span },
    UnterminatedBlockComment { span: Span },
    InvalidNumber { span: Span },
}

impl LexerError {
    pub fn span(&self) -> &Span {
        match self {
            LexerError::UnexpectedCharacter { span, .. } => span,
            LexerError::UnterminatedString { span } => span,
            LexerError::UnterminatedChar { span } => span,
            LexerError::InvalidEscape { span, .. } => span,
            LexerError::InvalidCharacterValue { span, .. } => span,
            LexerError::UnterminatedInterpolation { span } => span,
            LexerError::UnmatchedStringBrace { span } => span,
            LexerError::UnterminatedBlockComment { span } => span,
            LexerError::InvalidNumber { span } => span,
        }
    }

    pub fn message(&self) -> String {
        match self {
            LexerError::UnexpectedCharacter { found, .. } => {
                format!("Unexpected character '{}'", found)
            }
            LexerError::UnterminatedString { .. } => "Unterminated string literal".to_string(),
            LexerError::UnterminatedChar { .. } => "Unterminated character literal".to_string(),
            LexerError::InvalidEscape { escape, .. } => {
                format!("Invalid escape sequence '{}'.", escape)
            }
            LexerError::InvalidCharacterValue { value, .. } => {
                format!(
                    "Character U+{:04X} does not encode as one byte in range 0..255",
                    value
                )
            }
            LexerError::UnterminatedInterpolation { .. } => {
                "Unterminated string interpolation".to_string()
            }
            LexerError::UnmatchedStringBrace { .. } => {
                "Unmatched '}' in string literal".to_string()
            }
            LexerError::UnterminatedBlockComment { .. } => {
                "Unterminated block comment".to_string()
            }
            LexerError::InvalidNumber { .. } => "Invalid numeric literal.".to_string(),
        }
    }
}
