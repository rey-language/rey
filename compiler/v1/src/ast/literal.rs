#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    String(String),
    Char(char),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
}
