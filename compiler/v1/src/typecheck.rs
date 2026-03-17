use crate::ast::{Expr, Literal, Parameter, Stmt, Type};
use crate::lexer::TokenKind;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
enum Ty {
    Any,
    Void,
    Null,
    Bool,
    String,
    Int,
    Float,
    Array(Box<Ty>),
    Dict(Box<Ty>, Box<Ty>),
    Function { params: Vec<Ty>, ret: Box<Ty> },
}

impl Ty {
    fn fromAnnotation(ty: &Type) -> Result<Ty, String> {
        let name = ty.name.trim();
        match name {
            "Void" => Ok(Ty::Void),
            "null" => Ok(Ty::Null),
            "bool" => Ok(Ty::Bool),
            "String" => Ok(Ty::String),
            "int" => Ok(Ty::Int),
            "float" => Ok(Ty::Float),
            _ => {
                if let Some(inner) = name.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                    let inner = Ty::fromName(inner.trim())?;
                    return Ok(Ty::Array(Box::new(inner)));
                }
                if let Some(inner) = name.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                    let (k, v) = inner
                        .split_once(':')
                        .ok_or_else(|| "Invalid dict type annotation".to_string())?;
                    let key = Ty::fromName(k.trim())?;
                    let value = Ty::fromName(v.trim())?;
                    return Ok(Ty::Dict(Box::new(key), Box::new(value)));
                }
                Err(format!("Unknown type annotation '{}'", name))
            }
        }
    }

    fn fromName(name: &str) -> Result<Ty, String> {
        let ty = Type { name: name.to_string() };
        Ty::fromAnnotation(&ty)
    }

    fn isAssignableTo(&self, target: &Ty) -> bool {
        match (self, target) {
            (_, Ty::Any) => true,
            (Ty::Any, _) => true,
            (Ty::Null, Ty::Null) => true,
            (Ty::Null, _) => false,
            (Ty::Int, Ty::Float) => true,
            (a, b) => a == b,
        }
    }
}

#[derive(Default)]
pub struct TypeChecker {
    scopes: Vec<HashMap<String, Ty>>,
    functions: HashMap<String, Ty>,
    currentReturn: Option<Ty>,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut c = Self::default();
        c.scopes.push(HashMap::new());

        // builtins
        c.functions.insert(
            "println".to_string(),
            Ty::Function {
                params: vec![],
                ret: Box::new(Ty::Void),
            },
        );
        c.functions.insert(
            "len".to_string(),
            Ty::Function {
                params: vec![Ty::Any],
                ret: Box::new(Ty::Int),
            },
        );
        c.functions.insert(
            "push".to_string(),
            Ty::Function {
                params: vec![Ty::Any, Ty::Any],
                ret: Box::new(Ty::Void),
            },
        );
        c.functions.insert(
            "pop".to_string(),
            Ty::Function {
                params: vec![Ty::Any],
                ret: Box::new(Ty::Any),
            },
        );
        c.functions.insert(
            "input".to_string(),
            Ty::Function {
                params: vec![],
                ret: Box::new(Ty::String),
            },
        );

        c
    }

    pub fn checkProgram(&mut self, statements: &[Stmt]) -> Result<(), String> {
        // First pass: register all functions so calls can be checked.
        for stmt in statements {
            if let Stmt::FuncDecl {
                name,
                params,
                return_ty,
                ..
            } = stmt
            {
                self.registerFunction(name, params, return_ty)?;
            }
        }

        for stmt in statements {
            self.checkStmt(stmt)?;
        }
        Ok(())
    }

    fn registerFunction(
        &mut self,
        name: &str,
        params: &[Parameter],
        return_ty: &Option<Type>,
    ) -> Result<(), String> {
        let mut ptys = Vec::new();
        for p in params {
            if let Some(ty) = &p.ty {
                ptys.push(Ty::fromAnnotation(ty)?);
            } else {
                ptys.push(Ty::Any);
            }
        }
        let ret = if let Some(r) = return_ty {
            Ty::fromAnnotation(r)?
        } else {
            Ty::Any
        };
        self.functions.insert(
            name.to_string(),
            Ty::Function {
                params: ptys,
                ret: Box::new(ret),
            },
        );
        Ok(())
    }

    fn checkStmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::VarDecl {
                name,
                ty,
                initializer,
            } => {
                let initTy = self.exprTy(initializer)?;
                let finalTy = if let Some(ann) = ty {
                    let annTy = Ty::fromAnnotation(ann)?;
                    let allowEmptyTypedArray = matches!(
                        (initializer, &initTy, &annTy),
                        (Expr::ArrayLiteral { elements }, Ty::Array(inner), Ty::Array(_))
                            if elements.is_empty() && matches!(inner.as_ref(), Ty::Any)
                    );
                    if !initTy.isAssignableTo(&annTy) && !allowEmptyTypedArray {
                        return Err(format!(
                            "Type error: variable '{}' expected {:?} but got {:?}",
                            name, annTy, initTy
                        ));
                    }
                    annTy
                } else {
                    initTy
                };
                self.define(name, finalTy);
                Ok(())
            }
            Stmt::ExprStmt(expr) => {
                self.exprTy(expr)?;
                Ok(())
            }
            Stmt::FuncDecl {
                name: _,
                params,
                return_ty,
                body,
            } => {
                self.pushScope();
                for p in params {
                    let pty = if let Some(ty) = &p.ty {
                        Ty::fromAnnotation(ty)?
                    } else {
                        Ty::Any
                    };
                    self.define(&p.name, pty);
                }
                let prevReturn = self.currentReturn.clone();
                self.currentReturn = if let Some(ty) = return_ty {
                    Some(Ty::fromAnnotation(ty)?)
                } else {
                    None
                };
                for s in body {
                    self.checkStmt(s)?;
                }
                self.currentReturn = prevReturn;
                self.popScope();
                Ok(())
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cty = self.exprTy(condition)?;
                if !cty.isAssignableTo(&Ty::Bool) {
                    return Err(format!("Type error: if condition must be bool, got {:?}", cty));
                }
                self.pushScope();
                for s in then_branch {
                    self.checkStmt(s)?;
                }
                self.popScope();
                if let Some(else_branch) = else_branch {
                    self.pushScope();
                    for s in else_branch {
                        self.checkStmt(s)?;
                    }
                    self.popScope();
                }
                Ok(())
            }
            Stmt::While { condition, body } => {
                let cty = self.exprTy(condition)?;
                if !cty.isAssignableTo(&Ty::Bool) {
                    return Err(format!("Type error: while condition must be bool, got {:?}", cty));
                }
                self.pushScope();
                for s in body {
                    self.checkStmt(s)?;
                }
                self.popScope();
                Ok(())
            }
            Stmt::For {
                variable,
                start,
                end,
                body,
            } => {
                let sty = self.exprTy(start)?;
                let ety = self.exprTy(end)?;
                if !sty.isAssignableTo(&Ty::Int) && !sty.isAssignableTo(&Ty::Float) {
                    return Err(format!("Type error: range start must be numeric, got {:?}", sty));
                }
                if !ety.isAssignableTo(&Ty::Int) && !ety.isAssignableTo(&Ty::Float) {
                    return Err(format!("Type error: range end must be numeric, got {:?}", ety));
                }
                self.pushScope();
                self.define(variable, Ty::Int);
                for s in body {
                    self.checkStmt(s)?;
                }
                self.popScope();
                Ok(())
            }
            Stmt::Break | Stmt::Continue => Ok(()),
            Stmt::Return(expr) => {
                let rty = self.exprTy(expr)?;
                if let Some(expected) = &self.currentReturn {
                    if !rty.isAssignableTo(expected) {
                        return Err(format!(
                            "Type error: return expected {:?} but got {:?}",
                            expected, rty
                        ));
                    }
                }
                Ok(())
            }
        }
    }

    fn exprTy(&mut self, expr: &Expr) -> Result<Ty, String> {
        match expr {
            Expr::Literal(lit) => Ok(self.literalTy(lit)),
            Expr::Variable(name) => self.lookup(name),
            Expr::Assign { name, value } => {
                let vty = self.exprTy(value)?;
                let cur = self.lookup(name)?;
                if !vty.isAssignableTo(&cur) {
                    return Err(format!(
                        "Type error: assignment to '{}' expected {:?} but got {:?}",
                        name, cur, vty
                    ));
                }
                Ok(cur)
            }
            Expr::Binary { left, op, right } => {
                let l = self.exprTy(left)?;
                let r = self.exprTy(right)?;
                self.binaryTy(&l, op, &r)
            }
            Expr::Unary { op, right } => {
                let r = self.exprTy(right)?;
                self.unaryTy(op, &r)
            }
            Expr::Call { callee, args } => {
                if let Expr::Variable(name) = callee.as_ref() {
                    return self.checkCallByName(name, args);
                }
                let cty = self.exprTy(callee)?;
                match cty {
                    Ty::Function { params, ret } => {
                        if params.len() != args.len() {
                            return Err(format!(
                                "Type error: expected {} arguments but got {}",
                                params.len(),
                                args.len()
                            ));
                        }
                        for (p, a) in params.iter().zip(args.iter()) {
                            let aty = self.exprTy(a)?;
                            if !aty.isAssignableTo(p) {
                                return Err(format!(
                                    "Type error: argument expected {:?} but got {:?}",
                                    p, aty
                                ));
                            }
                        }
                        Ok(*ret)
                    }
                    _ => Err("Type error: can only call functions".to_string()),
                }
            }
            Expr::ArrayLiteral { elements } => {
                let mut inner = Ty::Any;
                for e in elements {
                    let ety = self.exprTy(e)?;
                    inner = self.join(&inner, &ety);
                }
                Ok(Ty::Array(Box::new(inner)))
            }
            Expr::DictLiteral { entries } => {
                let mut valueTy = Ty::Any;
                for (_k, v) in entries {
                    let vty = self.exprTy(v)?;
                    valueTy = self.join(&valueTy, &vty);
                }
                Ok(Ty::Dict(Box::new(Ty::String), Box::new(valueTy)))
            }
            Expr::Index { target, index } => {
                let tty = self.exprTy(target)?;
                let ity = self.exprTy(index)?;
                match tty {
                    Ty::Array(inner) => {
                        if !ity.isAssignableTo(&Ty::Int) {
                            return Err(format!("Type error: array index must be int, got {:?}", ity));
                        }
                        Ok(*inner)
                    }
                    Ty::Dict(k, v) => {
                        if !ity.isAssignableTo(&k) {
                            return Err(format!("Type error: dict index must be {:?}, got {:?}", k, ity));
                        }
                        Ok(*v)
                    }
                    _ => Err("Type error: indexing only supported for arrays and dictionaries".to_string()),
                }
            }
            Expr::Get { object, name: _ } => {
                let tty = self.exprTy(object)?;
                match tty {
                    Ty::Dict(_k, v) => Ok(*v),
                    _ => Err("Type error: property access only supported for dictionaries".to_string()),
                }
            }
            Expr::MethodCall { receiver, name, args } => {
                let rty = self.exprTy(receiver)?;
                self.methodTy(&rty, name, args)
            }
        }
    }

    fn methodTy(&mut self, receiver: &Ty, name: &str, args: &[Expr]) -> Result<Ty, String> {
        match (receiver, name) {
            (Ty::Array(_), "length") => {
                if !args.is_empty() {
                    return Err("Type error: Array.length() expects 0 arguments".to_string());
                }
                Ok(Ty::Int)
            }
            (Ty::Array(inner), "push") => {
                if args.len() != 1 {
                    return Err("Type error: Array.push() expects 1 argument".to_string());
                }
                let a0 = self.exprTy(&args[0])?;
                if !a0.isAssignableTo(inner.as_ref()) {
                    return Err(format!(
                        "Type error: Array.push() expected element {:?} but got {:?}",
                        inner, a0
                    ));
                }
                Ok(Ty::Void)
            }
            (Ty::String, "length") | (Ty::String, "upper") | (Ty::String, "lower") => {
                if !args.is_empty() {
                    return Err(format!("Type error: String.{}() expects 0 arguments", name));
                }
                Ok(match name {
                    "length" => Ty::Int,
                    _ => Ty::String,
                })
            }
            (Ty::String, "contains") => {
                if args.len() != 1 {
                    return Err("Type error: String.contains() expects 1 argument".to_string());
                }
                let a0 = self.exprTy(&args[0])?;
                if !a0.isAssignableTo(&Ty::String) {
                    return Err("Type error: String.contains() expects a string".to_string());
                }
                Ok(Ty::Bool)
            }
            (Ty::String, "split") => {
                if args.len() != 1 {
                    return Err("Type error: String.split() expects 1 argument".to_string());
                }
                let a0 = self.exprTy(&args[0])?;
                if !a0.isAssignableTo(&Ty::String) {
                    return Err("Type error: String.split() expects a string delimiter".to_string());
                }
                Ok(Ty::Array(Box::new(Ty::String)))
            }
            _ => Err(format!("Type error: method '{}' not supported on {:?}", name, receiver)),
        }
    }

    fn checkCallByName(&mut self, name: &str, args: &[Expr]) -> Result<Ty, String> {
        if name == "println" {
            for a in args {
                self.exprTy(a)?;
            }
            return Ok(Ty::Void);
        }

        if name == "len" {
            if args.len() != 1 {
                return Err(format!("Type error: len expects 1 argument, got {}", args.len()));
            }
            self.exprTy(&args[0])?;
            return Ok(Ty::Int);
        }

        if name == "input" {
            if args.len() > 1 {
                return Err(format!("Type error: input expects 0 or 1 arguments, got {}", args.len()));
            }
            if args.len() == 1 {
                let t = self.exprTy(&args[0])?;
                if !t.isAssignableTo(&Ty::String) {
                    return Err("Type error: input prompt must be string".to_string());
                }
            }
            return Ok(Ty::String);
        }

        if name == "push" {
            if args.len() != 2 {
                return Err(format!("Type error: push expects 2 arguments, got {}", args.len()));
            }
            let arrTy = self.exprTy(&args[0])?;
            let elTy = self.exprTy(&args[1])?;
            return match arrTy {
                Ty::Array(inner) => {
                    if !elTy.isAssignableTo(&inner) {
                        return Err(format!(
                            "Type error: push expected element {:?} but got {:?}",
                            inner, elTy
                        ));
                    }
                    Ok(Ty::Void)
                }
                _ => Err("Type error: push expects an array".to_string()),
            };
        }

        if name == "pop" {
            if args.len() != 1 {
                return Err(format!("Type error: pop expects 1 argument, got {}", args.len()));
            }
            let arrTy = self.exprTy(&args[0])?;
            return match arrTy {
                Ty::Array(inner) => Ok(*inner),
                _ => Err("Type error: pop expects an array".to_string()),
            };
        }

        match self.functions.get(name).cloned() {
            Some(Ty::Function { params, ret }) => {
                if params.len() != args.len() {
                    return Err(format!(
                        "Type error: {} expects {} arguments but got {}",
                        name,
                        params.len(),
                        args.len()
                    ));
                }
                for (p, a) in params.iter().zip(args.iter()) {
                    let aty = self.exprTy(a)?;
                    if !aty.isAssignableTo(p) {
                        return Err(format!(
                            "Type error: {} argument expected {:?} but got {:?}",
                            name, p, aty
                        ));
                    }
                }
                Ok(*ret)
            }
            _ => Err(format!("Type error: unknown function '{}'", name)),
        }
    }

    fn binaryTy(&self, left: &Ty, op: &TokenKind, right: &Ty) -> Result<Ty, String> {
        use TokenKind::*;
        match op {
            Plus => match (left, right) {
                (Ty::String, Ty::String) => Ok(Ty::String),
                (Ty::Int, Ty::Int) => Ok(Ty::Int),
                (Ty::Float, Ty::Float) => Ok(Ty::Float),
                (Ty::Int, Ty::Float) | (Ty::Float, Ty::Int) => Ok(Ty::Float),
                _ => Err("Type error: invalid '+' operands".to_string()),
            },
            Minus | Star => match (left, right) {
                (Ty::Int, Ty::Int) => Ok(Ty::Int),
                (Ty::Float, Ty::Float) => Ok(Ty::Float),
                (Ty::Int, Ty::Float) | (Ty::Float, Ty::Int) => Ok(Ty::Float),
                _ => Err("Type error: invalid numeric operands".to_string()),
            },
            Slash => match (left, right) {
                (Ty::Int, Ty::Int)
                | (Ty::Float, Ty::Float)
                | (Ty::Int, Ty::Float)
                | (Ty::Float, Ty::Int) => Ok(Ty::Float),
                _ => Err("Type error: invalid '/' operands".to_string()),
            },
            EqualEqual | NotEqual | Less | LessEqual | Greater | GreaterEqual => Ok(Ty::Bool),
            AndAnd | OrOr => Ok(Ty::Bool),
            _ => Ok(Ty::Any),
        }
    }

    fn unaryTy(&self, op: &TokenKind, right: &Ty) -> Result<Ty, String> {
        match op {
            TokenKind::Minus => match right {
                Ty::Int | Ty::Float => Ok(right.clone()),
                _ => Err("Type error: unary '-' expects numeric".to_string()),
            },
            TokenKind::Not => Ok(Ty::Bool),
            _ => Ok(Ty::Any),
        }
    }

    fn literalTy(&self, lit: &Literal) -> Ty {
        match lit {
            Literal::String(_) => Ty::String,
            Literal::Bool(_) => Ty::Bool,
            Literal::Null => Ty::Null,
            Literal::Number(n) => {
                if n.fract() == 0.0 {
                    Ty::Int
                } else {
                    Ty::Float
                }
            }
        }
    }

    fn join(&self, a: &Ty, b: &Ty) -> Ty {
        if a == &Ty::Any {
            return b.clone();
        }
        if b == &Ty::Any {
            return a.clone();
        }
        if a == b {
            return a.clone();
        }
        match (a, b) {
            (Ty::Int, Ty::Float) | (Ty::Float, Ty::Int) => Ty::Float,
            _ => Ty::Any,
        }
    }

    fn define(&mut self, name: &str, ty: Ty) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ty);
        }
    }

    fn lookup(&self, name: &str) -> Result<Ty, String> {
        for scope in self.scopes.iter().rev() {
            if let Some(t) = scope.get(name) {
                return Ok(t.clone());
            }
        }
        if let Some(t) = self.functions.get(name) {
            return Ok(t.clone());
        }
        Err(format!("Type error: undefined variable '{}'", name))
    }

    fn pushScope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn popScope(&mut self) {
        self.scopes.pop();
    }
}
