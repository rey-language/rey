use crate::ast::Literal;

use super::function::Function;
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Function(Function),
    Array(Rc<RefCell<Vec<Value>>>),
    Dict(Rc<RefCell<HashMap<String, Value>>>),
    Null,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Function(a), Value::Function(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                a.as_slice() == b.as_slice()
            }
            (Value::Dict(a), Value::Dict(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                if a.len() != b.len() {
                    return false;
                }
                for (k, av) in a.iter() {
                    match b.get(k) {
                        Some(bv) if av == bv => {}
                        _ => return false,
                    }
                }
                true
            }
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }
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
