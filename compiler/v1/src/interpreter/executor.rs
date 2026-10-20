use super::control_flow::ControlFlow;
use super::environment::Environment;
use super::function::Function;
use super::value::{floatToInt, SharedBuf, SharedStr, StructDef, Value};
use crate::ast::{Expr, Stmt};
use crate::lexer::span::Span;
use crate::lexer::TokenKind;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Executor {
    program_args: Vec<String>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            program_args: Vec::new(),
        }
    }

    pub fn with_program_args(program_args: Vec<String>) -> Self {
        if program_args.is_empty() {
            Self::new()
        } else {
            Self { program_args }
        }
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
                    Some(env.clone()),
                );
                env.define(name.clone(), Value::function(function));
                Ok(ControlFlow::normal(Value::Null))
            }
            Stmt::StructDecl {
                name,
                fields,
                methods,
                ..
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
                    match self.execute_scoped_block_with_control_flow(then_branch, env)? {
                        ControlFlow::Normal(_) => {}
                        ControlFlow::Return(value) => return Ok(ControlFlow::Return(value)),
                        ControlFlow::Break => return Ok(ControlFlow::Break),
                        ControlFlow::Continue => return Ok(ControlFlow::Continue),
                    }
                } else if let Some(else_branch) = else_branch {
                    match self.execute_scoped_block_with_control_flow(else_branch, env)? {
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
                    match self.execute_scoped_block_with_control_flow(body, env)? {
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
                    match self.execute_scoped_block_with_control_flow(body, env)? {
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
                                floatToInt(n)?
                            }
                            _ => return Err("Range start must be a number".to_string()),
                        };
                        let end_num = match end_val {
                            Value::Int(n) => n,
                            Value::Float(n) => {
                                if n.fract() != 0.0 {
                                    return Err("Range end must be an integer".to_string());
                                }
                                floatToInt(n)?
                            }
                            _ => return Err("Range end must be a number".to_string()),
                        };

                        for i in start_num..end_num {
                            let mut iteration_env = Environment::with_parent(env.clone());
                            iteration_env.define(variable.clone(), Value::Int(i));

                            match self.execute_block_with_control_flow(body, &mut iteration_env)? {
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
                                let items = arr.borrow().clone();
                                for item in items {
                                    let mut iteration_env = Environment::with_parent(env.clone());
                                    iteration_env.define(variable.clone(), item);

                                    match self.execute_block_with_control_flow(
                                        body,
                                        &mut iteration_env,
                                    )? {
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
            Stmt::EnumDecl { name, variants, .. } => {
                env.register_enum(name.clone(), variants.clone());
                Ok(ControlFlow::normal(Value::Null))
            }
            Stmt::EnumDeclWithPayload { name, variants, .. } => {
                env.register_enum(
                    name.clone(),
                    variants.iter().map(|variant| variant.name.clone()).collect(),
                );
                Ok(ControlFlow::normal(Value::Null))
            }
            Stmt::Match { expr, arms } => {
                let value = self.evaluate_expr(expr, env)?;

                for arm in arms {
                    if let Some(bindings) = self.patternMatch(&arm.pattern, &value, env) {
                        let mut arm_env = Environment::with_parent(env.clone());
                        for (name, val) in bindings {
                            arm_env.define(name, val);
                        }

                        // Execute the arm body
                        let cf = self.execute_block_with_control_flow(&arm.body, &mut arm_env)?;
                        if !matches!(cf, ControlFlow::Normal(_)) {
                            return Ok(cf);
                        }

                        return Ok(ControlFlow::normal(Value::Null));
                    }
                }

                Err("error[match]: non-exhaustive patterns".to_string())
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
                Pattern::Range {
                    start: crate::ast::Literal::Int(start),
                    end: crate::ast::Literal::Int(end),
                },
                Value::Int(value),
            ) if start <= value && value <= end => Some(Vec::new()),
            (
                Pattern::Range {
                    start: crate::ast::Literal::Char(start),
                    end: crate::ast::Literal::Char(end),
                },
                Value::Char(value),
            ) if (*start as u32) <= u8::MAX as u32
                && (*end as u32) <= u8::MAX as u32
                && (*start as u8) <= *value
                && *value <= (*end as u8) =>
            {
                Some(Vec::new())
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
                Pattern::EnumVariant(enum_name, variant),
                Value::EnumVariantPayload { enum_name: en, variant: v, .. },
            ) => {
                if enum_name == en && variant == v {
                    Some(Vec::new())
                } else {
                    None
                }
            }
            (
                Pattern::EnumVariantPayload {
                    enum_name,
                    variant,
                    payloads,
                },
                Value::EnumVariantPayload {
                    enum_name: en,
                    variant: v,
                    payloads: values,
                },
            ) => {
                if enum_name.as_ref().is_some_and(|name| name != en)
                    || variant != v
                    || payloads.len() != values.len()
                {
                    return None;
                }
                let mut bindings = Vec::new();
                for (payload, value) in payloads.iter().zip(values) {
                    bindings.extend(self.patternMatch(payload, value, env)?);
                }
                Some(bindings)
            }
            (
                Pattern::EnumVariantPayload {
                    enum_name,
                    variant,
                    payloads,
                },
                Value::EnumVariant {
                    enum_name: en,
                    variant: v,
                },
            ) if payloads.is_empty()
                && enum_name.as_ref().map_or(true, |name| name == en)
                && variant == v =>
            {
                Some(Vec::new())
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
                if let Some(Value::EnumVariant { enum_name: patternEnum, variant: patternVariant }) = env.get(name) {
                    let matches = match val {
                        Value::EnumVariant { enum_name, variant }
                        | Value::EnumVariantPayload { enum_name, variant, .. } => {
                            patternEnum.asStr() == enum_name.asStr()
                                && patternVariant.asStr() == variant.asStr()
                        }
                        _ => false,
                    };
                    return if matches { Some(Vec::new()) } else { None };
                }
                Some(vec![(name.clone(), val.clone())])
            }
            _ => None,
        }
    }

    pub fn evaluate_expr(&self, expr: &Expr, env: &mut Environment) -> Result<Value, String> {
        match expr {
            Expr::Literal {
                value: crate::ast::Literal::Char(c),
                ..
            } if (*c as u32) > u8::MAX as u32 => {
                Err("char literal out of range 0..=255".to_string())
            }
            Expr::Literal { value, .. } => Ok(Value::from(value.clone())),
            Expr::Variable { name, .. } => env
                .get(name)
                .ok_or_else(|| format!("Undefined variable '{}'", name)),
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => {
                let left_val = self.evaluate_expr(left, env)?;
                match (&left_val, op) {
                    (Value::Bool(false), TokenKind::AndAnd) => return Ok(Value::Bool(false)),
                    (Value::Bool(true), TokenKind::OrOr) => return Ok(Value::Bool(true)),
                    _ => {}
                }
                // parser-lowered interpolation joins use a zero-width synthetic span
                if *op == TokenKind::Plus && span.start == 0 && span.end == 0 && left_val.stringBytes().is_some()
                {
                    let right_val = self.evaluate_expr(right, env)?;
                    let extra = super::std::StdLib::formatValueBytes(&right_val);
                    return Ok(appendFormatted(left_val, &extra));
                }
                let right_val = self.evaluate_expr(right, env)?;
                self.evaluate_binary(left_val, op, right_val, span.start)
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
                    m.insert(k.as_bytes().to_vec(), value);
                }
                Ok(Value::Dict(Rc::new(RefCell::new(m))))
            }
            Expr::DictByteLiteral { entries, .. } => {
                let mut values = HashMap::new();
                for (key, expression) in entries {
                    values.insert(key.clone(), self.evaluate_expr(expression, env)?);
                }
                Ok(Value::Dict(Rc::new(RefCell::new(values))))
            }
            Expr::Index {
                target,
                index,
                span,
            } => {
                let target_val = self.evaluate_expr(target, env)?;
                let index_val = self.evaluate_expr(index, env)?;
                if matches!(target_val, Value::Null) {
                    return Err(self.null_dereference_error(span.start));
                }
                match (target_val, index_val) {
                    (Value::Array(arr), Value::Int(n)) => {
                        let idx = nonNegativeIndex(n, "Array")?;
                        let arr_ref = arr.borrow();
                        arr_ref
                            .get(idx)
                            .cloned()
                            .ok_or_else(|| self.index_out_of_bounds_error(idx, arr_ref.len()))
                    }
                    (Value::Array(arr), Value::Float(n)) => {
                        let idx = nonNegativeFloatIndex(n, "Array")?;
                        let arr_ref = arr.borrow();
                        arr_ref
                            .get(idx)
                            .cloned()
                            .ok_or_else(|| self.index_out_of_bounds_error(idx, arr_ref.len()))
                    }
                    (Value::Dict(d) | Value::Namespace(d), key)
                        if key.stringBytes().is_some() => d
                        .borrow()
                        .get(key.stringBytes().unwrap())
                        .cloned()
                        .ok_or_else(|| "Dictionary key not found".to_string()),
                    (string, Value::Int(n)) if string.stringBytes().is_some() => {
                        let idx = nonNegativeIndex(n, "String")?;
                        let bytes = string.stringBytes().unwrap();
                        match bytes.get(idx) {
                            Some(byte) => Ok(Value::Char(*byte)),
                            None => Err(self.index_out_of_bounds_error(idx, bytes.len())),
                        }
                    }
                    (string, Value::Float(n)) if string.stringBytes().is_some() => {
                        let idx = nonNegativeFloatIndex(n, "String")?;
                        let bytes = string.stringBytes().unwrap();
                        match bytes.get(idx) {
                            Some(byte) => Ok(Value::Char(*byte)),
                            None => Err(self.index_out_of_bounds_error(idx, bytes.len())),
                        }
                    }
                    (Value::Vec(v), Value::Int(n)) => {
                        let idx = nonNegativeIndex(n, "Vec")?;
                        let v_ref = v.borrow();
                        v_ref.get(idx).cloned().ok_or_else(|| self.index_out_of_bounds_error(idx, v_ref.len()))
                    }
                    _ => Err("Indexing is only supported for arrays (number index), dictionaries (string key), and strings (number index)".to_string()),
                }
            }
            Expr::Get { object, name, span } => {
                let obj = self.evaluate_expr(object, env)?;
                if matches!(obj, Value::Null) {
                    return Err(self.null_dereference_error(span.start));
                }
                match obj {
                    Value::Dict(d) | Value::Namespace(d) => d
                        .borrow()
                        .get(name.as_bytes())
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
                if name == "toString" && args.is_empty() && matches!(recv, Value::Null) {
                    return Ok(Value::utf8("null"));
                }
                if matches!(recv, Value::Null) {
                    return Err(self.null_dereference_error(span.start));
                }
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
                        struct_name.asStr(),
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
                    struct_name: SharedStr::from(name.as_str()),
                    fields: Rc::new(RefCell::new(field_map)),
                })
            }
            Expr::EnumVariant {
                enum_name,
                variant,
                args,
                ..
            } => {
                let explicit_owner = enum_name.clone();
                let enum_name = match enum_name {
                    Some(name) => name.clone(),
                    None => env
                        .get_enum_for_variant(variant)
                        .ok_or_else(|| format!("Unknown enum variant '{}'", variant))?,
                };
                let mut payloads = Vec::new();
                for arg in args {
                    payloads.push(self.evaluate_expr(arg, env)?);
                }
                if let Some(owner) = explicit_owner.as_deref() {
                    if let Some(value) = Self::taggedConstructor(owner, variant, &payloads)? {
                        return Ok(value);
                    }
                }
                let enumName = SharedStr::from(enum_name);
                let variantName = SharedStr::from(variant.as_str());
                if payloads.is_empty() {
                    Ok(Value::EnumVariant {
                        enum_name: enumName,
                        variant: variantName,
                    })
                } else {
                    Ok(Value::EnumVariantPayload {
                        enum_name: enumName,
                        variant: variantName,
                        payloads,
                    })
                }
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
                    return Self::taggedConstructor(struct_name, method, &evaluated_args)?
                        .ok_or_else(|| format!("Unknown Option method: {}", method));
                }
                if struct_name == "Result" {
                    let mut evaluated_args = Vec::new();
                    for a in args {
                        evaluated_args.push(self.evaluate_expr(a, env)?);
                    }
                    return Self::taggedConstructor(struct_name, method, &evaluated_args)?
                        .ok_or_else(|| format!("Unknown Result method: {}", method));
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
                if method == "create" && env.soleStructName().as_deref() == Some(struct_name.as_str()) {
                    if let Some(Value::Function(func)) = env.get("create") {
                        let mut evaluatedArgs = Vec::new();
                        for arg in args {
                            evaluatedArgs.push(self.evaluate_expr(arg, env)?);
                        }
                        return self.call_value_function(&func, &evaluatedArgs, env);
                    }
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

                let method_parent = env
                    .get_struct_context(struct_name)
                    .map(|context| (*context).clone())
                    .unwrap_or_else(|| env.clone());
                let mut method_env = Environment::with_parent(method_parent);

                // Bind parameters
                for (param, val) in method_decl.params.iter().zip(&evaluated_args) {
                    method_env.define(param.name.clone(), val.clone());
                }

                // Execute the static method body
                self.execute_block(&method_decl.body, &mut method_env)
            }
            Expr::Unary { op, right, span } => {
                let right_val = self.evaluate_expr(right, env)?;
                self.evaluate_unary(op, right_val, span.start)
            }
            Expr::InstanceOf { value, ty, .. } => {
                let v = self.evaluate_expr(value, env)?;
                let ok = match (v, ty.name.trim()) {
                    (Value::String(_) | Value::ByteString(_), "string") => true,
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
                    Some(env.snapshot_for_capture()),
                );
                Ok(Value::function(func))
            }
            Expr::Assign { name, value, .. } => {
                let parts = Self::collect_string_concat_parts(value, name);
                if !parts.is_empty() {
                    if let Some(result) = self.assignStringConcat(name, &parts, env)? {
                        return Ok(result);
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
                span,
            } => {
                if matches!(object.as_ref(), Expr::Get { .. }) {
                    return Err(format!(
                        "error[E010]: nested field assignment is not supported (got '.{} = ...')",
                        name
                    ));
                }
                let obj_val = self.evaluate_expr(object, env)?;
                if matches!(obj_val, Value::Null) {
                    return Err(self.null_dereference_error(span.start));
                }
                let val = self.evaluate_expr(value, env)?;
                match obj_val {
                    Value::Dict(d) => {
                        d.borrow_mut()
                            .insert(name.as_bytes().to_vec(), val.clone());
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
                span,
            } => {
                let target_val = self.evaluate_expr(target, env)?;
                if matches!(target_val, Value::Null) {
                    return Err(self.null_dereference_error(span.start));
                }
                let index_val = self.evaluate_expr(index, env)?;
                let val = self.evaluate_expr(value, env)?;
                match (target_val, index_val) {
                    (Value::Array(arr), Value::Int(n)) => {
                        let idx = nonNegativeIndex(n, "Array")?;
                        let mut arr_mut = arr.borrow_mut();
                        if idx >= arr_mut.len() {
                            return Err(self.index_out_of_bounds_error(idx, arr_mut.len()));
                        }
                        arr_mut[idx] = val.clone();
                        Ok(val)
                    }
                    (Value::Array(arr), Value::Float(n)) => {
                        let idx = nonNegativeFloatIndex(n, "Array")?;
                        let mut arr_mut = arr.borrow_mut();
                        if idx >= arr_mut.len() {
                            return Err(self.index_out_of_bounds_error(idx, arr_mut.len()));
                        }
                        arr_mut[idx] = val.clone();
                        Ok(val)
                    }
                    (Value::Dict(d), key) if key.stringBytes().is_some() => {
                        d.borrow_mut()
                            .insert(key.stringBytes().unwrap().to_vec(), val.clone());
                        Ok(val)
                    }
                    _ => Err("Index assignment is only supported for arrays (number index) and dictionaries (string key)".to_string()),
                }
            }
            Expr::Update {
                name,
                op,
                prefix,
                span,
            } => self.evaluate_update_lvalue(
                &Expr::Variable {
                    name: name.clone(),
                    span: *span,
                },
                op,
                *prefix,
                env,
            ),
            Expr::UpdateLValue {
                target,
                op,
                prefix,
                ..
            } => self.evaluate_update_lvalue(target, op, *prefix, env),
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
                    if let Some(result) = super::std::StdLib::call_builtin_function(
                        name,
                        &evaluated_args,
                        &self.program_args,
                    )
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
                            return Self::taggedConstructor("Option", "Some", &evaluated_args)?
                                .ok_or_else(|| "Unknown Option method: Some".to_string())
                        }
                        "Option.None" | "Option::None" => {
                            return Self::taggedConstructor("Option", "None", &evaluated_args)?
                                .ok_or_else(|| "Unknown Option method: None".to_string())
                        }
                        "Result.Ok" | "Result::Ok" => {
                            return Self::taggedConstructor("Result", "Ok", &evaluated_args)?
                                .ok_or_else(|| "Unknown Result method: Ok".to_string())
                        }
                        "Result.Err" | "Result::Err" => {
                            return Self::taggedConstructor("Result", "Err", &evaluated_args)?
                                .ok_or_else(|| "Unknown Result method: Err".to_string())
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
                            Environment::with_parent_rc(Rc::clone(closure))
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

    fn evaluate_update_lvalue(
        &self,
        target: &Expr,
        op: &TokenKind,
        prefix: bool,
        env: &mut Environment,
    ) -> Result<Value, String> {
        let update = |current: Value| -> Result<(Value, Value), String> {
            let n = match current {
                Value::Int(n) => n,
                _ => return Err("Can only apply ++/-- to int lvalues".to_string()),
            };
            let delta = match op {
                TokenKind::PlusPlus => 1,
                TokenKind::MinusMinus => -1,
                _ => return Err("Invalid update operator".to_string()),
            };
            let new_n = n
                .checked_add(delta)
                .ok_or_else(|| "Integer overflow in ++/--".to_string())?;
            let result = if prefix { new_n } else { n };
            Ok((Value::Int(new_n), Value::Int(result)))
        };

        match target {
            Expr::Variable { name, .. } => {
                let current = env
                    .get(name)
                    .ok_or_else(|| format!("Undefined variable '{}'", name))?;
                let (new_value, result) = update(current)?;
                env.assign(name, new_value)?;
                Ok(result)
            }
            Expr::Get { object, name, span } => {
                let object_value = self.evaluate_expr(object, env)?;
                if matches!(object_value, Value::Null) {
                    return Err(self.null_dereference_error(span.start));
                }
                match object_value {
                    Value::Dict(dict) => {
                        let current = dict
                            .borrow()
                            .get(name.as_bytes())
                            .cloned()
                            .ok_or_else(|| "Dictionary key not found".to_string())?;
                        let (new_value, result) = update(current)?;
                        dict.borrow_mut()
                            .insert(name.as_bytes().to_vec(), new_value);
                        Ok(result)
                    }
                    Value::StructInstance {
                        struct_name,
                        fields,
                    } => {
                        let def = env
                            .get_struct(&struct_name)
                            .ok_or_else(|| format!("Undefined struct '{}'", struct_name))?;
                        let field = def.fields.iter().find(|field| field.name == *name).ok_or_else(
                            || {
                                format!(
                                    "error[E004]: unknown field '{}' on struct '{}'",
                                    name, struct_name
                                )
                            },
                        )?;
                        if !field.is_pub {
                            return Err(format!(
                                "error[E011]: cannot mutate private field '{}' on struct '{}'",
                                name, struct_name
                            ));
                        }
                        let current = fields
                            .borrow()
                            .get(name)
                            .cloned()
                            .ok_or_else(|| format!("Unknown field '{}'", name))?;
                        let (new_value, result) = update(current)?;
                        fields.borrow_mut().insert(name.clone(), new_value);
                        Ok(result)
                    }
                    _ => Err("Field update requires a dictionary or struct".to_string()),
                }
            }
            Expr::Index {
                target,
                index,
                span,
            } => {
                let container = self.evaluate_expr(target, env)?;
                if matches!(container, Value::Null) {
                    return Err(self.null_dereference_error(span.start));
                }
                let index_value = self.evaluate_expr(index, env)?;
                if let Some(key) = index_value.stringBytes() {
                    let Value::Dict(dict) = &container else {
                        return Err("String index update requires a dictionary".to_string());
                    };
                    let dict = dict.clone();
                    let key = key.to_vec();
                    let current = dict
                        .borrow()
                        .get(&key)
                        .cloned()
                        .ok_or_else(|| "Dictionary key not found".to_string())?;
                    let (new_value, result) = update(current)?;
                    dict.borrow_mut().insert(key, new_value);
                    return Ok(result);
                }
                let index = match index_value {
                    Value::Int(n) => nonNegativeIndex(n, "Index")?,
                    Value::Float(n) => nonNegativeFloatIndex(n, "Index")?,
                    _ => return Err("Index must be an integer or dictionary key".to_string()),
                };

                match container {
                    Value::Array(array) => {
                        let current = {
                            let values = array.borrow();
                            values
                                .get(index)
                                .cloned()
                                .ok_or_else(|| self.index_out_of_bounds_error(index, values.len()))?
                        };
                        let (new_value, result) = update(current)?;
                        array.borrow_mut()[index] = new_value;
                        Ok(result)
                    }
                    Value::Vec(values) => {
                        let current = {
                            let values_ref = values.borrow();
                            values_ref.get(index).cloned().ok_or_else(|| {
                                self.index_out_of_bounds_error(index, values_ref.len())
                            })?
                        };
                        let (new_value, result) = update(current)?;
                        values.borrow_mut()[index] = new_value;
                        Ok(result)
                    }
                    _ => Err("Index update requires an array, Vec, or dictionary".to_string()),
                }
            }
            _ => Err("++/-- target must be an assignable variable, field, or index".to_string()),
        }
    }

    /// Call a method on a struct instance.
    /// Struct fields become the method's "global scope".
    fn call_struct_method(
        &self,
        struct_name: &str,
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
        let method_parent = env
            .get_struct_context(&struct_name)
            .map(|context| (*context).clone())
            .unwrap_or_else(|| env.clone());
        let mut method_env = Environment::with_parent(method_parent);

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

    fn taggedConstructor(owner: &str, variant: &str, args: &[Value]) -> Result<Option<Value>, String> {
        let value = match (owner, variant) {
            ("Option", "Some") => {
                if args.len() != 1 {
                    return Err("Option::Some expects 1 argument".to_string());
                }
                Value::Option(Rc::new(RefCell::new(Some(args[0].clone()))))
            }
            ("Option", "None") => {
                if !args.is_empty() {
                    return Err("Option::None expects 0 arguments".to_string());
                }
                Value::Option(Rc::new(RefCell::new(None)))
            }
            ("Result", "Ok") => {
                if args.len() != 1 {
                    return Err("Result::Ok expects 1 argument".to_string());
                }
                Value::Result(Rc::new(RefCell::new(Ok(args[0].clone()))))
            }
            ("Result", "Err") => {
                if args.len() != 1 {
                    return Err("Result::Err expects 1 argument".to_string());
                }
                Value::Result(Rc::new(RefCell::new(Err(args[0].clone()))))
            }
            _ => return Ok(None),
        };
        Ok(Some(value))
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
            return Err(format!(
                "error[null]: cannot call method '{}' on null",
                name
            ));
        }
        if name == "toString" {
            if !args.is_empty() {
                return Err(format!(
                    "toString() expects 0 arguments, got {}",
                    args.len()
                ));
            }
            let bytes = receiver
                .stringBytes()
                .map(|bytes| bytes.to_vec())
                .unwrap_or_else(|| super::std::StdLib::formatValueBytes(&receiver));
            return Ok(Value::fromStringBytes(bytes));
        }
        if let Some(bytes) = receiver.stringBytes() {
            return self.evaluate_string_method(bytes, name, args);
        }
        match (receiver, name) {
            (Value::Namespace(d), method_name) => {
                let value = d
                    .borrow()
                    .get(method_name.as_bytes())
                    .cloned()
                    .ok_or_else(|| format!("Namespace function '{}' not found", method_name))?;
                match value {
                    Value::Function(func) => self.call_value_function(&func, args, env),
                    _ => Err(format!(
                        "Namespace member '{}' is not callable",
                        method_name
                    )),
                }
            }
            (Value::Dict(dict), "len") => {
                if !args.is_empty() {
                    return Err(format!("Dict.len() expects 0 arguments, got {}", args.len()));
                }
                Ok(Value::Int(dict.borrow().len() as i64))
            }
            (Value::String(s), "toInt") => {
                if !args.is_empty() {
                    return Err(format!("toInt() expects 0 arguments, got {}", args.len()));
                }
                parseAsciiDecimal(s.as_bytes())
                    .and_then(floatToInt)
                    .map(Value::Int)
            }
            (Value::String(s), "toFloat") => {
                if !args.is_empty() {
                    return Err(format!("toFloat() expects 0 arguments, got {}", args.len()));
                }
                parseAsciiDecimal(s.as_bytes()).map(Value::Float)
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
                floatToInt(n)
                    .map(Value::Int)
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
            (Value::Array(arr), "len") => {
                if !args.is_empty() {
                    return Err(format!(
                        "{}.len() expects 0 arguments, got {}",
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
            (Value::String(s), "len") => {
                if !args.is_empty() {
                    return Err(format!(
                        "{}.len() expects 0 arguments, got {}",
                        "String",
                        args.len()
                    ));
                }
                Ok(Value::Int(s.len() as i64))
            }
            // returns Unicode code point of first char — used for char arithmetic in std
            (Value::String(s), "hash") => {
                if !args.is_empty() {
                    return Err(format!("hash() expects 0 arguments, got {}", args.len()));
                }
                match s.chars().next() {
                    Some(c) => Ok(Value::Int(c as i64)),
                    None => Err("hash() called on empty string".to_string()),
                }
            }
            (Value::String(s), "toUpper") => {
                if !args.is_empty() {
                    return Err(format!(
                        "String.toUpper() expects 0 arguments, got {}",
                        args.len()
                    ));
                }
                Ok(Value::String(SharedStr::from(s.to_ascii_uppercase())))
            }
            (Value::String(s), "toLower") => {
                if !args.is_empty() {
                    return Err(format!(
                        "String.toLower() expects 0 arguments, got {}",
                        args.len()
                    ));
                }
                Ok(Value::String(SharedStr::from(s.to_ascii_lowercase())))
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
                    Value::String(needle) => Ok(Value::Bool(s.contains(needle.asStr()))),
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
                    s.split(delim.asStr()).map(|p| p.to_string()).collect::<Vec<_>>()
                };
                let arr = parts
                    .into_iter()
                    .map(|part| Value::String(SharedStr::from(part)))
                    .collect::<Vec<_>>();
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
                Ok(Value::String(SharedStr::from(s.trim())))
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
                    Value::String(prefix) => Ok(Value::Bool(s.starts_with(prefix.asStr()))),
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
                    Value::String(suffix) => Ok(Value::Bool(s.ends_with(suffix.asStr()))),
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
                    Value::String(v) => v.asStr(),
                    _ => return Err("String.replace() expects string 'from'".to_string()),
                };
                let to = match &args[1] {
                    Value::String(v) => v.asStr(),
                    _ => return Err("String.replace() expects string 'to'".to_string()),
                };
                Ok(Value::String(SharedStr::from(s.replace(from, to))))
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
                        Value::Int(n) => isize::try_from(*n)
                            .map_err(|_| "String.slice() index is out of range".to_string()),
                        Value::Float(n) => {
                            if n.fract() != 0.0 {
                                return Err("String.slice() index must be an integer".to_string());
                            }
                            isize::try_from(floatToInt(*n)?)
                                .map_err(|_| "String.slice() index is out of range".to_string())
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
                    return Ok(Value::utf8(""));
                }
                let out = chars[start..end].iter().collect::<String>();
                Ok(Value::String(SharedStr::from(out)))
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
                match s.find(needle.asStr()) {
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
                            floatToInt(*v)?
                    }
                    _ => return Err("String.repeat() expects int count".to_string()),
                };
                if n < 0 {
                    return Err("String.repeat() count must be non-negative".to_string());
                }
                Ok(Value::String(SharedStr::from(s.repeat(n as usize))))
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
                        floatToInt(*v)?
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
                out.push_str(s.asStr());
                Ok(Value::String(SharedStr::from(out)))
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
                        floatToInt(*v)?
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
                let mut out = s.to_string();
                out.extend(std::iter::repeat(pad_ch).take(missing));
                Ok(Value::String(SharedStr::from(out)))
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
            (Value::Vec(v) | Value::Array(v), "map") => {
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
            (Value::Vec(v) | Value::Array(v), "filter") => {
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
            (Value::Vec(v) | Value::Array(v), "reduce") => {
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
            (Value::Vec(v) | Value::Array(v), "reverse") => {
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
                    Value::String(_) | Value::ByteString(_) => SortKind::String,
                    Value::Bool(_) => SortKind::Bool,
                    _ => {
                        return Err("Vec.sort() only supports int, float, String, bool".to_string())
                    }
                };

                for item in v_mut.iter() {
                    let ok = match (&kind, item) {
                        (SortKind::Int, Value::Int(_)) => true,
                        (SortKind::Float, Value::Float(_)) => true,
                        (SortKind::String, value) if value.stringBytes().is_some() => true,
                        (SortKind::Bool, Value::Bool(_)) => true,
                        _ => false,
                    };
                    if !ok {
                        return Err(
                            "Vec.sort() requires all elements to be the same comparable type"
                                .to_string(),
                        );
                    }
                }

                match kind {
                    SortKind::Int => v_mut.sort_by(|a, b| match (a, b) {
                        (Value::Int(x), Value::Int(y)) => x.cmp(y),
                        _ => std::cmp::Ordering::Equal,
                    }),
                    SortKind::Float => v_mut.sort_by(|a, b| match (a, b) {
                        (Value::Float(x), Value::Float(y)) => {
                            x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        _ => std::cmp::Ordering::Equal,
                    }),
                    SortKind::String => v_mut.sort_by(|a, b| match (a, b) {
                        (x, y) => x
                            .stringBytes()
                            .unwrap_or_default()
                            .cmp(y.stringBytes().unwrap_or_default()),
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
                        Value::Int(n) => isize::try_from(*n)
                            .map_err(|_| "Vec.slice() index is out of range".to_string()),
                        Value::Float(n) => {
                            if n.fract() != 0.0 {
                                return Err("Vec.slice() index must be an integer".to_string());
                            }
                            isize::try_from(floatToInt(*n)?)
                                .map_err(|_| "Vec.slice() index is out of range".to_string())
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
            (Value::Vec(v) | Value::Array(v), "join") => {
                if args.len() != 1 {
                    return Err("Vec.join() expects 1 argument".to_string());
                }
                let sep = args[0]
                    .stringBytes()
                    .ok_or_else(|| "Vec.join() expects string separator".to_string())?;
                let mut out = Vec::new();
                for (index, item) in v.borrow().iter().enumerate() {
                    if index > 0 {
                        out.extend_from_slice(sep);
                    }
                    out.extend(super::std::StdLib::formatValueBytes(item));
                }
                Ok(Value::fromStringBytes(out))
            }
            // HashMap methods
            (Value::HashMap(m) | Value::Dict(m), "set") => {
                if args.len() != 2 {
                    return Err("HashMap.set() expects 2 arguments".to_string());
                }
                let key = args[0]
                    .stringBytes()
                    .ok_or_else(|| "HashMap.set() expects string key".to_string())?;
                m.borrow_mut().insert(key.to_vec(), args[1].clone());
                Ok(Value::Null)
            }
            (Value::HashMap(m) | Value::Dict(m), "get") => {
                if args.len() != 1 {
                    return Err("HashMap.get() expects 1 argument".to_string());
                }
                let key = args[0]
                    .stringBytes()
                    .ok_or_else(|| "HashMap.get() expects string key".to_string())?;
                match m.borrow().get(key) {
                    Some(val) => Ok(val.clone()),
                    None => Err("HashMap key not found".to_string()),
                }
            }
            (Value::HashMap(m) | Value::Dict(m), "delete") => {
                if args.len() != 1 {
                    return Err("HashMap.delete() expects 1 argument".to_string());
                }
                let key = args[0]
                    .stringBytes()
                    .ok_or_else(|| "HashMap.delete() expects string key".to_string())?;
                m.borrow_mut().remove(key);
                Ok(Value::Null)
            }
            (Value::HashMap(m) | Value::Dict(m), "has") => {
                if args.len() != 1 {
                    return Err("HashMap.has() expects 1 argument".to_string());
                }
                let key = args[0]
                    .stringBytes()
                    .ok_or_else(|| "HashMap.has() expects string key".to_string())?;
                Ok(Value::Bool(m.borrow().contains_key(key)))
            }
            (Value::HashMap(m), "len") => {
                if !args.is_empty() {
                    return Err("HashMap.len() expects 0 arguments".to_string());
                }
                Ok(Value::Int(m.borrow().len() as i64))
            }
            (Value::HashMap(m) | Value::Dict(m), "keys") => {
                if !args.is_empty() {
                    return Err("HashMap.keys() expects 0 arguments".to_string());
                }
                let keys = m
                    .borrow()
                    .keys()
                    .cloned()
                    .map(Value::fromStringBytes)
                    .collect::<Vec<_>>();
                Ok(Value::Vec(Rc::new(RefCell::new(keys))))
            }
            (Value::HashMap(m) | Value::Dict(m), "values") => {
                if !args.is_empty() {
                    return Err("HashMap.values() expects 0 arguments".to_string());
                }
                let values = m.borrow().values().cloned().collect::<Vec<_>>();
                Ok(Value::Vec(Rc::new(RefCell::new(values))))
            }
            (Value::HashMap(m) | Value::Dict(m), "entries") => {
                if !args.is_empty() {
                    return Err("HashMap.entries() expects 0 arguments".to_string());
                }
                let entries = m
                    .borrow()
                    .iter()
                    .map(|(k, v)| {
                        Value::Array(Rc::new(RefCell::new(vec![
                            Value::fromStringBytes(k.clone()),
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
                    Err(e) => Err(format!(
                        "Cannot unwrap Err: {}",
                        super::std::StdLib::formatValue(e)
                    )),
                }
            }
            (Value::Result(r), "unwrapErr") => {
                if !args.is_empty() {
                    return Err("Result.unwrapErr() expects 0 arguments".to_string());
                }
                match r.borrow().as_ref() {
                    Ok(value) => Err(format!(
                        "Cannot unwrapErr Ok: {}",
                        super::std::StdLib::formatValue(value)
                    )),
                    Err(error) => Ok(error.clone()),
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

    fn evaluate_string_method(
        &self,
        value: &[u8],
        name: &str,
        args: &[Value],
    ) -> Result<Value, String> {
        let no_args = || {
            if args.is_empty() {
                Ok(())
            } else {
                Err(format!("String.{}() expects 0 arguments, got {}", name, args.len()))
            }
        };
        let one_string_arg = || -> Result<&[u8], String> {
            if args.len() != 1 {
                return Err(format!("String.{}() expects 1 argument, got {}", name, args.len()));
            }
            args[0]
                .stringBytes()
                .ok_or_else(|| format!("String.{}() expects a string argument", name))
        };
        let to_index = |arg: &Value| -> Result<isize, String> {
            match arg {
                Value::Int(n) => isize::try_from(*n)
                    .map_err(|_| "String index is out of range".to_string()),
                Value::Float(n) if n.fract() == 0.0 => isize::try_from(floatToInt(*n)?)
                    .map_err(|_| "String index is out of range".to_string()),
                Value::Float(_) => Err("String index must be an integer".to_string()),
                _ => Err("String index must be an integer".to_string()),
            }
        };

        match name {
            "toInt" => {
                no_args()?;
                parseAsciiDecimal(value)
                    .and_then(floatToInt)
                    .map(Value::Int)
            }
            "toFloat" => {
                no_args()?;
                parseAsciiDecimal(value).map(Value::Float)
            }
            "len" => {
                no_args()?;
                Ok(Value::Int(value.len() as i64))
            }
            "hash" => {
                no_args()?;
                value
                    .first()
                    .map(|byte| Value::Int(*byte as i64))
                    .ok_or_else(|| "hash() called on empty string".to_string())
            }
            "toUpper" | "toLower" => {
                no_args()?;
                let mut bytes = value.to_vec();
                if name == "toUpper" {
                    bytes.make_ascii_uppercase();
                } else {
                    bytes.make_ascii_lowercase();
                }
                Ok(Value::fromStringBytes(bytes))
            }
            "contains" => {
                let needle = one_string_arg()?;
                Ok(Value::Bool(containsBytes(value, needle)))
            }
            "split" => {
                let delimiter = one_string_arg()?;
                let parts = splitBytes(value, delimiter)
                    .into_iter()
                    .map(Value::fromStringBytes)
                    .collect();
                Ok(Value::Array(Rc::new(RefCell::new(parts))))
            }
            "trim" => {
                no_args()?;
                let start = value
                    .iter()
                    .position(|byte| !byte.is_ascii_whitespace())
                    .unwrap_or(value.len());
                let end = value
                    .iter()
                    .rposition(|byte| !byte.is_ascii_whitespace())
                    .map(|i| i + 1)
                    .unwrap_or(start);
                Ok(Value::fromStringBytes(value[start..end].to_vec()))
            }
            "startsWith" => {
                let prefix = one_string_arg()?;
                Ok(Value::Bool(value.starts_with(prefix)))
            }
            "endsWith" => {
                let suffix = one_string_arg()?;
                Ok(Value::Bool(value.ends_with(suffix)))
            }
            "replace" => {
                if args.len() != 2 {
                    return Err(format!("String.replace() expects 2 arguments, got {}", args.len()));
                }
                let from = args[0]
                    .stringBytes()
                    .ok_or_else(|| "String.replace() expects string 'from'".to_string())?;
                let to = args[1]
                    .stringBytes()
                    .ok_or_else(|| "String.replace() expects string 'to'".to_string())?;
                Ok(Value::fromStringBytes(replaceBytes(value, from, to)))
            }
            "slice" => {
                if args.len() != 2 {
                    return Err(format!("String.slice() expects 2 arguments, got {}", args.len()));
                }
                let start = to_index(&args[0])?;
                let end = to_index(&args[1])?;
                if start < 0 || end < 0 {
                    return Err("String.slice() indices must be non-negative".to_string());
                }
                let len = value.len() as isize;
                let start = start.min(len) as usize;
                let end = end.min(len) as usize;
                if end < start {
                    return Ok(Value::fromStringBytes(Vec::new()));
                }
                Ok(Value::fromStringBytes(value[start..end].to_vec()))
            }
            "indexOf" => {
                let needle = one_string_arg()?;
                Ok(Value::Int(findBytes(value, needle).map_or(-1, |i| i as i64)))
            }
            "repeat" => {
                if args.len() != 1 {
                    return Err(format!("String.repeat() expects 1 argument, got {}", args.len()));
                }
                let count = match &args[0] {
                    Value::Int(n) => *n,
                    Value::Float(n) if n.fract() == 0.0 => floatToInt(*n)?,
                    _ => return Err("String.repeat() expects an integer count".to_string()),
                };
                if count < 0 {
                    return Err("String.repeat() count must be non-negative".to_string());
                }
                Ok(Value::fromStringBytes(value.repeat(count as usize)))
            }
            "padLeft" | "padRight" => {
                if args.len() != 2 {
                    return Err(format!("String.{}() expects 2 arguments, got {}", name, args.len()));
                }
                let width = match &args[0] {
                    Value::Int(n) => *n,
                    Value::Float(n) if n.fract() == 0.0 => floatToInt(*n)?,
                    _ => return Err(format!("String.{}() expects int width", name)),
                };
                if width < 0 {
                    return Err(format!("String.{}() width must be non-negative", name));
                }
                let pad = args[1]
                    .stringBytes()
                    .and_then(|bytes| bytes.first())
                    .ok_or_else(|| format!("String.{}() expects non-empty string pad", name))?;
                let missing = (width as usize).saturating_sub(value.len());
                let mut out = Vec::with_capacity(value.len().saturating_add(missing));
                if name == "padLeft" {
                    out.extend(std::iter::repeat(*pad).take(missing));
                }
                out.extend_from_slice(value);
                if name == "padRight" {
                    out.extend(std::iter::repeat(*pad).take(missing));
                }
                Ok(Value::fromStringBytes(out))
            }
            _ => Err(format!("Method call not supported on String: {}", name)),
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
            Environment::with_parent_rc(Rc::clone(closure))
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

            let def = param.default.as_ref().ok_or_else(|| {
                format!("Missing argument '{}' and no default provided", param.name)
            })?;
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

    fn assignStringConcat(
        &self,
        name: &str,
        parts: &[&Expr],
        env: &mut Environment,
    ) -> Result<Option<Value>, String> {
        let Some(initial) = env.get(name) else {
            return Ok(None);
        };
        if initial.stringBytes().is_none() {
            return Ok(None);
        }
        let mut pieces = Vec::with_capacity(parts.len());
        let mut utf8 = initial.isUtf8();
        for part in parts {
            let value = self.evaluate_expr(part, env)?;
            if value.stringBytes().is_none() {
                return Err("Invalid binary operation".to_string());
            }
            utf8 &= value.isUtf8();
            pieces.push(value);
        }
        let snapshot = initial.bufferPtr();
        if env.get(name).as_ref().and_then(Value::bufferPtr) == snapshot {
            env.assign(name, Value::Null)?;
        }
        let result = concatOwned(initial, &pieces, utf8);
        env.assign(name, result.clone())?;
        Ok(Some(result))
    }

    fn evaluate_binary(
        &self,
        left: Value,
        op: &TokenKind,
        right: Value,
        line_hint: usize,
    ) -> Result<Value, String> {
        use TokenKind::*;

        if matches!(left, Value::Null) || matches!(right, Value::Null) {
            if matches!(op, EqualEqual | NotEqual) {
                let bothNull = matches!(left, Value::Null) && matches!(right, Value::Null);
                return Ok(Value::Bool(if matches!(op, EqualEqual) { bothNull } else { !bothNull }));
            }
            return Err(self.null_dereference_error(line_hint));
        }

        if left.stringBytes().is_some() && right.stringBytes().is_some() {
            return match op {
                Plus => Ok(concatValues(left, &right)),
                EqualEqual => Ok(Value::Bool(left.stringBytes() == right.stringBytes())),
                NotEqual => Ok(Value::Bool(left.stringBytes() != right.stringBytes())),
                Less => Ok(Value::Bool(left.stringBytes() < right.stringBytes())),
                LessEqual => Ok(Value::Bool(left.stringBytes() <= right.stringBytes())),
                Greater => Ok(Value::Bool(left.stringBytes() > right.stringBytes())),
                GreaterEqual => Ok(Value::Bool(left.stringBytes() >= right.stringBytes())),
                _ => Err("Invalid binary operation".to_string()),
            };
        }

        match (left, op, right) {
            (Value::Int(l), Plus, Value::Int(r)) => l
                .checked_add(r)
                .map(Value::Int)
                .ok_or_else(|| "Integer overflow in addition".to_string()),
            (Value::Int(l), Minus, Value::Int(r)) => l
                .checked_sub(r)
                .map(Value::Int)
                .ok_or_else(|| "Integer overflow in subtraction".to_string()),
            (Value::Int(l), Star, Value::Int(r)) => l
                .checked_mul(r)
                .map(Value::Int)
                .ok_or_else(|| "Integer overflow in multiplication".to_string()),
            (Value::Int(l), Slash, Value::Int(r)) => {
                if r == 0 {
                    Err("Division by zero".to_string())
                } else if l == i64::MIN && r == -1 {
                    Err("Integer overflow in division".to_string())
                } else {
                    Ok(Value::Int(l / r))
                }
            }
            (Value::Int(l), Percent, Value::Int(r)) => {
                if r == 0 {
                    Err("Division by zero".to_string())
                } else if l == i64::MIN && r == -1 {
                    Err("Integer overflow in remainder".to_string())
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

            (Value::String(l), Plus, Value::String(r)) => Ok(Value::String(l.appendStr(r.asStr()))),
            (Value::String(l), EqualEqual, Value::String(r)) => Ok(Value::Bool(l == r)),
            (Value::String(l), NotEqual, Value::String(r)) => Ok(Value::Bool(l != r)),
            (Value::String(l), Less, Value::String(r)) => Ok(Value::Bool(l < r)),
            (Value::String(l), LessEqual, Value::String(r)) => Ok(Value::Bool(l <= r)),
            (Value::String(l), Greater, Value::String(r)) => Ok(Value::Bool(l > r)),
            (Value::String(l), GreaterEqual, Value::String(r)) => Ok(Value::Bool(l >= r)),
            (
                Value::EnumVariant { enum_name: leftEnum, variant: leftVariant },
                EqualEqual,
                Value::EnumVariant { enum_name: rightEnum, variant: rightVariant },
            ) => Ok(Value::Bool(leftEnum == rightEnum && leftVariant == rightVariant)),
            (
                Value::EnumVariant { enum_name: leftEnum, variant: leftVariant },
                NotEqual,
                Value::EnumVariant { enum_name: rightEnum, variant: rightVariant },
            ) => Ok(Value::Bool(leftEnum != rightEnum || leftVariant != rightVariant)),
            (
                Value::EnumVariantPayload { enum_name: leftEnum, variant: leftVariant, payloads: leftPayloads },
                EqualEqual,
                Value::EnumVariantPayload { enum_name: rightEnum, variant: rightVariant, payloads: rightPayloads },
            ) => Ok(Value::Bool(leftEnum == rightEnum && leftVariant == rightVariant && leftPayloads == rightPayloads)),
            (
                Value::EnumVariantPayload { enum_name: leftEnum, variant: leftVariant, payloads: leftPayloads },
                NotEqual,
                Value::EnumVariantPayload { enum_name: rightEnum, variant: rightVariant, payloads: rightPayloads },
            ) => Ok(Value::Bool(leftEnum != rightEnum || leftVariant != rightVariant || leftPayloads != rightPayloads)),
            (Value::Tuple(left), EqualEqual, Value::Tuple(right)) => {
                Ok(Value::Bool(left.borrow().as_slice() == right.borrow().as_slice()))
            }
            (Value::Tuple(left), NotEqual, Value::Tuple(right)) => {
                Ok(Value::Bool(left.borrow().as_slice() != right.borrow().as_slice()))
            }
            (Value::Array(a), EqualEqual, Value::Array(b)) => Ok(Value::Bool(Rc::ptr_eq(&a, &b))),
            (Value::Array(a), NotEqual, Value::Array(b)) => Ok(Value::Bool(!Rc::ptr_eq(&a, &b))),
            (Value::Dict(a), EqualEqual, Value::Dict(b)) => Ok(Value::Bool(Rc::ptr_eq(&a, &b))),
            (Value::Dict(a), NotEqual, Value::Dict(b)) => Ok(Value::Bool(!Rc::ptr_eq(&a, &b))),
            (Value::HashMap(a), EqualEqual, Value::HashMap(b)) => {
                Ok(Value::Bool(Rc::ptr_eq(&a, &b)))
            }
            (Value::HashMap(a), NotEqual, Value::HashMap(b)) => {
                Ok(Value::Bool(!Rc::ptr_eq(&a, &b)))
            }
            (Value::Namespace(a), EqualEqual, Value::Namespace(b)) => {
                Ok(Value::Bool(Rc::ptr_eq(&a, &b)))
            }
            (Value::Namespace(a), NotEqual, Value::Namespace(b)) => {
                Ok(Value::Bool(!Rc::ptr_eq(&a, &b)))
            }
            (
                Value::StructInstance { fields: a, .. },
                EqualEqual,
                Value::StructInstance { fields: b, .. },
            ) => Ok(Value::Bool(Rc::ptr_eq(&a, &b))),
            (
                Value::StructInstance { fields: a, .. },
                NotEqual,
                Value::StructInstance { fields: b, .. },
            ) => Ok(Value::Bool(!Rc::ptr_eq(&a, &b))),

            (Value::Option(left), EqualEqual, Value::Option(right)) => {
                Ok(Value::Bool(left.borrow().as_ref() == right.borrow().as_ref()))
            }
            (Value::Option(left), NotEqual, Value::Option(right)) => {
                Ok(Value::Bool(left.borrow().as_ref() != right.borrow().as_ref()))
            }
            (Value::Result(left), EqualEqual, Value::Result(right)) => {
                Ok(Value::Bool(*left.borrow() == *right.borrow()))
            }
            (Value::Result(left), NotEqual, Value::Result(right)) => {
                Ok(Value::Bool(*left.borrow() != *right.borrow()))
            }
            (Value::Bool(l), EqualEqual, Value::Bool(r)) => Ok(Value::Bool(l == r)),
            (Value::Bool(l), NotEqual, Value::Bool(r)) => Ok(Value::Bool(l != r)),
            (Value::Char(l), EqualEqual, Value::Char(r)) => Ok(Value::Bool(l == r)),
            (Value::Char(l), NotEqual, Value::Char(r)) => Ok(Value::Bool(l != r)),
            (Value::Char(l), Less, Value::Char(r)) => Ok(Value::Bool(l < r)),
            (Value::Char(l), LessEqual, Value::Char(r)) => Ok(Value::Bool(l <= r)),
            (Value::Char(l), Greater, Value::Char(r)) => Ok(Value::Bool(l > r)),
            (Value::Char(l), GreaterEqual, Value::Char(r)) => Ok(Value::Bool(l >= r)),
            (Value::Bool(l), AndAnd, Value::Bool(r)) => Ok(Value::Bool(l && r)),
            (Value::Bool(l), OrOr, Value::Bool(r)) => Ok(Value::Bool(l || r)),

            _ => Err("Invalid binary operation".to_string()),
        }
    }

    // collect parts of a left-recursive `base + a + b + c` chain rooted at `base_name`
    // returns the list [a, b, c] if the chain starts with Variable(base_name); empty otherwise
    fn collect_string_concat_parts<'a>(expr: &'a Expr, base_name: &str) -> Vec<&'a Expr> {
        match expr {
            Expr::Binary {
                left, op, right, ..
            } if *op == TokenKind::Plus => {
                let mut parts = Self::collect_string_concat_parts(left, base_name);
                if !parts.is_empty()
                    || matches!(left.as_ref(), Expr::Variable { name, .. } if name == base_name)
                {
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

    fn evaluate_unary(&self, op: &TokenKind, right: Value, line_hint: usize) -> Result<Value, String> {
        use TokenKind::*;

        if matches!(right, Value::Null) {
            return Err(self.null_dereference_error(line_hint));
        }

        match (op, right) {
            (Minus, Value::Int(n)) => n
                .checked_neg()
                .map(Value::Int)
                .ok_or_else(|| "Integer overflow in negation".to_string()),
            (Minus, Value::Float(n)) => Ok(Value::Float(-n)),
            (Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
            _ => Err("Invalid unary operation".to_string()),
        }
    }

    fn null_dereference_error(&self, line_hint: usize) -> String {
        format!("null dereference at line {}", line_hint + 1)
    }

    fn index_out_of_bounds_error(&self, i: usize, len: usize) -> String {
        format!("index out of bounds (i={}, len={})", i, len)
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

    fn execute_scoped_block_with_control_flow(
        &self,
        statements: &[Stmt],
        env: &mut Environment,
    ) -> Result<ControlFlow, String> {
        let mut scope = Environment::with_parent(env.clone());
        self.execute_block_with_control_flow(statements, &mut scope)
    }
}

/// Simple edit distance helper for "did you mean?" suggestions  
fn concatOwned(base: Value, parts: &[Value], utf8: bool) -> Value {
    if utf8 {
        let Value::String(text) = base else {
            return Value::Null;
        };
        let slices: Vec<&str> = parts
            .iter()
            .map(|part| std::str::from_utf8(part.stringBytes().unwrap()).unwrap())
            .collect();
        return Value::String(text.appendStrs(&slices));
    }
    let buf = match base {
        Value::String(text) => SharedBuf::new(text.asBytes().to_vec()),
        Value::ByteString(buf) => buf,
        _ => return Value::Null,
    };
    let slices: Vec<&[u8]> = parts
        .iter()
        .map(|part| part.stringBytes().unwrap())
        .collect();
    let out = buf.appendMany(&slices);
    bytesToValue(out)
}

fn concatValues(left: Value, right: &Value) -> Value {
    let utf8 = left.isUtf8() && right.isUtf8();
    concatOwned(left, std::slice::from_ref(right), utf8)
}

fn appendFormatted(left: Value, extra: &[u8]) -> Value {
    if left.isUtf8() && std::str::from_utf8(extra).is_ok() {
        let Value::String(text) = left else {
            return Value::fromStringBytes(extra.to_vec());
        };
        let extra = std::str::from_utf8(extra).unwrap();
        return Value::String(text.appendStr(extra));
    }
    let buf = match left {
        Value::String(text) => SharedBuf::new(text.asBytes().to_vec()),
        Value::ByteString(buf) => buf,
        _ => return Value::fromStringBytes(extra.to_vec()),
    };
    bytesToValue(buf.appendBytes(extra))
}

fn bytesToValue(buf: SharedBuf) -> Value {
    match std::str::from_utf8(buf.asBytes()) {
        Ok(text) => Value::String(SharedStr::from(text)),
        Err(_) => Value::ByteString(buf),
    }
}

fn findBytes(value: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    value
        .windows(needle.len())
        .position(|window| window == needle)
}

fn containsBytes(value: &[u8], needle: &[u8]) -> bool {
    findBytes(value, needle).is_some()
}

fn splitBytes(value: &[u8], delimiter: &[u8]) -> Vec<Vec<u8>> {
    if delimiter.is_empty() {
        return value.iter().map(|byte| vec![*byte]).collect();
    }
    let mut parts = Vec::new();
    let mut offset = 0;
    while let Some(index) = findBytes(&value[offset..], delimiter) {
        parts.push(value[offset..offset + index].to_vec());
        offset += index + delimiter.len();
    }
    parts.push(value[offset..].to_vec());
    parts
}

fn replaceBytes(value: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    if from.is_empty() {
        let mut out = Vec::with_capacity(value.len().saturating_add(
            to.len().saturating_mul(value.len().saturating_add(1)),
        ));
        out.extend_from_slice(to);
        for byte in value {
            out.push(*byte);
            out.extend_from_slice(to);
        }
        return out;
    }
    let mut out = Vec::new();
    let mut offset = 0;
    while let Some(index) = findBytes(&value[offset..], from) {
        out.extend_from_slice(&value[offset..offset + index]);
        out.extend_from_slice(to);
        offset += index + from.len();
    }
    out.extend_from_slice(&value[offset..]);
    out
}

fn parseAsciiDecimal(bytes: &[u8]) -> Result<f64, String> {
    if bytes.is_empty() {
        return Err("Invalid ASCII decimal text".to_string());
    }
    let mut index = 0;
    let mut normalized = Vec::with_capacity(bytes.len());
    if bytes.first().is_some_and(|byte| *byte == b'+' || *byte == b'-') {
        normalized.push(bytes[0]);
        index = 1;
    }
    let mut digit_count = 0;
    while index < bytes.len() {
        if bytes[index].is_ascii_digit() {
            normalized.push(bytes[index]);
            digit_count += 1;
            index += 1;
        } else if bytes[index] == b'_' {
            if index == 0
                || !bytes[index - 1].is_ascii_digit()
                || !bytes.get(index + 1).is_some_and(u8::is_ascii_digit)
            {
                break;
            }
            index += 1;
        } else {
            break;
        }
    }
    if digit_count == 0 {
        return Err("Invalid ASCII decimal text".to_string());
    }
    if index < bytes.len() && bytes[index] == b'.' {
        normalized.push(b'.');
        index += 1;
        while index < bytes.len() {
            if bytes[index].is_ascii_digit() {
                normalized.push(bytes[index]);
                index += 1;
            } else if bytes[index] == b'_' {
                if index == 0
                    || !bytes[index - 1].is_ascii_digit()
                    || !bytes.get(index + 1).is_some_and(u8::is_ascii_digit)
                {
                    break;
                }
                index += 1;
            } else {
                break;
            }
        }
    }
    if index != bytes.len() {
        return Err("Invalid ASCII decimal text".to_string());
    }
    let text = std::str::from_utf8(&normalized)
        .map_err(|_| "Invalid ASCII decimal text".to_string())?;
    let value = text
        .parse::<f64>()
        .map_err(|_| "Invalid ASCII decimal text".to_string())?;
    if !value.is_finite() {
        return Err("Decimal value out of float range".to_string());
    }
    Ok(value)
}

fn nonNegativeIndex(value: i64, kind: &str) -> Result<usize, String> {
    if value < 0 {
        return Err(format!("{} index must be non-negative", kind));
    }
    usize::try_from(value).map_err(|_| format!("{} index is out of range", kind))
}

fn nonNegativeFloatIndex(value: f64, kind: &str) -> Result<usize, String> {
    if value.fract() != 0.0 {
        return Err(format!("{} index must be an integer", kind));
    }
    nonNegativeIndex(floatToInt(value)?, kind)
}

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
