use crate::ast::Literal;

use super::function::Function;
use crate::ast::{FieldDecl, MethodDecl};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A stored struct definition (registered when `struct Foo { ... }` is executed)
#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub methods: Vec<MethodDecl>,
}

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Char(char),
    Int(i64),
    Float(f64),
    Bool(bool),
    EnumVariant {
        enum_name: String,
        variant: String,
    },
    Function(Function),
    Tuple(Rc<RefCell<Vec<Value>>>),
    Array(Rc<RefCell<Vec<Value>>>),
    Dict(Rc<RefCell<HashMap<String, Value>>>),
    StructInstance {
        struct_name: String,
        fields: Rc<RefCell<HashMap<String, Value>>>,
    },
    Null,
    Vec(Rc<RefCell<Vec<Value>>>),
    LinkedList(Rc<RefCell<Vec<Value>>>),
    HashMap(Rc<RefCell<HashMap<String, Value>>>),
    Stack(Rc<RefCell<Vec<Value>>>),
    Queue(Rc<RefCell<Vec<Value>>>),
    Option(Rc<RefCell<Option<Value>>>),
    Result(Rc<RefCell<Result<Value, String>>>),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Char(c) => write!(f, "{}", c),
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::Bool(b) => write!(f, "{}", b),
            Value::EnumVariant { enum_name, variant } => write!(f, "{}::{}", enum_name, variant),
            Value::Function(func) => write!(f, "<func {}>", func.name),
            Value::Tuple(items) => {
                let items = items.borrow();
                let parts: Vec<String> = items.iter().map(|v| format!("{}", v)).collect();
                write!(f, "({})", parts.join(", "))
            }
            Value::Array(arr) => {
                let arr = arr.borrow();
                let items: Vec<String> = arr.iter().map(|v| format!("{}", v)).collect();
                write!(f, "[{}]", items.join(", "))
            }
            Value::Dict(d) => {
                let d = d.borrow();
                let items: Vec<String> = d.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
                write!(f, "{{{}}}", items.join(", "))
            }
            Value::StructInstance {
                struct_name,
                fields,
            } => {
                let fields = fields.borrow();
                let items: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                write!(f, "{} {{ {} }}", struct_name, items.join(", "))
            }
            Value::Null => write!(f, "null"),
            Value::Vec(v) => {
                let v = v.borrow();
                let items: Vec<String> = v.iter().map(|x| format!("{}", x)).collect();
                write!(f, "[{}]", items.join(", "))
            }
            Value::LinkedList(l) => {
                let l = l.borrow();
                let items: Vec<String> = l.iter().map(|x| format!("{}", x)).collect();
                write!(f, "LinkedList([{}])", items.join(", "))
            }
            Value::HashMap(m) => {
                let m = m.borrow();
                let items: Vec<String> = m.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
                write!(f, "{{{}}}", items.join(", "))
            }
            Value::Stack(s) => {
                let s = s.borrow();
                write!(f, "Stack(top: {})", s.len())
            }
            Value::Queue(q) => {
                let q = q.borrow();
                write!(f, "Queue(front: {})", q.len())
            }
            Value::Option(o) => {
                let o = o.borrow();
                match o.as_ref() {
                    Some(v) => write!(f, "Some({})", v),
                    None => write!(f, "None"),
                }
            }
            Value::Result(r) => {
                let r = r.borrow();
                match r.as_ref() {
                    Ok(v) => write!(f, "Ok({})", v),
                    Err(e) => write!(f, "Err({})", e),
                }
            }
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => {
                (*a as f64) == *b
            }
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (
                Value::EnumVariant {
                    enum_name: en1,
                    variant: v1,
                },
                Value::EnumVariant {
                    enum_name: en2,
                    variant: v2,
                },
            ) => en1 == en2 && v1 == v2,
            (Value::Function(a), Value::Function(b)) => a == b,
            (Value::Tuple(a), Value::Tuple(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                a.as_slice() == b.as_slice()
            }
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
            (Value::Vec(a), Value::Vec(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                a.as_slice() == b.as_slice()
            }
            (Value::LinkedList(a), Value::LinkedList(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                a.as_slice() == b.as_slice()
            }
            (Value::HashMap(a), Value::HashMap(b)) => {
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
            (Value::Stack(a), Value::Stack(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                a.as_slice() == b.as_slice()
            }
            (Value::Queue(a), Value::Queue(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                a.as_slice() == b.as_slice()
            }
            (Value::Option(a), Value::Option(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                match (a.as_ref(), b.as_ref()) {
                    (Some(av), Some(bv)) => av == bv,
                    (None, None) => true,
                    _ => false,
                }
            }
            (Value::Result(a), Value::Result(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                match (a.as_ref(), b.as_ref()) {
                    (Ok(av), Ok(bv)) => av == bv,
                    (Err(ae), Err(be)) => ae == be,
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

impl From<Literal> for Value {
    fn from(lit: Literal) -> Self {
        match lit {
            Literal::String(s) => Value::String(s),
            Literal::Char(c) => Value::Char(c),
            Literal::Int(n) => Value::Int(n),
            Literal::Float(n) => Value::Float(n),
            Literal::Bool(b) => Value::Bool(b),
            Literal::Null => Value::Null,
        }
    }
}
