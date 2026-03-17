use crate::ast::Expr;
use crate::lexer::TokenKind;
use super::value::Value;
use super::environment::Environment;

pub struct Evaluator;

impl Evaluator {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(&self, expr: &Expr, env:&mut Environment) -> Result<Value, String> {
        match expr {
            Expr::Literal(lit) => Ok(Value::from(lit.clone())),
            Expr::Variable(name) => env
                .get(name)
                .cloned()
                .ok_or_else(|| format!("Undefined variable '{}'", name)),
            Expr::Binary { left, op, right } => {
                let left_val = self.evaluate(left, env)?;
                let right_val = self.evaluate(right, env)?;
                self.evaluate_binary(left_val, op, right_val)
            }
            Expr::Unary { op, right } => {
                let right_val = self.evaluate(right, env)?;
                self.evaluate_unary(op, right_val)
            }
            Expr::Assign { name, value } => {
                let val = self.evaluate(value, env)?;
            
                Err("Assignment should be handled by executor".to_string())
            }
            Expr::Call { callee, args } => {
                Err("Function calls must be handled as statements".to_string())
            }
            Expr::Get { .. } => {
                Err("Property access not implemented yet".to_string())
            }
        }
    }

    fn evaluate_binary(&self, left: Value, op: &TokenKind, right: Value) -> Result<Value, String> {
        use TokenKind::*;

        match (left, op, right) {
            (Value::Number(l), Plus, Value::Number(r)) => Ok(Value::Number(l + r)),
            (Value::Number(l), Minus, Value::Number(r)) => Ok(Value::Number(l - r)),
            (Value::Number(l), Star, Value::Number(r)) => Ok(Value::Number(l * r)),
            (Value::Number(l), Slash, Value::Number(r)) => {
                if r == 0.0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Number(l / r))
                }
            }
            (Value::Number(l), EqualEqual, Value::Number(r)) => Ok(Value::Bool(l == r)),
            (Value::Number(l), BangEqual, Value::Number(r)) => Ok(Value::Bool(l != r)),
            (Value::Number(l), Less, Value::Number(r)) => Ok(Value::Bool(l < r)),
            (Value::Number(l), LessEqual, Value::Number(r)) => Ok(Value::Bool(l <= r)),
            (Value::Number(l), Greater, Value::Number(r)) => Ok(Value::Bool(l > r)),
            (Value::Number(l), GreaterEqual, Value::Number(r)) => Ok(Value::Bool(l >= r)),

            (Value::String(l), Plus, Value::String(r)) => Ok(Value::String(l + &r)),
            (Value::String(l), EqualEqual, Value::String(r)) => Ok(Value::Bool(l == r)),
            (Value::String(l), BangEqual, Value::String(r)) => Ok(Value::Bool(l != r)),

            (Value::Bool(l), EqualEqual, Value::Bool(r)) => Ok(Value::Bool(l == r)),
            (Value::Bool(l), BangEqual, Value::Bool(r)) => Ok(Value::Bool(l != r)),
            (Value::Bool(l), And, Value::Bool(r)) => Ok(Value::Bool(l && r)),
            (Value::Bool(l), Or, Value::Bool(r)) => Ok(Value::Bool(l || r)),

            _ => Err("Invalid binary operation".to_string()),
        }
    }

    fn evaluate_unary(&self, op: &TokenKind, right: Value) -> Result<Value, String> {
        use TokenKind::*;

        match (op, right) {
            (Minus, Value::Number(n)) => Ok(Value::Number(-n)),
            (Bang, Value::Bool(b)) => Ok(Value::Bool(!b)),
            _ => Err("Invalid unary operation".to_string()),
        }
    }
}