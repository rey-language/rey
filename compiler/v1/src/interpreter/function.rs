use super::environment::Environment;
use crate::ast::{Parameter, Stmt};
use crate::lexer::span::Span;

#[derive(Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Parameter>,
    pub body: Vec<Stmt>,
    pub span: Span,
    pub closure: Option<Environment>,
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Function")
            .field("name", &self.name)
            .field("params", &self.params)
            .field("body", &self.body)
            .field("span", &self.span)
            .finish()
    }
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.params == other.params
            && self.body == other.body
            && self.span == other.span
    }
}

impl Function {
    pub fn new(
        name: String,
        params: Vec<Parameter>,
        body: Vec<Stmt>,
        span: Span,
        closure: Option<Environment>,
    ) -> Self {
        Self {
            name,
            params,
            body,
            span,
            closure,
        }
    }

    pub fn arity(&self) -> usize {
        self.params.len()
    }
}
