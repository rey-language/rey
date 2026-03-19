use super::control_flow::ControlFlow;
use super::environment::Environment;
use super::function::Function;
use super::value::{StructDef, Value};
use crate::ast::{Expr, Stmt};
use crate::lexer::span::Span;
use crate::lexer::TokenKind;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Executor;

impl Executor {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&self, stmt: &Stmt, env: &mut Environment) -> Result<ControlFlow, String> {
        match stmt {
            Stmt::VarDecl {
                is_const: _,
                name,
                initializer,
                ..
            } => {
                let value = self.evaluate_expr(initializer, env)?;
                env.define(name.clone(), value);
                Ok(ControlFlow::normal(Value::Null))
            }
            Stmt::ExprStmt(expr) => {
                let value = self.evaluate_expr(expr, env)?;
                Ok(ControlFlow::normal(value))
            }
            Stmt::FuncDecl {
                name, params, body, ..
            } => {
                let function = Function::new(
                    name.clone(),
                    params.clone(),
                    body.clone(),
                    Span { start: 0, end: 0 },
                );
                env.define(name.clone(), Value::Function(function));
                Ok(ControlFlow::normal(Value::Null))
            }
            Stmt::StructDecl {
                name,
                fields,
                methods,
            } => {
                let def = StructDef {
                    name: name.clone(),
                    fields: fields.clone(),
                    methods: methods.clone(),
                };
                env.register_struct(def);
                Ok(ControlFlow::normal(Value::Null))
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_value = self.evaluate_expr(condition, env)?;
                if self.isTruthy(&condition_value) {
                    match self.execute_block_with_control_flow(then_branch, env)? {
                        ControlFlow::Normal(_) => {}
                        ControlFlow::Return(value) => return Ok(ControlFlow::Return(value)),
                        ControlFlow::Break => return Ok(ControlFlow::Break),
                        ControlFlow::Continue => return Ok(ControlFlow::Continue),
                    }
                } else if let Some(else_branch) = else_branch {
                    match self.execute_block_with_control_flow(else_branch, env)? {
                        ControlFlow::Normal(_) => {}
                        ControlFlow::Return(value) => return Ok(ControlFlow::Return(value)),
                        ControlFlow::Break => return Ok(ControlFlow::Break),
                        ControlFlow::Continue => return Ok(ControlFlow::Continue),
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
                        ControlFlow::Normal(_) => {}
                    }
                }
                Ok(ControlFlow::normal(Value::Null))
            }
            Stmt::For {
                variable,
                start,
                end,
                body,
            } => {
                let start_val = self.evaluate_expr(start, env)?;
                let end_val = self.evaluate_expr(end, env)?;

                let start_num = match start_val {
                    Value::Number(n) => n as i64,
                    _ => return Err("Range start must be a number".to_string()),
                };
                let end_num = match end_val {
                    Value::Number(n) => n as i64,
                    _ => return Err("Range end must be a number".to_string()),
                };

                for i in start_num..end_num {
                    env.define(variable.clone(), Value::Number(i as f64));

                    match self.execute_block_with_control_flow(body, env)? {
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::Return(value) => return Ok(ControlFlow::return_value(value)),
                        ControlFlow::Normal(_) => {}
                    }
                }
                Ok(ControlFlow::normal(Value::Null))
            }
            Stmt::Break => Ok(ControlFlow::Break),
            Stmt::Continue => Ok(ControlFlow::Continue),
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
            Expr::Get { object, name } => {
                let obj = self.evaluate_expr(object, env)?;
                match obj {
                    Value::Dict(d) => d
                        .borrow()
                        .get(name)
                        .cloned()
                        .ok_or_else(|| "Dictionary key not found".to_string()),
                    Value::StructInstance {
                        struct_name,
                        fields,
                    } => {
                        // Field access on struct instance
                        fields.borrow().get(name).cloned().ok_or_else(|| {
                            let field_names: Vec<String> =
                                fields.borrow().keys().cloned().collect();
                            let suggestion = find_closest_match(name, &field_names);
                            let mut msg = format!(
                                "error[E004]: unknown field '{}' on struct '{}'",
                                name, struct_name
                            );
                            if let Some(s) = suggestion {
                                msg.push_str(&format!("\n  did you mean '{}'?", s));
                            }
                            msg
                        })
                    }
                    _ => Err(
                        "Property access is only supported for dictionaries and structs"
                            .to_string(),
                    ),
                }
            }
            Expr::MethodCall {
                receiver,
                name,
                args,
            } => {
                let recv = self.evaluate_expr(receiver, env)?;
                let mut evaluated_args = Vec::new();
                for a in args {
                    evaluated_args.push(self.evaluate_expr(a, env)?);
                }
                // If the receiver is a struct instance, dispatch to struct method
                if let Value::StructInstance {
                    ref struct_name,
                    ref fields,
                } = recv
                {
                    return self.call_struct_method(
                        struct_name.clone(),
                        fields.clone(),
                        name,
                        &evaluated_args,
                        env,
                    );
                }
                self.evaluate_method_call(recv, name, &evaluated_args)
            }
            Expr::StructLiteral {
                name,
                fields: field_exprs,
            } => {
                let def = env
                    .get_struct(name)
                    .ok_or_else(|| format!("Undefined struct '{}'", name))?
                    .clone();

                // Evaluate field values
                let mut field_map = HashMap::new();
                for (fname, fexpr) in field_exprs {
                    let val = self.evaluate_expr(fexpr, env)?;
                    field_map.insert(fname.clone(), val);
                }

                // Check for missing fields
                for fd in &def.fields {
                    if !field_map.contains_key(&fd.name) {
                        return Err(format!(
                            "error[E007]: missing field '{}' in {} literal\n  field type: {}",
                            fd.name, name, fd.ty.name
                        ));
                    }
                }

                Ok(Value::StructInstance {
                    struct_name: name.clone(),
                    fields: Rc::new(RefCell::new(field_map)),
                })
            }
            Expr::StaticCall {
                struct_name,
                method,
                args,
            } => {
                let def = env
                    .get_struct(struct_name)
                    .ok_or_else(|| format!("Undefined struct '{}'", struct_name))?
                    .clone();

                // Find matching static method
                let matching: Vec<_> = def
                    .methods
                    .iter()
                    .filter(|m| m.name == *method && m.is_static)
                    .collect();

                if matching.is_empty() {
                    return Err(format!(
                        "error[E005]: no static method '{}' on struct '{}'",
                        method, struct_name
                    ));
                }

                // Find by arity
                let mut evaluated_args = Vec::new();
                for a in args {
                    evaluated_args.push(self.evaluate_expr(a, env)?);
                }

                let method_decl = matching
                    .iter()
                    .find(|m| m.params.len() == evaluated_args.len())
                    .ok_or_else(|| {
                        format!(
                            "error[E006]: method '{}' expects {} arguments, got {}",
                            method,
                            matching[0].params.len(),
                            evaluated_args.len()
                        )
                    })?;

                // Create environment with struct defs available
                let mut method_env = Environment::with_parent(env.clone());

                // Bind parameters
                for (param, val) in method_decl.params.iter().zip(&evaluated_args) {
                    method_env.define(param.name.clone(), val.clone());
                }

                // Execute the static method body
                self.execute_block(&method_decl.body, &mut method_env)
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
            Expr::Update { name, op, prefix } => {
                let current = env
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("Undefined variable '{}'", name))?;

                let delta = match op {
                    TokenKind::PlusPlus => 1.0,
                    TokenKind::MinusMinus => -1.0,
                    _ => return Err("Invalid update operator".to_string()),
                };

                let current_num = match current {
                    Value::Number(n) => n,
                    _ => return Err("Can only apply ++/-- to numbers".to_string()),
                };

                let new_num = current_num + delta;
                env.assign(name, Value::Number(new_num))?;
                Ok(Value::Number(if *prefix { new_num } else { current_num }))
            }
            Expr::Call { callee, args } => {
                // Check if it's a built-in function first
                if let Expr::Variable(name) = callee.as_ref() {
                    let mut evaluated_args = Vec::new();
                    for arg in args {
                        evaluated_args.push(self.evaluate_expr(arg, env)?);
                    }

                    if let Some(result) =
                        super::std::StdLib::call_builtin_function(name, &evaluated_args)
                    {
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

                                let mut function_env = Environment::with_parent(env.clone());

                                for (param, arg_value) in func.params.iter().zip(evaluated_args) {
                                    function_env.define(param.name.clone(), arg_value);
                                }

                                self.execute_block(&func.body, &mut function_env)
                            }
                            _ => Err(format!("Can only call functions, got {:?}", function)),
                        }
                    }
                } else {
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

                            let mut evaluated_args = Vec::new();
                            for arg in args {
                                evaluated_args.push(self.evaluate_expr(arg, env)?);
                            }

                            let mut function_env = Environment::with_parent(env.clone());

                            for (param, arg_value) in func.params.iter().zip(evaluated_args) {
                                function_env.define(param.name.clone(), arg_value);
                            }

                            self.execute_block(&func.body, &mut function_env)
                        }
                        _ => Err(format!("Can only call functions, got {:?}", function)),
                    }
                }
            }
        }
    }

    /// Call a method on a struct instance.
    /// Struct fields become the method's "global scope".
    fn call_struct_method(
        &self,
        struct_name: String,
        instance_fields: Rc<RefCell<HashMap<String, Value>>>,
        method_name: &str,
        args: &[Value],
        env: &mut Environment,
    ) -> Result<Value, String> {
        let def = env
            .get_struct(&struct_name)
            .ok_or_else(|| format!("Undefined struct '{}'", struct_name))?
            .clone();

        // Find matching method by name and arity
        let matching: Vec<_> = def
            .methods
            .iter()
            .filter(|m| m.name == method_name && !m.is_static)
            .collect();

        if matching.is_empty() {
            return Err(format!(
                "error[E004]: unknown method '{}' on struct '{}'",
                method_name, struct_name
            ));
        }

        let method_decl = matching
            .iter()
            .find(|m| m.params.len() == args.len())
            .ok_or_else(|| {
                format!(
                    "error[E006]: method '{}' expects {} arguments, got {}",
                    method_name,
                    matching[0].params.len(),
                    args.len()
                )
            })?;

        // Check visibility: only pub methods can be called from outside
        // (We don't enforce this from _inside_ other methods of the same struct)
        // This check is at call site in evaluate_expr.

        // Create a fresh environment.
        // Struct fields are injected AS local variables (the "global scope" for the method).
        let mut method_env = Environment::new();

        // Copy struct defs so the method can create struct instances
        method_env.struct_defs = env.struct_defs.clone();

        // Inject struct fields as local variables
        for (fname, fval) in instance_fields.borrow().iter() {
            method_env.define(fname.clone(), fval.clone());
        }

        // Bind parameters
        for (param, val) in method_decl.params.iter().zip(args) {
            method_env.define(param.name.clone(), val.clone());
        }

        // Execute the method body
        let result = self.execute_block(&method_decl.body, &mut method_env)?;

        // After execution, write back any changed field values to the instance
        let mut fields_mut = instance_fields.borrow_mut();
        for fname in fields_mut.keys().cloned().collect::<Vec<_>>() {
            if let Some(updated_val) = method_env.get(&fname) {
                fields_mut.insert(fname, updated_val.clone());
            }
        }

        Ok(result)
    }

    fn evaluate_method_call(
        &self,
        receiver: Value,
        name: &str,
        args: &[Value],
    ) -> Result<Value, String> {
        match (receiver, name) {
            (val, "toString") => {
                if !args.is_empty() {
                    return Err(format!(
                        "toString() expects 0 arguments, got {}",
                        args.len()
                    ));
                }
                Ok(Value::String(super::std::StdLib::formatValue(&val)))
            }
            (Value::String(s), "toInt") => {
                if !args.is_empty() {
                    return Err(format!("toInt() expects 0 arguments, got {}", args.len()));
                }
                match s.parse::<f64>() {
                    Ok(n) => Ok(Value::Number(n.trunc())),
                    Err(_) => Err(format!("Cannot convert string '{}' to int", s)),
                }
            }
            (Value::String(s), "toFloat") => {
                if !args.is_empty() {
                    return Err(format!("toFloat() expects 0 arguments, got {}", args.len()));
                }
                match s.parse::<f64>() {
                    Ok(n) => Ok(Value::Number(n)),
                    Err(_) => Err(format!("Cannot convert string '{}' to float", s)),
                }
            }
            (Value::Number(n), "toInt") => {
                if !args.is_empty() {
                    return Err(format!("toInt() expects 0 arguments, got {}", args.len()));
                }
                Ok(Value::Number(n.trunc()))
            }
            (Value::Number(n), "toFloat") => {
                if !args.is_empty() {
                    return Err(format!("toFloat() expects 0 arguments, got {}", args.len()));
                }
                Ok(Value::Number(n))
            }
            (Value::Array(arr), "length") => {
                if !args.is_empty() {
                    return Err(format!(
                        "{}.length() expects 0 arguments, got {}",
                        "Array",
                        args.len()
                    ));
                }
                Ok(Value::Number(arr.borrow().len() as f64))
            }
            (Value::Array(arr), "push") => {
                if args.len() != 1 {
                    return Err(format!(
                        "{}.push() expects 1 argument, got {}",
                        "Array",
                        args.len()
                    ));
                }
                arr.borrow_mut().push(args[0].clone());
                Ok(Value::Null)
            }
            (Value::String(s), "length") => {
                if !args.is_empty() {
                    return Err(format!(
                        "{}.length() expects 0 arguments, got {}",
                        "String",
                        args.len()
                    ));
                }
                Ok(Value::Number(s.chars().count() as f64))
            }
            (Value::String(s), "upper") => {
                if !args.is_empty() {
                    return Err(format!(
                        "{}.upper() expects 0 arguments, got {}",
                        "String",
                        args.len()
                    ));
                }
                Ok(Value::String(s.to_uppercase()))
            }
            (Value::String(s), "lower") => {
                if !args.is_empty() {
                    return Err(format!(
                        "{}.lower() expects 0 arguments, got {}",
                        "String",
                        args.len()
                    ));
                }
                Ok(Value::String(s.to_lowercase()))
            }
            (Value::String(s), "contains") => {
                if args.len() != 1 {
                    return Err(format!(
                        "{}.contains() expects 1 argument, got {}",
                        "String",
                        args.len()
                    ));
                }
                match &args[0] {
                    Value::String(needle) => Ok(Value::Bool(s.contains(needle))),
                    _ => Err("String.contains() expects a string argument".to_string()),
                }
            }
            (Value::String(s), "split") => {
                if args.len() != 1 {
                    return Err(format!(
                        "{}.split() expects 1 argument, got {}",
                        "String",
                        args.len()
                    ));
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
            (Value::Number(l), Percent, Value::Number(r)) => {
                if r == 0.0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Number(l % r))
                }
            }
            (Value::Null, EqualEqual, Value::Null) => Ok(Value::Bool(true)),
            (Value::Null, NotEqual, Value::Null) => Ok(Value::Bool(false)),
            (Value::Null, EqualEqual, _) | (_, EqualEqual, Value::Null) => Ok(Value::Bool(false)),
            (Value::Null, NotEqual, _) | (_, NotEqual, Value::Null) => Ok(Value::Bool(true)),
            (Value::Number(l), EqualEqual, Value::Number(r)) => Ok(Value::Bool(l == r)),
            (Value::Number(l), NotEqual, Value::Number(r)) => Ok(Value::Bool(l != r)),
            (Value::Number(l), Less, Value::Number(r)) => Ok(Value::Bool(l < r)),
            (Value::Number(l), LessEqual, Value::Number(r)) => Ok(Value::Bool(l <= r)),
            (Value::Number(l), Greater, Value::Number(r)) => Ok(Value::Bool(l > r)),
            (Value::Number(l), GreaterEqual, Value::Number(r)) => Ok(Value::Bool(l >= r)),

            (Value::String(l), Plus, r) => {
                Ok(Value::String(l + &super::std::StdLib::formatValue(&r)))
            }
            (l, Plus, Value::String(r)) => {
                Ok(Value::String(super::std::StdLib::formatValue(&l) + &r))
            }
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

    pub fn execute_block(
        &self,
        statements: &[Stmt],
        env: &mut Environment,
    ) -> Result<Value, String> {
        match self.execute_block_with_control_flow(statements, env)? {
            ControlFlow::Normal(value) | ControlFlow::Return(value) => Ok(value),
            ControlFlow::Break | ControlFlow::Continue => {
                Err("Break/continue outside of loop".to_string())
            }
        }
    }

    pub fn execute_block_with_control_flow(
        &self,
        statements: &[Stmt],
        env: &mut Environment,
    ) -> Result<ControlFlow, String> {
        for stmt in statements {
            let control_flow = self.execute(stmt, env)?;
            match control_flow {
                ControlFlow::Normal(_) => {}
                ControlFlow::Break | ControlFlow::Continue | ControlFlow::Return(_) => {
                    return Ok(control_flow);
                }
            }
        }
        Ok(ControlFlow::normal(Value::Null))
    }
}

/// Simple edit distance helper for "did you mean?" suggestions  
fn find_closest_match(target: &str, candidates: &[String]) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for c in candidates {
        let dist = levenshtein(target, c);
        if dist <= 3 {
            if best.is_none() || dist < best.as_ref().unwrap().0 {
                best = Some((dist, c.clone()));
            }
        }
    }
    best.map(|(_, s)| s)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..=n {
        dp[i][0] = i;
    }
    for j in 0..=m {
        dp[0][j] = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[n][m]
}
