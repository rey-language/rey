#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    String(String),
    ByteString(Vec<u8>),
    Char(char),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
}
