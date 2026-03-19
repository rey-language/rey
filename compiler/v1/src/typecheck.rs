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
    Char,
    Int,
    UInt,
    Float,
    Double,
    Byte,
    Nullable(Box<Ty>),
    Array(Box<Ty>),
    Dict(Box<Ty>, Box<Ty>),
    Function { params: Vec<Ty>, ret: Box<Ty> },
}

impl Ty {
    fn fromAnnotation(ty: &Type) -> Result<Ty, String> {
        let name = ty.name.trim();
        if let Some(base) = name.strip_suffix('?') {
            let baseTy = Ty::fromAnnotation(&Type {
                name: base.trim().to_string(),
            })?;
            return Ok(Ty::Nullable(Box::new(baseTy)));
        }
        match name {
            "Void" => Ok(Ty::Void),
            "null" => Ok(Ty::Null),
            "bool" => Ok(Ty::Bool),
            "String" => Ok(Ty::String),
            "char" => Ok(Ty::Char),
            "int" => Ok(Ty::Int),
            "uint" => Ok(Ty::UInt),
            "float" => Ok(Ty::Float),
            "double" => Ok(Ty::Double),
            "byte" => Ok(Ty::Byte),
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
        let ty = Type {
            name: name.to_string(),
        };
        Ty::fromAnnotation(&ty)
    }

    fn isAssignableTo(&self, target: &Ty) -> bool {
        fn isIntLike(t: &Ty) -> bool {
            matches!(t, Ty::Int | Ty::UInt | Ty::Byte)
        }
        fn isFloatLike(t: &Ty) -> bool {
            matches!(t, Ty::Float | Ty::Double)
        }

        match (self, target) {
            (_, Ty::Any) => true,
            (Ty::Any, _) => true,
            (Ty::Null, Ty::Null) => true,
            (Ty::Null, Ty::Nullable(_)) => true,
            (Ty::Null, _) => false,
            (Ty::Nullable(inner), Ty::Nullable(targetInner)) => inner.isAssignableTo(targetInner),
            (inner, Ty::Nullable(targetInner)) => inner.isAssignableTo(targetInner),
            (Ty::Nullable(_), _) => false,
            (a, b) if isIntLike(a) && isIntLike(b) => true,
            (a, b) if isFloatLike(a) && isFloatLike(b) => true,
            (a, b) if isIntLike(a) && isFloatLike(b) => true,
            (a, b) => a == b,
        }
    }
}

#[derive(Default)]
pub struct TypeChecker {
    scopes: Vec<HashMap<String, (Ty, bool)>>,
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
            "print".to_string(),
            Ty::Function {
                params: vec![],
                ret: Box::new(Ty::Void),
            },
        );
        c.functions.insert(
            "abs".to_string(),
            Ty::Function {
                params: vec![Ty::Any],
                ret: Box::new(Ty::Any),
            },
        );
        c.functions.insert(
            "max".to_string(),
            Ty::Function {
                params: vec![Ty::Any, Ty::Any],
                ret: Box::new(Ty::Any),
            },
        );
        c.functions.insert(
            "min".to_string(),
            Ty::Function {
                params: vec![Ty::Any, Ty::Any],
                ret: Box::new(Ty::Any),
            },
        );
        c.functions.insert(
            "random".to_string(),
            Ty::Function {
                params: vec![],
                ret: Box::new(Ty::Any),
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
                is_const,
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
                } else if !*is_const {
                    Ty::Any
                } else {
                    initTy
                };
                self.define(name, finalTy, *is_const);
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
                    self.define(&p.name, pty, false);
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
                    return Err(format!(
                        "Type error: if condition must be bool, got {:?}",
                        cty
                    ));
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
                    return Err(format!(
                        "Type error: while condition must be bool, got {:?}",
                        cty
                    ));
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
                    return Err(format!(
                        "Type error: range start must be numeric, got {:?}",
                        sty
                    ));
                }
                if !ety.isAssignableTo(&Ty::Int) && !ety.isAssignableTo(&Ty::Float) {
                    return Err(format!(
                        "Type error: range end must be numeric, got {:?}",
                        ety
                    ));
                }
                self.pushScope();
                self.define(variable, Ty::Int, false);
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
            Expr::Variable(name) => self.lookup(name).map(|(t, _)| t),
            Expr::Assign { name, value } => {
                let vty = self.exprTy(value)?;
                let (cur, is_const) = self.lookup(name)?;
                if is_const {
                    return Err(format!(
                        "Type error: cannot assign to constant variable '{}'",
                        name
                    ));
                }
                if cur != Ty::Any && !vty.isAssignableTo(&cur) {
                    return Err(format!(
                        "Type error: assignment to '{}' expected {:?} but got {:?}",
                        name, cur, vty
                    ));
                }
                Ok(cur)
            }
            Expr::Update {
                name,
                op: _,
                prefix: _,
            } => {
                let (cur, is_const) = self.lookup(name)?;
                if is_const {
                    return Err(format!(
                        "Type error: cannot assign to constant variable '{}'",
                        name
                    ));
                }
                if !matches!(
                    cur,
                    Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Double | Ty::Any
                ) {
                    return Err(format!(
                        "Type error: ++/-- requires numeric variable, got {:?}",
                        cur
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
                            return Err(format!(
                                "Type error: array index must be int, got {:?}",
                                ity
                            ));
                        }
                        Ok(*inner)
                    }
                    Ty::Dict(k, v) => {
                        if !ity.isAssignableTo(&k) {
                            return Err(format!(
                                "Type error: dict index must be {:?}, got {:?}",
                                k, ity
                            ));
                        }
                        Ok(*v)
                    }
                    Ty::Any => Ok(Ty::Any),
                    _ => Err(
                        "Type error: indexing only supported for arrays and dictionaries"
                            .to_string(),
                    ),
                }
            }
            Expr::Get { object, name: _ } => {
                let tty = self.exprTy(object)?;
                match tty {
                    Ty::Dict(_k, v) => Ok(*v),
                    Ty::Any => Ok(Ty::Any),
                    _ => Err(
                        "Type error: property access only supported for dictionaries".to_string(),
                    ),
                }
            }
            Expr::MethodCall {
                receiver,
                name,
                args,
            } => {
                let rty = self.exprTy(receiver)?;
                self.methodTy(&rty, name, args)
            }
        }
    }

    fn methodTy(&mut self, receiver: &Ty, name: &str, args: &[Expr]) -> Result<Ty, String> {
        match name {
            "toString" => {
                if !args.is_empty() {
                    return Err("Type error: toString() expects 0 arguments".to_string());
                }
                return Ok(Ty::String);
            }
            "toInt" | "toFloat" => {
                if !args.is_empty() {
                    return Err(format!("Type error: {}() expects 0 arguments", name));
                }
                return Ok(if name == "toInt" { Ty::Int } else { Ty::Float });
            }
            _ => {}
        }

        if matches!(receiver, Ty::Any) {
            return Ok(Ty::Any);
        }

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
            _ => Err(format!(
                "Type error: method '{}' not supported on {:?}",
                name, receiver
            )),
        }
    }

    fn checkCallByName(&mut self, name: &str, args: &[Expr]) -> Result<Ty, String> {
        if name == "println" || name == "print" {
            for a in args {
                self.exprTy(a)?;
            }
            return Ok(Ty::Void);
        }

        if name == "len" {
            if args.len() != 1 {
                return Err(format!(
                    "Type error: len expects 1 argument, got {}",
                    args.len()
                ));
            }
            self.exprTy(&args[0])?;
            return Ok(Ty::Int);
        }

        if name == "input" {
            if args.len() > 1 {
                return Err(format!(
                    "Type error: input expects 0 or 1 arguments, got {}",
                    args.len()
                ));
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
                return Err(format!(
                    "Type error: push expects 2 arguments, got {}",
                    args.len()
                ));
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
                Ty::Any => Ok(Ty::Void),
                _ => Err("Type error: push expects an array".to_string()),
            };
        }

        if name == "pop" {
            if args.len() != 1 {
                return Err(format!(
                    "Type error: pop expects 1 argument, got {}",
                    args.len()
                ));
            }
            let arrTy = self.exprTy(&args[0])?;
            return match arrTy {
                Ty::Array(inner) => Ok(*inner),
                Ty::Any => Ok(Ty::Any),
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

        fn isIntLike(t: &Ty) -> bool {
            matches!(t, Ty::Int | Ty::UInt | Ty::Byte)
        }
        fn isFloatLike(t: &Ty) -> bool {
            matches!(t, Ty::Float | Ty::Double)
        }
        fn isNumeric(t: &Ty) -> bool {
            isIntLike(t) || isFloatLike(t)
        }
        fn numericResult(a: &Ty, b: &Ty) -> Ty {
            if matches!((a, b), (Ty::Double, _) | (_, Ty::Double)) {
                return Ty::Double;
            }
            if isFloatLike(a) || isFloatLike(b) {
                return Ty::Float;
            }
            if a == &Ty::UInt && b == &Ty::UInt {
                return Ty::UInt;
            }
            if a == &Ty::Byte && b == &Ty::Byte {
                return Ty::Byte;
            }
            Ty::Int
        }

        if matches!(left, Ty::Any) || matches!(right, Ty::Any) {
            if op == &TokenKind::EqualEqual
                || op == &TokenKind::NotEqual
                || op == &TokenKind::Less
                || op == &TokenKind::LessEqual
                || op == &TokenKind::Greater
                || op == &TokenKind::GreaterEqual
                || op == &TokenKind::AndAnd
                || op == &TokenKind::OrOr
            {
                return Ok(Ty::Bool);
            }
            if op == &TokenKind::Plus && (matches!(left, Ty::String) || matches!(right, Ty::String))
            {
                return Ok(Ty::String);
            }
            return Ok(Ty::Any);
        }

        match op {
            Plus => match (left, right) {
                (Ty::String, _) | (_, Ty::String) => Ok(Ty::String),
                (l, r) if isNumeric(l) && isNumeric(r) => Ok(numericResult(l, r)),
                _ => Err("Type error: invalid '+' operands".to_string()),
            },
            Minus | Star | Percent => {
                if isNumeric(left) && isNumeric(right) {
                    Ok(numericResult(left, right))
                } else {
                    Err("Type error: invalid numeric operands".to_string())
                }
            }
            Slash => {
                if isNumeric(left) && isNumeric(right) {
                    Ok(
                        if matches!((left, right), (Ty::Double, _) | (_, Ty::Double)) {
                            Ty::Double
                        } else {
                            Ty::Float
                        },
                    )
                } else {
                    Err("Type error: invalid '/' operands".to_string())
                }
            }
            EqualEqual | NotEqual | Less | LessEqual | Greater | GreaterEqual => Ok(Ty::Bool),
            AndAnd | OrOr => Ok(Ty::Bool),
            _ => Ok(Ty::Any),
        }
    }

    fn unaryTy(&self, op: &TokenKind, right: &Ty) -> Result<Ty, String> {
        if matches!(right, Ty::Any) {
            if op == &TokenKind::Not {
                return Ok(Ty::Bool);
            }
            return Ok(Ty::Any);
        }
        match op {
            TokenKind::Minus => match right {
                Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Double => Ok(right.clone()),
                _ => Err("Type error: unary '-' expects numeric".to_string()),
            },
            TokenKind::Not => Ok(Ty::Bool),
            _ => Ok(Ty::Any),
        }
    }

    fn literalTy(&self, lit: &Literal) -> Ty {
        match lit {
            Literal::String(_) => Ty::String,
            Literal::Char(_) => Ty::Char,
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
            (Ty::Null, other) => {
                return match other {
                    Ty::Nullable(_) => other.clone(),
                    _ => Ty::Nullable(Box::new(other.clone())),
                };
            }
            (other, Ty::Null) => {
                return match other {
                    Ty::Nullable(_) => other.clone(),
                    _ => Ty::Nullable(Box::new(other.clone())),
                };
            }
            (Ty::Nullable(inner), other) | (other, Ty::Nullable(inner)) => {
                let joined = self.join(inner.as_ref(), other);
                return Ty::Nullable(Box::new(joined));
            }
            _ => {}
        }
        match (a, b) {
            (Ty::Int, Ty::Float) | (Ty::Float, Ty::Int) => Ty::Float,
            (Ty::Int, Ty::Double) | (Ty::Double, Ty::Int) => Ty::Double,
            (Ty::UInt, Ty::Float) | (Ty::Float, Ty::UInt) => Ty::Float,
            (Ty::UInt, Ty::Double) | (Ty::Double, Ty::UInt) => Ty::Double,
            (Ty::Byte, Ty::Float) | (Ty::Float, Ty::Byte) => Ty::Float,
            (Ty::Byte, Ty::Double) | (Ty::Double, Ty::Byte) => Ty::Double,
            (Ty::Float, Ty::Double) | (Ty::Double, Ty::Float) => Ty::Double,
            (Ty::Int, Ty::UInt) | (Ty::UInt, Ty::Int) => Ty::Int,
            (Ty::Int, Ty::Byte) | (Ty::Byte, Ty::Int) => Ty::Int,
            (Ty::UInt, Ty::Byte) | (Ty::Byte, Ty::UInt) => Ty::UInt,
            _ => Ty::Any,
        }
    }

    fn define(&mut self, name: &str, ty: Ty, is_const: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), (ty, is_const));
        }
    }

    fn lookup(&self, name: &str) -> Result<(Ty, bool), String> {
        for scope in self.scopes.iter().rev() {
            if let Some((t, c)) = scope.get(name) {
                return Ok((t.clone(), *c));
            }
        }
        if let Some(t) = self.functions.get(name) {
            return Ok((t.clone(), true));
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
