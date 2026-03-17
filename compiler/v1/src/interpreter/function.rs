use crate::ast::{Parameter, Stmt};
use crate::lexer::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Parameter>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

impl Function {
    pub fn new(name: String, params: Vec<Parameter>, body: Vec<Stmt>, span: Span) -> Self {
        Self {
            name,
            params,
            body,
            span,
        }
    }

    pub fn arity(&self) -> usize {
        self.params.len()
    }
}