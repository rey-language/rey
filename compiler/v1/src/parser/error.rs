use crate::lexer::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum ParserError {
    Custom { message: String, span: Span },
}

impl ParserError {
    pub fn span(&self) -> &Span {
        match self {
            ParserError::Custom { span, .. } => span,
        }
    }

    pub fn message(&self) -> String {
        match self {
            ParserError::Custom { message, .. } => message.clone(),
        }
    }
}
