use crate::ast::{Expr, Stmt};
use crate::lexer::span::Span;
use crate::lexer::TokenKind;
use super::control_flow::ControlFlow;
use super::environment::Environment;
use super::function::Function;
use super::value::Value;
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Executor;

impl Executor {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&self, stmt: &Stmt, env: &mut Environment) -> Result<ControlFlow, String> {
        match stmt {
            Stmt::VarDecl { name, initializer, .. } => {
                let value = self.evaluate_expr(initializer, env)?;
                env.define(name.clone(), value);
                Ok(ControlFlow::normal(Value::Null))
            }
            Stmt::ExprStmt(expr) => {
                let value = self.evaluate_expr(expr, env)?;
                Ok(ControlFlow::normal(value))
            }
            Stmt::FuncDecl { name, params, body, .. } => {
                let function = Function::new(
                    name.clone(),
                    params.clone(),
                    body.clone(),
                    Span { start: 0, end: 0 }, // TODO: Get actual span
                );
                env.define(name.clone(), Value::Function(function));
                Ok(ControlFlow::normal(Value::Null))
            }
            Stmt::If { condition, then_branch, else_branch } => {
                let condition_value = self.evaluate_expr(condition, env)?;
                if self.isTruthy(&condition_value) {
                    match self.execute_block_with_control_flow(then_branch, env)? {
                        ControlFlow::Normal(_) => {}
                        ControlFlow::Return(value) => return Ok(ControlFlow::Return(value)),
                        ControlFlow::Break | ControlFlow::Continue => {
                            return Err("Break/continue not allowed in if statement".to_string());
                        }
                    }
                } else if let Some(else_branch) = else_branch {
                    match self.execute_block_with_control_flow(else_branch, env)? {
                        ControlFlow::Normal(_) => {}
                        ControlFlow::Return(value) => return Ok(ControlFlow::Return(value)),
                        ControlFlow::Break | ControlFlow::Continue => {
                            return Err("Break/continue not allowed in if statement".to_string());
                        }
                    }
                }
                Ok(ControlFlow::normal(Value::Null))
            }
            Stmt::While { condition, body } => {
                while self.isTruthy(&self.evaluate_expr(condition, env)?) {
                    match self.execute_block_with_control_flow(body, env)? {
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::Return(value) => return Ok(ControlFlow::return_value(value)),
                        ControlFlow::Normal(_) => {} // Continue to next iteration
                    }
                }
                Ok(ControlFlow::normal(Value::Null))
            }
            Stmt::For { variable, start, end, body } => {
                // Evaluate start and end expressions
                let start_val = self.evaluate_expr(start, env)?;
                let end_val = self.evaluate_expr(end, env)?;

                // Extract numeric values
                let start_num = match start_val {
                    Value::Number(n) => n as i64,
                    _ => return Err("Range start must be a number".to_string()),
                };
                let end_num = match end_val {
                    Value::Number(n) => n as i64,
                    _ => return Err("Range end must be a number".to_string()),
                };

                // Loop from start to end-1
                for i in start_num..end_num {
                    // Set the loop variable
                    env.define(variable.clone(), Value::Number(i as f64));

                    // Execute body
                    match self.execute_block_with_control_flow(body, env)? {
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::Return(value) => return Ok(ControlFlow::return_value(value)),
                        ControlFlow::Normal(_) => {} // Continue to next iteration
                    }
                }
                Ok(ControlFlow::normal(Value::Null))
            }
            Stmt::Break => {
                Ok(ControlFlow::Break)
            }
            Stmt::Continue => {
                Ok(ControlFlow::Continue)
            }
            Stmt::Return(expr) => {
                let value = self.evaluate_expr(expr, env)?;
                Ok(ControlFlow::return_value(value))
            }
        }
    }

    pub fn evaluate_expr(&self, expr: &Expr, env: &mut Environment) -> Result<Value, String> {
        match expr {
            Expr::Literal(lit) => Ok(Value::from(lit.clone())),
            Expr::Variable(name) => env
                .get(name)
                .cloned()
                .ok_or_else(|| format!("Undefined variable '{}'", name)),
            Expr::Binary { left, op, right } => {
                let left_val = self.evaluate_expr(left, env)?;
                let right_val = self.evaluate_expr(right, env)?;
                self.evaluate_binary(left_val, op, right_val)
            }
            Expr::ArrayLiteral { elements } => {
                let mut evaluated = Vec::new();
                for el in elements {
                    evaluated.push(self.evaluate_expr(el, env)?);
                }
                Ok(Value::Array(Rc::new(RefCell::new(evaluated))))
            }
            Expr::DictLiteral { entries } => {
                let mut m = HashMap::new();
                for (k, v) in entries {
                    let value = self.evaluate_expr(v, env)?;
                    m.insert(k.clone(), value);
                }
                Ok(Value::Dict(Rc::new(RefCell::new(m))))
            }
            Expr::Index { target, index } => {
                let target_val = self.evaluate_expr(target, env)?;
                let index_val = self.evaluate_expr(index, env)?;
                match (target_val, index_val) {
                    (Value::Array(arr), Value::Number(n)) => {
                        if n.fract() != 0.0 {
                            return Err("Array index must be an integer".to_string());
                        }
                        let idx = n as isize;
                        if idx < 0 {
                            return Err("Array index must be non-negative".to_string());
                        }
                        let idx = idx as usize;
                        arr.borrow()
                            .get(idx)
                            .cloned()
                            .ok_or_else(|| "Array index out of bounds".to_string())
                    }
                    (Value::Dict(d), Value::String(s)) => d
                        .borrow()
                        .get(&s)
                        .cloned()
                        .ok_or_else(|| "Dictionary key not found".to_string()),
                    _ => Err("Indexing is only supported for arrays (number index) and dictionaries (string key)".to_string()),
                }
            }
            Expr::MethodCall { receiver, name, args } => {
                let recv = self.evaluate_expr(receiver, env)?;
                let mut evaluated_args = Vec::new();
                for a in args {
                    evaluated_args.push(self.evaluate_expr(a, env)?);
                }
                self.evaluate_method_call(recv, name, &evaluated_args)
            }
            Expr::Unary { op, right } => {
                let right_val = self.evaluate_expr(right, env)?;
                self.evaluate_unary(op, right_val)
            }
            Expr::Assign { name, value } => {
                let val = self.evaluate_expr(value, env)?;
                env.assign(name, val.clone())?;
                Ok(val)
            }
            Expr::Call { callee, args } => {
                // Check if it's a built-in function first
                if let Expr::Variable(name) = callee.as_ref() {
                    let mut evaluated_args = Vec::new();
                    for arg in args {
                        evaluated_args.push(self.evaluate_expr(arg, env)?);
                    }

                    if let Some(result) = super::std::StdLib::call_builtin_function(name, &evaluated_args) {
                        result
                    } else {
                        // Not a built-in, check if it's a user-defined function
                        let function = self.evaluate_expr(callee, env)?;
                        match function {
                            Value::Function(func) => {
                                if args.len() != func.arity() {
                                    return Err(format!(
                                        "Expected {} arguments but got {}",
                                        func.arity(),
                                        args.len()
                                    ));
                                }

                                // Create new environment with function parameters
                                // The function environment should have access to the global environment
                                let mut function_env = Environment::with_parent(env.clone());

                                // Bind parameters to arguments
                                for (param, arg_value) in func.params.iter().zip(evaluated_args) {
                                    function_env.define(param.name.clone(), arg_value);
                                }

                                // Execute function body
                                self.execute_block(&func.body, &mut function_env)
                            }
                            _ => Err(format!("Can only call functions, got {:?}", function)),
                        }
                    }
                } else {
                    // Dynamic function call (function as expression)
                    let function = self.evaluate_expr(callee, env)?;
                    match function {
                        Value::Function(func) => {
                            if args.len() != func.arity() {
                                return Err(format!(
                                    "Expected {} arguments but got {}",
                                    func.arity(),
                                    args.len()
                                ));
                            }

                            // Evaluate arguments
                            let mut evaluated_args = Vec::new();
                            for arg in args {
                                evaluated_args.push(self.evaluate_expr(arg, env)?);
                            }

                            // Create new environment with function parameters
                            // The function environment should have access to the global environment
                            let mut function_env = Environment::with_parent(env.clone());

                            // Bind parameters to arguments
                            for (param, arg_value) in func.params.iter().zip(evaluated_args) {
                                function_env.define(param.name.clone(), arg_value);
                            }

                            // Execute function body
                            self.execute_block(&func.body, &mut function_env)
                        }
                        _ => Err(format!("Can only call functions, got {:?}", function)),
                    }
                }
            }
        }
    }

    fn evaluate_method_call(&self, receiver: Value, name: &str, args: &[Value]) -> Result<Value, String> {
        match (receiver, name) {
            (Value::String(s), "length") => {
                if !args.is_empty() {
                    return Err(format!("{}.length() expects 0 arguments, got {}", "String", args.len()));
                }
                Ok(Value::Number(s.chars().count() as f64))
            }
            (Value::String(s), "upper") => {
                if !args.is_empty() {
                    return Err(format!("{}.upper() expects 0 arguments, got {}", "String", args.len()));
                }
                Ok(Value::String(s.to_uppercase()))
            }
            (Value::String(s), "lower") => {
                if !args.is_empty() {
                    return Err(format!("{}.lower() expects 0 arguments, got {}", "String", args.len()));
                }
                Ok(Value::String(s.to_lowercase()))
            }
            (Value::String(s), "contains") => {
                if args.len() != 1 {
                    return Err(format!("{}.contains() expects 1 argument, got {}", "String", args.len()));
                }
                match &args[0] {
                    Value::String(needle) => Ok(Value::Bool(s.contains(needle))),
                    _ => Err("String.contains() expects a string argument".to_string()),
                }
            }
            (Value::String(s), "split") => {
                if args.len() != 1 {
                    return Err(format!("{}.split() expects 1 argument, got {}", "String", args.len()));
                }
                let delim = match &args[0] {
                    Value::String(d) => d.clone(),
                    _ => return Err("String.split() expects a string delimiter".to_string()),
                };
                let parts = if delim.is_empty() {
                    s.chars().map(|c| c.to_string()).collect::<Vec<_>>()
                } else {
                    s.split(&delim).map(|p| p.to_string()).collect::<Vec<_>>()
                };
                let arr = parts.into_iter().map(Value::String).collect::<Vec<_>>();
                Ok(Value::Array(Rc::new(RefCell::new(arr))))
            }
            (other, _) => Err(format!("Method call not supported on {:?}", other)),
        }
    }

    fn isTruthy(&self, value: &Value) -> bool {
        match value {
            Value::Bool(false) => false,
            Value::Null => false,
            Value::Number(n) => *n != 0.0,
            _ => true,
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
            (Value::Number(l), NotEqual, Value::Number(r)) => Ok(Value::Bool(l != r)),
            (Value::Number(l), Less, Value::Number(r)) => Ok(Value::Bool(l < r)),
            (Value::Number(l), LessEqual, Value::Number(r)) => Ok(Value::Bool(l <= r)),
            (Value::Number(l), Greater, Value::Number(r)) => Ok(Value::Bool(l > r)),
            (Value::Number(l), GreaterEqual, Value::Number(r)) => Ok(Value::Bool(l >= r)),

            (Value::String(l), Plus, Value::String(r)) => Ok(Value::String(l + &r)),
            (Value::String(l), EqualEqual, Value::String(r)) => Ok(Value::Bool(l == r)),
            (Value::String(l), NotEqual, Value::String(r)) => Ok(Value::Bool(l != r)),

            (Value::Bool(l), EqualEqual, Value::Bool(r)) => Ok(Value::Bool(l == r)),
            (Value::Bool(l), NotEqual, Value::Bool(r)) => Ok(Value::Bool(l != r)),
            (Value::Bool(l), AndAnd, Value::Bool(r)) => Ok(Value::Bool(l && r)),
            (Value::Bool(l), OrOr, Value::Bool(r)) => Ok(Value::Bool(l || r)),

            _ => Err("Invalid binary operation".to_string()),
        }
    }

    fn evaluate_unary(&self, op: &TokenKind, right: Value) -> Result<Value, String> {
        use TokenKind::*;

        match (op, right) {
            (Minus, Value::Number(n)) => Ok(Value::Number(-n)),
            (Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
            _ => Err("Invalid unary operation".to_string()),
        }
    }

    pub fn execute_block(&self, statements: &[Stmt], env: &mut Environment) -> Result<Value, String> {
        match self.execute_block_with_control_flow(statements, env)? {
            ControlFlow::Normal(value) | ControlFlow::Return(value) => Ok(value),
            ControlFlow::Break | ControlFlow::Continue => Err("Break/continue outside of loop".to_string()),
        }
    }

    pub fn execute_block_with_control_flow(&self, statements: &[Stmt], env: &mut Environment) -> Result<ControlFlow, String> {
        for stmt in statements {
            let control_flow = self.execute(stmt, env)?;
            match control_flow {
                ControlFlow::Normal(_) => {} // Continue execution
                ControlFlow::Break | ControlFlow::Continue | ControlFlow::Return(_) => {
                    return Ok(control_flow);
                }
            }
        }
        Ok(ControlFlow::normal(Value::Null))
    }
}
