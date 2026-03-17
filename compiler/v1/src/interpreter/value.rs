use crate::ast::Literal;

use super::function::Function;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Function(Function),
    Null,
}

impl From<Literal> for Value {
    fn from(lit: Literal) -> Self {
        match lit {
            Literal::String(s) => Value::String(s),
            Literal::Number(n) => Value::Number(n),
            Literal::Bool(b) => Value::Bool(b),
            Literal::Null => Value::Null, }
    }
}