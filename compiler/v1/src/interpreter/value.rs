use crate::ast::Literal;

use super::function::Function;
use crate::ast::{FieldDecl, MethodDecl};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedStr(Rc<String>);

impl SharedStr {
    pub fn new(text: String) -> Self {
        Self(Rc::new(text))
    }

    pub fn asStr(&self) -> &str {
        &self.0
    }

    pub fn asBytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn appendStr(&self, extra: &str) -> Self {
        let mut rc = Rc::clone(&self.0);
        Rc::make_mut(&mut rc).push_str(extra);
        Self(rc)
    }

    pub fn appendStrs(&self, parts: &[&str]) -> Self {
        let extra = parts.iter().map(|part| part.len()).sum();
        let mut rc = Rc::clone(&self.0);
        {
            let text = Rc::make_mut(&mut rc);
            text.reserve(extra);
            for part in parts {
                text.push_str(part);
            }
        }
        Self(rc)
    }

    pub fn ptr(&self) -> *const String {
        Rc::as_ptr(&self.0)
    }
}

impl From<&str> for SharedStr {
    fn from(value: &str) -> Self {
        Self::new(value.to_string())
    }
}

impl From<String> for SharedStr {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl std::ops::Deref for SharedStr {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for SharedStr {
    fn eq(&self, other: &str) -> bool {
        self.asStr() == other
    }
}

impl PartialEq<&str> for SharedStr {
    fn eq(&self, other: &&str) -> bool {
        self.asStr() == *other
    }
}

impl PartialEq<SharedStr> for String {
    fn eq(&self, other: &SharedStr) -> bool {
        self.as_str() == other.asStr()
    }
}

impl PartialEq<String> for SharedStr {
    fn eq(&self, other: &String) -> bool {
        self.asStr() == other.as_str()
    }
}

impl PartialOrd for SharedStr {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.asStr().cmp(other.asStr()))
    }
}

impl std::fmt::Display for SharedStr {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.asStr())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedBuf(Rc<Vec<u8>>);

impl SharedBuf {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(Rc::new(bytes))
    }

    pub fn asBytes(&self) -> &[u8] {
        &self.0
    }

    pub fn appendBytes(&self, extra: &[u8]) -> Self {
        let mut rc = Rc::clone(&self.0);
        {
            let vec = Rc::make_mut(&mut rc);
            vec.reserve(extra.len());
            vec.extend_from_slice(extra);
        }
        Self(rc)
    }

    pub fn appendMany(&self, parts: &[&[u8]]) -> Self {
        let extra = parts.iter().map(|part| part.len()).sum();
        let mut rc = Rc::clone(&self.0);
        {
            let vec = Rc::make_mut(&mut rc);
            vec.reserve(extra);
            for part in parts {
                vec.extend_from_slice(part);
            }
        }
        Self(rc)
    }

    pub fn ptr(&self) -> *const Vec<u8> {
        Rc::as_ptr(&self.0)
    }
}

impl PartialEq<[u8]> for SharedBuf {
    fn eq(&self, other: &[u8]) -> bool {
        self.asBytes() == other
    }
}

impl PartialEq<&[u8]> for SharedBuf {
    fn eq(&self, other: &&[u8]) -> bool {
        self.asBytes() == *other
    }
}

/// A stored struct definition (registered when `struct Foo { ... }` is executed)
#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub methods: Vec<MethodDecl>,
}

#[derive(Debug, Clone)]
pub enum Value {
    String(SharedStr),
    ByteString(SharedBuf),
    Char(u8),
    Int(i64),
    Float(f64),
    Bool(bool),
    EnumVariant {
        enum_name: SharedStr,
        variant: SharedStr,
    },
    EnumVariantPayload {
        enum_name: SharedStr,
        variant: SharedStr,
        payloads: Vec<Value>,
    },
    Function(Rc<Function>),
    Tuple(Rc<RefCell<Vec<Value>>>),
    Array(Rc<RefCell<Vec<Value>>>),
    Dict(Rc<RefCell<HashMap<Vec<u8>, Value>>>),
    Namespace(Rc<RefCell<HashMap<Vec<u8>, Value>>>),
    StructInstance {
        struct_name: SharedStr,
        fields: Rc<RefCell<HashMap<String, Value>>>,
    },
    Null,
    Vec(Rc<RefCell<Vec<Value>>>),
    LinkedList(Rc<RefCell<Vec<Value>>>),
    HashMap(Rc<RefCell<HashMap<Vec<u8>, Value>>>),
    Stack(Rc<RefCell<Vec<Value>>>),
    Queue(Rc<RefCell<Vec<Value>>>),
    Option(Rc<RefCell<Option<Value>>>),
    Result(Rc<RefCell<Result<Value, Value>>>),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s.asStr()),
            Value::ByteString(bytes) => write!(f, "b\"{}\"", escapeBytes(bytes.asBytes())),
            Value::Char(byte) => {
                if byte.is_ascii_graphic() || *byte == b' ' {
                    write!(f, "{}", *byte as char)
                } else {
                    write!(f, "\\x{:02X}", byte)
                }
            }
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", formatFloat(*n)),
            Value::Bool(b) => write!(f, "{}", b),
            Value::EnumVariant { enum_name, variant } => write!(f, "{}::{}", enum_name, variant),
            Value::EnumVariantPayload {
                enum_name,
                variant,
                payloads,
            } => {
                let payloads = payloads
                    .iter()
                    .map(|value| format!("{}", value))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{}::{}({})", enum_name, variant, payloads)
            }
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
                let items: Vec<String> = d
                    .iter()
                    .map(|(k, v)| format!("{}: {}", displayBytes(k), v))
                    .collect();
                write!(f, "{{{}}}", items.join(", "))
            }
            Value::Namespace(_) => write!(f, "<namespace>"),
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
                let items: Vec<String> = m
                    .iter()
                    .map(|(k, v)| format!("{}: {}", displayBytes(k), v))
                    .collect();
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

pub fn escapeBytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    for byte in bytes {
        match *byte {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\\""),
            0x20..=0x7e => out.push(*byte as char),
            _ => out.push_str(&format!("\\x{:02X}", byte)),
        }
    }
    out
}

pub fn displayBytes(bytes: &[u8]) -> String {
    match String::from_utf8(bytes.to_vec()) {
        Ok(value) => value,
        Err(error) => format!("b\"{}\"", escapeBytes(&error.into_bytes())),
    }
}

impl Value {
    pub fn stringBytes(&self) -> Option<&[u8]> {
        match self {
            Value::String(value) => Some(value.asBytes()),
            Value::ByteString(bytes) => Some(bytes.asBytes()),
            _ => None,
        }
    }

    pub fn fromStringBytes(bytes: Vec<u8>) -> Self {
        match String::from_utf8(bytes) {
            Ok(value) => Value::String(SharedStr::from(value)),
            Err(error) => Value::ByteString(SharedBuf::new(error.into_bytes())),
        }
    }

    pub fn utf8(text: &str) -> Self {
        Value::String(SharedStr::from(text))
    }

    pub fn function(function: Function) -> Self {
        Value::Function(Rc::new(function))
    }

    pub fn bufferPtr(&self) -> Option<*const ()> {
        match self {
            Value::String(value) => Some(value.ptr() as *const ()),
            Value::ByteString(value) => Some(value.ptr() as *const ()),
            _ => None,
        }
    }

    pub fn isUtf8(&self) -> bool {
        matches!(self, Value::String(_))
    }
}

pub fn formatFloat(value: f64) -> String {
    let text = format!("{:?}", value);
    let Some(stripped) = text.strip_suffix(".0") else {
        return text;
    };
    let digits = stripped.strip_prefix('-').unwrap_or(stripped);
    if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) {
        stripped.to_string()
    } else {
        text
    }
}

pub fn floatToInt(value: f64) -> Result<i64, String> {
    const LOWER_INCLUSIVE: f64 = -9_223_372_036_854_775_808.0;
    const UPPER_EXCLUSIVE: f64 = 9_223_372_036_854_775_808.0;
    if !value.is_finite() || value < LOWER_INCLUSIVE || value >= UPPER_EXCLUSIVE {
        return Err("Float value out of int range".to_string());
    }
    Ok(value.trunc() as i64)
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(a), Value::String(b)) => a.asBytes() == b.asBytes(),
            (Value::String(a), Value::ByteString(b))
            | (Value::ByteString(b), Value::String(a)) => a.asBytes() == b.asBytes(),
            (Value::ByteString(a), Value::ByteString(b)) => a.asBytes() == b.asBytes(),
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
            (
                Value::EnumVariantPayload {
                    enum_name: en1,
                    variant: v1,
                    payloads: p1,
                },
                Value::EnumVariantPayload {
                    enum_name: en2,
                    variant: v2,
                    payloads: p2,
                },
            ) => en1 == en2 && v1 == v2 && p1 == p2,
            (Value::Function(a), Value::Function(b)) => a == b,
            (Value::Tuple(a), Value::Tuple(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                a.as_slice() == b.as_slice()
            }
            (Value::Array(a), Value::Array(b)) => Rc::ptr_eq(a, b),
            (Value::Dict(a), Value::Dict(b)) => Rc::ptr_eq(a, b),
            (Value::Namespace(a), Value::Namespace(b)) => Rc::ptr_eq(a, b),
            (
                Value::StructInstance { struct_name: name_a, fields: fields_a },
                Value::StructInstance { struct_name: name_b, fields: fields_b },
            ) => name_a == name_b && Rc::ptr_eq(fields_a, fields_b),
            (Value::Null, Value::Null) => true,
            (Value::Vec(a), Value::Vec(b)) => Rc::ptr_eq(a, b),
            (Value::LinkedList(a), Value::LinkedList(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                a.as_slice() == b.as_slice()
            }
            (Value::HashMap(a), Value::HashMap(b)) => Rc::ptr_eq(a, b),
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
            Literal::String(s) => Value::fromStringBytes(s.into_bytes()),
            Literal::ByteString(bytes) => Value::fromStringBytes(bytes),
            Literal::Char(c) => Value::Char(c as u8),
            Literal::Int(n) => Value::Int(n),
            Literal::Float(n) => Value::Float(n),
            Literal::Bool(b) => Value::Bool(b),
            Literal::Null => Value::Null,
        }
    }
}
