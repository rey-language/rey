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
                name,
                visibility: _,
                params,
                body,
                ..
            } => {
                let function = Function::new(
                    name.clone(),
                    params.clone(),
                    body.clone(),
                    Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        column: 0,
                    },
                    None,
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
            Stmt::Loop { body } => {
                loop {
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
                iterator,
                body,
            } => {
                use crate::ast::stmt::ForIterator;
                match iterator {
                    ForIterator::Range { start, end } => {
                        let start_val = self.evaluate_expr(start, env)?;
                        let end_val = self.evaluate_expr(end, env)?;

                        let start_num = match start_val {
                            Value::Int(n) => n,
                            Value::Float(n) => {
                                if n.fract() != 0.0 {
                                    return Err("Range start must be an integer".to_string());
                                }
                                n as i64
                            }
                            _ => return Err("Range start must be a number".to_string()),
                        };
                        let end_num = match end_val {
                            Value::Int(n) => n,
                            Value::Float(n) => {
                                if n.fract() != 0.0 {
                                    return Err("Range end must be an integer".to_string());
                                }
                                n as i64
                            }
                            _ => return Err("Range end must be a number".to_string()),
                        };

                        for i in start_num..end_num {
                            env.define(variable.clone(), Value::Int(i));

                            match self.execute_block_with_control_flow(body, env)? {
                                ControlFlow::Break => break,
                                ControlFlow::Continue => continue,
                                ControlFlow::Return(value) => {
                                    return Ok(ControlFlow::return_value(value))
                                }
                                ControlFlow::Normal(_) => {}
                            }
                        }
                    }
                    ForIterator::Array(expr) => {
                        let arr_val = self.evaluate_expr(expr, env)?;
                        match arr_val {
                            Value::Array(arr) => {
                                for item in arr.borrow().iter() {
                                    env.define(variable.clone(), item.clone());

                                    match self.execute_block_with_control_flow(body, env)? {
                                        ControlFlow::Break => break,
                                        ControlFlow::Continue => continue,
                                        ControlFlow::Return(value) => {
                                            return Ok(ControlFlow::return_value(value))
                                        }
                                        ControlFlow::Normal(_) => {}
                                    }
                                }
                            }
                            _ => return Err("For-in requires an array".to_string()),
                        }
                    }
                }
                Ok(ControlFlow::normal(Value::Null))
            }
            Stmt::Break => Ok(ControlFlow::Break),
            Stmt::Continue => Ok(ControlFlow::Continue),
            Stmt::Return(expr) => {
                let value = match expr {
                    Some(expr) => self.evaluate_expr(expr, env)?,
                    None => Value::Null,
                };
                Ok(ControlFlow::return_value(value))
            }
            Stmt::EnumDecl { name, variants } => {
                env.register_enum(name.clone(), variants.clone());
                Ok(ControlFlow::normal(Value::Null))
            }
            Stmt::Match { expr, arms } => {
                let value = self.evaluate_expr(expr, env)?;

                for arm in arms {
                    if let Some(bindings) = self.patternMatch(&arm.pattern, &value, env) {
                        for (name, val) in bindings {
                            env.define(name, val);
                        }

                        // Execute the arm body
                        for stmt in &arm.body {
                            let cf = self.execute(stmt, env)?;
                            if !matches!(cf, ControlFlow::Normal(_)) {
                                return Ok(cf);
                            }
                        }

                        return Ok(ControlFlow::normal(Value::Null));
                    }
                }

                Err("No matching arm in match expression".to_string())
            }
            Stmt::Import { .. } => Ok(ControlFlow::normal(Value::Null)),
        }
    }

    fn patternMatch(
        &self,
        pattern: &crate::ast::stmt::Pattern,
        value: &Value,
        env: &Environment,
    ) -> Option<Vec<(String, Value)>> {
        use crate::ast::stmt::Pattern;
        match (pattern, value) {
            (Pattern::Wildcard, _) => Some(Vec::new()),
            (Pattern::Literal(lit), val) => {
                let pattern_val = Value::from(lit.clone());
                if pattern_val == *val {
                    Some(Vec::new())
                } else {
                    None
                }
            }
            (
                Pattern::EnumVariant(enum_name, variant),
                Value::EnumVariant {
                    enum_name: en,
                    variant: v,
                },
            ) => {
                if enum_name == en && variant == v {
                    Some(Vec::new())
                } else {
                    None
                }
            }
            (
                Pattern::Struct {
                    struct_name,
                    fields,
                },
                Value::StructInstance {
                    struct_name: sn,
                    fields: vals,
                },
            ) => {
                if struct_name != sn {
                    return None;
                }
                let mut bindings = Vec::new();
                for (field_name, field_pat) in fields {
                    let vals_ref = vals.borrow();
                    let field_val = vals_ref.get(field_name)?;
                    let mut b = self.patternMatch(field_pat, field_val, env)?;
                    bindings.append(&mut b);
                }
                Some(bindings)
            }
            (Pattern::Variable(name), val) => {
                // Disambiguate enum-variant constants from variable-binding patterns.
                // If the identifier resolves to an enum variant value, treat it as a constant pattern.
                if let (
                    Some(Value::EnumVariant {
                        enum_name: enp,
                        variant: vp,
                    }),
                    Value::EnumVariant {
                        enum_name: envv,
                        variant: vv,
                    },
                ) = (env.get(name), val)
                {
                    if enp == envv && vp == vv {
                        return Some(Vec::new());
                    }
                    return None;
                }
                Some(vec![(name.clone(), val.clone())])
            }
            _ => None,
        }
    }

    pub fn evaluate_expr(&self, expr: &Expr, env: &mut Environment) -> Result<Value, String> {
        match expr {
            Expr::Literal { value, .. } => Ok(Value::from(value.clone())),
            Expr::Variable { name, .. } => env
                .get(name)
                .cloned()
                .ok_or_else(|| format!("Undefined variable '{}'", name)),
            Expr::Binary {
                left, op, right, ..
            } => {
                let left_val = self.evaluate_expr(left, env)?;
                let right_val = self.evaluate_expr(right, env)?;
                self.evaluate_binary(left_val, op, right_val)
            }
            Expr::ArrayLiteral { elements, .. } => {
                let mut evaluated = Vec::new();
                for el in elements {
                    evaluated.push(self.evaluate_expr(el, env)?);
                }
                Ok(Value::Array(Rc::new(RefCell::new(evaluated))))
            }
            Expr::TupleLiteral { elements, .. } => {
                let mut evaluated = Vec::new();
                for el in elements {
                    evaluated.push(self.evaluate_expr(el, env)?);
                }
                Ok(Value::Tuple(Rc::new(RefCell::new(evaluated))))
            }
            Expr::DictLiteral { entries, .. } => {
                let mut m = HashMap::new();
                for (k, v) in entries {
                    let value = self.evaluate_expr(v, env)?;
                    m.insert(k.clone(), value);
                }
                Ok(Value::Dict(Rc::new(RefCell::new(m))))
            }
            Expr::Index { target, index, .. } => {
                let target_val = self.evaluate_expr(target, env)?;
                let index_val = self.evaluate_expr(index, env)?;
                match (target_val, index_val) {
                    (Value::Array(arr), Value::Int(n)) => {
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
                    (Value::Array(arr), Value::Float(n)) => {
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
                    (Value::String(s), Value::Int(n)) => {
                        let idx = n as isize;
                        if idx < 0 {
                            return Err("String index must be non-negative".to_string());
                        }
                        let idx = idx as usize;
                        match s.chars().nth(idx) {
                            Some(c) => Ok(Value::String(c.to_string())),
                            None => Err("String index out of bounds".to_string()),
                        }
                    }
                    (Value::String(s), Value::Float(n)) => {
                        if n.fract() != 0.0 {
                            return Err("String index must be an integer".to_string());
                        }
                        let idx = n as isize;
                        if idx < 0 {
                            return Err("String index must be non-negative".to_string());
                        }
                        let idx = idx as usize;
                        match s.chars().nth(idx) {
                            Some(c) => Ok(Value::String(c.to_string())),
                            None => Err("String index out of bounds".to_string()),
                        }
                    }
                    _ => Err("Indexing is only supported for arrays (number index), dictionaries (string key), and strings (number index)".to_string()),
                }
            }
            Expr::Get { object, name, .. } => {
                let obj = self.evaluate_expr(object, env)?;
                match obj {
                    Value::Dict(d) => d
                        .borrow()
                        .get(name)
                        .cloned()
                        .ok_or_else(|| "Dictionary key not found".to_string()),
                    Value::Tuple(items) => {
                        let idx: usize = name
                            .parse()
                            .map_err(|_| "Tuple access must be a numeric index".to_string())?;
                        items
                            .borrow()
                            .get(idx)
                            .cloned()
                            .ok_or_else(|| "Tuple index out of bounds".to_string())
                    }
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
                span,
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
                self.evaluate_method_call(recv, name, &evaluated_args, *span, env)
            }
            Expr::StructLiteral {
                name,
                fields: field_exprs,
                ..
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
                ..
            } => {
                // Handle Option and Result constructors specially
                if struct_name == "Option" {
                    let mut evaluated_args = Vec::new();
                    for a in args {
                        evaluated_args.push(self.evaluate_expr(a, env)?);
                    }
                    match method.as_str() {
                        "Some" => {
                            if evaluated_args.len() != 1 {
                                return Err("Option::Some expects 1 argument".to_string());
                            }
                            return Ok(Value::Option(Rc::new(RefCell::new(Some(
                                evaluated_args.remove(0),
                            )))));
                        }
                        "None" => {
                            return Ok(Value::Option(Rc::new(RefCell::new(None))));
                        }
                        _ => return Err(format!("Unknown Option method: {}", method)),
                    }
                }
                if struct_name == "Result" {
                    let mut evaluated_args = Vec::new();
                    for a in args {
                        evaluated_args.push(self.evaluate_expr(a, env)?);
                    }
                    match method.as_str() {
                        "Ok" => {
                            if evaluated_args.len() != 1 {
                                return Err("Result::Ok expects 1 argument".to_string());
                            }
                            return Ok(Value::Result(Rc::new(RefCell::new(Ok(
                                evaluated_args.remove(0)
                            )))));
                        }
                        "Err" => {
                            if evaluated_args.len() != 1 {
                                return Err("Result::Err expects 1 argument".to_string());
                            }
                            let err_msg = match evaluated_args.remove(0) {
                                Value::String(s) => s,
                                other => other.to_string(),
                            };
                            return Ok(Value::Result(Rc::new(RefCell::new(Err(err_msg)))));
                        }
                        _ => return Err(format!("Unknown Result method: {}", method)),
                    }
                }
                // Handle container constructors: Vec.new(), HashMap.new(), etc.
                if struct_name == "Vec" && method == "new" {
                    return Ok(Value::Vec(Rc::new(RefCell::new(vec![]))));
                }
                if struct_name == "LinkedList" && method == "new" {
                    return Ok(Value::LinkedList(Rc::new(RefCell::new(vec![]))));
                }
                if struct_name == "HashMap" && method == "new" {
                    return Ok(Value::HashMap(Rc::new(RefCell::new(
                        std::collections::HashMap::new(),
                    ))));
                }
                if struct_name == "Stack" && method == "new" {
                    return Ok(Value::Stack(Rc::new(RefCell::new(vec![]))));
                }
                if struct_name == "Queue" && method == "new" {
                    return Ok(Value::Queue(Rc::new(RefCell::new(vec![]))));
                }

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
            Expr::Unary { op, right, .. } => {
                let right_val = self.evaluate_expr(right, env)?;
                self.evaluate_unary(op, right_val)
            }
            Expr::InstanceOf { value, ty, .. } => {
                let v = self.evaluate_expr(value, env)?;
                let ok = match (v, ty.name.trim()) {
                    (Value::String(_), "String") => true,
                    (Value::Char(_), "char") => true,
                    (Value::Bool(_), "bool") => true,
                    (Value::Int(_), "int") => true,
                    (Value::Float(_), "int") => false,
                    (Value::Int(_), "float") => true,
                    (Value::Float(_), "float") => true,
                    (Value::Int(_), "double") => true,
                    (Value::Float(_), "double") => true,
                    (Value::Array(_), t) => t.starts_with('[') && t.ends_with(']'),
                    (Value::Dict(_), t) => t.starts_with('{') && t.ends_with('}'),
                    (Value::Tuple(_), "Tuple") => true,
                    (Value::StructInstance { struct_name, .. }, t) => struct_name == t,
                    (Value::Null, "null") => true,
                    (Value::Null, _) => false,
                    _ => false,
                };
                Ok(Value::Bool(ok))
            }
            Expr::Lambda { params, body, .. } => {
                let func = Function::new(
                    "<lambda>".to_string(),
                    params.clone(),
                    vec![Stmt::Return(Some(*body.clone()))],
                    Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        column: 0,
                    },
                    Some(env.clone()),
                );
                Ok(Value::Function(func))
            }
            Expr::Assign { name, value, .. } => {
                // fast path: x = x + a + b + ...  →  zero-copy in-place string append
                // Take ownership of x from env (avoids O(n) clone), append parts, restore.
                let parts = Self::collect_string_concat_parts(value, name);
                if !parts.is_empty() {
                    if let Some(Value::String(_)) = env.get(name) {
                        // take ownership — x is temporarily absent from env
                        if let Some(Value::String(mut base)) = env.take_local(name) {
                            let mut all_ok = true;
                            for part in &parts {
                                // try zero-copy borrow for simple variable and field references
                                let appended = match part {
                                    Expr::Variable { name: pname, .. } => {
                                        match env.get(pname) {
                                            Some(Value::String(r)) => { base.push_str(r); true }
                                            Some(Value::Int(n)) => { base.push_str(&n.to_string()); true }
                                            _ => false,
                                        }
                                    }
                                    Expr::Get { object, name: fname, .. } => {
                                        let obj = self.evaluate_expr(object, env)?;
                                        let fields_opt = match &obj {
                                            Value::StructInstance { fields, .. } => Some(fields.clone()),
                                            Value::Dict(d) => Some(d.clone()),
                                            _ => None,
                                        };
                                        if let Some(fields_rc) = fields_opt {
                                            match fields_rc.borrow().get(fname.as_str()) {
                                                Some(Value::String(r)) => { base.push_str(r); true }
                                                Some(Value::Int(n)) => { base.push_str(&n.to_string()); true }
                                                _ => false,
                                            }
                                        } else { false }
                                    }
                                    Expr::Literal { value: lit, .. } => {
                                        match lit {
                                            crate::ast::Literal::String(s) => { base.push_str(s); true }
                                            crate::ast::Literal::Int(n) => { base.push_str(&n.to_string()); true }
                                            _ => false,
                                        }
                                    }
                                    _ => {
                                        let part_val = self.evaluate_expr(part, env)?;
                                        match part_val {
                                            Value::String(r) => { base.push_str(&r); true }
                                            Value::Int(n) => { base.push_str(&n.to_string()); true }
                                            _ => false,
                                        }
                                    }
                                };
                                if !appended { all_ok = false; break; }
                            }
                            env.restore_local(name.to_string(), Value::String(base));
                            if all_ok {
                                return Ok(env.get(name).cloned().unwrap_or(Value::Null));
                            }
                            // partial failure: re-evaluate normally (base may be partially modified,
                            // but this only triggers for non-string types which don't occur in practice)
                        }
                    }
                }
                let val = self.evaluate_expr(value, env)?;
                env.assign(name, val.clone())?;
                Ok(val)
            }
            Expr::Set {
                object,
                name,
                value,
                ..
            } => {
                if matches!(object.as_ref(), Expr::Get { .. }) {
                    return Err(format!(
                        "error[E010]: nested field assignment is not supported (got '.{} = ...')",
                        name
                    ));
                }
                let obj_val = self.evaluate_expr(object, env)?;
                let val = self.evaluate_expr(value, env)?;
                match obj_val {
                    Value::Dict(d) => {
                        d.borrow_mut().insert(name.clone(), val.clone());
                        Ok(val)
                    }
                    Value::StructInstance {
                        struct_name,
                        fields,
                    } => {
                        let def = env
                            .get_struct(&struct_name)
                            .ok_or_else(|| format!("Undefined struct '{}'", struct_name))?;
                        let field =
                            def.fields.iter().find(|f| f.name == *name).ok_or_else(|| {
                                format!(
                                    "error[E004]: unknown field '{}' on struct '{}'",
                                    name, struct_name
                                )
                            })?;
                        if !field.is_pub {
                            return Err(format!(
                                "error[E011]: cannot mutate private field '{}' on struct '{}'",
                                name, struct_name
                            ));
                        }
                        if fields.borrow().contains_key(name) {
                            fields.borrow_mut().insert(name.clone(), val.clone());
                            Ok(val)
                        } else {
                            Err(format!(
                                "error[E004]: unknown field '{}' on struct '{}'",
                                name, struct_name
                            ))
                        }
                    }
                    _ => Err(
                        "Field assignment is only supported for dictionaries and structs"
                            .to_string(),
                    ),
                }
            }
            Expr::IndexSet {
                target,
                index,
                value,
                ..
            } => {
                let target_val = self.evaluate_expr(target, env)?;
                let index_val = self.evaluate_expr(index, env)?;
                let val = self.evaluate_expr(value, env)?;
                match (target_val, index_val) {
                    (Value::Array(arr), Value::Int(n)) => {
                        let idx = n as isize;
                        if idx < 0 {
                            return Err("Array index must be non-negative".to_string());
                        }
                        let idx = idx as usize;
                        let mut arr_mut = arr.borrow_mut();
                        if idx >= arr_mut.len() {
                            return Err("Array index out of bounds".to_string());
                        }
                        arr_mut[idx] = val.clone();
                        Ok(val)
                    }
                    (Value::Array(arr), Value::Float(n)) => {
                        if n.fract() != 0.0 {
                            return Err("Array index must be an integer".to_string());
                        }
                        let idx = n as isize;
                        if idx < 0 {
                            return Err("Array index must be non-negative".to_string());
                        }
                        let idx = idx as usize;
                        let mut arr_mut = arr.borrow_mut();
                        if idx >= arr_mut.len() {
                            return Err("Array index out of bounds".to_string());
                        }
                        arr_mut[idx] = val.clone();
                        Ok(val)
                    }
                    (Value::Dict(d), Value::String(s)) => {
                        d.borrow_mut().insert(s, val.clone());
                        Ok(val)
                    }
                    _ => Err("Index assignment is only supported for arrays (number index) and dictionaries (string key)".to_string()),
                }
            }
            Expr::Update {
                name, op, prefix, ..
            } => {
                let current = env
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("Undefined variable '{}'", name))?;

                match current {
                    Value::Int(n) => {
                        let delta = match op {
                            TokenKind::PlusPlus => 1,
                            TokenKind::MinusMinus => -1,
                            _ => return Err("Invalid update operator".to_string()),
                        };
                        let new_n = n + delta;
                        env.assign(name, Value::Int(new_n))?;
                        Ok(if *prefix {
                            Value::Int(new_n)
                        } else {
                            Value::Int(n)
                        })
                    }
                    Value::Float(n) => {
                        let delta = match op {
                            TokenKind::PlusPlus => 1.0,
                            TokenKind::MinusMinus => -1.0,
                            _ => return Err("Invalid update operator".to_string()),
                        };
                        let new_n = n + delta;
                        env.assign(name, Value::Float(new_n))?;
                        Ok(if *prefix {
                            Value::Float(new_n)
                        } else {
                            Value::Float(n)
                        })
                    }
                    _ => Err("Can only apply ++/-- to numbers".to_string()),
                }
            }
            Expr::Call { callee, args, span } => {
                let mut evaluated_args = Vec::new();
                for arg in args {
                    evaluated_args.push(self.evaluate_expr(arg, env)?);
                }

                if let Expr::Variable { name, .. } = callee.as_ref() {
                    if name == "assert" {
                        if evaluated_args.len() != 2 {
                            return Err(format!(
                                "assert() expects 2 arguments, got {}",
                                evaluated_args.len()
                            ));
                        }
                        let ok = match evaluated_args[0] {
                            Value::Bool(b) => b,
                            _ => return Err("assert() expects boolean condition".to_string()),
                        };
                        if !ok {
                            let msg = super::std::StdLib::formatValue(&evaluated_args[1]);
                            if span.line > 0 {
                                eprintln!(
                                    "\x1b[1;31merror[assert]\x1b[0m: {} (line {})",
                                    msg, span.line
                                );
                            } else {
                                eprintln!("\x1b[1;31merror[assert]\x1b[0m: {}", msg);
                            }
                            std::process::exit(1);
                        }
                        return Ok(Value::Null);
                    }
                    if let Some(result) =
                        super::std::StdLib::call_builtin_function(name, &evaluated_args)
                    {
                        return result;
                    }
                    // Handle constructor calls
                    match name.as_str() {
                        "Vec.new" => return Ok(Value::Vec(Rc::new(RefCell::new(vec![])))),
                        "LinkedList.new" => {
                            return Ok(Value::LinkedList(Rc::new(RefCell::new(vec![]))))
                        }
                        "HashMap.new" => {
                            return Ok(Value::HashMap(Rc::new(RefCell::new(
                                std::collections::HashMap::new(),
                            ))))
                        }
                        "Stack.new" => return Ok(Value::Stack(Rc::new(RefCell::new(vec![])))),
                        "Queue.new" => return Ok(Value::Queue(Rc::new(RefCell::new(vec![])))),
                        "Option.Some" | "Option::Some" => {
                            if args.len() != 1 {
                                return Err("Option.Some expects 1 argument".to_string());
                            }
                            return Ok(Value::Option(Rc::new(RefCell::new(Some(
                                evaluated_args.remove(0),
                            )))));
                        }
                        "Option.None" | "Option::None" => {
                            return Ok(Value::Option(Rc::new(RefCell::new(None))))
                        }
                        "Result.Ok" | "Result::Ok" => {
                            if args.len() != 1 {
                                return Err("Result.Ok expects 1 argument".to_string());
                            }
                            return Ok(Value::Result(Rc::new(RefCell::new(Ok(
                                evaluated_args.remove(0)
                            )))));
                        }
                        "Result::Err" => {
                            if args.len() != 1 {
                                return Err("Result::Err expects 1 argument".to_string());
                            }
                            let err_msg = match evaluated_args.remove(0) {
                                Value::String(s) => s,
                                other => other.to_string(),
                            };
                            return Ok(Value::Result(Rc::new(RefCell::new(Err(err_msg)))));
                        }
                        _ => {}
                    }
                }

                let function = self.evaluate_expr(callee, env)?;
                match function {
                    Value::Function(func) => {
                        let hasVariadic = func.params.last().map(|p| p.variadic).unwrap_or(false);
                        let minArgs = func
                            .params
                            .iter()
                            .filter(|p| p.default.is_none() && !p.variadic)
                            .count();
                        if evaluated_args.len() < minArgs
                            || (!hasVariadic && evaluated_args.len() > func.arity())
                        {
                            return Err(format!(
                                "Expected {}..={} arguments but got {}",
                                minArgs,
                                func.arity(),
                                evaluated_args.len()
                            ));
                        }

                        let mut function_env = if let Some(closure) = &func.closure {
                            Environment::with_parent(closure.clone())
                        } else {
                            Environment::with_parent(env.clone())
                        };

                        let mut argIndex = 0usize;
                        for param in func.params.iter() {
                            if param.variadic {
                                let mut rest = Vec::new();
                                while argIndex < evaluated_args.len() {
                                    rest.push(evaluated_args[argIndex].clone());
                                    argIndex += 1;
                                }
                                function_env.define(
                                    param.name.clone(),
                                    Value::Array(Rc::new(RefCell::new(rest))),
                                );
                                continue;
                            }

                            if argIndex < evaluated_args.len() {
                                function_env
                                    .define(param.name.clone(), evaluated_args[argIndex].clone());
                                argIndex += 1;
                                continue;
                            }

                            let def = param.default.as_ref().ok_or_else(|| {
                                format!("Missing argument '{}' and no default provided", param.name)
                            })?;
                            let val = self.evaluate_expr(def, &mut function_env)?;
                            function_env.define(param.name.clone(), val);
                        }

                        self.execute_block(&func.body, &mut function_env)
                    }
                    other => Err(format!("Can only call functions, got {}", other)),
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
        callSpan: Span,
        env: &mut Environment,
    ) -> Result<Value, String> {
        if matches!(receiver, Value::Null) {
            if callSpan.line > 0 {
                return Err(format!(
                    "error[null]: cannot call method '{}' on null (line {})",
                    name, callSpan.line
                ));
            }
            return Err(format!("error[null]: cannot call method '{}' on null", name));
        }
        match (receiver, name) {
            (Value::Dict(d), method_name) => {
                let value = d
                    .borrow()
                    .get(method_name)
                    .cloned()
                    .ok_or_else(|| format!("Namespace function '{}' not found", method_name))?;
                match value {
                    Value::Function(func) => {
                        if args.len() != func.arity() {
                            return Err(format!(
                                "Expected {} arguments but got {}",
                                func.arity(),
                                args.len()
                            ));
                        }
                        let mut function_env = Environment::with_parent(env.clone());
                        for (param, arg_value) in func.params.iter().zip(args.iter()) {
                            function_env.define(param.name.clone(), arg_value.clone());
                        }
                        self.execute_block(&func.body, &mut function_env)
                    }
                    _ => Err(format!(
                        "Namespace member '{}' is not callable",
                        method_name
                    )),
                }
            }
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
                    Ok(n) => Ok(Value::Int(n.trunc() as i64)),
                    Err(_) => Err(format!("Cannot convert string '{}' to int", s)),
                }
            }
            (Value::String(s), "toFloat") => {
                if !args.is_empty() {
                    return Err(format!("toFloat() expects 0 arguments, got {}", args.len()));
                }
                match s.parse::<f64>() {
                    Ok(n) => Ok(Value::Float(n)),
                    Err(_) => Err(format!("Cannot convert string '{}' to float", s)),
                }
            }
            (Value::Int(n), "toInt") => {
                if !args.is_empty() {
                    return Err(format!("toInt() expects 0 arguments, got {}", args.len()));
                }
                Ok(Value::Int(n))
            }
            (Value::Float(n), "toInt") => {
                if !args.is_empty() {
                    return Err(format!("toInt() expects 0 arguments, got {}", args.len()));
                }
                Ok(Value::Int(n.trunc() as i64))
            }
            (Value::Int(n), "toFloat") => {
                if !args.is_empty() {
                    return Err(format!("toFloat() expects 0 arguments, got {}", args.len()));
                }
                Ok(Value::Float(n as f64))
            }
            (Value::Float(n), "toFloat") => {
                if !args.is_empty() {
                    return Err(format!("toFloat() expects 0 arguments, got {}", args.len()));
                }
                Ok(Value::Float(n))
            }
            (Value::Array(arr), "length") => {
                if !args.is_empty() {
                    return Err(format!(
                        "{}.length() expects 0 arguments, got {}",
                        "Array",
                        args.len()
                    ));
                }
                Ok(Value::Int(arr.borrow().len() as i64))
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
                Ok(Value::Int(s.chars().count() as i64))
            }
            (Value::String(s), "len") => {
                if !args.is_empty() {
                    return Err(format!(
                        "{}.len() expects 0 arguments, got {}",
                        "String",
                        args.len()
                    ));
                }
                Ok(Value::Int(s.chars().count() as i64))
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
            (Value::String(s), "trim") => {
                if !args.is_empty() {
                    return Err(format!(
                        "{}.trim() expects 0 arguments, got {}",
                        "String",
                        args.len()
                    ));
                }
                Ok(Value::String(s.trim().to_string()))
            }
            (Value::String(s), "startsWith") => {
                if args.len() != 1 {
                    return Err(format!(
                        "{}.startsWith() expects 1 argument, got {}",
                        "String",
                        args.len()
                    ));
                }
                match &args[0] {
                    Value::String(prefix) => Ok(Value::Bool(s.starts_with(prefix))),
                    _ => Err("String.startsWith() expects a string prefix".to_string()),
                }
            }
            (Value::String(s), "endsWith") => {
                if args.len() != 1 {
                    return Err(format!(
                        "{}.endsWith() expects 1 argument, got {}",
                        "String",
                        args.len()
                    ));
                }
                match &args[0] {
                    Value::String(suffix) => Ok(Value::Bool(s.ends_with(suffix))),
                    _ => Err("String.endsWith() expects a string suffix".to_string()),
                }
            }
            (Value::String(s), "replace") => {
                if args.len() != 2 {
                    return Err(format!(
                        "{}.replace() expects 2 arguments, got {}",
                        "String",
                        args.len()
                    ));
                }
                let from = match &args[0] {
                    Value::String(v) => v.as_str(),
                    _ => return Err("String.replace() expects string 'from'".to_string()),
                };
                let to = match &args[1] {
                    Value::String(v) => v.as_str(),
                    _ => return Err("String.replace() expects string 'to'".to_string()),
                };
                Ok(Value::String(s.replace(from, to)))
            }
            (Value::String(s), "slice") => {
                if args.len() != 2 {
                    return Err(format!(
                        "{}.slice() expects 2 arguments, got {}",
                        "String",
                        args.len()
                    ));
                }
                let toIndex = |v: &Value| -> Result<isize, String> {
                    match v {
                        Value::Int(n) => Ok(*n as isize),
                        Value::Float(n) => {
                            if n.fract() != 0.0 {
                                return Err("String.slice() index must be an integer".to_string());
                            }
                            Ok(*n as isize)
                        }
                        _ => Err("String.slice() expects int indices".to_string()),
                    }
                };
                let start = toIndex(&args[0])?;
                let end = toIndex(&args[1])?;
                if start < 0 || end < 0 {
                    return Err("String.slice() indices must be non-negative".to_string());
                }
                let chars = s.chars().collect::<Vec<_>>();
                let len = chars.len() as isize;
                let start = start.min(len) as usize;
                let end = end.min(len) as usize;
                if end < start {
                    return Ok(Value::String(String::new()));
                }
                let out = chars[start..end].iter().collect::<String>();
                Ok(Value::String(out))
            }
            (Value::String(s), "indexOf") => {
                if args.len() != 1 {
                    return Err(format!(
                        "{}.indexOf() expects 1 argument, got {}",
                        "String",
                        args.len()
                    ));
                }
                let needle = match &args[0] {
                    Value::String(v) => v,
                    _ => return Err("String.indexOf() expects a string".to_string()),
                };
                match s.find(needle) {
                    Some(byte_idx) => Ok(Value::Int(s[..byte_idx].chars().count() as i64)),
                    None => Ok(Value::Int(-1)),
                }
            }
            (Value::String(s), "repeat") => {
                if args.len() != 1 {
                    return Err(format!(
                        "{}.repeat() expects 1 argument, got {}",
                        "String",
                        args.len()
                    ));
                }
                let n = match &args[0] {
                    Value::Int(v) => *v,
                    Value::Float(v) => {
                        if v.fract() != 0.0 {
                            return Err("String.repeat() expects int count".to_string());
                        }
                        *v as i64
                    }
                    _ => return Err("String.repeat() expects int count".to_string()),
                };
                if n < 0 {
                    return Err("String.repeat() count must be non-negative".to_string());
                }
                Ok(Value::String(s.repeat(n as usize)))
            }
            (Value::String(s), "padLeft") => {
                if args.len() != 2 {
                    return Err(format!(
                        "{}.padLeft() expects 2 arguments, got {}",
                        "String",
                        args.len()
                    ));
                }
                let width = match &args[0] {
                    Value::Int(v) => *v,
                    Value::Float(v) => {
                        if v.fract() != 0.0 {
                            return Err("String.padLeft() expects int width".to_string());
                        }
                        *v as i64
                    }
                    _ => return Err("String.padLeft() expects int width".to_string()),
                };
                if width < 0 {
                    return Err("String.padLeft() width must be non-negative".to_string());
                }
                let pad = match &args[1] {
                    Value::String(v) => v,
                    _ => return Err("String.padLeft() expects string pad char".to_string()),
                };
                let pad_ch = pad.chars().next().ok_or_else(|| {
                    "String.padLeft() expects non-empty string pad char".to_string()
                })?;
                let cur_len = s.chars().count() as i64;
                if cur_len >= width {
                    return Ok(Value::String(s));
                }
                let missing = (width - cur_len) as usize;
                let mut out = String::new();
                out.extend(std::iter::repeat(pad_ch).take(missing));
                out.push_str(&s);
                Ok(Value::String(out))
            }
            (Value::String(s), "padRight") => {
                if args.len() != 2 {
                    return Err(format!(
                        "{}.padRight() expects 2 arguments, got {}",
                        "String",
                        args.len()
                    ));
                }
                let width = match &args[0] {
                    Value::Int(v) => *v,
                    Value::Float(v) => {
                        if v.fract() != 0.0 {
                            return Err("String.padRight() expects int width".to_string());
                        }
                        *v as i64
                    }
                    _ => return Err("String.padRight() expects int width".to_string()),
                };
                if width < 0 {
                    return Err("String.padRight() width must be non-negative".to_string());
                }
                let pad = match &args[1] {
                    Value::String(v) => v,
                    _ => return Err("String.padRight() expects string pad char".to_string()),
                };
                let pad_ch = pad.chars().next().ok_or_else(|| {
                    "String.padRight() expects non-empty string pad char".to_string()
                })?;
                let cur_len = s.chars().count() as i64;
                if cur_len >= width {
                    return Ok(Value::String(s));
                }
                let missing = (width - cur_len) as usize;
                let mut out = s;
                out.extend(std::iter::repeat(pad_ch).take(missing));
                Ok(Value::String(out))
            }
            // Vec methods
            (Value::Vec(v), "push") => {
                if args.len() != 1 {
                    return Err("Vec.push() expects 1 argument".to_string());
                }
                v.borrow_mut().push(args[0].clone());
                Ok(Value::Null)
            }
            (Value::Vec(v), "pop") => {
                if !args.is_empty() {
                    return Err("Vec.pop() expects 0 arguments".to_string());
                }
                match v.borrow_mut().pop() {
                    Some(val) => Ok(val),
                    None => Err("Vec is empty".to_string()),
                }
            }
            (Value::Vec(v), "len") => {
                if !args.is_empty() {
                    return Err("Vec.len() expects 0 arguments".to_string());
                }
                Ok(Value::Int(v.borrow().len() as i64))
            }
            (Value::Vec(v), "get") => {
                if args.len() != 1 {
                    return Err("Vec.get() expects 1 argument".to_string());
                }
                match &args[0] {
                    Value::Int(idx) => {
                        let idx = *idx as usize;
                        match v.borrow().get(idx) {
                            Some(val) => Ok(val.clone()),
                            None => Err("Vec index out of bounds".to_string()),
                        }
                    }
                    _ => Err("Vec.get() expects int index".to_string()),
                }
            }
            (Value::Vec(v), "set") => {
                if args.len() != 2 {
                    return Err("Vec.set() expects 2 arguments".to_string());
                }
                match &args[0] {
                    Value::Int(idx) => {
                        let idx = *idx as usize;
                        let mut v_mut = v.borrow_mut();
                        if idx >= v_mut.len() {
                            return Err("Vec index out of bounds".to_string());
                        }
                        v_mut[idx] = args[1].clone();
                        Ok(Value::Null)
                    }
                    _ => Err("Vec.set() expects int index".to_string()),
                }
            }
            (Value::Vec(v), "contains") => {
                if args.len() != 1 {
                    return Err("Vec.contains() expects 1 argument".to_string());
                }
                Ok(Value::Bool(v.borrow().contains(&args[0])))
            }
            (Value::Vec(v), "indexOf") => {
                if args.len() != 1 {
                    return Err("Vec.indexOf() expects 1 argument".to_string());
                }
                let idx = v.borrow().iter().position(|x| x == &args[0]);
                match idx {
                    Some(i) => Ok(Value::Int(i as i64)),
                    None => Ok(Value::Int(-1)),
                }
            }
            (Value::Vec(v), "map") => {
                if args.len() != 1 {
                    return Err("Vec.map() expects 1 argument".to_string());
                }
                let func = match &args[0] {
                    Value::Function(f) => f,
                    _ => return Err("Vec.map() expects a function".to_string()),
                };
                let mut out = Vec::new();
                for item in v.borrow().iter() {
                    let val = self.call_value_function(func, &[item.clone()], env)?;
                    out.push(val);
                }
                Ok(Value::Vec(Rc::new(RefCell::new(out))))
            }
            (Value::Vec(v), "filter") => {
                if args.len() != 1 {
                    return Err("Vec.filter() expects 1 argument".to_string());
                }
                let func = match &args[0] {
                    Value::Function(f) => f,
                    _ => return Err("Vec.filter() expects a function".to_string()),
                };
                let mut out = Vec::new();
                for item in v.borrow().iter() {
                    let keep = self.call_value_function(func, &[item.clone()], env)?;
                    match keep {
                        Value::Bool(true) => out.push(item.clone()),
                        Value::Bool(false) => {}
                        _ => return Err("Vec.filter() predicate must return bool".to_string()),
                    }
                }
                Ok(Value::Vec(Rc::new(RefCell::new(out))))
            }
            (Value::Vec(v), "reduce") => {
                if args.len() != 2 {
                    return Err("Vec.reduce() expects 2 arguments".to_string());
                }
                let func = match &args[0] {
                    Value::Function(f) => f,
                    _ => return Err("Vec.reduce() expects a function".to_string()),
                };
                let mut acc = args[1].clone();
                for item in v.borrow().iter() {
                    acc = self.call_value_function(func, &[acc.clone(), item.clone()], env)?;
                }
                Ok(acc)
            }
            (Value::Vec(v), "reverse") => {
                if !args.is_empty() {
                    return Err("Vec.reverse() expects 0 arguments".to_string());
                }
                v.borrow_mut().reverse();
                Ok(Value::Null)
            }
            (Value::Vec(v), "sort") => {
                if !args.is_empty() {
                    return Err("Vec.sort() expects 0 arguments".to_string());
                }
                let mut v_mut = v.borrow_mut();
                if v_mut.len() <= 1 {
                    return Ok(Value::Null);
                }

                enum SortKind {
                    Int,
                    Float,
                    String,
                    Bool,
                }
                let kind = match &v_mut[0] {
                    Value::Int(_) => SortKind::Int,
                    Value::Float(_) => SortKind::Float,
                    Value::String(_) => SortKind::String,
                    Value::Bool(_) => SortKind::Bool,
                    _ => return Err("Vec.sort() only supports int, float, String, bool".to_string()),
                };

                for item in v_mut.iter() {
                    let ok = match (&kind, item) {
                        (SortKind::Int, Value::Int(_)) => true,
                        (SortKind::Float, Value::Float(_)) => true,
                        (SortKind::String, Value::String(_)) => true,
                        (SortKind::Bool, Value::Bool(_)) => true,
                        _ => false,
                    };
                    if !ok {
                        return Err("Vec.sort() requires all elements to be the same comparable type".to_string());
                    }
                }

                match kind {
                    SortKind::Int => v_mut.sort_by(|a, b| match (a, b) {
                        (Value::Int(x), Value::Int(y)) => x.cmp(y),
                        _ => std::cmp::Ordering::Equal,
                    }),
                    SortKind::Float => v_mut.sort_by(|a, b| match (a, b) {
                        (Value::Float(x), Value::Float(y)) => x
                            .partial_cmp(y)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        _ => std::cmp::Ordering::Equal,
                    }),
                    SortKind::String => v_mut.sort_by(|a, b| match (a, b) {
                        (Value::String(x), Value::String(y)) => x.cmp(y),
                        _ => std::cmp::Ordering::Equal,
                    }),
                    SortKind::Bool => v_mut.sort_by(|a, b| match (a, b) {
                        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
                        _ => std::cmp::Ordering::Equal,
                    }),
                }
                Ok(Value::Null)
            }
            (Value::Vec(v), "slice") => {
                if args.len() != 2 {
                    return Err("Vec.slice() expects 2 arguments".to_string());
                }
                let toIndex = |v: &Value| -> Result<isize, String> {
                    match v {
                        Value::Int(n) => Ok(*n as isize),
                        Value::Float(n) => {
                            if n.fract() != 0.0 {
                                return Err("Vec.slice() index must be an integer".to_string());
                            }
                            Ok(*n as isize)
                        }
                        _ => Err("Vec.slice() expects int indices".to_string()),
                    }
                };
                let start = toIndex(&args[0])?;
                let end = toIndex(&args[1])?;
                if start < 0 || end < 0 {
                    return Err("Vec.slice() indices must be non-negative".to_string());
                }
                let items = v.borrow();
                let len = items.len() as isize;
                let start = start.min(len) as usize;
                let end = end.min(len) as usize;
                if end < start {
                    return Ok(Value::Vec(Rc::new(RefCell::new(vec![]))));
                }
                Ok(Value::Vec(Rc::new(RefCell::new(
                    items[start..end].to_vec(),
                ))))
            }
            (Value::Vec(v), "join") => {
                if args.len() != 1 {
                    return Err("Vec.join() expects 1 argument".to_string());
                }
                let sep = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => return Err("Vec.join() expects string separator".to_string()),
                };
                let parts = v
                    .borrow()
                    .iter()
                    .map(|x| super::std::StdLib::formatValue(x))
                    .collect::<Vec<_>>();
                Ok(Value::String(parts.join(&sep)))
            }
            // HashMap methods
            (Value::HashMap(m), "set") => {
                if args.len() != 2 {
                    return Err("HashMap.set() expects 2 arguments".to_string());
                }
                match &args[0] {
                    Value::String(key) => {
                        m.borrow_mut().insert(key.clone(), args[1].clone());
                        Ok(Value::Null)
                    }
                    _ => Err("HashMap.set() expects string key".to_string()),
                }
            }
            (Value::HashMap(m), "get") => {
                if args.len() != 1 {
                    return Err("HashMap.get() expects 1 argument".to_string());
                }
                match &args[0] {
                    Value::String(key) => match m.borrow().get(key) {
                        Some(val) => Ok(val.clone()),
                        None => Err("HashMap key not found".to_string()),
                    },
                    _ => Err("HashMap.get() expects string key".to_string()),
                }
            }
            (Value::HashMap(m), "delete") => {
                if args.len() != 1 {
                    return Err("HashMap.delete() expects 1 argument".to_string());
                }
                match &args[0] {
                    Value::String(key) => {
                        m.borrow_mut().remove(key);
                        Ok(Value::Null)
                    }
                    _ => Err("HashMap.delete() expects string key".to_string()),
                }
            }
            (Value::HashMap(m), "has") => {
                if args.len() != 1 {
                    return Err("HashMap.has() expects 1 argument".to_string());
                }
                match &args[0] {
                    Value::String(key) => Ok(Value::Bool(m.borrow().contains_key(key))),
                    _ => Err("HashMap.has() expects string key".to_string()),
                }
            }
            (Value::HashMap(m), "len") => {
                if !args.is_empty() {
                    return Err("HashMap.len() expects 0 arguments".to_string());
                }
                Ok(Value::Int(m.borrow().len() as i64))
            }
            (Value::HashMap(m), "keys") => {
                if !args.is_empty() {
                    return Err("HashMap.keys() expects 0 arguments".to_string());
                }
                let keys = m
                    .borrow()
                    .keys()
                    .cloned()
                    .map(Value::String)
                    .collect::<Vec<_>>();
                Ok(Value::Vec(Rc::new(RefCell::new(keys))))
            }
            (Value::HashMap(m), "values") => {
                if !args.is_empty() {
                    return Err("HashMap.values() expects 0 arguments".to_string());
                }
                let values = m.borrow().values().cloned().collect::<Vec<_>>();
                Ok(Value::Vec(Rc::new(RefCell::new(values))))
            }
            (Value::HashMap(m), "entries") => {
                if !args.is_empty() {
                    return Err("HashMap.entries() expects 0 arguments".to_string());
                }
                let entries = m
                    .borrow()
                    .iter()
                    .map(|(k, v)| {
                        Value::Array(Rc::new(RefCell::new(vec![
                            Value::String(k.clone()),
                            v.clone(),
                        ])))
                    })
                    .collect::<Vec<_>>();
                Ok(Value::Vec(Rc::new(RefCell::new(entries))))
            }
            // Stack methods
            (Value::Stack(s), "push") => {
                if args.len() != 1 {
                    return Err("Stack.push() expects 1 argument".to_string());
                }
                s.borrow_mut().push(args[0].clone());
                Ok(Value::Null)
            }
            (Value::Stack(s), "pop") => {
                if !args.is_empty() {
                    return Err("Stack.pop() expects 0 arguments".to_string());
                }
                match s.borrow_mut().pop() {
                    Some(val) => Ok(val),
                    None => Err("Stack is empty".to_string()),
                }
            }
            (Value::Stack(s), "peek") => {
                if !args.is_empty() {
                    return Err("Stack.peek() expects 0 arguments".to_string());
                }
                match s.borrow().last() {
                    Some(val) => Ok(val.clone()),
                    None => Err("Stack is empty".to_string()),
                }
            }
            (Value::Stack(s), "isEmpty") => {
                if !args.is_empty() {
                    return Err("Stack.isEmpty() expects 0 arguments".to_string());
                }
                Ok(Value::Bool(s.borrow().is_empty()))
            }
            (Value::Stack(s), "len") => {
                if !args.is_empty() {
                    return Err("Stack.len() expects 0 arguments".to_string());
                }
                Ok(Value::Int(s.borrow().len() as i64))
            }
            // Queue methods
            (Value::Queue(q), "enqueue") => {
                if args.len() != 1 {
                    return Err("Queue.enqueue() expects 1 argument".to_string());
                }
                q.borrow_mut().push(args[0].clone());
                Ok(Value::Null)
            }
            (Value::Queue(q), "dequeue") => {
                if !args.is_empty() {
                    return Err("Queue.dequeue() expects 0 arguments".to_string());
                }
                let mut queue = q.borrow_mut();
                if queue.is_empty() {
                    return Err("Queue is empty".to_string());
                }
                Ok(queue.remove(0))
            }
            // LinkedList methods - same as Vec
            (Value::LinkedList(l), "push") => {
                if args.len() != 1 {
                    return Err("LinkedList.push() expects 1 argument".to_string());
                }
                l.borrow_mut().push(args[0].clone());
                Ok(Value::Null)
            }
            (Value::LinkedList(l), "pop") => {
                if !args.is_empty() {
                    return Err("LinkedList.pop() expects 0 arguments".to_string());
                }
                match l.borrow_mut().pop() {
                    Some(val) => Ok(val),
                    None => Err("LinkedList is empty".to_string()),
                }
            }
            (Value::LinkedList(l), "len") => {
                if !args.is_empty() {
                    return Err("LinkedList.len() expects 0 arguments".to_string());
                }
                Ok(Value::Int(l.borrow().len() as i64))
            }
            (Value::Queue(q), "peek") => {
                if !args.is_empty() {
                    return Err("Queue.peek() expects 0 arguments".to_string());
                }
                match q.borrow().get(0) {
                    Some(val) => Ok(val.clone()),
                    None => Err("Queue is empty".to_string()),
                }
            }
            (Value::Queue(q), "isEmpty") => {
                if !args.is_empty() {
                    return Err("Queue.isEmpty() expects 0 arguments".to_string());
                }
                Ok(Value::Bool(q.borrow().is_empty()))
            }
            (Value::Queue(q), "len") => {
                if !args.is_empty() {
                    return Err("Queue.len() expects 0 arguments".to_string());
                }
                Ok(Value::Int(q.borrow().len() as i64))
            }
            // Option methods
            (Value::Option(o), "unwrap") => {
                if !args.is_empty() {
                    return Err("Option.unwrap() expects 0 arguments".to_string());
                }
                match o.borrow().as_ref() {
                    Some(val) => Ok(val.clone()),
                    None => Err("Cannot unwrap None".to_string()),
                }
            }
            (Value::Option(o), "unwrapOr") => {
                if args.len() != 1 {
                    return Err("Option.unwrapOr() expects 1 argument".to_string());
                }
                match o.borrow().as_ref() {
                    Some(val) => Ok(val.clone()),
                    None => Ok(args[0].clone()),
                }
            }
            (Value::Option(o), "isSome") => {
                if !args.is_empty() {
                    return Err("Option.isSome() expects 0 arguments".to_string());
                }
                Ok(Value::Bool(o.borrow().is_some()))
            }
            (Value::Option(o), "isNone") => {
                if !args.is_empty() {
                    return Err("Option.isNone() expects 0 arguments".to_string());
                }
                Ok(Value::Bool(o.borrow().is_none()))
            }
            // Result methods
            (Value::Result(r), "unwrap") => {
                if !args.is_empty() {
                    return Err("Result.unwrap() expects 0 arguments".to_string());
                }
                match r.borrow().as_ref() {
                    Ok(val) => Ok(val.clone()),
                    Err(e) => Err(format!("Cannot unwrap Err: {}", e)),
                }
            }
            (Value::Result(r), "unwrapOr") => {
                if args.len() != 1 {
                    return Err("Result.unwrapOr() expects 1 argument".to_string());
                }
                match r.borrow().as_ref() {
                    Ok(val) => Ok(val.clone()),
                    Err(_) => Ok(args[0].clone()),
                }
            }
            (Value::Result(r), "isOk") => {
                if !args.is_empty() {
                    return Err("Result.isOk() expects 0 arguments".to_string());
                }
                Ok(Value::Bool(r.borrow().is_ok()))
            }
            (Value::Result(r), "isErr") => {
                if !args.is_empty() {
                    return Err("Result.isErr() expects 0 arguments".to_string());
                }
                Ok(Value::Bool(r.borrow().is_err()))
            }
            (other, _) => Err(format!("Method call not supported on {:?}", other)),
        }
    }

    fn call_value_function(
        &self,
        func: &Function,
        args: &[Value],
        env: &mut Environment,
    ) -> Result<Value, String> {
        let hasVariadic = func.params.last().map(|p| p.variadic).unwrap_or(false);
        let minArgs = func
            .params
            .iter()
            .filter(|p| p.default.is_none() && !p.variadic)
            .count();

        if args.len() < minArgs || (!hasVariadic && args.len() > func.arity()) {
            return Err(format!(
                "Expected {}..={} arguments but got {}",
                minArgs,
                func.arity(),
                args.len()
            ));
        }

        let mut function_env = if let Some(closure) = &func.closure {
            Environment::with_parent(closure.clone())
        } else {
            Environment::with_parent(env.clone())
        };

        let mut argIndex = 0usize;
        for param in func.params.iter() {
            if param.variadic {
                let mut rest = Vec::new();
                while argIndex < args.len() {
                    rest.push(args[argIndex].clone());
                    argIndex += 1;
                }
                function_env.define(
                    param.name.clone(),
                    Value::Array(Rc::new(RefCell::new(rest))),
                );
                continue;
            }

            if argIndex < args.len() {
                function_env.define(param.name.clone(), args[argIndex].clone());
                argIndex += 1;
                continue;
            }

            let def = param
                .default
                .as_ref()
                .ok_or_else(|| format!("Missing argument '{}' and no default provided", param.name))?;
            let val = self.evaluate_expr(def, &mut function_env)?;
            function_env.define(param.name.clone(), val);
        }

        self.execute_block(&func.body, &mut function_env)
    }

    fn isTruthy(&self, value: &Value) -> bool {
        match value {
            Value::Bool(false) => false,
            Value::Null => false,
            Value::Int(n) => *n != 0,
            Value::Float(n) => *n != 0.0,
            _ => true,
        }
    }

    fn evaluate_binary(&self, left: Value, op: &TokenKind, right: Value) -> Result<Value, String> {
        use TokenKind::*;

        match (left, op, right) {
            (Value::Int(l), Plus, Value::Int(r)) => Ok(Value::Int(l + r)),
            (Value::Int(l), Minus, Value::Int(r)) => Ok(Value::Int(l - r)),
            (Value::Int(l), Star, Value::Int(r)) => Ok(Value::Int(l * r)),
            (Value::Int(l), Slash, Value::Int(r)) => {
                if r == 0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Int(l / r))
                }
            }
            (Value::Int(l), Percent, Value::Int(r)) => {
                if r == 0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Int(l % r))
                }
            }

            (Value::Float(l), Plus, Value::Float(r)) => Ok(Value::Float(l + r)),
            (Value::Float(l), Minus, Value::Float(r)) => Ok(Value::Float(l - r)),
            (Value::Float(l), Star, Value::Float(r)) => Ok(Value::Float(l * r)),
            (Value::Float(l), Slash, Value::Float(r)) => {
                if r == 0.0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Float(l / r))
                }
            }
            (Value::Float(l), Percent, Value::Float(r)) => {
                if r == 0.0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Float(l % r))
                }
            }

            (Value::Int(l), Plus, Value::Float(r)) => Ok(Value::Float((l as f64) + r)),
            (Value::Float(l), Plus, Value::Int(r)) => Ok(Value::Float(l + (r as f64))),
            (Value::Int(l), Minus, Value::Float(r)) => Ok(Value::Float((l as f64) - r)),
            (Value::Float(l), Minus, Value::Int(r)) => Ok(Value::Float(l - (r as f64))),
            (Value::Int(l), Star, Value::Float(r)) => Ok(Value::Float((l as f64) * r)),
            (Value::Float(l), Star, Value::Int(r)) => Ok(Value::Float(l * (r as f64))),
            (Value::Int(l), Slash, Value::Float(r)) => {
                if r == 0.0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Float((l as f64) / r))
                }
            }
            (Value::Float(l), Slash, Value::Int(r)) => {
                if r == 0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Float(l / (r as f64)))
                }
            }
            (Value::Int(l), Percent, Value::Float(r)) => {
                if r == 0.0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Float((l as f64) % r))
                }
            }
            (Value::Float(l), Percent, Value::Int(r)) => {
                if r == 0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Float(l % (r as f64)))
                }
            }
            (Value::Null, EqualEqual, Value::Null) => Ok(Value::Bool(true)),
            (Value::Null, NotEqual, Value::Null) => Ok(Value::Bool(false)),
            (Value::Null, EqualEqual, _) | (_, EqualEqual, Value::Null) => Ok(Value::Bool(false)),
            (Value::Null, NotEqual, _) | (_, NotEqual, Value::Null) => Ok(Value::Bool(true)),
            (Value::Int(l), EqualEqual, Value::Int(r)) => Ok(Value::Bool(l == r)),
            (Value::Int(l), NotEqual, Value::Int(r)) => Ok(Value::Bool(l != r)),
            (Value::Int(l), Less, Value::Int(r)) => Ok(Value::Bool(l < r)),
            (Value::Int(l), LessEqual, Value::Int(r)) => Ok(Value::Bool(l <= r)),
            (Value::Int(l), Greater, Value::Int(r)) => Ok(Value::Bool(l > r)),
            (Value::Int(l), GreaterEqual, Value::Int(r)) => Ok(Value::Bool(l >= r)),

            (Value::Float(l), EqualEqual, Value::Float(r)) => Ok(Value::Bool(l == r)),
            (Value::Float(l), NotEqual, Value::Float(r)) => Ok(Value::Bool(l != r)),
            (Value::Float(l), Less, Value::Float(r)) => Ok(Value::Bool(l < r)),
            (Value::Float(l), LessEqual, Value::Float(r)) => Ok(Value::Bool(l <= r)),
            (Value::Float(l), Greater, Value::Float(r)) => Ok(Value::Bool(l > r)),
            (Value::Float(l), GreaterEqual, Value::Float(r)) => Ok(Value::Bool(l >= r)),

            (Value::Int(l), EqualEqual, Value::Float(r)) => Ok(Value::Bool((l as f64) == r)),
            (Value::Float(l), EqualEqual, Value::Int(r)) => Ok(Value::Bool(l == (r as f64))),
            (Value::Int(l), NotEqual, Value::Float(r)) => Ok(Value::Bool((l as f64) != r)),
            (Value::Float(l), NotEqual, Value::Int(r)) => Ok(Value::Bool(l != (r as f64))),
            (Value::Int(l), Less, Value::Float(r)) => Ok(Value::Bool((l as f64) < r)),
            (Value::Float(l), Less, Value::Int(r)) => Ok(Value::Bool(l < (r as f64))),
            (Value::Int(l), LessEqual, Value::Float(r)) => Ok(Value::Bool((l as f64) <= r)),
            (Value::Float(l), LessEqual, Value::Int(r)) => Ok(Value::Bool(l <= (r as f64))),
            (Value::Int(l), Greater, Value::Float(r)) => Ok(Value::Bool((l as f64) > r)),
            (Value::Float(l), Greater, Value::Int(r)) => Ok(Value::Bool(l > (r as f64))),
            (Value::Int(l), GreaterEqual, Value::Float(r)) => Ok(Value::Bool((l as f64) >= r)),
            (Value::Float(l), GreaterEqual, Value::Int(r)) => Ok(Value::Bool(l >= (r as f64))),

            (Value::String(l), Plus, r) => {
                Ok(Value::String(l + &super::std::StdLib::formatValue(&r)))
            }
            (l, Plus, Value::String(r)) => {
                Ok(Value::String(super::std::StdLib::formatValue(&l) + &r))
            }
            (Value::String(l), EqualEqual, Value::String(r)) => Ok(Value::Bool(l == r)),
            (Value::String(l), NotEqual, Value::String(r)) => Ok(Value::Bool(l != r)),
            (Value::String(l), Less, Value::String(r)) => Ok(Value::Bool(l < r)),
            (Value::String(l), LessEqual, Value::String(r)) => Ok(Value::Bool(l <= r)),
            (Value::String(l), Greater, Value::String(r)) => Ok(Value::Bool(l > r)),
            (Value::String(l), GreaterEqual, Value::String(r)) => Ok(Value::Bool(l >= r)),

            (Value::Bool(l), EqualEqual, Value::Bool(r)) => Ok(Value::Bool(l == r)),
            (Value::Bool(l), NotEqual, Value::Bool(r)) => Ok(Value::Bool(l != r)),
            (Value::Bool(l), AndAnd, Value::Bool(r)) => Ok(Value::Bool(l && r)),
            (Value::Bool(l), OrOr, Value::Bool(r)) => Ok(Value::Bool(l || r)),

            _ => Err("Invalid binary operation".to_string()),
        }
    }

    // collect parts of a left-recursive `base + a + b + c` chain rooted at `base_name`
    // returns the list [a, b, c] if the chain starts with Variable(base_name); empty otherwise
    fn collect_string_concat_parts<'a>(expr: &'a Expr, base_name: &str) -> Vec<&'a Expr> {
        match expr {
            Expr::Binary { left, op, right, .. } if *op == TokenKind::Plus => {
                let mut parts = Self::collect_string_concat_parts(left, base_name);
                if !parts.is_empty() || matches!(left.as_ref(), Expr::Variable { name, .. } if name == base_name) {
                    if parts.is_empty() {
                        // left is exactly Variable(base_name), start the list
                    }
                    parts.push(right.as_ref());
                    parts
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }

    fn evaluate_unary(&self, op: &TokenKind, right: Value) -> Result<Value, String> {
        use TokenKind::*;

        match (op, right) {
            (Minus, Value::Int(n)) => Ok(Value::Int(-n)),
            (Minus, Value::Float(n)) => Ok(Value::Float(-n)),
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
