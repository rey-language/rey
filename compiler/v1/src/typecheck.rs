use crate::ast::{Expr, Literal, Parameter, Stmt, Type};
use crate::lexer::span::Span;
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
    Tuple(Vec<Ty>),
    Union(Vec<Ty>),
    Function {
        params: Vec<Ty>,
        minArgs: usize,
        variadic: Option<Box<Ty>>,
        ret: Box<Ty>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<TypeError> for String {
    fn from(err: TypeError) -> Self {
        err.message
    }
}

impl From<String> for TypeError {
    fn from(message: String) -> Self {
        TypeError {
            message,
            span: Span::new(0, 0),
        }
    }
}

impl Ty {
    fn fromAnnotation(ty: &Type) -> Result<Ty, String> {
        let name = ty.name.trim();
        if name.contains('|') {
            let mut items = Vec::new();
            for part in name.split('|') {
                let inner = Ty::fromName(part.trim())?;
                items.push(inner);
            }
            return Ok(Ty::Union(items));
        }
        if let Some(base) = name.strip_suffix('?') {
            let baseTy = Ty::fromAnnotation(&Type {
                name: base.trim().to_string(),
            })?;
            return Ok(Ty::Nullable(Box::new(baseTy)));
        }
        match name {
            "Any" => Ok(Ty::Any),
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
            // collection types are dynamically dispatched at runtime; type params are erased
            "Vec" | "HashMap" | "LinkedList" | "Stack" | "Queue" | "Option" | "Result" => {
                Ok(Ty::Any)
            }
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
                if name
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                {
                    return Ok(Ty::Any);
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
            (src, Ty::Union(items)) => items.iter().any(|t| src.isAssignableTo(t)),
            (Ty::Union(items), dst) => items.iter().all(|t| t.isAssignableTo(dst)),
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
    enum_variants: HashMap<String, (String, String)>, // variant_name -> (enum_name, variant_name)
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut c = Self::default();
        c.scopes.push(HashMap::new());
        c.enum_variants = HashMap::new();

        // builtins
        c.functions.insert(
            "println".to_string(),
            Ty::Function {
                minArgs: 0,
                params: vec![],
                variadic: None,
                ret: Box::new(Ty::Void),
            },
        );
        c.functions.insert(
            "print".to_string(),
            Ty::Function {
                minArgs: 0,
                params: vec![],
                variadic: None,
                ret: Box::new(Ty::Void),
            },
        );
        c.functions.insert(
            "assert".to_string(),
            Ty::Function {
                minArgs: 2,
                params: vec![Ty::Bool, Ty::Any],
                variadic: None,
                ret: Box::new(Ty::Void),
            },
        );
        c.functions.insert(
            "abs".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::Any],
                variadic: None,
                ret: Box::new(Ty::Any),
            },
        );
        c.functions.insert(
            "max".to_string(),
            Ty::Function {
                minArgs: 2,
                params: vec![Ty::Any, Ty::Any],
                variadic: None,
                ret: Box::new(Ty::Any),
            },
        );
        c.functions.insert(
            "min".to_string(),
            Ty::Function {
                minArgs: 2,
                params: vec![Ty::Any, Ty::Any],
                variadic: None,
                ret: Box::new(Ty::Any),
            },
        );
        c.functions.insert(
            "random".to_string(),
            Ty::Function {
                minArgs: 0,
                params: vec![],
                variadic: None,
                ret: Box::new(Ty::Any),
            },
        );
        c.functions.insert(
            "floor".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::Any],
                variadic: None,
                ret: Box::new(Ty::Int),
            },
        );
        c.functions.insert(
            "ceil".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::Any],
                variadic: None,
                ret: Box::new(Ty::Int),
            },
        );
        c.functions.insert(
            "round".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::Any],
                variadic: None,
                ret: Box::new(Ty::Int),
            },
        );
        c.functions.insert(
            "sqrt".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::Any],
                variadic: None,
                ret: Box::new(Ty::Float),
            },
        );
        c.functions.insert(
            "pow".to_string(),
            Ty::Function {
                minArgs: 2,
                params: vec![Ty::Any, Ty::Any],
                variadic: None,
                ret: Box::new(Ty::Float),
            },
        );
        c.functions.insert(
            "log".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::Any],
                variadic: None,
                ret: Box::new(Ty::Float),
            },
        );
        c.functions.insert(
            "sin".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::Any],
                variadic: None,
                ret: Box::new(Ty::Float),
            },
        );
        c.functions.insert(
            "cos".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::Any],
                variadic: None,
                ret: Box::new(Ty::Float),
            },
        );
        c.functions.insert(
            "tan".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::Any],
                variadic: None,
                ret: Box::new(Ty::Float),
            },
        );
        c.functions.insert(
            "len".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::Any],
                variadic: None,
                ret: Box::new(Ty::Int),
            },
        );
        c.functions.insert(
            "push".to_string(),
            Ty::Function {
                minArgs: 2,
                params: vec![Ty::Any, Ty::Any],
                variadic: None,
                ret: Box::new(Ty::Void),
            },
        );
        c.functions.insert(
            "pop".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::Any],
                variadic: None,
                ret: Box::new(Ty::Any),
            },
        );
        c.functions.insert(
            "input".to_string(),
            Ty::Function {
                minArgs: 0,
                params: vec![],
                variadic: None,
                ret: Box::new(Ty::String),
            },
        );

        // FS builtins - using Any for flexible return types
        c.functions.insert(
            "readFile".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::String],
                variadic: None,
                ret: Box::new(Ty::Any),
            },
        );
        c.functions.insert(
            "writeFile".to_string(),
            Ty::Function {
                minArgs: 2,
                params: vec![Ty::String, Ty::String],
                variadic: None,
                ret: Box::new(Ty::Any),
            },
        );
        c.functions.insert(
            "appendFile".to_string(),
            Ty::Function {
                minArgs: 2,
                params: vec![Ty::String, Ty::String],
                variadic: None,
                ret: Box::new(Ty::Any),
            },
        );
        c.functions.insert(
            "fileExists".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::String],
                variadic: None,
                ret: Box::new(Ty::Bool),
            },
        );
        c.functions.insert(
            "deleteFile".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::String],
                variadic: None,
                ret: Box::new(Ty::Any),
            },
        );
        c.functions.insert(
            "mkdir".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::String],
                variadic: None,
                ret: Box::new(Ty::Any),
            },
        );
        c.functions.insert(
            "listDir".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::String],
                variadic: None,
                ret: Box::new(Ty::Any),
            },
        );
        c.functions.insert(
            "getEnv".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::String],
                variadic: None,
                ret: Box::new(Ty::Any),
            },
        );
        c.functions.insert(
            "args".to_string(),
            Ty::Function {
                minArgs: 0,
                params: vec![],
                variadic: None,
                ret: Box::new(Ty::Any),
            },
        );
        c.functions.insert(
            "exit".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::Int],
                variadic: None,
                ret: Box::new(Ty::Void),
            },
        );
        c.functions.insert(
            "exec".to_string(),
            Ty::Function {
                minArgs: 1,
                params: vec![Ty::String],
                variadic: None,
                ret: Box::new(Ty::Any),
            },
        );

        c
    }

    pub fn checkProgram(&mut self, statements: &[Stmt]) -> Result<(), TypeError> {
        // First pass: register all functions and enums so calls can be checked.
        for stmt in statements {
            if let Stmt::FuncDecl {
                name,
                visibility: _,
                params,
                return_ty,
                ..
            } = stmt
            {
                self.registerFunction(name, params, return_ty)?;
            }
            if let Stmt::EnumDecl { name, variants } = stmt {
                self.define(name, Ty::Any, true);
                for variant in variants {
                    self.enum_variants
                        .insert(variant.clone(), (name.clone(), variant.clone()));
                    self.define(variant, Ty::Any, true);
                }
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
    ) -> Result<(), TypeError> {
        let mut ptys = Vec::new();
        let mut minArgs = 0usize;
        let mut variadic: Option<Box<Ty>> = None;
        for p in params {
            let pty = if let Some(ty) = &p.ty {
                Ty::fromAnnotation(ty)?
            } else {
                Ty::Any
            };

            if p.variadic {
                match pty {
                    Ty::Array(inner) => variadic = Some(inner),
                    _ => {
                        return Err(TypeError {
                            message: "Type error: variadic param must be an array type".to_string(),
                            span: Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                column: 0,
                            },
                        })
                    }
                }
                continue;
            }

            ptys.push(pty);
            if p.default.is_none() {
                minArgs += 1;
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
                minArgs,
                params: ptys,
                variadic,
                ret: Box::new(ret),
            },
        );
        Ok(())
    }

    fn checkStmt(&mut self, stmt: &Stmt) -> Result<(), TypeError> {
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
                        (Expr::ArrayLiteral { elements, .. }, Ty::Array(inner), Ty::Array(_))
                            if elements.is_empty() && matches!(inner.as_ref(), Ty::Any)
                    );
                    if !initTy.isAssignableTo(&annTy) && !allowEmptyTypedArray {
                        return Err(TypeError {
                            message: format!(
                                "Type error: variable '{}' expected {:?} but got {:?}",
                                name, annTy, initTy
                            ),
                            span: initializer.span(),
                        });
                    }
                    annTy
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
                visibility: _,
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
                    if let Some(def) = &p.default {
                        let dty = self.exprTy(def)?;
                        if !dty.isAssignableTo(&pty) {
                            return Err(TypeError {
                                message: format!(
                                    "Type error: default value for '{}' expected {:?} but got {:?}",
                                    p.name, pty, dty
                                ),
                                span: def.span(),
                            });
                        }
                    }
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
                    return Err(TypeError {
                        message: format!("Type error: if condition must be bool, got {:?}", cty),
                        span: condition.span(),
                    });
                }
                self.pushScope();
                if let Expr::InstanceOf { value, ty, .. } = condition {
                    if let Expr::Variable { name: varName, .. } = value.as_ref() {
                        if let Ok((curTy, is_const)) = self.lookup(varName) {
                            if let Ty::Union(items) = curTy {
                                if let Ok(target) = Ty::fromAnnotation(ty) {
                                    if items.iter().any(|t| t.isAssignableTo(&target)) {
                                        self.define(varName, target, is_const);
                                    }
                                }
                            }
                        }
                    }
                }
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
                    return Err(TypeError {
                        message: format!("Type error: while condition must be bool, got {:?}", cty),
                        span: condition.span(),
                    });
                }
                self.pushScope();
                for s in body {
                    self.checkStmt(s)?;
                }
                self.popScope();
                Ok(())
            }
            Stmt::Loop { body } => {
                self.pushScope();
                for s in body {
                    self.checkStmt(s)?;
                }
                self.popScope();
                Ok(())
            }
            Stmt::For {
                variable,
                iterator,
                body,
            } => {
                use crate::ast::stmt::ForIterator;
                match iterator {
                    ForIterator::Range { start, end } => {
                        let sty = self.exprTy(start)?;
                        let ety = self.exprTy(end)?;
                        if !sty.isAssignableTo(&Ty::Int) && !sty.isAssignableTo(&Ty::Float) {
                            return Err(TypeError {
                                message: format!(
                                    "Type error: range start must be numeric, got {:?}",
                                    sty
                                ),
                                span: start.span(),
                            });
                        }
                        if !ety.isAssignableTo(&Ty::Int) && !ety.isAssignableTo(&Ty::Float) {
                            return Err(TypeError {
                                message: format!(
                                    "Type error: range end must be numeric, got {:?}",
                                    ety
                                ),
                                span: end.span(),
                            });
                        }
                        self.pushScope();
                        self.define(variable, Ty::Int, false);
                        for s in body {
                            self.checkStmt(s)?;
                        }
                        self.popScope();
                    }
                    ForIterator::Array(expr) => {
                        let arr_ty = self.exprTy(expr)?;
                        let elem_ty = match arr_ty {
                            Ty::Array(inner) => *inner,
                            Ty::Any => Ty::Any,
                            _ => {
                                return Err(TypeError {
                                    message: "Type error: for-in requires an array".to_string(),
                                    span: expr.span(),
                                })
                            }
                        };
                        self.pushScope();
                        self.define(variable, elem_ty, false);
                        for s in body {
                            self.checkStmt(s)?;
                        }
                        self.popScope();
                    }
                }
                Ok(())
            }
            Stmt::Import { .. } => Ok(()),
            Stmt::Break | Stmt::Continue => Ok(()),
            Stmt::Return(expr) => match expr {
                Some(expr) => {
                    let rty = self.exprTy(expr)?;
                    if let Some(expected) = &self.currentReturn {
                        if !rty.isAssignableTo(expected) {
                            return Err(TypeError {
                                message: format!(
                                    "Type error: return expected {:?} but got {:?}",
                                    expected, rty
                                ),
                                span: expr.span(),
                            });
                        }
                    }
                    Ok(())
                }
                None => {
                    if let Some(expected) = &self.currentReturn {
                        if *expected != Ty::Void {
                            return Err(TypeError {
                                message: "Type error: return value required".to_string(),
                                span: Span {
                                    start: 0,
                                    end: 0,
                                    line: 0,
                                    column: 0,
                                },
                            });
                        }
                    }
                    Ok(())
                }
            },
            Stmt::StructDecl {
                name: _,
                fields: _,
                methods: _,
            } => {
                // Structs bypass type checking for now
                Ok(())
            }
            Stmt::EnumDecl { name, variants } => {
                self.define(name, Ty::Any, true);
                // Register each variant as a valid variable
                for variant in variants {
                    self.enum_variants
                        .insert(variant.clone(), (name.clone(), variant.clone()));
                    // Also define it in the current scope as a constant
                    self.define(variant, Ty::Any, true);
                }
                Ok(())
            }
            Stmt::Match { expr, arms } => {
                let expr_ty = self.exprTy(expr)?;
                // Typecheck each arm's pattern against the expression type
                for arm in arms {
                    use crate::ast::stmt::Pattern;
                    fn collectBindings(p: &Pattern, out: &mut Vec<String>) {
                        match p {
                            Pattern::Variable(name) => out.push(name.clone()),
                            Pattern::Struct { fields, .. } => {
                                for (_, fp) in fields {
                                    collectBindings(fp, out);
                                }
                            }
                            _ => {}
                        }
                    }
                    match &arm.pattern {
                        Pattern::Wildcard => {}
                        Pattern::Variable(_) => {}
                        Pattern::Literal(lit) => {
                            let lit_ty = match lit {
                                Literal::String(_) => Ty::String,
                                Literal::Char(_) => Ty::Char,
                                Literal::Int(_) => Ty::Int,
                                Literal::Float(_) => Ty::Float,
                                Literal::Bool(_) => Ty::Bool,
                                Literal::Null => Ty::Null,
                            };
                            if !lit_ty.isAssignableTo(&expr_ty) && expr_ty != Ty::Any {
                                return Err(TypeError {
                                    message: "Type error: pattern doesn't match expression type"
                                        .to_string(),
                                    span: expr.span(),
                                });
                            }
                        }
                        Pattern::EnumVariant(_, _) => {
                            // Enum patterns are always valid for now
                        }
                        Pattern::Struct { .. } => {
                            // Struct patterns are accepted for now (field checks happen at runtime).
                        }
                    }
                    // Typecheck the arm body
                    self.pushScope();
                    let mut bindings = Vec::new();
                    collectBindings(&arm.pattern, &mut bindings);
                    for name in bindings {
                        self.define(&name, Ty::Any, false);
                    }
                    for s in &arm.body {
                        self.checkStmt(s)?;
                    }
                    self.popScope();
                }
                Ok(())
            }
        }
    }

    fn exprTy(&mut self, expr: &Expr) -> Result<Ty, TypeError> {
        match expr {
            Expr::Literal { value: lit, .. } => Ok(match lit {
                Literal::String(_) => Ty::String,
                Literal::Char(_) => Ty::Char,
                Literal::Int(_) => Ty::Int,
                Literal::Float(_) => Ty::Float,
                Literal::Bool(_) => Ty::Bool,
                Literal::Null => Ty::Null,
            }),
            Expr::Variable { name, .. } => Ok(self.lookup(name)?.0),
            Expr::Assign { name, value, .. } => {
                let (cur, is_const) = self.lookup(name)?;
                if is_const {
                    return Err(TypeError {
                        message: format!(
                            "Type error: cannot assign to constant variable '{}'",
                            name
                        ),
                        span: value.span(),
                    });
                }
                let vty = self.exprTy(value)?;
                if !vty.isAssignableTo(&cur) {
                    return Err(TypeError {
                        message: format!(
                            "Type error: variable '{}' expected {:?} but got {:?}",
                            name, cur, vty
                        ),
                        span: value.span(),
                    });
                }
                Ok(cur)
            }
            Expr::Update { name, .. } => {
                let (cur, is_const) = self.lookup(name)?;
                if is_const {
                    return Err(TypeError {
                        message: format!(
                            "Type error: cannot assign to constant variable '{}'",
                            name
                        ),
                        span: expr.span(),
                    });
                }
                if !matches!(
                    cur,
                    Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Double | Ty::Any
                ) {
                    return Err(TypeError {
                        message: format!(
                            "Type error: ++/-- requires numeric variable, got {:?}",
                            cur
                        ),
                        span: expr.span(),
                    });
                }
                Ok(cur)
            }
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => {
                let l = self.exprTy(left)?;
                let r = self.exprTy(right)?;
                self.binaryTy(&l, op, &r, *span)
            }
            Expr::Unary { op, right, span } => {
                let r = self.exprTy(right)?;
                self.unaryTy(op, &r, *span)
            }
            Expr::InstanceOf { ty, .. } => {
                Ty::fromAnnotation(ty)?;
                Ok(Ty::Bool)
            }
            Expr::Lambda { params, body, .. } => {
                let mut ptys = Vec::new();
                for p in params {
                    let pty = if let Some(ty) = &p.ty {
                        Ty::fromAnnotation(ty)?
                    } else {
                        Ty::Any
                    };
                    ptys.push(pty);
                }
                self.pushScope();
                for (p, pty) in params.iter().zip(ptys.iter()) {
                    self.define(&p.name, pty.clone(), false);
                }
                let retTy = self.exprTy(body)?;
                self.popScope();
                Ok(Ty::Function {
                    minArgs: ptys.len(),
                    params: ptys,
                    variadic: None,
                    ret: Box::new(retTy),
                })
            }
            Expr::Call { callee, args, .. } => {
                if let Expr::Variable { name, .. } = callee.as_ref() {
                    if self.functions.contains_key(name) {
                        return self.checkCallByName(name, args, expr.span());
                    }
                }
                let cty = self.exprTy(callee)?;
                match cty {
                    Ty::Function {
                        params,
                        minArgs,
                        variadic,
                        ret,
                    } => {
                        let fixedCount = params.len();
                        let maxOk = variadic.is_some() || args.len() <= fixedCount;
                        if args.len() < minArgs || !maxOk {
                            return Err(TypeError {
                                message: format!(
                                    "Type error: expected {}..={} arguments but got {}",
                                    minArgs,
                                    if variadic.is_some() {
                                        usize::MAX
                                    } else {
                                        fixedCount
                                    },
                                    args.len()
                                ),
                                span: expr.span(),
                            });
                        }
                        for (p, a) in params.iter().zip(args.iter().take(fixedCount)) {
                            let aty = self.exprTy(a)?;
                            if !aty.isAssignableTo(p) {
                                return Err(TypeError {
                                    message: format!(
                                        "Type error: argument expected {:?} but got {:?}",
                                        p, aty
                                    ),
                                    span: a.span(),
                                });
                            }
                        }
                        if let Some(vty) = variadic.as_ref() {
                            for a in args.iter().skip(fixedCount) {
                                let aty = self.exprTy(a)?;
                                if !aty.isAssignableTo(vty.as_ref()) {
                                    return Err(TypeError {
                                        message: format!(
                                            "Type error: variadic argument expected {:?} but got {:?}",
                                            vty, aty
                                        ),
                                        span: a.span(),
                                    });
                                }
                            }
                        }
                        Ok(*ret)
                    }
                    other => Err(TypeError {
                        message: format!("Type error: can only call functions, got {:?}", other),
                        span: callee.span(),
                    }),
                }
            }
            Expr::ArrayLiteral { elements, .. } => {
                let mut inner = Ty::Any;
                for e in elements {
                    let ety = self.exprTy(e)?;
                    inner = self.join(&inner, &ety);
                }
                Ok(Ty::Array(Box::new(inner)))
            }
            Expr::TupleLiteral { elements, .. } => {
                let mut tys = Vec::new();
                for el in elements {
                    tys.push(self.exprTy(el)?);
                }
                Ok(Ty::Tuple(tys))
            }
            Expr::DictLiteral { entries, .. } => {
                let mut valueTy = Ty::Any;
                for (_k, v) in entries {
                    let vty = self.exprTy(v)?;
                    valueTy = self.join(&valueTy, &vty);
                }
                Ok(Ty::Dict(Box::new(Ty::String), Box::new(valueTy)))
            }
            Expr::Index { target, index, .. } => {
                let tty = self.exprTy(target)?;
                let ity = self.exprTy(index)?;
                match tty {
                    Ty::Array(inner) => {
                        if !ity.isAssignableTo(&Ty::Int) {
                            return Err(TypeError {
                                message: format!(
                                    "Type error: array index must be int, got {:?}",
                                    ity
                                ),
                                span: index.span(),
                            });
                        }
                        Ok(*inner)
                    }
                    Ty::Dict(k, v) => {
                        if !ity.isAssignableTo(&k) {
                            return Err(TypeError {
                                message: format!(
                                    "Type error: dict index must be {:?}, got {:?}",
                                    k, ity
                                ),
                                span: index.span(),
                            });
                        }
                        Ok(*v)
                    }
                    Ty::Any => Ok(Ty::Any),
                    Ty::String => {
                        if !ity.isAssignableTo(&Ty::Int) {
                            return Err(TypeError {
                                message: format!(
                                    "Type error: string index must be int, got {:?}",
                                    ity
                                ),
                                span: index.span(),
                            });
                        }
                        Ok(Ty::String)
                    }
                    _ => Err(TypeError {
                        message:
                            "Type error: indexing only supported for arrays, dictionaries, and strings".to_string(),
                        span: target.span(),
                    }),
                }
            }
            Expr::Get { object, name, .. } => {
                let tty = self.exprTy(object)?;
                match tty {
                    Ty::Dict(_k, v) => Ok(*v),
                    Ty::Tuple(items) => {
                        let idx: usize = name.parse().map_err(|_| TypeError {
                            message: "Type error: tuple access must be a numeric index".to_string(),
                            span: expr.span(),
                        })?;
                        items.get(idx).cloned().ok_or_else(|| TypeError {
                            message: format!("Type error: tuple index {} out of bounds", idx),
                            span: expr.span(),
                        })
                    }
                    Ty::Any => Ok(Ty::Any),
                    _ => Err(TypeError {
                        message:
                            "Type error: property access only supported for dictionaries and tuples"
                                .to_string(),
                        span: expr.span(),
                    }),
                }
            }
            Expr::MethodCall {
                receiver,
                name,
                args,
                span,
            } => {
                let rty = self.exprTy(receiver)?;
                self.methodTy(&rty, name, args, *span)
            }
            Expr::StructLiteral { .. } => Ok(Ty::Any),
            Expr::StaticCall { .. } => Ok(Ty::Any),
            Expr::Set { value, .. } => self.exprTy(value),
            Expr::IndexSet { value, .. } => self.exprTy(value),
        }
    }

    fn methodTy(
        &mut self,
        receiver: &Ty,
        name: &str,
        args: &[Expr],
        callSpan: Span,
    ) -> Result<Ty, TypeError> {
        match name {
            "toString" => {
                if !args.is_empty() {
                    return Err(TypeError {
                        message: format!("Type error: {}() expects 0 arguments", name),
                        span: callSpan,
                    });
                }
                return Ok(Ty::String);
            }
            "toInt" | "toFloat" => {
                if !args.is_empty() {
                    return Err(TypeError {
                        message: format!("Type error: {}() expects 0 arguments", name),
                        span: callSpan,
                    });
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
                    return Err(TypeError {
                        message: "Type error: Array.length() expects 0 arguments".to_string(),
                        span: callSpan,
                    });
                }
                Ok(Ty::Int)
            }
            (Ty::Array(inner), "push") => {
                if args.len() != 1 {
                    return Err(TypeError {
                        message: "Type error: Array.push() expects 1 argument".to_string(),
                        span: callSpan,
                    });
                }
                let a0 = self.exprTy(&args[0])?;
                if !a0.isAssignableTo(inner.as_ref()) {
                    return Err(TypeError {
                        message: format!(
                            "Type error: Array.push() expected element {:?} but got {:?}",
                            inner, a0
                        ),
                        span: args[0].span(),
                    });
                }
                Ok(Ty::Void)
            }
            (Ty::String, "length")
            | (Ty::String, "len")
            | (Ty::String, "upper")
            | (Ty::String, "lower")
            | (Ty::String, "trim") => {
                if !args.is_empty() {
                    return Err(TypeError {
                        message: format!("Type error: String.{}() expects 0 arguments", name),
                        span: callSpan,
                    });
                }
                Ok(match name {
                    "length" | "len" => Ty::Int,
                    _ => Ty::String,
                })
            }
            (Ty::String, "startsWith") | (Ty::String, "endsWith") => {
                if args.len() != 1 {
                    return Err(TypeError {
                        message: format!("Type error: String.{}() expects 1 argument", name),
                        span: callSpan,
                    });
                }
                let a0 = self.exprTy(&args[0])?;
                if !a0.isAssignableTo(&Ty::String) {
                    return Err(TypeError {
                        message: format!("Type error: String.{}() expects a string", name),
                        span: args[0].span(),
                    });
                }
                Ok(Ty::Bool)
            }
            (Ty::String, "replace") => {
                if args.len() != 2 {
                    return Err(TypeError {
                        message: "Type error: String.replace() expects 2 arguments".to_string(),
                        span: callSpan,
                    });
                }
                let a0 = self.exprTy(&args[0])?;
                let a1 = self.exprTy(&args[1])?;
                if !a0.isAssignableTo(&Ty::String) || !a1.isAssignableTo(&Ty::String) {
                    return Err(TypeError {
                        message: "Type error: String.replace() expects string arguments".to_string(),
                        span: callSpan,
                    });
                }
                Ok(Ty::String)
            }
            (Ty::String, "slice") => {
                if args.len() != 2 {
                    return Err(TypeError {
                        message: "Type error: String.slice() expects 2 arguments".to_string(),
                        span: callSpan,
                    });
                }
                let a0 = self.exprTy(&args[0])?;
                let a1 = self.exprTy(&args[1])?;
                if !a0.isAssignableTo(&Ty::Int) || !a1.isAssignableTo(&Ty::Int) {
                    return Err(TypeError {
                        message: "Type error: String.slice() expects int indices".to_string(),
                        span: callSpan,
                    });
                }
                Ok(Ty::String)
            }
            (Ty::String, "indexOf") => {
                if args.len() != 1 {
                    return Err(TypeError {
                        message: "Type error: String.indexOf() expects 1 argument".to_string(),
                        span: callSpan,
                    });
                }
                let a0 = self.exprTy(&args[0])?;
                if !a0.isAssignableTo(&Ty::String) {
                    return Err(TypeError {
                        message: "Type error: String.indexOf() expects a string".to_string(),
                        span: args[0].span(),
                    });
                }
                Ok(Ty::Int)
            }
            (Ty::String, "repeat") => {
                if args.len() != 1 {
                    return Err(TypeError {
                        message: "Type error: String.repeat() expects 1 argument".to_string(),
                        span: callSpan,
                    });
                }
                let a0 = self.exprTy(&args[0])?;
                if !a0.isAssignableTo(&Ty::Int) {
                    return Err(TypeError {
                        message: "Type error: String.repeat() expects an int".to_string(),
                        span: args[0].span(),
                    });
                }
                Ok(Ty::String)
            }
            (Ty::String, "padLeft") | (Ty::String, "padRight") => {
                if args.len() != 2 {
                    return Err(TypeError {
                        message: format!("Type error: String.{}() expects 2 arguments", name),
                        span: callSpan,
                    });
                }
                let a0 = self.exprTy(&args[0])?;
                let a1 = self.exprTy(&args[1])?;
                if !a0.isAssignableTo(&Ty::Int) || !a1.isAssignableTo(&Ty::String) {
                    return Err(TypeError {
                        message: format!("Type error: String.{}() expects (int, String)", name),
                        span: callSpan,
                    });
                }
                Ok(Ty::String)
            }
            (Ty::String, "contains") => {
                if args.len() != 1 {
                    return Err(TypeError {
                        message: "Type error: String.contains() expects 1 argument".to_string(),
                        span: callSpan,
                    });
                }
                let a0 = self.exprTy(&args[0])?;
                if !a0.isAssignableTo(&Ty::String) {
                    return Err(TypeError {
                        message: "Type error: String.contains() expects a string".to_string(),
                        span: args[0].span(),
                    });
                }
                Ok(Ty::Bool)
            }
            (Ty::String, "split") => {
                if args.len() != 1 {
                    return Err(TypeError {
                        message: "Type error: String.split() expects 1 argument".to_string(),
                        span: callSpan,
                    });
                }
                let a0 = self.exprTy(&args[0])?;
                if !a0.isAssignableTo(&Ty::String) {
                    return Err(TypeError {
                        message: "Type error: String.split() expects a string delimiter"
                            .to_string(),
                        span: args[0].span(),
                    });
                }
                Ok(Ty::Array(Box::new(Ty::String)))
            }
            (Ty::Dict(_, _), _) => Ok(Ty::Any),
            _ => Err(TypeError {
                message: format!(
                    "Type error: method '{}' not supported on {:?}",
                    name, receiver
                ),
                span: callSpan,
            }),
        }
    }

    fn checkCallByName(
        &mut self,
        name: &str,
        args: &[Expr],
        callSpan: Span,
    ) -> Result<Ty, TypeError> {
        if name == "println" || name == "print" {
            for a in args {
                self.exprTy(a)?;
            }
            return Ok(Ty::Void);
        }

        if name == "len" {
            if args.len() != 1 {
                return Err(TypeError {
                    message: format!("Type error: len expects 1 argument, got {}", args.len()),
                    span: callSpan,
                });
            }
            self.exprTy(&args[0])?;
            return Ok(Ty::Int);
        }

        if name == "input" {
            if args.len() > 1 {
                return Err(TypeError {
                    message: format!(
                        "Type error: input expects 0 or 1 arguments, got {}",
                        args.len()
                    ),
                    span: callSpan,
                });
            }
            if args.len() == 1 {
                let t = self.exprTy(&args[0])?;
                if !t.isAssignableTo(&Ty::String) {
                    return Err(TypeError {
                        message: "Type error: input prompt must be string".to_string(),
                        span: args[0].span(),
                    });
                }
            }
            return Ok(Ty::String);
        }

        if name == "push" {
            if args.len() != 2 {
                return Err(TypeError {
                    message: format!("Type error: push expects 2 arguments, got {}", args.len()),
                    span: callSpan,
                });
            }
            let arrTy = self.exprTy(&args[0])?;
            let elTy = self.exprTy(&args[1])?;
            return match arrTy {
                Ty::Array(inner) => {
                    if !elTy.isAssignableTo(&inner) {
                        return Err(TypeError {
                            message: format!(
                                "Type error: push expected element {:?} but got {:?}",
                                inner, elTy
                            ),
                            span: args[1].span(),
                        });
                    }
                    Ok(Ty::Void)
                }
                Ty::Any => Ok(Ty::Void),
                _ => Err(TypeError {
                    message: "Type error: push expects an array".to_string(),
                    span: args[0].span(),
                }),
            };
        }

        if name == "pop" {
            if args.len() != 1 {
                return Err(TypeError {
                    message: format!("Type error: pop expects 1 argument, got {}", args.len()),
                    span: callSpan,
                });
            }
            let arrTy = self.exprTy(&args[0])?;
            return match arrTy {
                Ty::Array(inner) => Ok(*inner),
                Ty::Any => Ok(Ty::Any),
                _ => Err(TypeError {
                    message: "Type error: pop expects an array".to_string(),
                    span: args[0].span(),
                }),
            };
        }

        match self.functions.get(name).cloned() {
            Some(Ty::Function {
                params,
                minArgs,
                variadic,
                ret,
            }) => {
                let fixedCount = params.len();
                let maxOk = variadic.is_some() || args.len() <= fixedCount;
                if args.len() < minArgs || !maxOk {
                    return Err(TypeError {
                        message: format!(
                            "Type error: {} expects {}..={} arguments but got {}",
                            name,
                            minArgs,
                            if variadic.is_some() {
                                usize::MAX
                            } else {
                                fixedCount
                            },
                            args.len()
                        ),
                        span: callSpan,
                    });
                }
                for (p, a) in params.iter().zip(args.iter().take(fixedCount)) {
                    let aty = self.exprTy(a)?;
                    if !aty.isAssignableTo(p) {
                        return Err(TypeError {
                            message: format!(
                                "Type error: {} argument expected {:?} but got {:?}",
                                name, p, aty
                            ),
                            span: a.span(),
                        });
                    }
                }
                if let Some(vty) = variadic.as_ref() {
                    for a in args.iter().skip(fixedCount) {
                        let aty = self.exprTy(a)?;
                        if !aty.isAssignableTo(vty.as_ref()) {
                            return Err(TypeError {
                                message: format!(
                                    "Type error: {} variadic argument expected {:?} but got {:?}",
                                    name, vty, aty
                                ),
                                span: a.span(),
                            });
                        }
                    }
                }
                Ok(*ret)
            }
            _ => Err(TypeError {
                message: format!("Type error: unknown function '{}'", name),
                span: callSpan,
            }),
        }
    }

    fn binaryTy(&self, left: &Ty, op: &TokenKind, right: &Ty, span: Span) -> Result<Ty, TypeError> {
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
                _ => Err(TypeError {
                    message: "Type error: invalid '+' operands".to_string(),
                    span,
                }),
            },
            Minus | Star | Percent => {
                if isNumeric(left) && isNumeric(right) {
                    Ok(numericResult(left, right))
                } else {
                    Err(TypeError {
                        message: "Type error: invalid numeric operands".to_string(),
                        span,
                    })
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
                    Err(TypeError {
                        message: "Type error: invalid '/' operands".to_string(),
                        span,
                    })
                }
            }
            EqualEqual | NotEqual | Less | LessEqual | Greater | GreaterEqual => Ok(Ty::Bool),
            AndAnd | OrOr => Ok(Ty::Bool),
            _ => Ok(Ty::Any),
        }
    }

    fn unaryTy(&self, op: &TokenKind, right: &Ty, span: Span) -> Result<Ty, TypeError> {
        if matches!(right, Ty::Any) {
            if op == &TokenKind::Not {
                return Ok(Ty::Bool);
            }
            return Ok(Ty::Any);
        }
        match op {
            TokenKind::Minus => match right {
                Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Double => Ok(right.clone()),
                _ => Err(TypeError {
                    message: "Type error: unary '-' expects numeric".to_string(),
                    span,
                }),
            },
            TokenKind::Not => Ok(Ty::Bool),
            _ => Ok(Ty::Any),
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

        if let Ty::Union(items) = a {
            if items.iter().any(|t| t == b) {
                return a.clone();
            }
        }
        if let Ty::Union(items) = b {
            if items.iter().any(|t| t == a) {
                return b.clone();
            }
        }

        if matches!((a, b), (Ty::Union(_), _) | (_, Ty::Union(_))) {
            let mut out = Vec::new();
            let pushUnique = |t: Ty, out: &mut Vec<Ty>| {
                if !out.iter().any(|x| x == &t) {
                    out.push(t);
                }
            };
            match a {
                Ty::Union(items) => {
                    for t in items {
                        pushUnique(t.clone(), &mut out);
                    }
                }
                other => pushUnique(other.clone(), &mut out),
            }
            match b {
                Ty::Union(items) => {
                    for t in items {
                        pushUnique(t.clone(), &mut out);
                    }
                }
                other => pushUnique(other.clone(), &mut out),
            }
            return Ty::Union(out);
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
