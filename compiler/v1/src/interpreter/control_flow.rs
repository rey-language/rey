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

    pub fn unwrap_normal(self) -> Value {
        match self {
            ControlFlow::Normal(value) => value,
            _ => panic!("Expected normal control flow"),
        }
    }

    pub fn unwrap_return(self) -> Value {
        match self {
            ControlFlow::Return(value) => value,
            _ => panic!("Expected return control flow"),
        }
    }
}