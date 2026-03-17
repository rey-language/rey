use super::value::Value;

#[derive(Debug, Clone)]
pub enum ControlFlow {
    Normal(Value),
    Return(Value),
    Break,
    Continue,
}

impl ControlFlow {
    pub fn normal(value: Value) -> Self {
        ControlFlow::Normal(value)
    }

    pub fn return_value(value: Value) -> Self {
        ControlFlow::Return(value)
    }
}
