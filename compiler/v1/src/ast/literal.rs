#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    String(String),
    Char(char),
    Number(f64),
    Bool(bool),
    Null,
}
