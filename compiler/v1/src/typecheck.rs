use crate::ast::{Expr, FunctionVisibility, Literal, Parameter, Stmt, Type};
use crate::imports::{ResolvedProgram, ResolvedSymbol, StatementProvenance};
use crate::lexer::span::Span;
use crate::lexer::TokenKind;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// ─── Phase 1: locked type representation ────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Ty {
    // primitives
    Int,
    Float,
    Bool,
    Char,
    // Ty::String shadows std::string::String in value positions — use
    // std::string::String explicitly where needed
    String,
    Void,

    // built-in applied types retain their static arguments; runtime values erase them
    Vec(Box<Ty>),
    HashMap(Box<Ty>),           // key is always String per spec
    Tuple(Vec<Ty>),
    Option(Box<Ty>),
    Result(Box<Ty>, Box<Ty>),

    // nominal user-defined types (Phase 6 fills their definitions)
    Struct(std::string::String),
    Enum(std::string::String),
    Function(Box<FuncSig>),
    Namespace(HashMap<std::string::String, FuncSig>),

    // type constructors
    Nullable(Box<Ty>),          // T?  ≡  T | null
    Union(Vec<Ty>),             // deduped, flattened

    // escape hatch — disables checking when used
    Any,

    // internal sentinel for the null literal; never surfaces as annotation
    Null,
}

// ─── Function signature (replaces old Ty::Function) ─────────────────────────

#[derive(Debug, Clone, PartialEq)]
struct FuncSig {
    params: Vec<Ty>,
    minArgs: usize,
    variadic: Option<Box<Ty>>,
    ret: Box<Ty>,
    inferReturn: bool,
    returnResolved: bool,
    // Preserved now; enforcing private access needs caller-module ownership from imports.rs.
    visibility: Option<FunctionVisibility>,
}

// ─── Placeholder defs for Phases 6+ ─────────────────────────────────────────

#[derive(Debug, Clone, Default)]
struct StructDef {
    fields: HashMap<std::string::String, FieldDef>,
    methods: HashMap<std::string::String, FuncSig>,
    staticMethods: HashMap<std::string::String, FuncSig>,
}

#[derive(Debug, Clone)]
struct FieldDef {
    ty: Ty,
    isPub: bool,
}

#[derive(Debug, Clone, Default)]
struct EnumDef {
    variants: Vec<std::string::String>,
    payloads: HashMap<std::string::String, Vec<Ty>>,
}

// ─── Errors ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TypeError {
    pub message: std::string::String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocatedTypeError {
    pub file: PathBuf,
    pub source: std::string::String,
    pub error: TypeError,
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<TypeError> for std::string::String {
    fn from(err: TypeError) -> Self {
        err.message
    }
}

impl From<std::string::String> for TypeError {
    fn from(message: std::string::String) -> Self {
        TypeError { message, span: Span::new(0, 0) }
    }
}

// ─── Ty impl ────────────────────────────────────────────────────────────────

impl Ty {
    // Parse the canonical type spelling produced by the parser.
    fn fromAnnotation(ty: &Type) -> Result<Ty, std::string::String> {
        Self::parseStr(ty.name.trim())
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn fromName(name: &str) -> Result<Ty, std::string::String> {
        Self::parseStr(name.trim())
    }

    fn parseStr(s: &str) -> Result<Ty, std::string::String> {
        if s.starts_with('(') {
            if let Some(close) = matchingParen(s) {
                let rest = s[close + 1..].trim_start();
                if let Some(returnName) = rest.strip_prefix("->") {
                    let paramsText = &s[1..close];
                    let mut params = Vec::new();
                    if !paramsText.trim().is_empty() {
                        for param in splitTopLevel(paramsText, ',') {
                            params.push(Self::parseStr(param.trim())?);
                        }
                    }
                    let ret = Self::parseStr(returnName.trim())?;
                    return Ok(Ty::Function(Box::new(FuncSig {
                        minArgs: params.len(),
                        params,
                        variadic: None,
                        ret: Box::new(ret),
                        inferReturn: false,
                        returnResolved: true,
                        visibility: None,
                    })));
                }
                if close == s.len() - 1 {
                    let tupleText = &s[1..close];
                    let mut parts = splitTopLevel(tupleText, ',');
                    if parts.last().is_some_and(|part| part.trim().is_empty())
                        && tupleText.trim_end().ends_with(',')
                    {
                        parts.pop();
                    }
                    if parts.is_empty() || parts.iter().any(|part| part.trim().is_empty())
                        || (parts.len() == 1 && !tupleText.trim_end().ends_with(','))
                    {
                        return Err(format!("TypeError: invalid tuple type '{}'", s));
                    }
                    return Ok(Ty::Tuple(
                        parts.into_iter().map(|part| Self::parseStr(part.trim())).collect::<Result<Vec<_>, _>>()?,
                    ));
                }
            }
        }

        // Union separators inside function, vector, and dictionary types are nested.
        let unionParts = splitTopLevel(s, '|');
        if unionParts.len() > 1 {
            let mut items = Vec::new();
            for part in unionParts {
                let inner = Self::parseStr(part.trim())?;
                items.push(inner);
            }
            return Ok(Self::flattenUnion(items));
        }

        // Nullable: trailing '?'
        if let Some(base) = s.strip_suffix('?') {
            let inner = Self::parseStr(base.trim())?;
            return Ok(Ty::Nullable(Box::new(inner)));
        }

        // [T] → Vec<T>
        if let Some(inner) = s.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
            let innerTy = Self::parseStr(inner.trim())?;
            return Ok(Ty::Vec(Box::new(innerTy)));
        }

        // {K:V} → HashMap<V>  (K must be String)
        if let Some(inner) = s.strip_prefix('{').and_then(|t| t.strip_suffix('}')) {
            let (k, v) = inner.split_once(':')
                .ok_or_else(|| "Invalid dict type annotation".to_string())?;
            let keyName = k.trim();
            if keyName != "string" {
                if keyName == "String" {
                    return Err("TypeError: use lowercase 'string' instead of 'String'".to_string());
                }
                return Err(format!(
                    "TypeError: HashMap key must be string, got '{}'",
                    keyName
                ));
            }
            let valTy = Self::parseStr(v.trim())?;
            return Ok(Ty::HashMap(Box::new(valTy)));
        }

        if let Some(open) = topLevelGenericStart(s) {
            let close = matchingGenericClose(s, open)
                .ok_or_else(|| format!("TypeError: malformed type application '{}'", s))?;
            if close != s.len() - 1 {
                return Err(format!("TypeError: malformed type application '{}'", s));
            }
            let name = s[..open].trim();
            let argsText = s[open + 1..close].trim();
            let args = if argsText.is_empty() {
                Vec::new()
            } else {
                splitTopLevel(argsText, ',')
            };
            if args.is_empty() || args.iter().any(|arg| arg.trim().is_empty()) {
                return Err(format!("TypeError: '{}' requires type arguments", name));
            }
            let parsed = args.into_iter()
                .map(|arg| Self::parseStr(arg.trim()))
                .collect::<Result<Vec<_>, _>>()?;
            return match name {
                "Vec" if parsed.len() == 1 => Ok(Ty::Vec(Box::new(parsed[0].clone()))),
                "Vec" => Err(format!("TypeError: Vec expects 1 type argument, got {}", parsed.len())),
                "HashMap" if parsed.len() == 2 => {
                    if parsed[0] != Ty::String {
                        return Err(format!("TypeError: HashMap key must be string, got {:?}", parsed[0]));
                    }
                    Ok(Ty::HashMap(Box::new(parsed[1].clone())))
                }
                "HashMap" => Err(format!("TypeError: HashMap expects 2 type arguments, got {}", parsed.len())),
                "Option" if parsed.len() == 1 => Ok(Ty::Option(Box::new(parsed[0].clone()))),
                "Option" => Err(format!("TypeError: Option expects 1 type argument, got {}", parsed.len())),
                "Result" if parsed.len() == 2 => Ok(Ty::Result(Box::new(parsed[0].clone()), Box::new(parsed[1].clone()))),
                "Result" => Err(format!("TypeError: Result expects 2 type arguments, got {}", parsed.len())),
                _ => Err(format!("TypeError: generic arguments are only supported for built-in containers, got '{}'", name)),
            };
        }

        match s {
            "int" => Ok(Ty::Int),
            "float" => Ok(Ty::Float),
            "bool" => Ok(Ty::Bool),
            "char" => Ok(Ty::Char),
            "string" => Ok(Ty::String),
            "void" => Ok(Ty::Void),
            "null" => Ok(Ty::Null),
            "any" => Ok(Ty::Any),
            "Int" => Err("TypeError: use lowercase 'int' instead of 'Int'".to_string()),
            "Float" => Err("TypeError: use lowercase 'float' instead of 'Float'".to_string()),
            "Bool" => Err("TypeError: use lowercase 'bool' instead of 'Bool'".to_string()),
            "String" => Err("TypeError: use lowercase 'string' instead of 'String'".to_string()),
            "Void" => Err("TypeError: use lowercase 'void' instead of 'Void'".to_string()),
            "Char" => Err("TypeError: use lowercase 'char' instead of 'Char'".to_string()),
            "Any" => Err("TypeError: use lowercase 'any' instead of 'Any'".to_string()),

            // bare containers default their payload type(s) to any
            "Vec" => Ok(Ty::Vec(Box::new(Ty::Any))),
            "HashMap" => Ok(Ty::HashMap(Box::new(Ty::Any))),
            "Option" => Ok(Ty::Option(Box::new(Ty::Any))),
            "Result" => Ok(Ty::Result(Box::new(Ty::Any), Box::new(Ty::Any))),
            // unknown UpperCase → provisional Struct (reconciled with Enum in Phase 6)
            name if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) => {
                Ok(Ty::Struct(name.to_string()))
            }

            other => Err(format!("TypeError: unknown type annotation '{}'", other)),
        }
    }

    // Flatten nested unions and deduplicate.
    fn flattenUnion(items: Vec<Ty>) -> Ty {
        let mut out: Vec<Ty> = Vec::new();
        for item in items {
            match item {
                Ty::Union(inner) => {
                    for t in inner {
                        if !out.contains(&t) {
                            out.push(t);
                        }
                    }
                }
                t => {
                    if !out.contains(&t) {
                        out.push(t);
                    }
                }
            }
        }
        if out.len() == 1 {
            out.remove(0)
        } else {
            Ty::Union(out)
        }
    }

    fn isAssignableTo(&self, target: &Ty) -> bool {
        match (self, target) {
            // Any is assignable to/from everything
            (_, Ty::Any) | (Ty::Any, _) => true,

            // exact match (covers primitives, Struct/Enum by name, nested generics)
            (a, b) if a == b => true,

            (Ty::Function(source), Ty::Function(target)) => {
                source.params == target.params
                    && source.minArgs == target.minArgs
                    && source.variadic == target.variadic
                    && source.ret == target.ret
            }

            // null is assignable to Nullable<_> or Null
            (Ty::Null, Ty::Nullable(_)) | (Ty::Null, Ty::Null) => true,
            (Ty::Null, _) => false,

            // T is assignable to Nullable<T>
            (inner, Ty::Nullable(targetInner)) => inner.isAssignableTo(targetInner),

            // Nullable<T> is not assignable to T (need explicit unwrap)
            (Ty::Nullable(_), _) => false,

            // T assignable to Union<…> if it satisfies at least one branch
            (src, Ty::Union(items)) => items.iter().any(|t| src.isAssignableTo(t)),

            // Union<…> assignable to T only if every branch satisfies T
            (Ty::Union(items), dst) => items.iter().all(|t| t.isAssignableTo(dst)),

            // generic containers are invariant — inner types must match exactly
            (Ty::Vec(a), Ty::Vec(b)) => a == b,
            (Ty::HashMap(a), Ty::HashMap(b)) => a == b,
            (Ty::Option(source), Ty::Option(target)) => source == target || **target == Ty::Any,
            (Ty::Result(sourceOk, sourceErr), Ty::Result(targetOk, targetErr)) => {
                (sourceOk == targetOk || **targetOk == Ty::Any)
                    && (sourceErr == targetErr || **targetErr == Ty::Any)
            }
            (Ty::Tuple(source), Ty::Tuple(target)) => {
                source.len() == target.len()
                    && source.iter().zip(target).all(|(a, b)| a.isAssignableTo(b))
            }

            // numeric widening: int is safely assignable to float (one-way)
            (Ty::Int, Ty::Float) => true,

            _ => false,
        }
    }

    // Least-upper-bound for array literal element unification.
    fn join(a: &Ty, b: &Ty) -> Ty {
        if *a == Ty::Any { return b.clone(); }
        if *b == Ty::Any { return a.clone(); }
        if a == b { return a.clone(); }

        // null + T → Nullable<T>
        match (a, b) {
            (Ty::Null, other) | (other, Ty::Null) => {
                return match other {
                    Ty::Nullable(_) => other.clone(),
                    t => Ty::Nullable(Box::new(t.clone())),
                };
            }
            (Ty::Nullable(inner), other) | (other, Ty::Nullable(inner)) => {
                let joined = Ty::join(inner.as_ref(), other);
                return Ty::Nullable(Box::new(joined));
            }
            _ => {}
        }

        // flatten into union
        let mut out: Vec<Ty> = Vec::new();
        let push = |items: &mut Vec<Ty>, t: Ty| {
            if !items.contains(&t) { items.push(t); }
        };
        match a {
            Ty::Union(items) => { for t in items { push(&mut out, t.clone()); } }
            t => push(&mut out, t.clone()),
        }
        match b {
            Ty::Union(items) => { for t in items { push(&mut out, t.clone()); } }
            t => push(&mut out, t.clone()),
        }
        Ty::Union(out)
    }
}

fn matchingParen(value: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in value.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 { return Some(index); }
            }
            _ => {}
        }
    }
    None
}

fn topLevelGenericStart(value: &str) -> Option<usize> {
    let mut depths = [0usize; 3];
    for (index, ch) in value.char_indices() {
        match ch {
            '(' => depths[0] += 1,
            ')' => depths[0] = depths[0].saturating_sub(1),
            '[' => depths[1] += 1,
            ']' => depths[1] = depths[1].saturating_sub(1),
            '{' => depths[2] += 1,
            '}' => depths[2] = depths[2].saturating_sub(1),
            '<' if depths.iter().all(|depth| *depth == 0) => return Some(index),
            _ => {}
        }
    }
    None
}

fn matchingGenericClose(value: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (relativeIndex, ch) in value[open..].char_indices() {
        let index = open + relativeIndex;
        match ch {
            '<' => depth += 1,
            '>' if index == 0 || value.as_bytes()[index - 1] != b'-' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 { return Some(index); }
            }
            _ => {}
        }
    }
    None
}

fn splitTopLevel(value: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depths = [0usize; 4];
    let mut start = 0;
    for (index, ch) in value.char_indices() {
        match ch {
            '(' => depths[0] += 1,
            ')' => depths[0] = depths[0].saturating_sub(1),
            '[' => depths[1] += 1,
            ']' => depths[1] = depths[1].saturating_sub(1),
            '{' => depths[2] += 1,
            '}' => depths[2] = depths[2].saturating_sub(1),
            '<' => depths[3] += 1,
            '>' if index == 0 || value.as_bytes()[index - 1] != b'-' => {
                depths[3] = depths[3].saturating_sub(1)
            }
            _ => {}
        }
        if ch == separator && depths.iter().all(|depth| *depth == 0) {
            parts.push(&value[start..index]);
            start = index + ch.len_utf8();
        }
    }
    parts.push(&value[start..]);
    parts
}

// ─── TypeChecker ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct TypeChecker {
    scopes: Vec<HashMap<std::string::String, (Ty, bool)>>,
    functions: HashMap<std::string::String, FuncSig>,
    directFunctions: HashMap<std::string::String, FuncSig>,
    namespaceFunctions: HashMap<std::string::String, HashMap<std::string::String, FuncSig>>,
    structs: HashMap<std::string::String, StructDef>,   // Phase 6
    enums: HashMap<std::string::String, EnumDef>,       // Phase 6
    enum_variants: HashMap<std::string::String, (std::string::String, std::string::String)>,
    nominalAliases: HashMap<std::string::String, std::string::String>,
    nominalNames: HashMap<std::string::String, std::string::String>,
    createStaticTarget: Option<std::string::String>,
    createFallbackAllowed: bool,
    currentReturn: Option<Ty>,
    returnTypes: Vec<Ty>,
    returnTypePending: Vec<bool>,
    collectingReturnType: bool,
    unresolvedReturnCall: bool,
    loopDepth: usize,
    lambdaCaptureDepths: Vec<usize>,
    currentStruct: Option<std::string::String>,
    localModule: Option<PathBuf>,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self {
            scopes: Vec::new(),
            functions: HashMap::new(),
            directFunctions: HashMap::new(),
            namespaceFunctions: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            enum_variants: HashMap::new(),
            nominalAliases: HashMap::new(),
            nominalNames: HashMap::new(),
            createStaticTarget: None,
            createFallbackAllowed: false,
            currentReturn: None,
            returnTypes: Vec::new(),
            returnTypePending: Vec::new(),
            collectingReturnType: false,
            unresolvedReturnCall: false,
            loopDepth: 0,
            lambdaCaptureDepths: Vec::new(),
            currentStruct: None,
            localModule: None,
        }
    }
}

// Helper to build a FuncSig quickly.
fn sig(params: Vec<Ty>, minArgs: usize, ret: Ty) -> FuncSig {
    FuncSig { params, minArgs, variadic: None, ret: Box::new(ret), inferReturn: false, returnResolved: true, visibility: None }
}

fn variadicSig(params: Vec<Ty>, minArgs: usize, varTy: Ty, ret: Ty) -> FuncSig {
    FuncSig {
        params,
        minArgs,
        variadic: Some(Box::new(varTy)),
        ret: Box::new(ret),
        inferReturn: false,
        returnResolved: true,
        visibility: None,
    }
}

fn nominalName(stmt: &Stmt) -> Option<&str> {
    match stmt {
        Stmt::StructDecl { name, .. }
        | Stmt::EnumDecl { name, .. }
        | Stmt::EnumDeclWithPayload { name, .. } => Some(name),
        _ => None,
    }
}

fn resolvedNominalId(moduleId: &PathBuf, name: &str) -> String {
    format!("{}::{}", moduleId.display(), name)
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut c = Self::default();
        c.scopes.push(HashMap::new());

        // builtins
        c.functions.insert("println".into(), variadicSig(vec![], 0, Ty::Any, Ty::Void));
        c.functions.insert("print".into(),   variadicSig(vec![], 0, Ty::Any, Ty::Void));
        c.functions.insert("eprintln".into(), variadicSig(vec![], 0, Ty::Any, Ty::Void));
        c.functions.insert("eprint".into(),   variadicSig(vec![], 0, Ty::Any, Ty::Void));
        c.functions.insert("assert".into(),  sig(vec![Ty::Bool, Ty::Any], 2, Ty::Void));
        c.functions.insert("abs".into(),     sig(vec![Ty::Any], 1, Ty::Any));
        c.functions.insert("max".into(),     sig(vec![Ty::Any, Ty::Any], 2, Ty::Any));
        c.functions.insert("min".into(),     sig(vec![Ty::Any, Ty::Any], 2, Ty::Any));
        c.functions.insert("random".into(),  sig(vec![], 0, Ty::Any));
        c.functions.insert("floor".into(),   sig(vec![Ty::Any], 1, Ty::Int));
        c.functions.insert("ceil".into(),    sig(vec![Ty::Any], 1, Ty::Int));
        c.functions.insert("round".into(),   sig(vec![Ty::Any], 1, Ty::Int));
        c.functions.insert("sqrt".into(),    sig(vec![Ty::Any], 1, Ty::Float));
        c.functions.insert("pow".into(),     sig(vec![Ty::Any, Ty::Any], 2, Ty::Float));
        c.functions.insert("log".into(),     sig(vec![Ty::Any], 1, Ty::Float));
        c.functions.insert("sin".into(),     sig(vec![Ty::Any], 1, Ty::Float));
        c.functions.insert("cos".into(),     sig(vec![Ty::Any], 1, Ty::Float));
        c.functions.insert("tan".into(),     sig(vec![Ty::Any], 1, Ty::Float));
        c.functions.insert("len".into(),     sig(vec![Ty::Any], 1, Ty::Int));
        c.functions.insert("int".into(),     sig(vec![Ty::Union(vec![Ty::Char, Ty::Int])], 1, Ty::Int));
        c.functions.insert("char".into(),    sig(vec![Ty::Union(vec![Ty::Int, Ty::Char])], 1, Ty::Char));
        c.functions.insert("float".into(),   sig(vec![Ty::Union(vec![Ty::Int, Ty::Float])], 1, Ty::Float));
        c.functions.insert("string".into(),  sig(vec![Ty::Any], 1, Ty::String));
        c.functions.insert("push".into(),    sig(vec![Ty::Any, Ty::Any], 2, Ty::Void));
        c.functions.insert("pop".into(),     sig(vec![Ty::Any], 1, Ty::Any));
        c.functions.insert("input".into(),   sig(vec![], 0, Ty::String));
        c.functions.insert("readFile".into(),    sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("writeFile".into(),   sig(vec![Ty::String, Ty::String], 2, Ty::Any));
        c.functions.insert("appendFile".into(),  sig(vec![Ty::String, Ty::String], 2, Ty::Any));
        c.functions.insert("fileExists".into(),  sig(vec![Ty::String], 1, Ty::Bool));
        c.functions.insert("deleteFile".into(),  sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("mkdir".into(),       sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("listDir".into(),     sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("getEnv".into(),      sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("args".into(),        sig(vec![], 0, Ty::Vec(Box::new(Ty::String))));
        c.functions.insert("exit".into(),        sig(vec![Ty::Int], 1, Ty::Void));
        c.functions.insert("exec".into(),        sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("execDetailed".into(), sig(vec![Ty::String], 1, Ty::Any));

        // extra fs/process builtins
        c.functions.insert("cwd".into(),           sig(vec![], 0, Ty::String));
        c.functions.insert("nativeCwd".into(),     sig(vec![], 0, Ty::String));
        c.functions.insert("chdir".into(),         sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("nativeChdir".into(),   sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("tempDir".into(),       sig(vec![], 0, Ty::String));
        c.functions.insert("nativeTempDir".into(), sig(vec![], 0, Ty::String));

        c.functions.insert("mkdirAll".into(),       sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("nativeMkdirAll".into(), sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("removeDir".into(),      sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("removeDirAll".into(),   sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("nativeRemoveDir".into(),    sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("nativeRemoveDirAll".into(), sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("copyFile".into(),       sig(vec![Ty::String, Ty::String], 2, Ty::Any));
        c.functions.insert("moveFile".into(),       sig(vec![Ty::String, Ty::String], 2, Ty::Any));
        c.functions.insert("nativeCopyFile".into(), sig(vec![Ty::String, Ty::String], 2, Ty::Any));
        c.functions.insert("nativeMoveFile".into(), sig(vec![Ty::String, Ty::String], 2, Ty::Any));
        c.functions.insert("isFile".into(),         sig(vec![Ty::String], 1, Ty::Bool));
        c.functions.insert("isDir".into(),          sig(vec![Ty::String], 1, Ty::Bool));
        c.functions.insert("nativeIsFile".into(),   sig(vec![Ty::String], 1, Ty::Bool));
        c.functions.insert("nativeIsDir".into(),    sig(vec![Ty::String], 1, Ty::Bool));
        c.functions.insert("listDirPaths".into(),   sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("nativeListDirPaths".into(), sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("pathJoin".into(),       sig(vec![Ty::String, Ty::String], 2, Ty::String));
        c.functions.insert("basename".into(),       sig(vec![Ty::String], 1, Ty::String));
        c.functions.insert("dirname".into(),        sig(vec![Ty::String], 1, Ty::String));
        c.functions.insert("extname".into(),        sig(vec![Ty::String], 1, Ty::String));
        c.functions.insert("nativePathJoin".into(), sig(vec![Ty::String, Ty::String], 2, Ty::String));
        c.functions.insert("nativeBasename".into(), sig(vec![Ty::String], 1, Ty::String));
        c.functions.insert("nativeDirname".into(),  sig(vec![Ty::String], 1, Ty::String));
        c.functions.insert("nativeExtname".into(),  sig(vec![Ty::String], 1, Ty::String));

        c.functions.insert("setEnv".into(),     sig(vec![Ty::String, Ty::String], 2, Ty::Any));
        c.functions.insert("unsetEnv".into(),   sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("envAll".into(),     sig(vec![], 0, Ty::Any));
        c.functions.insert("sleepMs".into(),    sig(vec![Ty::Any], 1, Ty::Any));
        c.functions.insert("which".into(),      sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("nativeSetEnv".into(),   sig(vec![Ty::String, Ty::String], 2, Ty::Any));
        c.functions.insert("nativeUnsetEnv".into(), sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("nativeEnvAll".into(),   sig(vec![], 0, Ty::Any));
        c.functions.insert("nativeSleepMs".into(),  sig(vec![Ty::Any], 1, Ty::Any));
        c.functions.insert("nativeWhich".into(),    sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("nativeExecDetailed".into(), sig(vec![Ty::String], 1, Ty::Any));

        // aliased natives used by std wrappers to avoid self-recursion
        c.functions.insert("nativeMkdir".into(),   sig(vec![Ty::String], 1, Ty::Any));
        c.functions.insert("nativeArgs".into(),    sig(vec![], 0, Ty::Vec(Box::new(Ty::String))));
        c.functions.insert("nativeExit".into(),    sig(vec![Ty::Any], 1, Ty::Void));
        c.functions.insert("nativeSqrt".into(),    sig(vec![Ty::Any], 1, Ty::Float));
        c.functions.insert("nativePow".into(),     sig(vec![Ty::Any, Ty::Any], 2, Ty::Float));
        c.functions.insert("nativeLog".into(),     sig(vec![Ty::Any], 1, Ty::Float));
        c.functions.insert("nativeRandom".into(),  sig(vec![], 0, Ty::Any));

        c
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn checkProgram(&mut self, statements: &[Stmt]) -> Result<(), TypeError> {
        self.registerNominals(statements)?;
        for stmt in statements {
            if let Stmt::FuncDecl { name, visibility, params, return_ty, .. } = stmt {
                self.registerFunction(name, *visibility, params, return_ty)?;
            }
        }
        self.inferFunctionReturns(statements)?;
        self.createStaticTarget = Self::createTarget(statements, &self.functions);
        self.createFallbackAllowed = statements.iter()
            .filter(|stmt| matches!(stmt, Stmt::StructDecl { .. }))
            .count() != 1;
        for stmt in statements {
            self.checkStmt(stmt)?;
        }
        self.rejectBadEntryMain()?;
        Ok(())
    }

    fn inferFunctionReturns(&mut self, statements: &[Stmt]) -> Result<(), TypeError> {
        let maxPasses = statements.len() + 1;
        let mut successfullyChecked = HashSet::new();
        let mut inferenceErrors: HashMap<std::string::String, TypeError> = HashMap::new();
        for _ in 0..maxPasses {
            let mut changed = false;
            for stmt in statements {
                let Stmt::FuncDecl { name, return_ty, .. } = stmt else { continue; };
                if return_ty.is_some() { continue; }
                let Some(previous) = self.functions.get(name).cloned() else { continue; };
                if !previous.inferReturn { continue; }
                let mut probe = self.clone();
                match probe.checkStmt(stmt) {
                    Ok(()) => {
                        successfullyChecked.insert(name.clone());
                        inferenceErrors.remove(name);
                        if let Some(inferred) = probe.functions.get(name).cloned() {
                            if inferred.ret != previous.ret || inferred.returnResolved != previous.returnResolved {
                                self.functions.insert(name.clone(), inferred);
                                changed = true;
                            }
                        }
                    }
                    Err(error) => {
                        if !successfullyChecked.contains(name) { inferenceErrors.insert(name.clone(), error); }
                    }
                }
            }
            if !changed { break; }
        }
        for stmt in statements {
            let Stmt::FuncDecl { name, return_ty, .. } = stmt else { continue; };
            if return_ty.is_none() && self.functions.get(name).is_some_and(|signature| signature.inferReturn && !signature.returnResolved) {
                if !successfullyChecked.contains(name) {
                    if let Some(error) = inferenceErrors.get(name) { return Err(error.clone()); }
                }
                return Err(TypeError {
                    message: format!("TypeError: cannot infer recursive return type of '{}'; add a return annotation", name),
                    span: Span::new(0, 0),
                });
            }
        }
        Ok(())
    }

    pub fn checkResolvedProgram(
        program: &ResolvedProgram,
    ) -> Result<(), LocatedTypeError> {
        if program.statements.len() != program.statementProvenance.len() {
            return Err(LocatedTypeError {
                file: PathBuf::new(),
                source: std::string::String::new(),
                error: TypeError {
                    message: "TypeError: resolved statement provenance is not aligned".to_string(),
                    span: Span::new(0, 0),
                },
            });
        }

        let mut modules = Vec::new();
        let mut nominalStatementsByModule: HashMap<PathBuf, Vec<Stmt>> = HashMap::new();
        let mut localTypeAliasesByModule: HashMap<PathBuf, HashMap<String, String>> = HashMap::new();
        let mut nominalNames = HashMap::new();
        for provenance in &program.statementProvenance {
            if !modules.contains(&provenance.moduleId) {
                modules.push(provenance.moduleId.clone());
            }
        }
        for (stmt, provenance) in program.statements.iter().zip(&program.statementProvenance) {
            let Some(name) = nominalName(stmt) else { continue; };
            let id = resolvedNominalId(&provenance.moduleId, name);
            nominalNames.insert(id.clone(), name.to_string());
            localTypeAliasesByModule.entry(provenance.moduleId.clone())
                .or_default().insert(name.to_string(), id);
            nominalStatementsByModule.entry(provenance.moduleId.clone())
                .or_default().push(stmt.clone());
        }

        let mut moduleTypeAliases = HashMap::new();
        for moduleId in &modules {
            let mut aliases = localTypeAliasesByModule.get(moduleId).cloned().unwrap_or_default();
            let mut visibleOwners: HashMap<String, PathBuf> = aliases.keys()
                .map(|name| (name.clone(), moduleId.clone())).collect();
            if let Some(bindings) = program.moduleBindings.get(moduleId) {
                for (alias, symbol) in &bindings.types {
                    if let Some(previousOwner) = visibleOwners.insert(alias.clone(), symbol.moduleId.clone()) {
                        if previousOwner != symbol.moduleId {
                            let span = program.statementProvenance.iter()
                                .find(|provenance| &provenance.moduleId == moduleId)
                                .map(|provenance| provenance.span)
                                .unwrap_or_else(|| Span::new(0, 0));
                            return Err(LocatedTypeError {
                                file: moduleId.clone(),
                                source: program.sources.get(moduleId).cloned().unwrap_or_default(),
                                error: TypeError {
                                    message: format!("TypeError: imported type name '{}' is ambiguous", alias),
                                    span,
                                },
                            });
                        }
                    }
                    aliases.insert(alias.clone(), resolvedNominalId(&symbol.moduleId, &symbol.name));
                }
            }
            moduleTypeAliases.insert(moduleId.clone(), aliases);
        }

        // Register every owner's nominal names before resolving any field or payload types.
        let mut structs = HashMap::new();
        let mut enums = HashMap::new();
        for moduleId in &modules {
            let mut owner = TypeChecker::new();
            owner.nominalAliases = localTypeAliasesByModule.get(moduleId).cloned().unwrap_or_default();
            owner.nominalNames = nominalNames.clone();
            let statements = nominalStatementsByModule.get(moduleId).map(Vec::as_slice).unwrap_or(&[]);
            owner.registerNominalNames(statements);
            structs.extend(owner.structs);
            enums.extend(owner.enums);
        }
        for moduleId in &modules {
            let mut owner = TypeChecker::new();
            owner.nominalAliases = moduleTypeAliases.get(moduleId).cloned().unwrap_or_default();
            owner.nominalNames = nominalNames.clone();
            owner.structs = structs.clone();
            owner.enums = enums.clone();
            let statements = nominalStatementsByModule.get(moduleId).map(Vec::as_slice).unwrap_or(&[]);
            owner.registerNominalDefinitions(statements).map_err(|error| LocatedTypeError {
                file: moduleId.clone(),
                source: program.sources.get(moduleId).cloned().unwrap_or_default(),
                error,
            })?;
            structs = owner.structs;
            enums = owner.enums;
        }

        let mut signatures = HashMap::new();
        for moduleId in &modules {
            let mut signatureBuilder = TypeChecker::new();
            signatureBuilder.nominalAliases = moduleTypeAliases.get(moduleId).cloned().unwrap_or_default();
            signatureBuilder.nominalNames = nominalNames.clone();
            signatureBuilder.structs = structs.clone();
            signatureBuilder.enums = enums.clone();
            for (stmt, provenance) in program.statements.iter().zip(&program.statementProvenance) {
                if &provenance.moduleId != moduleId { continue; }
                if let Stmt::FuncDecl { name, visibility, params, return_ty, .. } = stmt {
                    let signature = signatureBuilder
                        .buildSignature(params, return_ty, Some(*visibility))
                        .map_err(|error| LocatedTypeError {
                            file: moduleId.clone(),
                            source: program.sources.get(moduleId).cloned().unwrap_or_default(),
                            error,
                        })?;
                    signatures.insert(
                        ResolvedSymbol { moduleId: moduleId.clone(), name: name.clone() },
                        signature,
                    );
                }
            }
        }

        let dependedOn = program
            .moduleBindings
            .values()
            .flat_map(|bindings| bindings.dependencies.iter().cloned())
            .collect::<HashSet<_>>();
        let entryModules = program
            .moduleBindings
            .keys()
            .filter(|moduleId| !dependedOn.contains(*moduleId))
            .cloned()
            .collect::<HashSet<_>>();

        for moduleId in modules {
            let mut checker = TypeChecker::new();
            checker.localModule = Some(moduleId.clone());
            checker.nominalAliases = moduleTypeAliases.get(&moduleId).cloned().unwrap_or_default();
            checker.nominalNames = nominalNames.clone();
            checker.structs = structs.clone();
            checker.enums = enums.clone();
            let visibleAliases = checker.nominalAliases.clone();
            checker.registerVisibleNominalBindings(&visibleAliases);
            for (stmt, provenance) in program.statements.iter().zip(&program.statementProvenance) {
                if provenance.moduleId == moduleId {
                    if let Stmt::FuncDecl { name, .. } = stmt {
                        let symbol = ResolvedSymbol { moduleId: moduleId.clone(), name: name.clone() };
                        if let Some(signature) = signatures.get(&symbol) {
                            checker.functions.insert(name.clone(), signature.clone());
                        }
                    }
                }
            }

            if let Some(bindings) = program.moduleBindings.get(&moduleId) {
                for (alias, symbol) in &bindings.direct {
                    let signature = signatures.get(symbol).ok_or_else(|| LocatedTypeError {
                        file: moduleId.clone(),
                        source: program.sources.get(&moduleId).cloned().unwrap_or_default(),
                        error: TypeError {
                            message: format!("TypeError: imported function '{}' has no resolved signature", alias),
                            span: Span::new(0, 0),
                        },
                    })?;
                    if symbol.moduleId != moduleId
                        && !matches!(signature.visibility, Some(FunctionVisibility::Pub))
                    {
                        return Err(LocatedTypeError {
                            file: moduleId.clone(),
                            source: program.sources.get(&moduleId).cloned().unwrap_or_default(),
                            error: TypeError {
                                message: format!("TypeError: function '{}' is private", alias),
                                span: Span::new(0, 0),
                            },
                        });
                    }
                    checker.directFunctions.insert(alias.clone(), signature.clone());
                }
                for (namespace, symbols) in &bindings.namespaces {
                    let mut namespaceSignatures = HashMap::new();
                    for (name, symbol) in symbols {
                        let signature = signatures.get(symbol).ok_or_else(|| LocatedTypeError {
                            file: moduleId.clone(),
                            source: program.sources.get(&moduleId).cloned().unwrap_or_default(),
                            error: TypeError {
                                message: format!("TypeError: namespace function '{}.{}' has no resolved signature", namespace, name),
                                span: Span::new(0, 0),
                            },
                        })?;
                        if !matches!(signature.visibility, Some(FunctionVisibility::Pub)) {
                            return Err(LocatedTypeError {
                                file: moduleId.clone(),
                                source: program.sources.get(&moduleId).cloned().unwrap_or_default(),
                                error: TypeError {
                                    message: format!("TypeError: function '{}.{}' is private", namespace, name),
                                    span: Span::new(0, 0),
                                },
                            });
                        }
                        namespaceSignatures.insert(name.clone(), signature.clone());
                    }
                    checker.namespaceFunctions.insert(namespace.clone(), namespaceSignatures);
                }
            }

            let localStatements: Vec<Stmt> = program.statements.iter().zip(&program.statementProvenance)
                .filter_map(|(stmt, provenance)| (provenance.moduleId == moduleId).then(|| stmt.clone()))
                .collect();
            checker.inferFunctionReturns(&localStatements).map_err(|error| LocatedTypeError {
                file: moduleId.clone(),
                source: program.sources.get(&moduleId).cloned().unwrap_or_default(),
                error,
            })?;
            for stmt in &localStatements {
                if let Stmt::FuncDecl { name, .. } = stmt {
                    if let Some(signature) = checker.functions.get(name).cloned() {
                        signatures.insert(ResolvedSymbol { moduleId: moduleId.clone(), name: name.clone() }, signature);
                    }
                }
            }

            let localStatements: Vec<Stmt> = program.statements.iter().zip(&program.statementProvenance)
                .filter_map(|(stmt, provenance)| (provenance.moduleId == moduleId).then(|| stmt.clone()))
                .collect();
            checker.inferFunctionReturns(&localStatements).map_err(|error| LocatedTypeError {
                file: moduleId.clone(),
                source: program.sources.get(&moduleId).cloned().unwrap_or_default(),
                error,
            })?;
            for stmt in &localStatements {
                if let Stmt::FuncDecl { name, .. } = stmt {
                    if let Some(signature) = checker.functions.get(name).cloned() {
                        signatures.insert(ResolvedSymbol { moduleId: moduleId.clone(), name: name.clone() }, signature);
                    }
                }
            }

            checker.createStaticTarget = Self::createTargetForModule(
                &program.statements,
                &program.statementProvenance,
                &moduleId,
                &checker.functions,
            );
            checker.createFallbackAllowed = program.statements.iter().zip(&program.statementProvenance)
                .filter(|(stmt, provenance)| provenance.moduleId == moduleId && matches!(stmt, Stmt::StructDecl { .. }))
                .count() != 1;
            for (stmt, provenance) in program.statements.iter().zip(&program.statementProvenance) {
                if provenance.moduleId == moduleId {
                    checker.checkStmt(stmt).map_err(|error| LocatedTypeError {
                        file: moduleId.clone(),
                        source: program.sources.get(&moduleId).cloned().unwrap_or_default(),
                        error,
                    })?;
                }
            }
            if entryModules.contains(&moduleId) {
                checker.rejectBadEntryMain().map_err(|error| LocatedTypeError {
                    file: moduleId.clone(),
                    source: program.sources.get(&moduleId).cloned().unwrap_or_default(),
                    error,
                })?;
            }
        }
        Ok(())
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn registerNominals(&mut self, statements: &[Stmt]) -> Result<(), TypeError> {
        self.registerNominalNames(statements);
        self.registerNominalDefinitions(statements)
    }

    fn nominalKey(&self, name: &str) -> std::string::String {
        self.nominalAliases.get(name).cloned().unwrap_or_else(|| name.to_string())
    }

    fn nominalLabel(&self, name: &str) -> std::string::String {
        self.nominalNames.get(name).cloned().unwrap_or_else(|| name.to_string())
    }

    fn registerNominalNames(&mut self, statements: &[Stmt]) {
        for stmt in statements {
            if let Stmt::StructDecl { name, .. } = stmt {
                let id = self.nominalKey(name);
                self.nominalNames.insert(id.clone(), name.clone());
                self.structs.entry(id).or_default();
            }
            if let Stmt::EnumDecl { name, variants, .. } = stmt {
                let payloads = variants.iter().map(|variant| (variant.clone(), Vec::new())).collect();
                let id = self.nominalKey(name);
                self.nominalNames.insert(id.clone(), name.clone());
                self.enums.insert(id.clone(), EnumDef { variants: variants.clone(), payloads });
                self.define(name, Ty::Enum(id.clone()), true);
                for variant in variants {
                    self.enum_variants
                        .insert(variant.clone(), (id.clone(), variant.clone()));
                    self.define(variant, Ty::Enum(id.clone()), true);
                }
            }
            if let Stmt::EnumDeclWithPayload { name, variants, .. } = stmt {
                let names = variants.iter().map(|variant| variant.name.clone()).collect::<Vec<_>>();
                let payloads = variants.iter().map(|variant| (variant.name.clone(), Vec::new())).collect();
                let id = self.nominalKey(name);
                self.nominalNames.insert(id.clone(), name.clone());
                self.enums.insert(id.clone(), EnumDef { variants: names, payloads });
                self.define(name, Ty::Enum(id.clone()), true);
                for variant in variants {
                    self.enum_variants
                        .insert(variant.name.clone(), (id.clone(), variant.name.clone()));
                    self.define(&variant.name, Ty::Enum(id.clone()), true);
                }
            }
        }
    }

    fn registerNominalDefinitions(&mut self, statements: &[Stmt]) -> Result<(), TypeError> {
        for stmt in statements {
            if let Stmt::StructDecl { name, fields, methods, .. } = stmt {
                let id = self.nominalKey(name);
                let mut def = StructDef::default();
                for field in fields {
                    def.fields.insert(field.name.clone(), FieldDef {
                        ty: self.annotationTy(&field.ty, Span::new(0, 0))?,
                        isPub: field.is_pub,
                    });
                }
                for method in methods {
                    let visibility = Some(if method.is_pub { FunctionVisibility::Pub } else { FunctionVisibility::Private });
                    let signature = self.buildSignature(&method.params, &method.return_ty, visibility)?;
                    if method.is_static {
                        def.staticMethods.insert(method.name.clone(), signature);
                    } else {
                        def.methods.insert(method.name.clone(), signature);
                    }
                }
                self.structs.insert(id, def);
            }
            if let Stmt::EnumDeclWithPayload { name, variants, .. } = stmt {
                let id = self.nominalKey(name);
                let mut defs = EnumDef {
                    variants: variants.iter().map(|variant| variant.name.clone()).collect(),
                    payloads: HashMap::new(),
                };
                for variant in variants {
                    let mut payloadTypes = Vec::new();
                    for payload in &variant.payloads {
                        payloadTypes.push(self.annotationTy(&payload.ty, payload.span)?);
                    }
                    defs.payloads.insert(variant.name.clone(), payloadTypes);
                }
                self.enums.insert(id, defs);
            }
        }
        Ok(())
    }

    fn registerVisibleNominalBindings(&mut self, aliases: &HashMap<String, String>) {
        let mut variants = HashMap::new();
        let mut ambiguousVariants = HashSet::new();
        for (alias, id) in aliases {
            if self.enums.contains_key(id) {
                self.define(alias, Ty::Enum(id.clone()), true);
                if let Some(def) = self.enums.get(id) {
                    for variant in &def.variants {
                        let owner = (id.clone(), variant.clone());
                        if variants.get(variant).is_some_and(|previous| previous != &owner) {
                            ambiguousVariants.insert(variant.clone());
                        } else {
                            variants.insert(variant.clone(), owner);
                        }
                    }
                }
            } else if self.structs.contains_key(id) {
                self.define(alias, Ty::Struct(id.clone()), true);
            }
        }
        self.enum_variants = variants.into_iter()
            .filter(|(variant, _)| !ambiguousVariants.contains(variant))
            .collect();
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn createTarget(statements: &[Stmt], functions: &HashMap<std::string::String, FuncSig>) -> Option<String> {
        if statements.iter().filter(|stmt| matches!(stmt, Stmt::StructDecl { .. })).count() != 1 {
            return None;
        }
        let structName = statements.iter().find_map(|stmt| match stmt {
            Stmt::StructDecl { name, .. } => Some(name.clone()),
            _ => None,
        })?;
        functions.get("create").and_then(|signature| {
            (signature.visibility == Some(FunctionVisibility::Pub)
                && *signature.ret == Ty::Struct(structName.clone()))
                .then_some(structName)
        })
    }

    fn createTargetForModule(
        statements: &[Stmt],
        provenance: &[StatementProvenance],
        moduleId: &PathBuf,
        functions: &HashMap<std::string::String, FuncSig>,
    ) -> Option<String> {
        let localStructs = statements.iter().zip(provenance)
            .filter(|(_, origin)| &origin.moduleId == moduleId)
            .filter_map(|(stmt, _)| match stmt {
                Stmt::StructDecl { name, .. } => Some(name.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        if localStructs.len() != 1 { return None; }
        let structName = localStructs.into_iter().next()?;
        let structId = resolvedNominalId(moduleId, &structName);
        functions.get("create").and_then(|signature| {
            (signature.visibility == Some(FunctionVisibility::Pub)
                && *signature.ret == Ty::Struct(structId))
                .then_some(structName)
        })
    }

    fn preserveAnnotatedContainerArgs(_actual: &Ty, annotated: &Ty) -> Ty {
        annotated.clone()
    }

    fn bindPattern(&mut self, pattern: &crate::ast::stmt::Pattern, expected: &Ty, span: Span) -> Result<(), TypeError> {
        use crate::ast::stmt::Pattern;
        match pattern {
            Pattern::Wildcard => Ok(()),
            Pattern::Variable(name) => {
                if matches!(expected, Ty::Enum(enumName)
                    if self.enums.get(enumName).is_some_and(|def| def.variants.contains(name)))
                {
                    return Ok(());
                }
                self.define(name, expected.clone(), false);
                Ok(())
            }
            Pattern::Literal(literal) => {
                let actual = Self::literalTy(literal);
                if actual.isAssignableTo(expected) || *expected == Ty::Any {
                    Ok(())
                } else {
                    Err(TypeError {
                        message: format!("TypeError: pattern type {:?} does not match {:?}", actual, expected),
                        span,
                    })
                }
            }
            Pattern::Range { start, end } => {
                let startTy = Self::literalTy(start);
                let endTy = Self::literalTy(end);
                if startTy == endTy && startTy.isAssignableTo(expected) && endTy.isAssignableTo(expected) {
                    Ok(())
                } else {
                    Err(TypeError {
                        message: format!("TypeError: range pattern endpoints must match scrutinee type {:?}", expected),
                        span,
                    })
                }
            }
            Pattern::Struct { struct_name, fields } => {
                let structId = self.nominalKey(struct_name);
                if expected != &Ty::Struct(structId.clone()) {
                    return Err(TypeError {
                        message: format!("TypeError: struct pattern '{}' does not match {:?}", struct_name, expected),
                        span,
                    });
                }
                let structDef = self.structs.get(&structId).cloned().ok_or_else(|| TypeError {
                    message: format!("TypeError: unknown struct '{}'", struct_name),
                    span,
                })?;
                for (fieldName, nested) in fields {
                    let field = structDef.fields.get(fieldName).ok_or_else(|| TypeError {
                        message: format!("TypeError: struct '{}' has no field '{}'", struct_name, fieldName),
                        span,
                    })?;
                    if !field.isPub && !self.canAccessPrivateField(struct_name) {
                        return Err(TypeError {
                            message: format!("TypeError: cannot access private field '{}' on struct '{}'", fieldName, struct_name),
                            span,
                        });
                    }
                    self.bindPattern(nested, &field.ty, span)?;
                }
                Ok(())
            }
            Pattern::EnumVariant(enumName, variant) => {
                let enumId = self.nominalKey(enumName);
                if expected != &Ty::Enum(enumId.clone()) {
                    return Err(TypeError {
                        message: format!("TypeError: enum pattern '{}' does not match {:?}", enumName, expected),
                        span,
                    });
                }
                let payloads = self.enums.get(&enumId)
                    .and_then(|def| def.payloads.get(variant))
                    .ok_or_else(|| TypeError {
                        message: format!("TypeError: unknown variant '{}::{}'", enumName, variant),
                        span,
                    })?;
                let _ = payloads;
                Ok(())
            }
            Pattern::EnumVariantPayload { enum_name, variant, payloads } => {
                let enumName = match (enum_name, expected) {
                    (Some(name), Ty::Enum(scrutinee)) if self.nominalKey(name) == *scrutinee => scrutinee.as_str(),
                    (Some(name), _) => return Err(TypeError {
                        message: format!("TypeError: enum pattern '{}' does not match {:?}", name, expected),
                        span,
                    }),
                    (None, Ty::Enum(scrutinee)) => scrutinee,
                    (None, _) => return Err(TypeError {
                        message: format!("TypeError: payload variant '{}' does not match {:?}", variant, expected),
                        span,
                    }),
                };
                let expectedPayloads = self.enums.get(enumName)
                    .and_then(|def| def.payloads.get(variant)).cloned().ok_or_else(|| TypeError {
                        message: format!("TypeError: unknown variant '{}::{}'", enumName, variant),
                        span,
                    })?;
                if payloads.len() != expectedPayloads.len() {
                    return Err(TypeError {
                        message: format!("TypeError: variant '{}::{}' expects {} payload patterns but got {}", enumName, variant, expectedPayloads.len(), payloads.len()),
                        span,
                    });
                }
                for (nested, payloadTy) in payloads.iter().zip(&expectedPayloads) {
                    self.bindPattern(nested, payloadTy, span)?;
                }
                Ok(())
            }
        }
    }

    fn patternIsIrrefutable(&self, pattern: &crate::ast::stmt::Pattern, expected: &Ty) -> bool {
        use crate::ast::stmt::Pattern;
        match pattern {
            Pattern::Wildcard => true,
            Pattern::Variable(name) => !matches!(expected, Ty::Enum(enumName)
                if self.enums.get(enumName).is_some_and(|def| def.variants.contains(name))),
            _ => false,
        }
    }

    fn isMutableCapture(&self, name: &str) -> bool {
        let Some(captureDepth) = self.lambdaCaptureDepths.last().copied() else { return false; };
        self.scopes.iter().enumerate().rev().find_map(|(index, scope)| {
            scope.get(name).map(|(_, isConst)| index < captureDepth && !isConst)
        }).unwrap_or(false)
    }

    fn rootVariable(expr: &Expr) -> Option<&str> {
        match expr {
            Expr::Variable { name, .. } => Some(name),
            Expr::Get { object, .. } => Self::rootVariable(object),
            Expr::Index { target, .. } => Self::rootVariable(target),
            _ => None,
        }
    }

    fn canAccessPrivateField(&self, structName: &str) -> bool {
        self.currentStruct.as_deref() == Some(structName)
    }

    fn rejectBadEntryMain(&self) -> Result<(), TypeError> {
        let Some(signature) = self.functions.get("main") else {
            return Ok(());
        };
        let noParameters = signature.params.is_empty() && signature.variadic.is_none();
        if noParameters && *signature.ret == Ty::Void {
            return Ok(());
        }
        Err(TypeError {
            message: "TypeError: entry main must take no parameters and return void".to_string(),
            span: Span::new(0, 0),
        })
    }

    fn canCallPrivateMethod(&self, structId: &str) -> bool {
        if self.currentStruct.as_deref() == Some(structId) {
            return true;
        }
        match &self.localModule {
            None => true,
            Some(module) => structId.starts_with(&format!("{}::", module.display())),
        }
    }

    fn checkCapturedMutation(&self, target: &Expr, span: Span) -> Result<(), TypeError> {
        if let Some(name) = Self::rootVariable(target) {
            if self.isMutableCapture(name) {
                return Err(TypeError {
                    message: format!("TypeError: mutable captures are not supported ('{}')", name),
                    span,
                });
            }
        }
        Ok(())
    }

    fn checkStructMethods(&mut self, name: &str, methods: &[crate::ast::MethodDecl]) -> Result<(), TypeError> {
        let structId = self.nominalKey(name);
        let fields = self.structs.get(&structId).map(|def| def.fields.clone()).unwrap_or_default();
        for method in methods {
            let previousStruct = self.currentStruct.clone();
            let previousReturn = self.currentReturn.clone();
            let previousLoopDepth = self.loopDepth;
            self.currentStruct = (!method.is_static).then(|| structId.clone());
            self.currentReturn = method.return_ty.as_ref()
                .map(|ty| self.annotationTy(ty, Span::new(0, 0)))
                .transpose()?;
            self.loopDepth = 0;
            self.pushScope();
            if !method.is_static {
                for (fieldName, field) in &fields {
                    self.define(fieldName, field.ty.clone(), false);
                }
            }
            for param in &method.params {
                let paramTy = if let Some(ty) = &param.ty {
                    self.annotationTy(ty, param.default.as_ref().map(Expr::span).unwrap_or(Span::new(0, 0)))?
                } else {
                    Ty::Any
                };
                if let Some(default) = &param.default {
                    let actual = self.exprTyExpected(default, &paramTy)?;
                    if !actual.isAssignableTo(&paramTy) {
                        return Err(TypeError {
                            message: format!("TypeError: default value for '{}' expected {:?} but got {:?}", param.name, paramTy, actual),
                            span: default.span(),
                        });
                    }
                }
                self.define(&param.name, paramTy, false);
            }
            for stmt in &method.body { self.checkStmt(stmt)?; }
            if let Some(retTy) = &self.currentReturn {
                if !matches!(retTy, Ty::Void | Ty::Any) && !Self::alwaysReturns(&method.body) {
                    return Err(TypeError {
                        message: "TypeError: not all code paths return a value".to_string(),
                        span: Span::new(0, 0),
                    });
                }
            }
            self.popScope();
            self.currentStruct = previousStruct;
            self.currentReturn = previousReturn;
            self.loopDepth = previousLoopDepth;
        }
        Ok(())
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn registerFunction(
        &mut self,
        name: &str,
        visibility: FunctionVisibility,
        params: &[Parameter],
        return_ty: &Option<Type>,
    ) -> Result<(), TypeError> {
        let signature = self.buildSignature(params, return_ty, Some(visibility))?;
        self.functions.insert(name.to_string(), signature);
        Ok(())
    }

    fn buildSignature(
        &self,
        params: &[Parameter],
        return_ty: &Option<Type>,
        visibility: Option<FunctionVisibility>,
    ) -> Result<FuncSig, TypeError> {
        let mut ptys: Vec<Ty> = Vec::new();
        let mut minArgs = 0usize;
        let mut variadic: Option<Box<Ty>> = None;
        let mut sawDefault = false;

        for p in params {
            let pty = if let Some(ty) = &p.ty {
                self.annotationTy(ty, Span::new(0, 0))?
            } else {
                Ty::Any
            };

            if p.variadic {
                // variadic param type is Vec<T> (or Any); element type is T
                let elemTy = match pty {
                    Ty::Vec(inner) => *inner,
                    _ => Ty::Any,
                };
                variadic = Some(Box::new(elemTy));
                continue;
            }

            if p.default.is_none() && sawDefault {
                return Err(TypeError {
                    message: "TypeError: default parameters must be trailing".to_string(),
                    span: Span::new(0, 0),
                });
            }
            if p.default.is_some() {
                sawDefault = true;
            }
            ptys.push(pty);
            if p.default.is_none() {
                minArgs += 1;
            }
        }

        let inferReturn = return_ty.is_none();
        let ret = if let Some(r) = return_ty {
            self.annotationTy(r, Span::new(0, 0))?
        } else {
            Ty::Any
        };

        Ok(FuncSig { params: ptys, minArgs, variadic, ret: Box::new(ret), inferReturn, returnResolved: !inferReturn, visibility })
    }

    fn annotationTy(&self, annotation: &Type, span: Span) -> Result<Ty, TypeError> {
        let parsed = Ty::fromAnnotation(annotation).map_err(|message| TypeError { message, span })?;
        let ty = self.resolveNominals(parsed);
        self.validateKnownType(&ty, span)?;
        Ok(ty)
    }

    fn resolveNominals(&self, ty: Ty) -> Ty {
        match ty {
            Ty::Struct(name) => {
                let id = self.nominalKey(&name);
                if self.enums.contains_key(&id) { Ty::Enum(id) } else { Ty::Struct(id) }
            }
            Ty::Vec(inner) => Ty::Vec(Box::new(self.resolveNominals(*inner))),
            Ty::HashMap(inner) => Ty::HashMap(Box::new(self.resolveNominals(*inner))),
            Ty::Option(inner) => Ty::Option(Box::new(self.resolveNominals(*inner))),
            Ty::Tuple(items) => Ty::Tuple(items.into_iter().map(|item| self.resolveNominals(item)).collect()),
            Ty::Function(signature) => Ty::Function(Box::new(FuncSig {
                params: signature.params.into_iter().map(|param| self.resolveNominals(param)).collect(),
                minArgs: signature.minArgs,
                variadic: signature.variadic.map(|item| Box::new(self.resolveNominals(*item))),
                ret: Box::new(self.resolveNominals(*signature.ret)),
                inferReturn: signature.inferReturn,
                returnResolved: signature.returnResolved,
                visibility: signature.visibility,
            })),
            Ty::Nullable(inner) => Ty::Nullable(Box::new(self.resolveNominals(*inner))),
            Ty::Result(ok, err) => Ty::Result(
                Box::new(self.resolveNominals(*ok)),
                Box::new(self.resolveNominals(*err)),
            ),
            Ty::Union(items) => Ty::flattenUnion(items.into_iter().map(|t| self.resolveNominals(t)).collect()),
            other => other,
        }
    }

    fn validateKnownType(&self, ty: &Ty, span: Span) -> Result<(), TypeError> {
        let unknown = |name: &str| TypeError {
            message: format!("TypeError: unknown type '{}'", name),
            span,
        };
        match ty {
            Ty::Struct(name) if !self.structs.contains_key(name) => Err(unknown(&self.nominalLabel(name))),
            Ty::Enum(name) if !self.enums.contains_key(name) => Err(unknown(&self.nominalLabel(name))),
            Ty::Vec(inner) | Ty::HashMap(inner) | Ty::Option(inner) | Ty::Nullable(inner) => {
                self.validateKnownType(inner, span)
            }
            Ty::Result(ok, err) => {
                self.validateKnownType(ok, span)?;
                self.validateKnownType(err, span)
            }
            Ty::Function(signature) => {
                for param in &signature.params { self.validateKnownType(param, span)?; }
                if let Some(variadic) = &signature.variadic {
                    self.validateKnownType(variadic, span)?;
                }
                self.validateKnownType(&signature.ret, span)
            }
            Ty::Tuple(items) => {
                for item in items { self.validateKnownType(item, span)?; }
                Ok(())
            }
            Ty::Union(items) => {
                for item in items { self.validateKnownType(item, span)?; }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn applyNarrowing(&mut self, condition: &Expr, whenTrue: bool) -> Result<(), TypeError> {
        let narrowed = match condition {
            Expr::InstanceOf { value, ty, .. } => {
                if let Expr::Variable { name, span } = value.as_ref() {
                    let (current, isConst) = self.lookupAt(name, *span)?;
                    let target = self.annotationTy(ty, condition.span())?;
                    let result = if whenTrue {
                        Some(target)
                    } else {
                        Self::withoutType(&current, &target)
                    };
                    result.map(|next| (name.clone(), next, isConst))
                } else {
                    None
                }
            }
            Expr::Binary { left, op, right, .. }
                if matches!(op, TokenKind::EqualEqual | TokenKind::NotEqual) =>
            {
                let variable = match (left.as_ref(), right.as_ref()) {
                    (Expr::Variable { name, span }, Expr::Literal { value: Literal::Null, .. })
                    | (Expr::Literal { value: Literal::Null, .. }, Expr::Variable { name, span }) => {
                        Some((name, *span))
                    }
                    _ => None,
                };
                if let Some((name, span)) = variable {
                    let (current, isConst) = self.lookupAt(name, span)?;
                    let nonNullBranch = (*op == TokenKind::NotEqual) == whenTrue;
                    let next = if nonNullBranch {
                        Self::withoutNull(&current)
                    } else {
                        Some(Ty::Null)
                    };
                    next.map(|ty| (name.clone(), ty, isConst))
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some((name, ty, isConst)) = narrowed {
            self.define(&name, ty, isConst);
        }
        Ok(())
    }

    fn withoutNull(ty: &Ty) -> Option<Ty> {
        match ty {
            Ty::Nullable(inner) => Some((**inner).clone()),
            Ty::Union(items) => {
                let remaining = items.iter().filter(|item| **item != Ty::Null).cloned().collect::<Vec<_>>();
                match remaining.len() {
                    0 => None,
                    1 => Some(remaining.into_iter().next().unwrap()),
                    _ => Some(Ty::Union(remaining)),
                }
            }
            Ty::Any => Some(Ty::Any),
            other => Some(other.clone()),
        }
    }

    fn withoutType(current: &Ty, target: &Ty) -> Option<Ty> {
        match current {
            Ty::Union(items) => {
                let remaining = items.iter().filter(|item| *item != target).cloned().collect::<Vec<_>>();
                match remaining.len() {
                    0 => None,
                    1 => Some(remaining.into_iter().next().unwrap()),
                    _ => Some(Ty::Union(remaining)),
                }
            }
            Ty::Nullable(inner) if inner.as_ref() == target => Some(Ty::Null),
            other if other == target => None,
            other => Some(other.clone()),
        }
    }

    fn checkStmt(&mut self, stmt: &Stmt) -> Result<(), TypeError> {
        match stmt {
            Stmt::VarDecl { is_const, name, ty, initializer } => {
                self.checkRedecl(name, initializer.span())?;
                if ty.is_none() {
                    if let Some(namespace) = self.namespaceFunctions.get(name).cloned() {
                        let entriesMatch = match initializer {
                            Expr::DictLiteral { entries, .. } => {
                                entries.len() == namespace.len() && entries.iter().all(|(key, value)| {
                                    namespace.contains_key(key)
                                        && matches!(value, Expr::Variable { name: valueName, .. } if valueName == key)
                                })
                            }
                            _ => false,
                        };
                        if !entriesMatch {
                            return Err(TypeError {
                                message: format!("TypeError: '{}' conflicts with an imported namespace", name),
                                span: initializer.span(),
                            });
                        }
                        self.define(name, Ty::Namespace(namespace), *is_const);
                        return Ok(());
                    }
                }
                let finalTy = if let Some(ann) = ty {
                    let annTy = self.annotationTy(ann, initializer.span())?;
                    let initTy = self.exprTyExpected(initializer, &annTy)?;
                    if !initTy.isAssignableTo(&annTy) {
                        return Err(TypeError {
                            message: format!(
                                "TypeError: variable '{}' expected {:?} but got {:?}",
                                name, annTy, initTy
                            ),
                            span: initializer.span(),
                        });
                    }
                    Self::preserveAnnotatedContainerArgs(&initTy, &annTy)
                } else {
                    self.exprTy(initializer)?
                };
                self.define(name, finalTy, *is_const);
                Ok(())
            }

            Stmt::ExprStmt(expr) => { self.exprTy(expr)?; Ok(()) }

            Stmt::FuncDecl { name, params, return_ty, body, .. } => {
                let previousLoopDepth = self.loopDepth;
                self.loopDepth = 0;
                let previousReturnTypes = std::mem::take(&mut self.returnTypes);
                let previousReturnTypePending = std::mem::take(&mut self.returnTypePending);
                let previousCollectingReturnType = self.collectingReturnType;
                let previousUnresolvedReturnCall = self.unresolvedReturnCall;
                self.returnTypes = Vec::new();
                self.returnTypePending = Vec::new();
                self.collectingReturnType = false;
                self.unresolvedReturnCall = false;
                self.pushScope();
                for p in params {
                    let pty = if let Some(ty) = &p.ty {
                        self.annotationTy(ty, Span::new(0, 0))?
                    } else { Ty::Any };
                    if let Some(def) = &p.default {
                        self.checkRedecl(&p.name, def.span())?;
                        let dty = self.exprTyExpected(def, &pty)?;
                        if !dty.isAssignableTo(&pty) {
                            return Err(TypeError {
                                message: format!(
                                    "TypeError: default value for '{}' expected {:?} but got {:?}",
                                    p.name, pty, dty
                                ),
                                span: def.span(),
                            });
                        }
                    } else {
                        self.checkRedecl(&p.name, Span { start: 0, end: 0, line: 0, column: 0 })?;
                    }
                    self.define(&p.name, pty, false);
                }
                let prevReturn = self.currentReturn.clone();
                self.currentReturn = if let Some(ty) = return_ty {
                    Some(self.annotationTy(ty, Span::new(0, 0))?)
                } else {
                    None
                };
                for s in body { self.checkStmt(s)?; }

                let signature = self.functions.get(name).cloned();
                let inferReturn = signature.as_ref().is_some_and(|signature| signature.inferReturn);
                if inferReturn {
                    let mut sawVoid = false;
                    let mut sawValue = false;
                    let mut inferred: Option<Ty> = None;
                    let mut unresolvedReturn = false;
                    for (returnIndex, returnType) in self.returnTypes.iter().enumerate() {
                        if *returnType == Ty::Void {
                            sawVoid = true;
                        } else {
                            sawValue = true;
                            let pending = self.returnTypePending.get(returnIndex).copied().unwrap_or(false);
                            if pending && *returnType == Ty::Any {
                                unresolvedReturn = true;
                                continue;
                            }
                            inferred = Some(match inferred {
                                None => returnType.clone(),
                                Some(current) if current == Ty::Any || *returnType == Ty::Any => Ty::Any,
                                Some(current) => Ty::join(&current, returnType),
                            });
                        }
                    }
                    if sawVoid && sawValue {
                        return Err(TypeError {
                            message: format!("TypeError: function '{}' cannot mix value and void returns", name),
                            span: Span::new(0, 0),
                        });
                    }
                    let inferred = inferred.unwrap_or(if unresolvedReturn { Ty::Any } else { Ty::Void });
                    if inferred != Ty::Void && !Self::alwaysReturns(body) {
                        return Err(TypeError {
                            message: format!("TypeError: function '{}' must return {:?} on all paths", name, inferred),
                            span: Span { start: 0, end: 0, line: 0, column: 0 },
                        });
                    }
                    if let Some(mut signature) = signature {
                        signature.ret = Box::new(inferred);
                        signature.returnResolved = !unresolvedReturn;
                        self.functions.insert(name.clone(), signature);
                    }
                } else if let Some(retTy) = &self.currentReturn {
                    if !matches!(retTy, Ty::Void | Ty::Any) && !Self::alwaysReturns(body) {
                        return Err(TypeError {
                            message: "TypeError: not all code paths return a value".to_string(),
                            span: Span { start: 0, end: 0, line: 0, column: 0 },
                        });
                    }
                }

                self.currentReturn = prevReturn;
                self.returnTypes = previousReturnTypes;
                self.returnTypePending = previousReturnTypePending;
                self.collectingReturnType = previousCollectingReturnType;
                self.unresolvedReturnCall = previousUnresolvedReturnCall;
                self.popScope();
                self.loopDepth = previousLoopDepth;
                Ok(())
            }

            Stmt::If { condition, then_branch, else_branch } => {
                let cty = self.exprTy(condition)?;
                if cty != Ty::Bool {
                    return Err(TypeError {
                        message: format!("TypeError: if condition must be bool, got {:?}", cty),
                        span: condition.span(),
                    });
                }
                self.pushScope();
                self.applyNarrowing(condition, true)?;
                for s in then_branch { self.checkStmt(s)?; }
                self.popScope();
                if let Some(els) = else_branch {
                    self.pushScope();
                    self.applyNarrowing(condition, false)?;
                    for s in els { self.checkStmt(s)?; }
                    self.popScope();
                }
                Ok(())
            }

            Stmt::While { condition, body } => {
                let cty = self.exprTy(condition)?;
                if cty != Ty::Bool {
                    return Err(TypeError {
                        message: format!("TypeError: while condition must be bool, got {:?}", cty),
                        span: condition.span(),
                    });
                }
                self.pushScope();
                self.applyNarrowing(condition, true)?;
                self.loopDepth += 1;
                for s in body { self.checkStmt(s)?; }
                self.loopDepth -= 1;
                self.popScope();
                Ok(())
            }

            Stmt::Loop { body } => {
                self.pushScope();
                self.loopDepth += 1;
                for s in body { self.checkStmt(s)?; }
                self.loopDepth -= 1;
                self.popScope();
                Ok(())
            }

            Stmt::For { variable, iterator, body } => {
                use crate::ast::stmt::ForIterator;
                match iterator {
                    ForIterator::Range { start, end } => {
                        let sty = self.exprTy(start)?;
                        let ety = self.exprTy(end)?;
                        let isNum = |t: &Ty| matches!(t, Ty::Int | Ty::Float | Ty::Any);
                        if !isNum(&sty) {
                            return Err(TypeError {
                                message: format!(
                                    "TypeError: range start must be numeric, got {:?}", sty
                                ),
                                span: start.span(),
                            });
                        }
                        if !isNum(&ety) {
                            return Err(TypeError {
                                message: format!(
                                    "TypeError: range end must be numeric, got {:?}", ety
                                ),
                                span: end.span(),
                            });
                        }
                        self.pushScope();
                        self.define(variable, Ty::Int, false);
                        self.loopDepth += 1;
                        for s in body { self.checkStmt(s)?; }
                        self.loopDepth -= 1;
                        self.popScope();
                    }
                    ForIterator::Array(expr) => {
                        let arrTy = self.exprTy(expr)?;
                        let elemTy = match arrTy {
                            Ty::Vec(inner) => *inner,
                            Ty::Any => Ty::Any,
                            _ => return Err(TypeError {
                                message: "TypeError: for-in requires an array".to_string(),
                                span: expr.span(),
                            }),
                        };
                        self.pushScope();
                        self.define(variable, elemTy, false);
                        self.loopDepth += 1;
                        for s in body { self.checkStmt(s)?; }
                        self.loopDepth -= 1;
                        self.popScope();
                    }
                }
                Ok(())
            }

            Stmt::Import { .. } => Ok(()),
            Stmt::Break if self.loopDepth > 0 => Ok(()),
            Stmt::Continue if self.loopDepth > 0 => Ok(()),
            Stmt::Break => Err(TypeError {
                message: "TypeError: break is only valid inside a loop".to_string(),
                span: Span::new(0, 0),
            }),
            Stmt::Continue => Err(TypeError {
                message: "TypeError: continue is only valid inside a loop".to_string(),
                span: Span::new(0, 0),
            }),

            Stmt::Return(expr) => match expr {
                Some(expr) => {
                    let expectedType = self.currentReturn.clone();
                    let rty = if let Some(expected) = &expectedType {
                        self.exprTyExpected(expr, expected)?
                    } else {
                        let previousCollecting = self.collectingReturnType;
                        let previousUnresolved = self.unresolvedReturnCall;
                        self.collectingReturnType = true;
                        self.unresolvedReturnCall = false;
                        let actual = self.exprTy(expr);
                        let pending = self.unresolvedReturnCall;
                        self.unresolvedReturnCall = previousUnresolved;
                        self.collectingReturnType = previousCollecting;
                        let inferred = actual?;
                        self.returnTypePending.push(pending);
                        inferred
                    };
                    if let Some(expected) = &expectedType {
                        if !rty.isAssignableTo(expected) {
                            return Err(TypeError {
                                message: format!(
                                    "TypeError: return expected {:?} but got {:?}", expected, rty
                                ),
                                span: expr.span(),
                            });
                        }
                    } else {
                        self.returnTypes.push(rty.clone());
                    }
                    Ok(())
                }
                None => {
                    if let Some(expected) = &self.currentReturn {
                        if *expected != Ty::Void {
                            return Err(TypeError {
                                message: "TypeError: return value required".to_string(),
                                span: Span { start: 0, end: 0, line: 0, column: 0 },
                            });
                        }
                    } else {
                        self.returnTypes.push(Ty::Void);
                        self.returnTypePending.push(false);
                    }
                    Ok(())
                }
            },

            Stmt::StructDecl { name, methods, .. } => self.checkStructMethods(name, methods),

            // enum scope bindings are registered in the first pass of checkProgram
            Stmt::EnumDecl { .. } | Stmt::EnumDeclWithPayload { .. } => Ok(()),

            Stmt::Match { expr, arms } => {
                let exprTy = self.exprTy(expr)?;
                let mut coveredVariants = std::collections::HashSet::new();
                let mut coveredBool = [false, false];
                let mut hasCatchAll = false;
                for arm in arms {
                    use crate::ast::stmt::Pattern;
                    match &arm.pattern {
                        Pattern::Wildcard => hasCatchAll = true,
                        Pattern::Variable(name) => {
                            if let Ty::Enum(enumName) = &exprTy {
                                if self.enums.get(enumName).is_some_and(|def| def.variants.contains(name)) {
                                    coveredVariants.insert(name.clone());
                                } else {
                                    hasCatchAll = true;
                                }
                            } else {
                                hasCatchAll = true;
                            }
                        }
                        Pattern::EnumVariant(enumName, variant) => {
                            let enumId = self.nominalKey(enumName);
                            if !matches!(&exprTy, Ty::Enum(scrutinee) if scrutinee == &enumId) {
                                return Err(TypeError {
                                    message: format!("TypeError: enum pattern '{}' does not match {:?}", enumName, exprTy),
                                    span: expr.span(),
                                });
                            }
                            let payloads = self.enums.get(&enumId)
                                .filter(|def| def.variants.contains(variant))
                                .and_then(|def| def.payloads.get(variant));
                            if payloads.is_none() {
                                return Err(TypeError {
                                    message: format!("TypeError: unknown variant '{}::{}'", enumName, variant),
                                    span: expr.span(),
                                });
                            }
                            coveredVariants.insert(variant.clone());
                        }
                        Pattern::EnumVariantPayload { enum_name, variant, payloads } => {
                            let owner = enum_name.as_deref().map(|name| self.nominalKey(name))
                                .or_else(|| match &exprTy {
                                    Ty::Enum(name) => Some(name.clone()),
                                    _ => self.enum_variants.get(variant).map(|(name, _)| name.clone()),
                                }).ok_or_else(|| TypeError {
                                message: format!("TypeError: cannot resolve enum for variant '{}'", variant),
                                span: expr.span(),
                            })?;
                            if !matches!(&exprTy, Ty::Enum(scrutinee) if scrutinee == &owner) {
                                return Err(TypeError {
                                    message: format!("TypeError: enum pattern '{}' does not match {:?}", owner, exprTy),
                                    span: expr.span(),
                                });
                            }
                            let expectedPayloads = self.enums.get(&owner)
                                .filter(|def| def.variants.contains(&variant))
                                .and_then(|def| def.payloads.get(variant)).ok_or_else(|| TypeError {
                                    message: format!("TypeError: unknown variant '{}::{}'", owner, variant),
                                    span: expr.span(),
                                })?;
                            if payloads.len() != expectedPayloads.len() {
                                return Err(TypeError {
                                    message: format!("TypeError: variant '{}::{}' expects {} payload patterns but got {}", owner, variant, expectedPayloads.len(), payloads.len()),
                                    span: expr.span(),
                                });
                            }
                            if payloads.iter().zip(expectedPayloads)
                                .all(|(pattern, ty)| self.patternIsIrrefutable(pattern, ty))
                            {
                                coveredVariants.insert(variant.clone());
                            }
                        }
                        Pattern::Struct { .. } => {}
                        Pattern::Literal(lit) => {
                            let litTy = match lit {
                                Literal::String(_) | Literal::ByteString(_) => Ty::String,
                                Literal::Int(_)    => Ty::Int,
                                Literal::Float(_)  => Ty::Float,
                                Literal::Bool(_)   => Ty::Bool,
                                Literal::Null      => Ty::Null,
                                Literal::Char(_)   => Ty::Char,
                            };
                            if !litTy.isAssignableTo(&exprTy) && exprTy != Ty::Any {
                                return Err(TypeError {
                                    message: "TypeError: pattern doesn't match expression type"
                                        .to_string(),
                                    span: expr.span(),
                                });
                            }
                            if let Literal::Bool(value) = lit {
                                coveredBool[usize::from(*value)] = true;
                            }
                        }
                        Pattern::Range { start, end } => {
                            let startTy = Self::literalTy(start);
                            let endTy = Self::literalTy(end);
                            if startTy != endTy
                                || !startTy.isAssignableTo(&exprTy)
                                || !endTy.isAssignableTo(&exprTy)
                            {
                                return Err(TypeError {
                                    message: format!("TypeError: range pattern endpoints must match scrutinee type {:?}", exprTy),
                                    span: expr.span(),
                                });
                            }
                        }
                    }
                    self.pushScope();
                    self.bindPattern(&arm.pattern, &exprTy, expr.span())?;
                    for s in &arm.body { self.checkStmt(s)?; }
                    self.popScope();
                }
                if !hasCatchAll {
                    match &exprTy {
                        Ty::Enum(enumName) => {
                            if let Some(def) = self.enums.get(enumName) {
                                if def.variants.iter().any(|variant| !coveredVariants.contains(variant)) {
                                    return Err(TypeError {
                                        message: format!("TypeError: non-exhaustive match for enum '{}'", self.nominalLabel(enumName)),
                                        span: expr.span(),
                                    });
                                }
                            }
                        }
                        Ty::Bool if !coveredBool[0] || !coveredBool[1] => {
                            return Err(TypeError {
                                message: "TypeError: non-exhaustive match for bool; cover true and false".to_string(),
                                span: expr.span(),
                            });
                        }
                        _ => {}
                    }
                }
                Ok(())
            }
        }
    }

    fn exprTy(&mut self, expr: &Expr) -> Result<Ty, TypeError> {
        match expr {
            Expr::Literal { value: lit, .. } => Ok(match lit {
                Literal::String(_) | Literal::ByteString(_) => Ty::String,
                Literal::Int(_)    => Ty::Int,
                Literal::Float(_)  => Ty::Float,
                Literal::Bool(_)   => Ty::Bool,
                Literal::Null      => Ty::Null,
                Literal::Char(_)   => Ty::Char,
            }),

            Expr::Variable { name, span } => Ok(self.lookupAt(name, *span)?.0),

            Expr::Assign { name, value, span } => {
                let (cur, is_const) = self.lookupAt(name, *span)?;
                if is_const {
                    return Err(TypeError {
                        message: format!(
                            "TypeError: cannot assign to constant '{}'", name
                        ),
                        span: value.span(),
                    });
                }
                if self.isMutableCapture(name) {
                    return Err(TypeError {
                        message: format!("TypeError: mutable captures are not supported ('{}')", name),
                        span: value.span(),
                    });
                }
                let vty = self.exprTyExpected(value, &cur)?;
                if !vty.isAssignableTo(&cur) {
                    return Err(TypeError {
                        message: format!(
                            "TypeError: variable '{}' expected {:?} but got {:?}",
                            name, cur, vty
                        ),
                        span: value.span(),
                    });
                }
                Ok(cur)
            }

            Expr::Update { name, span, .. } => {
                let (cur, is_const) = self.lookupAt(name, *span)?;
                if is_const {
                    return Err(TypeError {
                        message: format!(
                            "TypeError: cannot assign to constant '{}'", name
                        ),
                        span: expr.span(),
                    });
                }
                if self.isMutableCapture(name) {
                    return Err(TypeError {
                        message: format!("TypeError: mutable captures are not supported ('{}')", name),
                        span: expr.span(),
                    });
                }
                if cur != Ty::Int {
                    return Err(TypeError {
                        message: format!(
                            "TypeError: ++/-- requires int lvalue, got {:?}", cur
                        ),
                        span: expr.span(),
                    });
                }
                Ok(cur)
            }

            Expr::UpdateLValue { target, span, .. } => {
                if let Some(name) = Self::rootVariable(target) {
                    if self.lookup(name).is_ok_and(|(_, isConst)| isConst) {
                        return Err(TypeError {
                            message: format!("TypeError: cannot update constant '{}'", name),
                            span: *span,
                        });
                    }
                    if self.isMutableCapture(name) {
                        return Err(TypeError {
                            message: format!("TypeError: mutable captures are not supported ('{}')", name),
                            span: *span,
                        });
                    }
                }
                let targetTy = self.exprTy(target)?;
                if targetTy != Ty::Int {
                    return Err(TypeError {
                        message: format!("TypeError: ++/-- requires int lvalue, got {:?}", targetTy),
                        span: *span,
                    });
                }
                Ok(Ty::Int)
            }

            Expr::Binary { left, op, right, span } => {
                let l = self.exprTy(left)?;
                let r = self.exprTy(right)?;
                self.binaryTy(&l, op, &r, *span)
            }

            Expr::Unary { op, right, span } => {
                let r = self.exprTy(right)?;
                self.unaryTy(op, &r, *span)
            }

            Expr::InstanceOf { value, ty, span } => {
                self.exprTy(value)?;
                self.annotationTy(ty, *span)?;
                Ok(Ty::Bool)
            }

            Expr::Lambda { params, body, .. } => {
                self.lambdaTy(params, body, None)
            }

            Expr::Call { callee, args, .. } => {
                // named function call — check against registered signature
                if let Expr::Variable { name, span } = callee.as_ref() {
                    if self.visibleFunction(name).is_some() {
                        return self.checkCallByName(name, args, expr.span());
                    }
                    if self.lookup(name).is_err() {
                        return Err(TypeError {
                            message: format!("TypeError: unknown function '{}'", name),
                            span: *span,
                        });
                    }
                }
                // callee from scope — PHASE-5 TODO: track function types in scope
                let cty = self.exprTy(callee)?;
                match cty {
                    Ty::Function(sig) => self.checkCallWithSig("function", &sig, args, expr.span()),
                    Ty::Any => {
                        for a in args { self.exprTy(a)?; }
                        Ok(Ty::Any)
                    }
                    other => Err(TypeError {
                        message: format!(
                            "TypeError: can only call functions, got {:?}", other
                        ),
                        span: callee.span(),
                    }),
                }
            }

            Expr::ArrayLiteral { elements, .. } => {
                let mut inner = Ty::Any;
                for e in elements {
                    let ety = self.exprTy(e)?;
                    inner = Ty::join(&inner, &ety);
                }
                Ok(Ty::Vec(Box::new(inner)))
            }

            Expr::TupleLiteral { elements, .. } => {
                let mut types = Vec::with_capacity(elements.len());
                for element in elements { types.push(self.exprTy(element)?); }
                Ok(Ty::Tuple(types))
            }

            Expr::DictLiteral { entries, .. } => {
                let mut valueTy = Ty::Any;
                let mut namespace = HashMap::new();
                let mut allFunctions = !entries.is_empty();
                for (key, v) in entries {
                    let vty = self.exprTy(v)?;
                    if let Ty::Function(sig) = vty.clone() {
                        namespace.insert(key.clone(), *sig);
                    } else {
                        allFunctions = false;
                    }
                    valueTy = Ty::join(&valueTy, &vty);
                }
                if allFunctions {
                    return Ok(Ty::Namespace(namespace));
                }
                Ok(Ty::HashMap(Box::new(valueTy)))
            }

            Expr::DictByteLiteral { entries, .. } => {
                let mut valueTy = Ty::Any;
                for (_, value) in entries {
                    let valueType = self.exprTy(value)?;
                    valueTy = Ty::join(&valueTy, &valueType);
                }
                Ok(Ty::HashMap(Box::new(valueTy)))
            }

            Expr::Index { target, index, .. } => {
                let tty = self.exprTy(target)?;
                let ity = self.exprTy(index)?;
                match tty {
                    Ty::Vec(inner) => {
                        if !ity.isAssignableTo(&Ty::Int) {
                            return Err(TypeError {
                                message: format!(
                                    "TypeError: array index must be int, got {:?}", ity
                                ),
                                span: index.span(),
                            });
                        }
                        Ok(*inner)
                    }
                    Ty::HashMap(v) => {
                        if !ity.isAssignableTo(&Ty::String) {
                            return Err(TypeError {
                                message: format!(
                                    "TypeError: dict index must be string, got {:?}", ity
                                ),
                                span: index.span(),
                            });
                        }
                        Ok(*v)
                    }
                    Ty::String => {
                        if !ity.isAssignableTo(&Ty::Int) {
                            return Err(TypeError {
                                message: format!(
                                    "TypeError: string index must be int, got {:?}", ity
                                ),
                                span: index.span(),
                            });
                        }
                        Ok(Ty::Char)
                    }
                    Ty::Any => Ok(Ty::Any),
                    _ => Err(TypeError {
                        message: "TypeError: indexing only supported for arrays, dicts, and strings"
                            .to_string(),
                        span: target.span(),
                    }),
                }
            }

            Expr::Get { object, name, span } => {
                let tty = self.exprTy(object)?;
                match tty {
                    Ty::HashMap(v) => Ok(*v),
                    Ty::Namespace(entries) => entries
                        .get(name)
                        .cloned()
                        .map(|sig| Ty::Function(Box::new(sig)))
                        .ok_or_else(|| TypeError {
                            message: format!("TypeError: namespace has no function '{}'", name),
                            span: *span,
                    }),
                    Ty::Struct(structName) => {
                        let def = self.structs.get(&structName).cloned().ok_or_else(|| TypeError {
                            message: format!("TypeError: unknown struct '{}'", self.nominalLabel(&structName)),
                            span: *span,
                        })?;
                        let field = def.fields.get(name).ok_or_else(|| TypeError {
                            message: format!("TypeError: struct '{}' has no field '{}'", self.nominalLabel(&structName), name),
                            span: *span,
                        })?;
                        if !field.isPub && !self.canAccessPrivateField(&structName) {
                            return Err(TypeError {
                                message: format!("TypeError: cannot access private field '{}' on struct '{}'", name, self.nominalLabel(&structName)),
                                span: *span,
                            });
                        }
                        Ok(field.ty.clone())
                    }
                    Ty::Tuple(elements) => {
                        let index = name.parse::<usize>().map_err(|_| TypeError {
                            message: "TypeError: tuple access must use a numeric index".to_string(),
                            span: *span,
                        })?;
                        elements.get(index).cloned().ok_or_else(|| TypeError {
                            message: format!("TypeError: tuple index {} out of bounds for tuple of length {}", index, elements.len()),
                            span: *span,
                        })
                    }
                    Ty::Enum(enumName) => match self.enums.get(&enumName)
                        .and_then(|def| def.payloads.get(name))
                    {
                        Some(payloads) if payloads.is_empty() => Ok(Ty::Enum(enumName)),
                        Some(_) => Err(TypeError {
                            message: format!("TypeError: enum variant '{}::{}' requires payload arguments", self.nominalLabel(&enumName), name),
                            span: *span,
                        }),
                        None => Err(TypeError {
                            message: format!("TypeError: enum '{}' has no variant '{}'", self.nominalLabel(&enumName), name),
                            span: *span,
                        }),
                    },
                    Ty::Any => Ok(Ty::Any),
                    Ty::Nullable(_) | Ty::Union(_) => Err(TypeError {
                        message: format!("TypeError: value of type {:?} must be narrowed before field access", tty),
                        span: *span,
                    }),
                    _ => Err(TypeError {
                        message: format!("TypeError: field access is not supported on {:?}", tty),
                        span: *span,
                    }),
                }
            }

            Expr::MethodCall { receiver, name, args, span } => {
                let rty = self.exprTy(receiver)?;
                self.methodTy(&rty, name, args, *span)
            }

            Expr::StructLiteral { name, fields, span } => {
                let structId = self.nominalKey(name);
                let def = self.structs.get(&structId).cloned().ok_or_else(|| TypeError {
                    message: format!("TypeError: unknown struct '{}'", name),
                    span: *span,
                })?;
                for (fieldName, value) in fields {
                    let expected = def.fields.get(fieldName).ok_or_else(|| TypeError {
                        message: format!("TypeError: struct '{}' has no field '{}'", name, fieldName),
                        span: value.span(),
                    })?;
                    let actual = self.exprTyExpected(value, &expected.ty)?;
                    if !actual.isAssignableTo(&expected.ty) {
                        return Err(TypeError {
                            message: format!("TypeError: field '{}' expected {:?} but got {:?}", fieldName, expected.ty, actual),
                            span: value.span(),
                        });
                    }
                }
                Ok(Ty::Struct(structId))
            }
            Expr::EnumVariant { enum_name, variant, args, span } => {
                if let Some(owner) = enum_name.as_deref() {
                    match (owner, variant.as_str()) {
                        ("Option", "Some") if args.len() == 1 => {
                            return Ok(Ty::Option(Box::new(self.exprTy(&args[0])?)));
                        }
                        ("Option", "Some") => return Err(TypeError {
                            message: "TypeError: Option::Some expects 1 payload value".to_string(),
                            span: *span,
                        }),
                        ("Option", "None") if args.is_empty() => {
                            return Ok(Ty::Option(Box::new(Ty::Any)));
                        }
                        ("Option", "None") => return Err(TypeError {
                            message: "TypeError: Option::None expects no payload values".to_string(),
                            span: *span,
                        }),
                        ("Result", "Ok") if args.len() == 1 => {
                            return Ok(Ty::Result(Box::new(self.exprTy(&args[0])?), Box::new(Ty::Any)));
                        }
                        ("Result", "Ok") => return Err(TypeError {
                            message: "TypeError: Result::Ok expects 1 payload value".to_string(),
                            span: *span,
                        }),
                        ("Result", "Err") if args.len() == 1 => {
                            let payloadTy = self.exprTy(&args[0])?;
                            return Ok(Ty::Result(Box::new(Ty::Any), Box::new(payloadTy)));
                        }
                        ("Result", "Err") => return Err(TypeError {
                            message: "TypeError: Result::Err expects 1 payload value".to_string(),
                            span: *span,
                        }),
                        _ => {}
                    }
                }
                let owner = match enum_name.as_deref() {
                    Some(name) => self.nominalKey(name),
                    None => self.enum_variants.get(variant).map(|(name, _)| name.clone()).ok_or_else(|| TypeError {
                        message: format!("TypeError: unknown enum variant '{}'", variant),
                        span: *span,
                    })?,
                };
                let payloadTypes = self.enums.get(&owner)
                    .filter(|def| def.variants.contains(variant))
                    .and_then(|def| def.payloads.get(variant)).cloned().ok_or_else(|| TypeError {
                        message: format!("TypeError: unknown variant '{}::{}'", self.nominalLabel(&owner), variant),
                        span: *span,
                    })?;
                if args.len() != payloadTypes.len() {
                    return Err(TypeError {
                        message: format!("TypeError: variant '{}::{}' expects {} payload values but got {}", self.nominalLabel(&owner), variant, payloadTypes.len(), args.len()),
                        span: *span,
                    });
                }
                for (arg, expected) in args.iter().zip(&payloadTypes) {
                    let actual = self.exprTyExpected(arg, expected)?;
                    if !actual.isAssignableTo(expected) {
                        return Err(TypeError {
                            message: format!("TypeError: variant '{}::{}' payload expected {:?} but got {:?}", self.nominalLabel(&owner), variant, expected, actual),
                            span: arg.span(),
                        });
                    }
                }
                Ok(Ty::Enum(owner))
            }
            Expr::StaticCall { struct_name, method, args, span } => {
                if struct_name == "Vec" && method == "new" {
                    if args.is_empty() { return Ok(Ty::Vec(Box::new(Ty::Any))); }
                    return Err(TypeError {
                        message: "TypeError: Vec.new() expects 0 arguments".to_string(),
                        span: *span,
                    });
                }
                if struct_name == "HashMap" && method == "new" {
                    if args.is_empty() { return Ok(Ty::HashMap(Box::new(Ty::Any))); }
                    return Err(TypeError {
                        message: "TypeError: HashMap.new() expects 0 arguments".to_string(),
                        span: *span,
                    });
                }
                if struct_name == "Result" {
                    match method.as_str() {
                        "Ok" | "Err" if args.len() == 1 => {
                            let payloadTy = self.exprTy(&args[0])?;
                            return Ok(if method == "Ok" {
                                Ty::Result(Box::new(payloadTy), Box::new(Ty::Any))
                            } else {
                                Ty::Result(Box::new(Ty::Any), Box::new(payloadTy))
                            });
                        }
                        "Ok" | "Err" => return Err(TypeError {
                            message: format!("TypeError: Result.{}() expects 1 argument", method),
                            span: *span,
                        }),
                        _ => {}
                    }
                }
                if struct_name == "Option" {
                    match method.as_str() {
                        "Some" if args.len() == 1 => return Ok(Ty::Option(Box::new(self.exprTy(&args[0])?))),
                        "Some" => return Err(TypeError {
                            message: "TypeError: Option.Some() expects 1 argument".to_string(),
                            span: *span,
                        }),
                        "None" if args.is_empty() => return Ok(Ty::Option(Box::new(Ty::Any))),
                        "None" => return Err(TypeError {
                            message: "TypeError: Option.None() expects 0 arguments".to_string(),
                            span: *span,
                        }),
                        _ => {}
                    }
                }
                if method == "create" && self.createStaticTarget.as_deref() == Some(struct_name) {
                    return self.checkCallByName("create", args, *span);
                }
                if method == "create" && self.createFallbackAllowed
                    && self.visibleFunction("create").is_some()
                {
                    return self.checkCallByName("create", args, *span);
                }
                let structId = self.nominalKey(struct_name);
                let sig = self.structs.get(&structId)
                    .and_then(|def| def.staticMethods.get(method)).cloned()
                    .ok_or_else(|| TypeError {
                        message: format!("TypeError: unknown static method '{}.{}'", struct_name, method),
                        span: *span,
                    })?;
                if sig.visibility == Some(FunctionVisibility::Private)
                    && self.currentStruct.as_deref() != Some(structId.as_str())
                {
                    return Err(TypeError {
                        message: format!("TypeError: static method '{}.{}' is private", struct_name, method),
                        span: *span,
                    });
                }
                self.checkCallWithSig(&format!("{}.{}", struct_name, method), &sig, args, *span)
            }
            Expr::Set { object, name, value, span } => {
                self.checkCapturedMutation(object, *span)?;
                let objectTy = self.exprTy(object)?;
                match objectTy {
                    Ty::Struct(structName) => {
                        let def = self.structs.get(&structName).cloned().ok_or_else(|| TypeError {
                            message: format!("TypeError: unknown struct '{}'", self.nominalLabel(&structName)),
                            span: *span,
                        })?;
                        let field = def.fields.get(name).ok_or_else(|| TypeError {
                            message: format!("TypeError: struct '{}' has no field '{}'", self.nominalLabel(&structName), name),
                            span: *span,
                        })?;
                        if !field.isPub && !self.canAccessPrivateField(&structName) {
                            return Err(TypeError {
                                message: format!("TypeError: cannot mutate private field '{}' on struct '{}'", name, self.nominalLabel(&structName)),
                                span: *span,
                            });
                        }
                        let valueTy = self.exprTyExpected(value, &field.ty)?;
                        if !valueTy.isAssignableTo(&field.ty) {
                            return Err(TypeError {
                                message: format!("TypeError: field '{}' expected {:?} but got {:?}", name, field.ty, valueTy),
                                span: value.span(),
                            });
                        }
                        Ok(valueTy)
                    }
                    Ty::HashMap(inner) => {
                        let valueTy = self.exprTyExpected(value, &inner)?;
                        if !valueTy.isAssignableTo(&inner) {
                            return Err(TypeError {
                                message: format!("TypeError: dictionary value expected {:?} but got {:?}", inner, valueTy),
                                span: value.span(),
                            });
                        }
                        Ok(valueTy)
                    }
                    Ty::Any => self.exprTy(value),
                    other => Err(TypeError {
                        message: format!("TypeError: field mutation is not supported on {:?}", other),
                        span: *span,
                    }),
                }
            }
            Expr::IndexSet { target, index, value, span } => {
                self.checkCapturedMutation(target, *span)?;
                let targetTy = self.exprTy(target)?;
                let indexTy = self.exprTy(index)?;
                match targetTy {
                    Ty::Vec(inner) => {
                        if indexTy != Ty::Int {
                            return Err(TypeError { message: format!("TypeError: array index must be int, got {:?}", indexTy), span: index.span() });
                        }
                        let valueTy = self.exprTyExpected(value, &inner)?;
                        if !valueTy.isAssignableTo(&inner) {
                            return Err(TypeError { message: format!("TypeError: array element expected {:?} but got {:?}", inner, valueTy), span: value.span() });
                        }
                        Ok(valueTy)
                    }
                    Ty::HashMap(inner) => {
                        if indexTy != Ty::String {
                            return Err(TypeError { message: format!("TypeError: dictionary index must be string, got {:?}", indexTy), span: index.span() });
                        }
                        let valueTy = self.exprTyExpected(value, &inner)?;
                        if !valueTy.isAssignableTo(&inner) {
                            return Err(TypeError { message: format!("TypeError: dictionary value expected {:?} but got {:?}", inner, valueTy), span: value.span() });
                        }
                        Ok(valueTy)
                    }
                    Ty::Any => self.exprTy(value),
                    other => Err(TypeError {
                        message: format!("TypeError: indexed assignment is not supported on {:?}", other),
                        span: *span,
                    }),
                }
            }
        }
    }

    fn exprTyExpected(&mut self, expr: &Expr, expected: &Ty) -> Result<Ty, TypeError> {
        if let (Expr::Lambda { params, body, .. }, Ty::Function(signature)) = (expr, expected) {
            return self.lambdaTy(params, body, Some(signature));
        }
        if let (Expr::EnumVariant { enum_name: Some(owner), variant, args, .. }, Ty::Option(inner)) = (expr, expected) {
            if owner == "Option" && variant == "Some" && args.len() == 1 {
                let payload = self.exprTyExpected(&args[0], inner)?;
                return Ok(Ty::Option(Box::new(payload)));
            }
            if owner == "Option" && variant == "None" && args.is_empty() {
                return Ok(expected.clone());
            }
        }
        if let (Expr::StaticCall { struct_name, method, args, .. }, Ty::Option(inner)) = (expr, expected) {
            if struct_name == "Option" && method == "Some" && args.len() == 1 {
                let payload = self.exprTyExpected(&args[0], inner)?;
                return Ok(Ty::Option(Box::new(payload)));
            }
            if struct_name == "Option" && method == "None" && args.is_empty() {
                return Ok(expected.clone());
            }
        }
        if let (Expr::EnumVariant { enum_name: Some(owner), variant, args, .. }, Ty::Result(ok, err)) = (expr, expected) {
            if owner == "Result" && variant == "Ok" && args.len() == 1 {
                let payload = self.exprTyExpected(&args[0], ok)?;
                return Ok(Ty::Result(Box::new(payload), err.clone()));
            }
            if owner == "Result" && variant == "Err" && args.len() == 1 {
                let payload = self.exprTyExpected(&args[0], err)?;
                return Ok(Ty::Result(ok.clone(), Box::new(payload)));
            }
        }
        if let (Expr::StaticCall { struct_name, method, args, .. }, Ty::Result(ok, err)) = (expr, expected) {
            if struct_name == "Result" && method == "Ok" && args.len() == 1 {
                let payload = self.exprTyExpected(&args[0], ok)?;
                return Ok(Ty::Result(Box::new(payload), err.clone()));
            }
            if struct_name == "Result" && method == "Err" && args.len() == 1 {
                let payload = self.exprTyExpected(&args[0], err)?;
                return Ok(Ty::Result(ok.clone(), Box::new(payload)));
            }
        }
        if let (Expr::TupleLiteral { elements, .. }, Ty::Tuple(expectedElements)) = (expr, expected) {
            if elements.len() != expectedElements.len() {
                return Err(TypeError {
                    message: format!("TypeError: tuple literal expects {} elements but type has {}", elements.len(), expectedElements.len()),
                    span: expr.span(),
                });
            }
            for (element, expectedElement) in elements.iter().zip(expectedElements) {
                let actual = self.exprTyExpected(element, expectedElement)?;
                if !actual.isAssignableTo(expectedElement) {
                    return Err(TypeError {
                        message: format!("TypeError: tuple element expected {:?} but got {:?}", expectedElement, actual),
                        span: element.span(),
                    });
                }
            }
            return Ok(expected.clone());
        }
        if let (Expr::ArrayLiteral { elements, .. }, Ty::Vec(inner)) = (expr, expected) {
            for element in elements {
                let actual = self.exprTyExpected(element, inner)?;
                if (actual == Ty::Any && **inner != Ty::Any) || !actual.isAssignableTo(inner) {
                    return Err(TypeError {
                        message: format!("TypeError: array element expected {:?} but got {:?}", inner, actual),
                        span: element.span(),
                    });
                }
            }
            return Ok(expected.clone());
        }
        if let (Expr::DictLiteral { entries, .. }, Ty::HashMap(valueType)) = (expr, expected) {
            for (_, value) in entries {
                let actual = self.exprTyExpected(value, valueType)?;
                if (actual == Ty::Any && **valueType != Ty::Any) || !actual.isAssignableTo(valueType) {
                    return Err(TypeError {
                        message: format!("TypeError: dictionary value expected {:?} but got {:?}", valueType, actual),
                        span: value.span(),
                    });
                }
            }
            return Ok(expected.clone());
        }
        if let (Expr::DictByteLiteral { entries, .. }, Ty::HashMap(valueType)) = (expr, expected) {
            for (_, value) in entries {
                let actual = self.exprTyExpected(value, valueType)?;
                if (actual == Ty::Any && **valueType != Ty::Any) || !actual.isAssignableTo(valueType) {
                    return Err(TypeError {
                        message: format!("TypeError: dictionary value expected {:?} but got {:?}", valueType, actual),
                        span: value.span(),
                    });
                }
            }
            return Ok(expected.clone());
        }
        if let (Expr::StaticCall { struct_name, method, args, .. }, Ty::Vec(_)) = (expr, expected) {
            if struct_name == "Vec" && method == "new" && args.is_empty() {
                return Ok(expected.clone());
            }
        }
        if let (Expr::StaticCall { struct_name, method, args, .. }, Ty::HashMap(_)) = (expr, expected) {
            if struct_name == "HashMap" && method == "new" && args.is_empty() {
                return Ok(expected.clone());
            }
        }
        self.exprTy(expr)
    }

    fn lambdaTy(
        &mut self,
        params: &[Parameter],
        body: &Expr,
        expected: Option<&FuncSig>,
    ) -> Result<Ty, TypeError> {
        if let Some(signature) = expected {
            if params.len() != signature.params.len() + usize::from(signature.variadic.is_some()) {
                return Err(TypeError {
                    message: format!("TypeError: lambda expects {} parameters but function type has {}", params.len(), signature.params.len()),
                    span: body.span(),
                });
            }
        }
        let captureDepth = self.scopes.len();
        self.lambdaCaptureDepths.push(captureDepth);
        self.pushScope();
        let mut parameterTypes = Vec::new();
        let mut minArgs = 0;
        let mut variadic = None;
        let mut fixedIndex = 0usize;
        for param in params {
            let expectedParam = expected.and_then(|sig| {
                if param.variadic { sig.variadic.as_deref() }
                else { sig.params.get(fixedIndex) }
            });
            let paramTy = if let Some(annotation) = &param.ty {
                self.annotationTy(annotation, Span::new(0, 0))?
            } else {
                expectedParam.cloned().unwrap_or(Ty::Any)
            };
            if let Some(expectedParam) = expectedParam {
                if !paramTy.isAssignableTo(expectedParam) || !expectedParam.isAssignableTo(&paramTy) {
                    return Err(TypeError {
                        message: format!("TypeError: lambda parameter expected {:?} but got {:?}", expectedParam, paramTy),
                        span: body.span(),
                    });
                }
            }
            if param.variadic {
                variadic = Some(Box::new(match paramTy.clone() {
                    Ty::Vec(inner) => *inner,
                    other => other,
                }));
            } else {
                if param.default.is_none() { minArgs += 1; }
                parameterTypes.push(paramTy.clone());
                fixedIndex += 1;
            }
            self.define(&param.name, paramTy, false);
        }
        let ret = self.exprTy(body)?;
        self.popScope();
        self.lambdaCaptureDepths.pop();
        if let Some(signature) = expected {
            if !ret.isAssignableTo(&signature.ret) {
                return Err(TypeError {
                    message: format!("TypeError: lambda return expected {:?} but got {:?}", signature.ret, ret),
                    span: body.span(),
                });
            }
            return Ok(Ty::Function(Box::new(signature.clone())));
        }
        Ok(Ty::Function(Box::new(FuncSig {
            params: parameterTypes,
            minArgs,
            variadic,
            ret: Box::new(ret),
            inferReturn: false,
            returnResolved: true,
            visibility: None,
        })))
    }

    fn literalTy(literal: &Literal) -> Ty {
        match literal {
            Literal::String(_) | Literal::ByteString(_) => Ty::String,
            Literal::Char(_) => Ty::Char,
            Literal::Int(_) => Ty::Int,
            Literal::Float(_) => Ty::Float,
            Literal::Bool(_) => Ty::Bool,
            Literal::Null => Ty::Null,
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
            "length" => return Err(TypeError {
                message: "TypeError: '.length()' was removed; use '.len()'".to_string(),
                span: callSpan,
            }),
            "upper" => return Err(TypeError {
                message: "TypeError: '.upper()' was removed; use '.toUpper()'".to_string(),
                span: callSpan,
            }),
            "lower" => return Err(TypeError {
                message: "TypeError: '.lower()' was removed; use '.toLower()'".to_string(),
                span: callSpan,
            }),
            _ => {}
        }

        if let Ty::Struct(structName) = receiver {
            if let Some(signature) = self.structs.get(structName)
                .and_then(|def| def.methods.get(name)).cloned()
            {
                if signature.visibility == Some(FunctionVisibility::Private)
                    && !self.canCallPrivateMethod(structName)
                {
                    return Err(TypeError {
                        message: format!("TypeError: method '{}.{}' is private", structName, name),
                        span: callSpan,
                    });
                }
                return self.checkCallWithSig(name, &signature, args, callSpan);
            }
        }

        if let Ty::Namespace(entries) = receiver {
            let signature = entries.get(name).ok_or_else(|| TypeError {
                message: format!("TypeError: namespace has no function '{}'", name),
                span: callSpan,
            })?;
            return self.checkCallWithSig(name, signature, args, callSpan);
        }

        // Any receiver — escape hatch
        if matches!(receiver, Ty::Any) {
            for a in args { self.exprTy(a)?; }
            return Ok(Ty::Any);
        }

        // universal conversion methods
        match name {
            "toString" => {
                if !args.is_empty() {
                    return Err(TypeError {
                        message: format!("TypeError: {}() expects 0 arguments", name),
                        span: callSpan,
                    });
                }
                return Ok(Ty::String);
            }
            "toInt" => {
                if !matches!(receiver, Ty::String | Ty::Int | Ty::Float) {
                    return Err(TypeError {
                        message: format!("TypeError: toInt() is not supported on {:?}", receiver),
                        span: callSpan,
                    });
                }
                if !args.is_empty() {
                    return Err(TypeError {
                        message: "TypeError: toInt() expects 0 arguments".to_string(),
                        span: callSpan,
                    });
                }
                return Ok(Ty::Int);
            }
            "toFloat" => {
                if !matches!(receiver, Ty::String | Ty::Int | Ty::Float) {
                    return Err(TypeError {
                        message: format!("TypeError: toFloat() is not supported on {:?}", receiver),
                        span: callSpan,
                    });
                }
                if !args.is_empty() {
                    return Err(TypeError {
                        message: "TypeError: toFloat() expects 0 arguments".to_string(),
                        span: callSpan,
                    });
                }
                return Ok(Ty::Float);
            }
            _ => {}
        }

        match (receiver, name) {
            // Vec methods
            (Ty::Vec(_), "len") => {
                if !args.is_empty() {
                    return Err(TypeError {
                        message: format!("TypeError: Vec.{}() expects 0 arguments", name),
                        span: callSpan,
                    });
                }
                Ok(Ty::Int)
            }
            (Ty::Vec(inner), "push") => {
                if args.len() != 1 {
                    return Err(TypeError {
                        message: "TypeError: Vec.push() expects 1 argument".to_string(),
                        span: callSpan,
                    });
                }
                let a0 = self.exprTyExpected(&args[0], inner)?;
                if !a0.isAssignableTo(inner.as_ref()) {
                    return Err(TypeError {
                        message: format!(
                            "TypeError: Vec.push() expected element {:?} but got {:?}",
                            inner, a0
                        ),
                        span: args[0].span(),
                    });
                }
                Ok(Ty::Void)
            }
            (Ty::Vec(inner), "pop") => {
                if !args.is_empty() {
                    return Err(TypeError {
                        message: "TypeError: Vec.pop() expects 0 arguments".to_string(),
                        span: callSpan,
                    });
                }
                Ok(*inner.clone())
            }
            (Ty::Vec(inner), "get") => {
                if args.len() != 1 {
                    return Err(TypeError {
                        message: "TypeError: Vec.get() expects 1 argument".to_string(),
                        span: callSpan,
                    });
                }
                let index = self.exprTy(&args[0])?;
                if !index.isAssignableTo(&Ty::Int) {
                    return Err(TypeError {
                        message: format!("TypeError: Vec.get() index must be int, got {:?}", index),
                        span: args[0].span(),
                    });
                }
                Ok(*inner.clone())
            }
            (Ty::Vec(inner), "set") => {
                if args.len() != 2 {
                    return Err(TypeError {
                        message: "TypeError: Vec.set() expects 2 arguments".to_string(),
                        span: callSpan,
                    });
                }
                let index = self.exprTy(&args[0])?;
                if !index.isAssignableTo(&Ty::Int) {
                    return Err(TypeError {
                        message: format!("TypeError: Vec.set() index must be int, got {:?}", index),
                        span: args[0].span(),
                    });
                }
                let value = self.exprTyExpected(&args[1], inner)?;
                if !value.isAssignableTo(inner) {
                    return Err(TypeError {
                        message: format!("TypeError: Vec.set() value expected {:?} but got {:?}", inner, value),
                        span: args[1].span(),
                    });
                }
                Ok(Ty::Void)
            }
            (Ty::Vec(inner), "contains") | (Ty::Vec(inner), "indexOf") => {
                if args.len() != 1 {
                    return Err(TypeError {
                        message: format!("TypeError: Vec.{}() expects 1 argument", name),
                        span: callSpan,
                    });
                }
                let value = self.exprTyExpected(&args[0], inner)?;
                if !value.isAssignableTo(inner) {
                    return Err(TypeError {
                        message: format!("TypeError: Vec.{}() expected element {:?} but got {:?}", name, inner, value),
                        span: args[0].span(),
                    });
                }
                Ok(if name == "contains" { Ty::Bool } else { Ty::Int })
            }
            (Ty::Vec(_), "join") => {
                if args.len() != 1 {
                    return Err(TypeError {
                        message: "TypeError: Vec.join() expects 1 argument".to_string(),
                        span: callSpan,
                    });
                }
                let separator = self.exprTy(&args[0])?;
                if !separator.isAssignableTo(&Ty::String) {
                    return Err(TypeError {
                        message: format!("TypeError: Vec.join() separator must be string, got {:?}", separator),
                        span: args[0].span(),
                    });
                }
                Ok(Ty::String)
            }
            (Ty::Vec(inner), "slice") => {
                if args.len() != 2 {
                    return Err(TypeError {
                        message: "TypeError: Vec.slice() expects 2 arguments".to_string(),
                        span: callSpan,
                    });
                }
                for arg in args {
                    let index = self.exprTy(arg)?;
                    if !index.isAssignableTo(&Ty::Int) {
                        return Err(TypeError {
                            message: format!("TypeError: Vec.slice() indices must be int, got {:?}", index),
                            span: arg.span(),
                        });
                    }
                }
                Ok(Ty::Vec(inner.clone()))
            }
            (Ty::Vec(_), "reverse") => {
                if !args.is_empty() {
                    return Err(TypeError {
                        message: "TypeError: Vec.reverse() expects 0 arguments".to_string(),
                        span: callSpan,
                    });
                }
                Ok(Ty::Void)
            }
            (Ty::Vec(_), "sort") => {
                if !args.is_empty() {
                    return Err(TypeError {
                        message: "TypeError: Vec.sort() expects 0 arguments".to_string(),
                        span: callSpan,
                    });
                }
                Ok(Ty::Void)
            }
            (Ty::Vec(inner), "map") => {
                if args.len() != 1 {
                    return Err(TypeError {
                        message: "TypeError: Vec.map() expects 1 argument".to_string(),
                        span: callSpan,
                    });
                }
                let callback = self.exprTy(&args[0])?;
                match callback {
                    Ty::Function(signature)
                        if signature.params.len() == 1
                            && signature.variadic.is_none()
                            && signature.params[0].isAssignableTo(inner)
                            && inner.isAssignableTo(&signature.params[0]) =>
                    {
                        Ok(Ty::Vec(signature.ret.clone()))
                    }
                    Ty::Any => Ok(Ty::Vec(Box::new(Ty::Any))),
                    other => Err(TypeError {
                        message: format!("TypeError: Vec.map() requires a function accepting {:?}, got {:?}", inner, other),
                        span: args[0].span(),
                    }),
                }
            }
            (Ty::Vec(inner), "filter") => {
                if args.len() != 1 {
                    return Err(TypeError {
                        message: "TypeError: Vec.filter() expects 1 argument".to_string(),
                        span: callSpan,
                    });
                }
                let callback = self.exprTy(&args[0])?;
                match callback {
                    Ty::Function(signature)
                        if signature.params.len() == 1
                            && signature.variadic.is_none()
                            && signature.params[0].isAssignableTo(inner)
                            && inner.isAssignableTo(&signature.params[0])
                            && signature.ret.isAssignableTo(&Ty::Bool) =>
                    {
                        Ok(Ty::Vec(inner.clone()))
                    }
                    Ty::Any => Ok(Ty::Vec(inner.clone())),
                    other => Err(TypeError {
                        message: format!("TypeError: Vec.filter() requires a function from {:?} to bool, got {:?}", inner, other),
                        span: args[0].span(),
                    }),
                }
            }
            (Ty::Vec(inner), "reduce") => {
                if args.len() != 2 {
                    return Err(TypeError {
                        message: "TypeError: Vec.reduce() expects 2 arguments".to_string(),
                        span: callSpan,
                    });
                }
                let initial = self.exprTy(&args[1])?;
                let callback = self.exprTy(&args[0])?;
                match callback {
                    Ty::Function(signature)
                        if signature.params.len() == 2
                            && signature.variadic.is_none()
                            && signature.params[0].isAssignableTo(&initial)
                            && initial.isAssignableTo(&signature.params[0])
                            && signature.params[1].isAssignableTo(inner)
                            && inner.isAssignableTo(&signature.params[1])
                            && signature.ret.isAssignableTo(&initial) =>
                    {
                        Ok(initial)
                    }
                    Ty::Any => Ok(initial),
                    other => Err(TypeError {
                        message: format!("TypeError: Vec.reduce() callback must accept ({:?}, {:?}) and return {:?}, got {:?}", initial, inner, initial, other),
                        span: args[0].span(),
                    }),
                }
            }

            (Ty::Option(inner), "unwrap") => {
                if !args.is_empty() {
                    return Err(TypeError { message: "TypeError: Option.unwrap() expects 0 arguments".to_string(), span: callSpan });
                }
                Ok(*inner.clone())
            }
            (Ty::Option(inner), "unwrapOr") => {
                if args.len() != 1 {
                    return Err(TypeError { message: "TypeError: Option.unwrapOr() expects 1 argument".to_string(), span: callSpan });
                }
                let actual = self.exprTyExpected(&args[0], inner)?;
                if !actual.isAssignableTo(inner) {
                    return Err(TypeError { message: format!("TypeError: Option.unwrapOr() expected {:?} but got {:?}", inner, actual), span: args[0].span() });
                }
                Ok(*inner.clone())
            }
            (Ty::Option(_), "isSome") | (Ty::Option(_), "isNone") => {
                if !args.is_empty() {
                    return Err(TypeError { message: format!("TypeError: Option.{}() expects 0 arguments", name), span: callSpan });
                }
                Ok(Ty::Bool)
            }
            (Ty::Result(ok, _), "unwrap") => {
                if !args.is_empty() {
                    return Err(TypeError { message: "TypeError: Result.unwrap() expects 0 arguments".to_string(), span: callSpan });
                }
                Ok(*ok.clone())
            }
            (Ty::Result(ok, _), "unwrapOr") => {
                if args.len() != 1 {
                    return Err(TypeError { message: "TypeError: Result.unwrapOr() expects 1 argument".to_string(), span: callSpan });
                }
                let actual = self.exprTyExpected(&args[0], ok)?;
                if !actual.isAssignableTo(ok) {
                    return Err(TypeError { message: format!("TypeError: Result.unwrapOr() expected {:?} but got {:?}", ok, actual), span: args[0].span() });
                }
                Ok(*ok.clone())
            }
            (Ty::Result(_, _), "isOk") | (Ty::Result(_, _), "isErr") => {
                if !args.is_empty() {
                    return Err(TypeError { message: format!("TypeError: Result.{}() expects 0 arguments", name), span: callSpan });
                }
                Ok(Ty::Bool)
            }

            // String methods
            (Ty::String, "hash") => {
                if !args.is_empty() {
                    return Err(TypeError {
                        message: format!("TypeError: String.hash() expects 0 arguments"),
                        span: callSpan,
                    });
                }
                Ok(Ty::Int)
            }
            (Ty::String, "len") => {
                if !args.is_empty() {
                    return Err(TypeError {
                        message: format!("TypeError: String.{}() expects 0 arguments", name),
                        span: callSpan,
                    });
                }
                Ok(Ty::Int)
            }
            (Ty::String, "toUpper") | (Ty::String, "toLower") | (Ty::String, "trim") => {
                if !args.is_empty() {
                    return Err(TypeError {
                        message: format!("TypeError: String.{}() expects 0 arguments", name),
                        span: callSpan,
                    });
                }
                Ok(Ty::String)
            }
            (Ty::String, "startsWith") | (Ty::String, "endsWith") | (Ty::String, "contains") => {
                if args.len() != 1 {
                    return Err(TypeError {
                        message: format!("TypeError: String.{}() expects 1 argument", name),
                        span: callSpan,
                    });
                }
                let a0 = self.exprTy(&args[0])?;
                if !a0.isAssignableTo(&Ty::String) {
                    return Err(TypeError {
                        message: format!("TypeError: String.{}() expects a string argument", name),
                        span: args[0].span(),
                    });
                }
                Ok(Ty::Bool)
            }
            (Ty::String, "indexOf") => {
                if args.len() != 1 {
                    return Err(TypeError {
                        message: "TypeError: String.indexOf() expects 1 argument".to_string(),
                        span: callSpan,
                    });
                }
                self.exprTy(&args[0])?;
                Ok(Ty::Int)
            }
            (Ty::String, "replace") => {
                if args.len() != 2 {
                    return Err(TypeError {
                        message: "TypeError: String.replace() expects 2 arguments".to_string(),
                        span: callSpan,
                    });
                }
                self.exprTy(&args[0])?;
                self.exprTy(&args[1])?;
                Ok(Ty::String)
            }
            (Ty::String, "slice") => {
                if args.len() != 2 {
                    return Err(TypeError {
                        message: "TypeError: String.slice() expects 2 arguments".to_string(),
                        span: callSpan,
                    });
                }
                let a0 = self.exprTy(&args[0])?;
                let a1 = self.exprTy(&args[1])?;
                if !a0.isAssignableTo(&Ty::Int) || !a1.isAssignableTo(&Ty::Int) {
                    return Err(TypeError {
                        message: "TypeError: String.slice() expects int indices".to_string(),
                        span: callSpan,
                    });
                }
                Ok(Ty::String)
            }
            (Ty::String, "repeat") => {
                if args.len() != 1 {
                    return Err(TypeError {
                        message: "TypeError: String.repeat() expects 1 argument".to_string(),
                        span: callSpan,
                    });
                }
                let a0 = self.exprTy(&args[0])?;
                if !a0.isAssignableTo(&Ty::Int) {
                    return Err(TypeError {
                        message: "TypeError: String.repeat() expects an int".to_string(),
                        span: args[0].span(),
                    });
                }
                Ok(Ty::String)
            }
            (Ty::String, "padLeft") | (Ty::String, "padRight") => {
                if args.len() != 2 {
                    return Err(TypeError {
                        message: format!("TypeError: String.{}() expects 2 arguments", name),
                        span: callSpan,
                    });
                }
                self.exprTy(&args[0])?;
                self.exprTy(&args[1])?;
                Ok(Ty::String)
            }
            (Ty::String, "split") => {
                if args.len() != 1 {
                    return Err(TypeError {
                        message: "TypeError: String.split() expects 1 argument".to_string(),
                        span: callSpan,
                    });
                }
                let a0 = self.exprTy(&args[0])?;
                if !a0.isAssignableTo(&Ty::String) {
                    return Err(TypeError {
                        message: "TypeError: String.split() expects a string delimiter".to_string(),
                        span: args[0].span(),
                    });
                }
                Ok(Ty::Vec(Box::new(Ty::String)))
            }
            (Ty::Result(_, errorTy), "unwrapErr") => {
                if !args.is_empty() {
                    return Err(TypeError {
                        message: "TypeError: Result.unwrapErr() expects 0 arguments".to_string(),
                        span: callSpan,
                    });
                }
                Ok(*errorTy.clone())
            }

            (Ty::HashMap(inner), "set") => {
                if args.len() != 2 {
                    return Err(TypeError { message: "TypeError: HashMap.set() expects 2 arguments".to_string(), span: callSpan });
                }
                let key = self.exprTy(&args[0])?;
                if key != Ty::String {
                    return Err(TypeError { message: format!("TypeError: HashMap.set() key must be string, got {:?}", key), span: args[0].span() });
                }
                let value = self.exprTyExpected(&args[1], inner)?;
                if !value.isAssignableTo(inner) {
                    return Err(TypeError { message: format!("TypeError: HashMap.set() value expected {:?} but got {:?}", inner, value), span: args[1].span() });
                }
                Ok(Ty::Void)
            }
            (Ty::HashMap(inner), "get") => {
                if args.len() != 1 {
                    return Err(TypeError { message: "TypeError: HashMap.get() expects 1 argument".to_string(), span: callSpan });
                }
                let key = self.exprTy(&args[0])?;
                if key != Ty::String {
                    return Err(TypeError { message: format!("TypeError: HashMap.get() key must be string, got {:?}", key), span: args[0].span() });
                }
                Ok(*inner.clone())
            }
            (Ty::HashMap(_), "has") | (Ty::HashMap(_), "delete") => {
                if args.len() != 1 {
                    return Err(TypeError { message: format!("TypeError: HashMap.{}() expects 1 argument", name), span: callSpan });
                }
                let key = self.exprTy(&args[0])?;
                if key != Ty::String {
                    return Err(TypeError { message: format!("TypeError: HashMap.{}() key must be string, got {:?}", name, key), span: args[0].span() });
                }
                Ok(if name == "has" { Ty::Bool } else { Ty::Void })
            }
            (Ty::HashMap(_), "len") => {
                if !args.is_empty() {
                    return Err(TypeError { message: "TypeError: HashMap.len() expects 0 arguments".to_string(), span: callSpan });
                }
                Ok(Ty::Int)
            }
            (Ty::HashMap(_), "keys") => {
                if !args.is_empty() {
                    return Err(TypeError { message: "TypeError: HashMap.keys() expects 0 arguments".to_string(), span: callSpan });
                }
                Ok(Ty::Vec(Box::new(Ty::String)))
            }
            (Ty::HashMap(inner), "values") => {
                if !args.is_empty() {
                    return Err(TypeError { message: "TypeError: HashMap.values() expects 0 arguments".to_string(), span: callSpan });
                }
                Ok(Ty::Vec(Box::new(*inner.clone())))
            }
            (Ty::HashMap(_), "entries") => {
                if !args.is_empty() {
                    return Err(TypeError { message: "TypeError: HashMap.entries() expects 0 arguments".to_string(), span: callSpan });
                }
                Ok(Ty::Vec(Box::new(Ty::Any)))
            }

            _ => Err(TypeError {
                message: format!(
                    "TypeError: method '{}' not found on {:?}", name, receiver
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
        // println/print accept any number of any args
        if name == "println" || name == "print" {
            for a in args { self.exprTy(a)?; }
            return Ok(Ty::Void);
        }
        if name == "len" {
            return Err(TypeError {
                message: "TypeError: free function 'len(x)' was removed; use 'x.len()'".to_string(),
                span: callSpan,
            });
        }
        if name == "input" {
            if args.len() > 1 {
                return Err(TypeError {
                    message: format!(
                        "TypeError: input expects 0 or 1 arguments, got {}", args.len()
                    ),
                    span: callSpan,
                });
            }
            if args.len() == 1 {
                let t = self.exprTy(&args[0])?;
                if !t.isAssignableTo(&Ty::String) {
                    return Err(TypeError {
                        message: "TypeError: input prompt must be string".to_string(),
                        span: args[0].span(),
                    });
                }
            }
            return Ok(Ty::String);
        }
        if name == "push" {
            if args.len() != 2 {
                return Err(TypeError {
                    message: format!("TypeError: push expects 2 arguments, got {}", args.len()),
                    span: callSpan,
                });
            }
            let arrTy = self.exprTy(&args[0])?;
            return match arrTy {
                Ty::Vec(inner) => {
                    let elTy = self.exprTyExpected(&args[1], &inner)?;
                    if !elTy.isAssignableTo(&inner) {
                        return Err(TypeError {
                            message: format!(
                                "TypeError: push expected element {:?} but got {:?}", inner, elTy
                            ),
                            span: args[1].span(),
                        });
                    }
                    Ok(Ty::Void)
                }
                Ty::Any => {
                    self.exprTy(&args[1])?;
                    Ok(Ty::Void)
                }
                _ => Err(TypeError {
                    message: "TypeError: push expects an array".to_string(),
                    span: args[0].span(),
                }),
            };
        }
        if name == "pop" {
            if args.len() != 1 {
                return Err(TypeError {
                    message: format!("TypeError: pop expects 1 argument, got {}", args.len()),
                    span: callSpan,
                });
            }
            let arrTy = self.exprTy(&args[0])?;
            return match arrTy {
                Ty::Vec(inner) => Ok(*inner),
                Ty::Any => Ok(Ty::Any),
                _ => Err(TypeError {
                    message: "TypeError: pop expects an array".to_string(),
                    span: args[0].span(),
                }),
            };
        }

        let visible = self.visibleFunction(name).cloned();
        if self.collectingReturnType && visible.as_ref().is_some_and(|signature| signature.inferReturn && !signature.returnResolved && *signature.ret == Ty::Any) {
            self.unresolvedReturnCall = true;
        }
        match visible {
            Some(signature) => self.checkCallWithSig(name, &signature, args, callSpan),
            None => Err(TypeError {
                message: format!("TypeError: unknown function '{}'", name),
                span: callSpan,
            }),
        }
    }

    fn checkCallWithSig(
        &mut self,
        name: &str,
        signature: &FuncSig,
        args: &[Expr],
        callSpan: Span,
    ) -> Result<Ty, TypeError> {
        for param in &signature.params {
            self.validateKnownType(param, callSpan)?;
        }
        if let Some(variadic) = &signature.variadic {
            self.validateKnownType(variadic, callSpan)?;
        }
        self.validateKnownType(&signature.ret, callSpan)?;
        let fixedCount = signature.params.len();
        let maxOk = signature.variadic.is_some() || args.len() <= fixedCount;
        if args.len() < signature.minArgs || !maxOk {
            return Err(TypeError {
                message: format!(
                    "TypeError: {} expects {}..={} arguments but got {}",
                    name,
                    signature.minArgs,
                    if signature.variadic.is_some() { usize::MAX } else { fixedCount },
                    args.len()
                ),
                span: callSpan,
            });
        }
        for (param, arg) in signature.params.iter().zip(args.iter().take(fixedCount)) {
            let actual = self.exprTyExpected(arg, param)?;
            if !actual.isAssignableTo(param) {
                return Err(TypeError {
                    message: format!(
                        "TypeError: {} argument expected {:?} but got {:?}", name, param, actual
                    ),
                    span: arg.span(),
                });
            }
        }
        if let Some(varTy) = signature.variadic.as_ref() {
            for arg in args.iter().skip(fixedCount) {
                let actual = self.exprTyExpected(arg, varTy)?;
                if !actual.isAssignableTo(varTy) {
                    return Err(TypeError {
                        message: format!(
                            "TypeError: {} variadic arg expected {:?} but got {:?}", name, varTy, actual
                        ),
                        span: arg.span(),
                    });
                }
            }
        }
        Ok(*signature.ret.clone())
    }

    fn binaryTy(&self, left: &Ty, op: &TokenKind, right: &Ty, span: Span) -> Result<Ty, TypeError> {
        use TokenKind::*;

        fn isNumeric(t: &Ty) -> bool { matches!(t, Ty::Int | Ty::Float) }
        fn isOrdered(t: &Ty) -> bool { matches!(t, Ty::Int | Ty::Float | Ty::String) }
        fn isChar(t: &Ty) -> bool { matches!(t, Ty::Char) }
        fn numericResult(a: &Ty, b: &Ty) -> Ty {
            // int op float → float (numeric promotion)
            if matches!((a, b), (Ty::Float, _) | (_, Ty::Float)) { Ty::Float } else { Ty::Int }
        }

        // The parser represents interpolation as synthetic '+' nodes with no source span.
        if op == &Plus && span.start == 0 && span.end == 0
            && (left == &Ty::String || right == &Ty::String)
        {
            return Ok(Ty::String);
        }

        if matches!(op, AndAnd | OrOr) {
            if left != &Ty::Bool || right != &Ty::Bool {
                return Err(TypeError {
                    message: format!("TypeError: '&&'/'||' requires bool operands, got {:?} and {:?}", left, right),
                    span,
                });
            }
            return Ok(Ty::Bool);
        }

        // Any operand — escape hatch, accept dynamic arithmetic and comparisons
        if matches!(left, Ty::Any) || matches!(right, Ty::Any) {
            return match op {
                EqualEqual | NotEqual | Less | LessEqual | Greater | GreaterEqual
                | AndAnd | OrOr => Ok(Ty::Bool),
                _ => Ok(Ty::Any),
            };
        }

        match op {
            // arithmetic
            Plus => match (left, right) {
                (Ty::String, Ty::String) => Ok(Ty::String),
                (l, r) if isNumeric(l) && isNumeric(r) => Ok(numericResult(l, r)),
                _ => Err(TypeError {
                    message: format!(
                        "TypeError: '+' not defined for {:?} and {:?}", left, right
                    ),
                    span,
                }),
            },
            Minus | Star | Percent => {
                if isNumeric(left) && isNumeric(right) {
                    Ok(numericResult(left, right))
                } else {
                    Err(TypeError {
                        message: format!(
                            "TypeError: arithmetic not defined for {:?} and {:?}", left, right
                        ),
                        span,
                    })
                }
            }
            Slash => {
                if isNumeric(left) && isNumeric(right) {
                    Ok(numericResult(left, right))
                } else {
                    Err(TypeError {
                        message: format!(
                            "TypeError: '/' not defined for {:?} and {:?}", left, right
                        ),
                        span,
                    })
                }
            }

            // equality — valid for any pair of compatible types
            EqualEqual | NotEqual => Ok(Ty::Bool),

            // ordering — requires numeric or String on both sides
            Less | LessEqual | Greater | GreaterEqual => {
                if (isChar(left) && isChar(right)) || (isOrdered(left) && isOrdered(right)) {
                    Ok(Ty::Bool)
                } else {
                    Err(TypeError {
                        message: format!(
                            "TypeError: ordering operator not defined for {:?} and {:?}",
                            left, right
                        ),
                        span,
                    })
                }
            }

            _ => Ok(Ty::Any),
        }
    }

    fn unaryTy(&self, op: &TokenKind, right: &Ty, span: Span) -> Result<Ty, TypeError> {
        if op == &TokenKind::Not && right != &Ty::Bool {
            return Err(TypeError {
                message: format!("TypeError: '!' requires bool, got {:?}", right),
                span,
            });
        }
        if matches!(right, Ty::Any) {
            return if op == &TokenKind::Not { Ok(Ty::Bool) } else { Ok(Ty::Any) };
        }
        match op {
            TokenKind::Minus => match right {
                Ty::Int | Ty::Float => Ok(right.clone()),
                _ => Err(TypeError {
                    message: "TypeError: unary '-' expects numeric".to_string(),
                    span,
                }),
            },
            TokenKind::Not => Ok(Ty::Bool),
            _ => Ok(Ty::Any),
        }
    }

    // ── return-path analysis ─────────────────────────────────────────────────

    // Conservative static check: true only when we can PROVE all paths return.
    // Does not catch every missing-return at runtime — that's ok for Phase 5.
    fn alwaysReturns(stmts: &[Stmt]) -> bool {
        for stmt in stmts {
            if Self::stmtAlwaysReturns(stmt) { return true; }
        }
        false
    }

    fn stmtAlwaysReturns(stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Return(_) => true,
            // if-else where BOTH branches always return
            Stmt::If { then_branch, else_branch: Some(else_branch), .. } => {
                Self::alwaysReturns(then_branch) && Self::alwaysReturns(else_branch)
            }
            // loop always runs its body — treat as returning if body always returns
            Stmt::Loop { body } => Self::alwaysReturns(body),
            // match returns if every arm returns
            Stmt::Match { arms, .. } => {
                !arms.is_empty() && arms.iter().all(|a| Self::alwaysReturns(&a.body))
            }
            _ => false,
        }
    }

    // ── scope helpers ────────────────────────────────────────────────────────

    fn define(&mut self, name: &str, ty: Ty, is_const: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), (ty, is_const));
        }
    }

    // Error if `name` already exists in the innermost scope (same-scope redeclaration).
    fn checkRedecl(&self, name: &str, span: Span) -> Result<(), TypeError> {
        if let Some(scope) = self.scopes.last() {
            if scope.contains_key(name) {
                return Err(TypeError {
                    message: format!("TypeError: '{}' is already declared in this scope", name),
                    span,
                });
            }
        }
        Ok(())
    }

    fn lookup(&self, name: &str) -> Result<(Ty, bool), std::string::String> {
        for scope in self.scopes.iter().rev() {
            if let Some((t, c)) = scope.get(name) {
                return Ok((t.clone(), *c));
            }
        }
        if let Some(namespace) = self.namespaceFunctions.get(name) {
            return Ok((Ty::Namespace(namespace.clone()), true));
        }
        if let Some(signature) = self.visibleFunction(name) {
            return Ok((Ty::Function(Box::new(signature.clone())), true));
        }
        Err(format!("TypeError: undefined variable '{}'", name))
    }

    fn visibleFunction(&self, name: &str) -> Option<&FuncSig> {
        self.functions.get(name).or_else(|| self.directFunctions.get(name))
    }

    // lookup with a proper span for the error
    fn lookupAt(&self, name: &str, span: Span) -> Result<(Ty, bool), TypeError> {
        self.lookup(name).map_err(|msg| TypeError { message: msg, span })
    }

    fn pushScope(&mut self) { self.scopes.push(HashMap::new()); }
    fn popScope(&mut self)  { self.scopes.pop(); }
}

// ─── Phase 1 unit tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod phase1Tests {
    use super::*;

    fn ann(s: &str) -> Type { Type { name: s.to_string() } }
    fn parse(s: &str) -> Result<Ty, std::string::String> { Ty::fromAnnotation(&ann(s)) }

    // 1. Primitive names
    #[test]
    fn primitives() {
        assert_eq!(parse("int"),    Ok(Ty::Int));
        assert_eq!(parse("float"),  Ok(Ty::Float));
        assert_eq!(parse("bool"),   Ok(Ty::Bool));
        assert_eq!(parse("string"), Ok(Ty::String));
        assert_eq!(parse("void"),   Ok(Ty::Void));
        assert_eq!(parse("any"),    Ok(Ty::Any));
        assert_eq!(parse("char"),   Ok(Ty::Char));
        assert!(parse("String").unwrap_err().contains("use lowercase 'string'"));
    }

    // 2. Bracket sugar forms
    #[test]
    fn bracketForms() {
        assert_eq!(parse("[int]"),        Ok(Ty::Vec(Box::new(Ty::Int))));
        // bare "Vec" keeps its any default
        assert_eq!(parse("Vec"),          Ok(Ty::Vec(Box::new(Ty::Any))));
        assert_eq!(parse("Vec<string>"),  Ok(Ty::Vec(Box::new(Ty::String))));
        // dict sugar
        assert_eq!(
            parse("{string:int}"),
            Ok(Ty::HashMap(Box::new(Ty::Int)))
        );
        // non-String key → error
        assert!(parse("{int:int}").is_err());
    }

    // 3. Bare built-ins use any defaults; explicit arguments survive parsing.
    #[test]
    fn builtInGenericArgsAreRetained() {
        assert_eq!(parse("Vec<string>"), Ok(Ty::Vec(Box::new(Ty::String))));
        assert_eq!(parse("HashMap"), Ok(Ty::HashMap(Box::new(Ty::Any))));
        assert_eq!(parse("HashMap<string,int>"), Ok(Ty::HashMap(Box::new(Ty::Int))));
        assert_eq!(parse("Option"),  Ok(Ty::Option(Box::new(Ty::Any))));
        assert_eq!(parse("Option<int>"), Ok(Ty::Option(Box::new(Ty::Int))));
        assert_eq!(parse("Result"),  Ok(Ty::Result(Box::new(Ty::Any), Box::new(Ty::Any))));
        assert_eq!(parse("Result<int,string>"), Ok(Ty::Result(Box::new(Ty::Int), Box::new(Ty::String))));
    }

    // 4. Nullable and union
    #[test]
    fn nullableAndUnion() {
        assert_eq!(parse("int?"), Ok(Ty::Nullable(Box::new(Ty::Int))));
        assert_eq!(
            parse("int | string"),
            Ok(Ty::Union(vec![Ty::Int, Ty::String]))
        );
        // single-element union collapses
        assert_eq!(parse("int"), Ok(Ty::Int));
    }

    // 5. Unsupported legacy types are errors, not Any
    #[test]
    fn unsupportedTypesFail() {
        assert!(parse("byte").is_err());
        assert!(parse("uint").is_err());
        assert!(parse("double").is_err());
    }

    // 6. isAssignableTo rules
    #[test]
    fn assignability() {
        // T → Nullable<T>  : ok
        assert!(Ty::Int.isAssignableTo(&Ty::Nullable(Box::new(Ty::Int))));
        // Int → Float  : OK (numeric widening added in Phase 4)
        assert!(Ty::Int.isAssignableTo(&Ty::Float));
        // Vec<Int> → Vec<Float>  : NOT ok
        assert!(
            !Ty::Vec(Box::new(Ty::Int))
                .isAssignableTo(&Ty::Vec(Box::new(Ty::Float)))
        );
        // Any → anything and anything → Any
        assert!(Ty::Int.isAssignableTo(&Ty::Any));
        assert!(Ty::Any.isAssignableTo(&Ty::Int));
        // Null → Nullable<_>
        assert!(Ty::Null.isAssignableTo(&Ty::Nullable(Box::new(Ty::String))));
        // T satisfies Union if it matches at least one branch
        assert!(
            Ty::Int.isAssignableTo(&Ty::Union(vec![Ty::Int, Ty::String]))
        );
    }
}

// ─── Phase 2 unit tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod phase2Tests {
    use super::*;

    fn check(src: &str) -> Result<(), TypeError> {
        use crate::lexer::token::Token;
        use crate::lexer::TokenKind;
        let mut lexer = crate::lexer::Lexer::new(src);
        let mut tokens: Vec<Token> = Vec::new();
        loop {
            let tok = lexer.nextToken().expect("lex error");
            let done = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if done { break; }
        }
        let stmts = crate::parser::Parser::new(tokens).parse().expect("parse error");
        TypeChecker::new().checkProgram(&stmts)
    }

    fn isUndefined(r: &Result<(), TypeError>) -> bool {
        r.as_ref().err().map(|e| e.message.contains("undefined variable")).unwrap_or(false)
    }

    fn isRedecl(r: &Result<(), TypeError>) -> bool {
        r.as_ref().err().map(|e| e.message.contains("already declared")).unwrap_or(false)
    }

    // 1. Valid program — valid variables, no redeclaration
    #[test]
    fn validDeclarations() {
        assert!(check("var x = 5; var y = 10;").is_ok());
        // inner scope shadows outer — not a redeclaration
        assert!(check("var x = 1; if (true) { var x = 2; }").is_ok());
    }

    // 2. Undefined variable caught
    #[test]
    fn undefinedVariable() {
        let r = check("println(z);");
        assert!(isUndefined(&r), "expected undefined error, got: {:?}", r);
    }

    // 3. Undefined variable on assignment
    #[test]
    fn undefinedAssignment() {
        let r = check("z = 5;");
        assert!(isUndefined(&r), "expected undefined error, got: {:?}", r);
    }

    // 4. Same-scope redeclaration caught
    #[test]
    fn redeclaration() {
        let r = check("var x = 1; var x = 2;");
        assert!(isRedecl(&r), "expected redecl error, got: {:?}", r);
    }

    // 5. Duplicate param name caught
    #[test]
    fn duplicateParam() {
        let r = check("func foo(a, a) { }");
        assert!(isRedecl(&r), "expected redecl error, got: {:?}", r);
    }
}

// ─── Phase 3 unit tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod phase3Tests {
    use super::*;

    fn check(src: &str) -> Result<(), TypeError> {
        use crate::lexer::token::Token;
        use crate::lexer::TokenKind;
        let mut lexer = crate::lexer::Lexer::new(src);
        let mut tokens: Vec<Token> = Vec::new();
        loop {
            let tok = lexer.nextToken().expect("lex error");
            let done = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if done { break; }
        }
        let stmts = crate::parser::Parser::new(tokens).parse().expect("parse error");
        TypeChecker::new().checkProgram(&stmts)
    }

    fn errMsg(r: &Result<(), TypeError>) -> &str {
        r.as_ref().err().map(|e| e.message.as_str()).unwrap_or("")
    }

    // 1. Arithmetic — literals and type results
    #[test]
    fn arithmeticTypes() {
        // int + int → ok (no annotation = Any inferred, so assignment is fine)
        assert!(check("var x = 1 + 2;").is_ok());
        // string concatenation requires two strings
        assert!(check("var s = \"hi\" + \"there\";").is_ok());
        // bool + int → error
        let r = check("var x = true + 1;");
        assert!(r.is_err(), "expected error for bool + int");
        assert!(errMsg(&r).contains("'+' not defined"));
    }

    // 2. Integer division preserves int; mixed division promotes to float
    #[test]
    fn divisionTypes() {
        assert!(check("var x: int = 10 / 2;").is_ok());
        assert!(check("var x: float = 10 / 2.0;").is_ok());
        assert!(check("var x: int = 10 / 2.0;").is_err());
    }

    // 3. Numeric promotion — int op float → float
    #[test]
    fn numericPromotion() {
        assert!(check("var x: float = 1 + 2.5;").is_ok());
        assert!(check("var x: float = 3 * 1.5;").is_ok());
    }

    // 4. Logical ops require Bool operands
    #[test]
    fn logicalOpsBoolRequired() {
        assert!(check("var x = true && false;").is_ok());
        assert!(check("var x = true || false;").is_ok());
        // int on left → error
        let r = check("var x = 1 && true;");
        assert!(r.is_err(), "expected error for int && bool");
        assert!(errMsg(&r).contains("'&&'/'||' requires bool"), "got: {}", errMsg(&r));
    }

    // 5. Ordering operators require comparable types
    #[test]
    fn orderingOps() {
        assert!(check("var x = 1 < 2;").is_ok());
        assert!(check("var x = \"a\" < \"b\";").is_ok());
        // bool < bool → error (not ordered)
        let r = check("var x = true < false;");
        assert!(r.is_err(), "expected error for bool < bool");
        assert!(errMsg(&r).contains("ordering operator not defined"), "got: {}", errMsg(&r));
    }

    // 6. Unary ops
    #[test]
    fn unaryOps() {
        assert!(check("var x = -5;").is_ok());
        assert!(check("var x = !true;").is_ok());
        // -true parses as (0 - true); binaryTy catches it
        let r = check("var x = -true;");
        assert!(r.is_err(), "expected error for -bool");
        assert!(errMsg(&r).contains("arithmetic not defined"), "got: {}", errMsg(&r));
    }
}

// ─── Phase 4 unit tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod phase4Tests {
    use super::*;

    fn check(src: &str) -> Result<(), TypeError> {
        use crate::lexer::token::Token;
        use crate::lexer::TokenKind;
        let mut lexer = crate::lexer::Lexer::new(src);
        let mut tokens: Vec<Token> = Vec::new();
        loop {
            let tok = lexer.nextToken().expect("lex error");
            let done = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if done { break; }
        }
        let stmts = crate::parser::Parser::new(tokens).parse().expect("parse error");
        TypeChecker::new().checkProgram(&stmts)
    }

    fn errMsg(r: &Result<(), TypeError>) -> &str {
        r.as_ref().err().map(|e| e.message.as_str()).unwrap_or("")
    }

    // 1. Exact-type annotation — happy paths
    #[test]
    fn exactAnnotation() {
        assert!(check("var x: int = 5;").is_ok());
        assert!(check("var s: string = \"hello\";").is_ok());
        assert!(check("var b: bool = true;").is_ok());
    }

    // 2. Numeric widening: int → float is safe
    #[test]
    fn intToFloatWidening() {
        assert!(check("var x: float = 5;").is_ok(), "int literal to float annotation");
        assert!(check("var x: float = 0;").is_ok());
    }

    // 3. Narrowing is rejected: float → int
    #[test]
    fn floatToIntRejected() {
        let r = check("var x: int = 3.14;");
        assert!(r.is_err(), "float → int should be rejected");
        assert!(errMsg(&r).contains("expected"), "got: {}", errMsg(&r));
    }

    // 4. Union compatibility: T assignable to T | U
    #[test]
    fn unionAssignment() {
        assert!(check("var x: int | string = 5;").is_ok());
        assert!(check("var x: int | string = \"hi\";").is_ok());
        // bool is not in the union → error
        let r = check("var x: int | string = true;");
        assert!(r.is_err(), "bool not in union[int,String]");
    }

    // 5. Nullable: null and T both assignable to T?
    #[test]
    fn nullableAssignment() {
        assert!(check("var x: int? = null;").is_ok());
        assert!(check("var x: int? = 5;").is_ok());
        assert!(check("var x: string? = \"hi\";").is_ok());
    }

    // 6. Reassignment rules
    #[test]
    fn reassignmentRules() {
        // var is mutable — same-type reassignment is valid
        assert!(check("var x = 5; x = 10;").is_ok(), "same-type reassign should pass");
        // widening on reassign: int var reassigned float → error (type locked after decl)
        let r = check("var x: int = 5; x = 3.14;");
        assert!(r.is_err(), "float reassigned to int var");
        // type mismatch on reassign
        let r2 = check("var x: int = 5; x = \"bad\";");
        assert!(r2.is_err(), "String assigned to int var");
        assert!(errMsg(&r2).contains("expected"), "got: {}", errMsg(&r2));
    }

    // 7. Type mismatch is deterministic
    #[test]
    fn typeMismatch() {
        let r = check("var x: int = \"oops\";");
        assert!(r.is_err());
        assert!(errMsg(&r).contains("TypeError"), "got: {}", errMsg(&r));
        assert!(errMsg(&r).contains("expected"), "got: {}", errMsg(&r));
    }
}

// ─── Phase 5 unit tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod phase5Tests {
    use super::*;

    fn check(src: &str) -> Result<(), TypeError> {
        use crate::lexer::token::Token;
        use crate::lexer::TokenKind;
        let mut lexer = crate::lexer::Lexer::new(src);
        let mut tokens: Vec<Token> = Vec::new();
        loop {
            let tok = lexer.nextToken().expect("lex error");
            let done = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if done { break; }
        }
        let stmts = crate::parser::Parser::new(tokens).parse().expect("parse error");
        TypeChecker::new().checkProgram(&stmts)
    }

    fn errMsg(r: &Result<(), TypeError>) -> &str {
        r.as_ref().err().map(|e| e.message.as_str()).unwrap_or("")
    }

    // 1. Valid function — typed params and return
    #[test]
    fn validFunction() {
        assert!(check("func add(a: int, b: int): int { return a + b; }").is_ok());
        assert!(check("func greet(name: string): string { return \"hi \" + name; }").is_ok());
    }

    // 2. Wrong arg type at call site
    #[test]
    fn wrongArgType() {
        let r = check("func add(a: int, b: int): int { return a + b; } var x = add(1, \"oops\");");
        assert!(r.is_err(), "expected type error for wrong arg");
        assert!(errMsg(&r).contains("argument"), "got: {}", errMsg(&r));
    }

    // 3. Wrong arg count
    #[test]
    fn wrongArgCount() {
        let r = check("func add(a: int, b: int): int { return a + b; } var x = add(1);");
        assert!(r.is_err(), "expected arity error");
    }

    // 4. Return type mismatch
    #[test]
    fn returnTypeMismatch() {
        let r = check("func f(): int { return \"nope\"; }");
        assert!(r.is_err(), "expected return type error");
        assert!(errMsg(&r).contains("return"), "got: {}", errMsg(&r));
    }

    // 5. Missing return path detected
    #[test]
    fn missingReturnPath() {
        // no return statement at all
        let r = check("func f(): int { var x = 5; }");
        assert!(r.is_err(), "expected missing-return error");
        assert!(errMsg(&r).contains("code paths"), "got: {}", errMsg(&r));
        // only a conditional return — else path missing
        let r2 = check("func f(): int { if (true) { return 1; } }");
        assert!(r2.is_err(), "expected missing-return error for if-only");
    }

    // 6. All paths return — passes
    #[test]
    fn allPathsReturn() {
        assert!(check(
            "func f(x: int): int { if (x > 0) { return x; } else { return 0; } }"
        ).is_ok());
        // final unconditional return after an if
        assert!(check(
            "func factorial(n: int): int { if (n <= 1) { return 1; } return n; }"
        ).is_ok());
    }

    // 7. Void function — no return required
    #[test]
    fn voidNoReturnRequired() {
        assert!(check("func greet(): void { println(\"hi\"); }").is_ok());
        // unannotated return type — also no requirement
        assert!(check("func greet() { println(\"hi\"); }").is_ok());
    }
}

#[cfg(test)]
mod v03Tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn check(source: &str) -> Result<(), TypeError> {
        let statements = parse(source);
        TypeChecker::new().checkProgram(&statements)
    }

    fn parse(source: &str) -> Vec<Stmt> {
        use crate::lexer::token::Token;
        let mut lexer = crate::lexer::Lexer::new(source);
        let mut tokens: Vec<Token> = Vec::new();
        loop {
            let token = lexer.nextToken().expect("lex error");
            let done = token.kind == TokenKind::Eof;
            tokens.push(token);
            if done { break; }
        }
        crate::parser::Parser::new(tokens).parse().expect("parse error")
    }

    fn resolveFixture(files: &[(&str, &str)]) -> (PathBuf, ResolvedProgram) {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let tempRoot = std::env::temp_dir().canonicalize().unwrap();
        let repoRoot = PathBuf::from(env!("CARGO_MANIFEST_DIR")).canonicalize().unwrap();
        let root = tempRoot.join(format!("rey-resolved-types-{}-{}", std::process::id(), nonce));
        assert!(!root.starts_with(&repoRoot) && !repoRoot.starts_with(&root));
        for (relative, contents) in files {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, contents).unwrap();
        }
        let program = crate::imports::resolveEntry(&root.join("main.rey")).unwrap();
        (root, program)
    }

    fn error(source: &str) -> TypeError {
        check(source).expect_err("expected type error")
    }

    #[test]
    fn lowercaseTypesAndFixIts() {
        assert!(check("var i: int = 1; var f: float = 1.0; var b: bool = true; var s: string = \"ok\"; var c: char = 'x'; func done(): void {} ").is_ok());
        for (source, fix) in [
            ("var s: String = \"x\";", "use lowercase 'string'"),
            ("var v: Void = null;", "use lowercase 'void'"),
            ("var i: Int = 1;", "use lowercase 'int'"),
            ("var f: Float = 1.0;", "use lowercase 'float'"),
            ("var b: Bool = true;", "use lowercase 'bool'"),
        ] {
            let err = error(source);
            assert!(err.message.contains(fix), "{}", err.message);
            println!("{} => {}", source, err.message);
        }
        let unknown = error("var x: Mystery = 1;");
        assert_eq!(unknown.message, "TypeError: unknown type 'Mystery'");
        println!("unknown type => {}", unknown.message);
    }

    #[test]
    fn canonicalMethodsAndRemovedLen() {
        assert!(check("var s = \"text\"; var n = s.len(); var u = s.toUpper(); var l = s.toLower();").is_ok());
        let method = error("var s = \"text\"; s.length();");
        assert_eq!(method.message, "TypeError: '.length()' was removed; use '.len()'");
        println!("length => {}", method.message);
        let upper = error("var s = \"text\"; s.upper();");
        assert_eq!(upper.message, "TypeError: '.upper()' was removed; use '.toUpper()'");
        println!("upper => {}", upper.message);
        let lower = error("var s = \"text\"; s.lower();");
        assert_eq!(lower.message, "TypeError: '.lower()' was removed; use '.toLower()'");
        let free = error("var n = len(\"text\");");
        assert_eq!(free.message, "TypeError: free function 'len(x)' was removed; use 'x.len()'");
        println!("free len => {}", free.message);
    }

    #[test]
    fn strictConditionsAndDivision() {
        assert!(check("if (true) {} while (false) {}").is_ok());
        for source in ["if (1) {}", "while (\"x\") {}"] {
            let err = error(source);
            assert!(err.message.contains("condition must be bool"), "{}", err.message);
            println!("{} => {}", source, err.message);
        }
        assert!(check("var i: int = 7 / 2; var f: float = 7 / 2.0;").is_ok());
        assert!(check("var i: int = 7 % 2; var f: float = 7 % 2.0;").is_ok());
        let mixed = error("var i: int = 7 / 2.0;");
        assert!(mixed.message.contains("expected Int but got Float"), "{}", mixed.message);
        println!("float division to int => {}", mixed.message);
    }

    #[test]
    fn stringPlusRequiresTwoStrings() {
        assert!(check("var a = \"a\" + \"b\"; var n = 1; var s = n.toString() + \"s\"; var interpolated = \"n={n}\";").is_ok());
        assert!(check("var s = \"a\"; s += \"b\";").is_ok());
        for source in [
            "var x = 1 + \"s\";",
            "var x = \"s\" + 1;",
            "var s = \"x\"; s += 1;",
            "var i = 1; i += \"s\";",
        ] {
            let err = error(source);
            assert!(err.message.contains("'+' not defined"), "{}", err.message);
            println!("{} => {}", source, err.message);
        }
    }

    #[test]
    fn narrowingAndUnknownInstanceOf() {
        let source = "struct Dog { name: string, } struct Cat { name: string, } func f(x: Dog | Cat) { if (x instanceof Dog) { var d: Dog = x; } else { var c: Cat = x; } } func g(s: string?) { if (s != null) { var t: string = s; s.len(); } }";
        assert!(check(source).is_ok());
        let before = error("func f(x: string?) { var s: string = x; }");
        assert!(before.message.contains("got Nullable"), "{}", before.message);
        println!("use before narrowing => {}", before.message);
        let unknown = error("var x = 1; if (x instanceof Missing) {} ");
        assert_eq!(unknown.message, "TypeError: unknown type 'Missing'");
        println!("unknown instanceof => {}", unknown.message);
    }

    #[test]
    fn enumBoolAndRangeMatches() {
        let valid = check("enum E { A, B, } func f(e: E) { match e { A => {}, B => {}, } } func g(b: bool) { match b { true => {}, false => {}, } } func h(n: int) { match n { 1..3 => {}, _ => {}, } }");
        assert!(valid.is_ok(), "{:?}", valid);
        let payloadVariantOnly = check("enum E { Item(int), } func f(e: E) { match e { E::Item => {}, } }");
        assert!(payloadVariantOnly.is_ok(), "{:?}", payloadVariantOnly);
        let enumError = error("enum E { A, B, } func f(e: E) { match e { E::A => {}, } }");
        assert_eq!(enumError.message, "TypeError: non-exhaustive match for enum 'E'");
        println!("enum exhaustiveness => {}", enumError.message);
        let boolError = error("func f(b: bool) { match b { true => {}, } }");
        assert_eq!(boolError.message, "TypeError: non-exhaustive match for bool; cover true and false");
        println!("bool exhaustiveness => {}", boolError.message);
        let rangeError = error("func f(n: string) { match n { 1..3 => {}, _ => {}, } }");
        assert!(rangeError.message.contains("range pattern endpoints must match"), "{}", rangeError.message);
    }

    #[test]
    fn inferredReturnsAreResolvedAndRecursiveCyclesAreDiagnosed() {
        let wrongCallType = error("func text() { return \"x\"; } var n: int = text();");
        assert!(wrongCallType.message.contains("expected Int but got String"), "{}", wrongCallType.message);

        let mixedArithmetic = error("func either(flag: bool) { if flag { return 7; } return \"x\"; } var value = either(true); var next = value + 1;");
        assert!(mixedArithmetic.message.contains("'+' not defined"), "{}", mixedArithmetic.message);

        let boolPredicate = check("func ready() { return true; } if ready() { println(true); } else { println(false); }");
        assert!(boolPredicate.is_ok(), "{:?}", boolPredicate);

        let recursive = error("func recurse() { return recurse(); }");
        assert!(recursive.message.contains("cannot infer recursive return type"), "{}", recursive.message);

        let recursiveWithBase = check("func countDown(n: int) { if n == 0 { return 0; } return countDown(n - 1); } var total: int = countDown(2);");
        assert!(recursiveWithBase.is_ok(), "{:?}", recursiveWithBase);
    }

    #[test]
    fn createRuleAndQualifiedVariadicSignature() {
        let create = "struct Thing { value: int, } pub func create(value: int): Thing { return Thing { value: value }; } var t: Thing = Thing.create(1);";
        assert!(check(create).is_ok());
        let ambiguous = "struct A {} struct B {} pub func create(): A { return A {}; } var a: A = A.create();";
        assert!(check(ambiguous).is_ok());
        let variadic = "func sum(values: ...int): int { return 0; } var math = {sum: sum}; var total: int = math.sum(1, 2, 3, 4);";
        let variadicResult = check(variadic);
        assert!(variadicResult.is_ok(), "{:?}", variadicResult);
        let badArity = error("func sum(a: int): int { return a; } var math = {sum: sum}; math.sum();");
        assert!(badArity.message.contains("sum expects"), "{}", badArity.message);
    }

    #[test]
    fn incrementUpdatesOnlyIntLvalues() {
        assert!(check("struct Counter { pub value: int, } var c = Counter { value: 0 }; c.value++; var values = [0]; values[0]++; var n = 0; ++n;").is_ok());
        let privateField = error("struct Counter { value: int, } var c = Counter { value: 0 }; c.value = 1;");
        assert!(privateField.message.contains("private field 'value'"), "{}", privateField.message);
        assert!(check("struct Counter { value: int, pub func set(n: int): void { value = n; } } var c = Counter { value: 0 }; c.set(2);").is_ok());
        let notInt = error("var f = 1.5; f++;");
        assert_eq!(notInt.message, "TypeError: ++/-- requires int lvalue, got Float");
        let constant = error("const n = 0; n++;");
        assert_eq!(constant.message, "TypeError: cannot assign to constant 'n'");
    }

    #[test]
    fn functionVisibilityIsRetainedInSignatures() {
        let statements = parse("func hidden() {} pub func visible() {}");
        let mut checker = TypeChecker::new();
        checker.checkProgram(&statements).unwrap();
        assert_eq!(checker.functions["hidden"].visibility, Some(FunctionVisibility::Private));
        assert_eq!(checker.functions["visible"].visibility, Some(FunctionVisibility::Pub));
    }

    #[test]
    fn functionTypesLoopContextAndImmutableCaptures() {
        assert!(check("var increment: (int) -> int = (x: int) => x + 1; var value: int = increment(2); func loops() { loop { break; } while (true) { continue; } }").is_ok());
        assert!(check("var captured = 3; var read = () => captured + 1; var value = read();").is_ok());

        let badType = error("var increment: (int) -> string = (x: int) => x + 1;");
        assert!(badType.message.contains("lambda return expected String but got Int"), "{}", badType.message);
        let breakOutside = error("break;");
        assert_eq!(breakOutside.message, "TypeError: break is only valid inside a loop");
        let continueOutside = error("continue;");
        assert_eq!(continueOutside.message, "TypeError: continue is only valid inside a loop");
        let mutableCapture = error("var value = 1; var writer = () => (value = 2);");
        assert!(mutableCapture.message.contains("mutable captures are not supported"), "{}", mutableCapture.message);
    }

    #[test]
    fn payloadEnumsConversionsAndTypedArrayLiterals() {
        let payloads = "enum Message { Count(value: int), Label(value: string), } var message: Message = Message::Count(7); func read(m: Message) { match m { Message::Count(count) => { var number: int = count; }, Message::Label(label) => { var text: string = label; }, } }";
        assert!(check(payloads).is_ok());
        let badPayload = error("enum Message { Count(value: int), } var message = Message::Count(\"wrong\");");
        assert!(badPayload.message.contains("payload expected Int but got String"), "{}", badPayload.message);

        assert!(check("var n: int = int('A'); var c: char = char(65); var f: float = float(n); var s: string = string(c); eprint(s, 7, true); eprintln(s, 7, true);").is_ok());
        let badInt = error("var n = int(true);");
        assert!(badInt.message.contains("argument expected Union"), "{}", badInt.message);
        assert!(check("var strings: [string] = [\"one\", \"two\"]; var empty: [string] = [];").is_ok());
        let badArray = error("var strings: [string] = [1];");
        assert!(badArray.message.contains("array element expected String but got Int"), "{}", badArray.message);

        assert!(check("var result = Result.Err(\"failure\"); var error: string = result.unwrapErr();").is_ok());
    }

    #[test]
    fn resultUnwrapErrUsesStoredErrorType() {
        assert!(check("var result = Result.Err(\"failure\"); var error: string = result.unwrapErr();").is_ok());
        assert!(check("var result: Result<string, int> = Result::Err(42); var code: int = result.unwrapErr();").is_ok());
        assert!(check("var result = Result::Err(42); var code: int = result.unwrapErr();").is_ok());
        let err = error("var result = Result.Err(\"failure\"); var number: int = result.unwrapErr();");
        assert!(err.message.contains("expected Int but got String"), "{}", err.message);
        let extraArg = error("var result = Result.Err(\"failure\"); result.unwrapErr(1);");
        assert_eq!(extraArg.message, "TypeError: Result.unwrapErr() expects 0 arguments");
    }

    #[test]
    fn rawByteStringsHaveLanguageStringType() {
        let expression = Expr::Literal {
            value: Literal::ByteString(vec![0, 0xff]),
            span: Span::new(1, 4),
        };
        assert_eq!(TypeChecker::new().exprTy(&expression), Ok(Ty::String));
    }

    #[test]
    fn recursiveTupleTypesInferredValuesAndProjections() {
        let valid = "func make(): (int, string) { return (1, \"one\"); } var pair: (int, string) = make(); var single: (int,) = (2,); var nested: ((int, string), bool) = ((3, \"three\"), true); var inner = nested.0; var text: string = inner.1; var apply: ((int, string), bool) -> void = (value: (int, string), ok: bool) => println(value.0, ok); apply(pair, true); var zero: () -> int = () => 0;";
        assert!(check(valid).is_ok());

        let wrongArity = error("var pair: (int, string) = (1,);");
        assert!(wrongArity.message.contains("tuple literal expects 1 elements but type has 2"), "{}", wrongArity.message);
        let wrongElement = error("var pair: (int, string) = (1, 2);");
        assert!(wrongElement.message.contains("tuple element expected String but got Int"), "{}", wrongElement.message);
        let outOfBounds = error("var pair: (int, string) = (1, \"one\"); var x = pair.2;");
        assert!(outOfBounds.message.contains("tuple index 2 out of bounds"), "{}", outOfBounds.message);
        assert!(Ty::fromName("(int)").unwrap_err().contains("invalid tuple type"));
        assert!(Ty::fromName("()").unwrap_err().contains("invalid tuple type"));
    }

    #[test]
    fn verifiedEnumFactoryAndCollectionGaps() {
        assert!(check("enum Direction { Left, Right, } var direction = Direction::Left; match direction { Direction::Left => {}, Direction::Right => {}, }").is_ok());
        let nonExhaustive = error("enum Direction { Left, Right, } var direction = Direction::Left; match direction { Direction::Left => {}, }");
        assert_eq!(nonExhaustive.message, "TypeError: non-exhaustive match for enum 'Direction'");

        assert!(check("var values: [int] = Vec.new(); values.reverse(); var map: {string:int} = HashMap.new(); map.set(\"answer\", 42); var answer: int = map.get(\"answer\"); var hasAnswer: bool = map.has(\"answer\"); map.delete(\"answer\"); var count: int = map.len();").is_ok());
        assert!(check("var values: [int] = Vec.new(); values.set(0, 4); var first: int = values.get(0); var has: bool = values.contains(4); var index: int = values.indexOf(4); var part: [int] = values.slice(0, 1); values.sort(); var mapped: [int] = values.map((item: int) => item + 1); var filtered: [int] = values.filter((item: int) => item > 0); var total: int = values.reduce((sum: int, item: int) => sum + item, 0);").is_ok());
        let badSet = error("var values: [int] = Vec.new(); values.set(0, \"wrong\");");
        assert!(badSet.message.contains("Vec.set() value expected Int but got String"), "{}", badSet.message);
        assert!(check("var option = Option::Some(7); var some: bool = option.isSome(); var value: int = option.unwrap(); var none = Option::None(); var empty: bool = none.isNone(); var result = Result::Ok(3); var ok: bool = result.isOk(); var recovered = result.unwrapOr(4); var failed = Result::Err(\"bad\"); var message: string = failed.unwrapErr();").is_ok());

        let unsafeCollectionCast = error("var erased: HashMap = HashMap.new(); var typed: {string:int} = erased;");
        assert!(unsafeCollectionCast.message.contains("expected HashMap(Int) but got HashMap(Any)"), "{}", unsafeCollectionCast.message);
        let badConversionReceiver = error("var values = Vec.new(); values.toInt();");
        assert!(badConversionReceiver.message.contains("toInt() is not supported on Vec"), "{}", badConversionReceiver.message);
        let unknownVectorMethod = error("var values = Vec.new(); values.nope();");
        assert!(unknownVectorMethod.message.contains("method 'nope' not found"), "{}", unknownVectorMethod.message);
    }

    #[test]
    fn contextualEmptyCollectionsReachStructFieldsReturnsAndCalls() {
        let valid = "struct Token {} struct LexerResult { tokens: [Token], lookup: {string:int}, } var result = LexerResult { tokens: [], lookup: {} }; func emptyTokens(): [Token] { return []; } func takeTokens(tokens: [Token]): void {} takeTokens([]);";
        assert!(check(valid).is_ok());
        let nested = Expr::ArrayLiteral {
            elements: vec![Expr::ArrayLiteral { elements: vec![], span: Span::new(0, 0) }],
            span: Span::new(0, 0),
        };
        let nestedType = Ty::Vec(Box::new(Ty::Vec(Box::new(Ty::String))));
        assert_eq!(TypeChecker::new().exprTyExpected(&nested, &nestedType), Ok(nestedType));
        let wrongElement = error("struct Token {} struct LexerResult { tokens: [Token], } var result = LexerResult { tokens: [1] };");
        assert!(wrongElement.message.contains("array element expected Struct(\"Token\") but got Int"), "{}", wrongElement.message);
    }

    #[test]
    fn resolvedNominalsKeepOwnerContextAndPrivateDependenciesHidden() {
        let (root, program) = resolveFixture(&[
            ("types/main.rey", "enum ExprKind { Name, } enum StmtKind { Return, } struct Expr { kind: ExprKind, statement: Stmt?, } struct Stmt { kind: StmtKind, expression: Expr?, } pub struct Wrapper { root: Expr, } pub func make(): Wrapper { return Wrapper { root: Expr { kind: ExprKind::Name, statement: null } }; }"),
            ("main.rey", "import types::{Wrapper, make}; var wrapper: Wrapper = make();"),
        ]);
        let rootId = std::fs::canonicalize(root.join("main.rey")).unwrap();
        let visibleTypes = &program.moduleBindings[&rootId].types;
        assert!(visibleTypes.contains_key("Wrapper"));
        assert!(!visibleTypes.contains_key("ExprKind"));
        let checked = TypeChecker::checkResolvedProgram(&program);
        assert!(checked.is_ok(), "{:?}", checked);

        let (privateRoot, privateProgram) = resolveFixture(&[
            ("types/main.rey", "enum ExprKind { Name, } pub struct Wrapper { kind: ExprKind, }"),
            ("main.rey", "import types::{Wrapper}; var kind: ExprKind = null;"),
        ]);
        let error = TypeChecker::checkResolvedProgram(&privateProgram).unwrap_err();
        assert_eq!(error.file, std::fs::canonicalize(privateRoot.join("main.rey")).unwrap());
        assert!(error.error.message.contains("unknown type 'ExprKind'"), "{}", error.error.message);
    }

    #[test]
    fn resolvedNominalIdentityIncludesItsDeclaringModule() {
        let (_, program) = resolveFixture(&[
            ("a/main.rey", "struct Hidden { pub value: int, } pub func make(): Hidden { return Hidden { value: 1 }; } pub func read(value: Hidden): int { return value.value; }"),
            ("b/main.rey", "struct Hidden { pub value: int, } pub func make(): Hidden { return Hidden { value: 2 }; } pub func read(value: Hidden): int { return value.value; }"),
            ("main.rey", "import a; import b; var left: int = a.read(a.make()); var right: int = b.read(b.make());"),
        ]);
        let checked = TypeChecker::checkResolvedProgram(&program);
        assert!(checked.is_ok(), "{:?}", checked);

        let (_, mismatchedProgram) = resolveFixture(&[
            ("a/main.rey", "struct Hidden { pub value: int, } pub func make(): Hidden { return Hidden { value: 1 }; } pub func read(value: Hidden): int { return value.value; }"),
            ("b/main.rey", "struct Hidden { pub value: int, } pub func make(): Hidden { return Hidden { value: 2 }; }"),
            ("main.rey", "import a; import b; var value: int = a.read(b.make());"),
        ]);
        let error = TypeChecker::checkResolvedProgram(&mismatchedProgram).unwrap_err();
        assert!(error.error.message.contains("argument expected Struct("), "{}", error.error.message);
        assert!(error.error.message.contains("but got Struct("), "{}", error.error.message);
    }

    #[test]
    fn resolvedEnumDiagnosticsUseDeclaredName() {
        let (root, program) = resolveFixture(&[
            ("directions/main.rey", "pub enum Direction { North, South, }"),
            ("main.rey", "import directions::{Direction}; func check(value: Direction): void { match value { Direction::North => {}, } }"),
        ]);
        let error = TypeChecker::checkResolvedProgram(&program).unwrap_err();
        assert_eq!(error.file, std::fs::canonicalize(root.join("main.rey")).unwrap());
        assert_eq!(error.error.message, "TypeError: non-exhaustive match for enum 'Direction'");
    }

    #[test]
    fn resolvedPrivateEnumsWithSameNameRemainDistinct() {
        let (_, program) = resolveFixture(&[
            ("a/main.rey", "enum Hidden { Left, Right, } pub func make(): Hidden { return Hidden::Left; } pub func accept(value: Hidden): int { return 1; }"),
            ("b/main.rey", "enum Hidden { Left, Right, } pub func make(): Hidden { return Hidden::Right; } pub func accept(value: Hidden): int { return 2; }"),
            ("main.rey", "import a; import b; var left: int = a.accept(a.make()); var right: int = b.accept(b.make());"),
        ]);
        assert!(TypeChecker::checkResolvedProgram(&program).is_ok());

        let (mismatchedRoot, mismatched) = resolveFixture(&[
            ("a/main.rey", "enum Hidden { Left, Right, } pub func make(): Hidden { return Hidden::Left; } pub func accept(value: Hidden): int { return 1; }"),
            ("b/main.rey", "enum Hidden { Left, Right, } pub func make(): Hidden { return Hidden::Right; }"),
            ("main.rey", "import a; import b; var value: int = a.accept(b.make());"),
        ]);
        let error = TypeChecker::checkResolvedProgram(&mismatched).unwrap_err();
        assert_eq!(error.file, std::fs::canonicalize(mismatchedRoot.join("main.rey")).unwrap());
        assert!(error.error.message.contains("argument expected Enum("), "{}", error.error.message);
        assert!(error.error.message.contains("but got Enum("), "{}", error.error.message);
    }

    #[test]
    fn appliedBuiltinsRetainArgumentsForStaticChecking() {
        assert_eq!(Ty::fromName("Vec<string>").unwrap(), Ty::fromName("[string]").unwrap());
        assert_eq!(Ty::fromName("HashMap<string,int>").unwrap(), Ty::HashMap(Box::new(Ty::Int)));
        assert_eq!(Ty::fromName("Option<Result<int,string>>").unwrap(), Ty::Option(Box::new(
            Ty::Result(Box::new(Ty::Int), Box::new(Ty::String)),
        )));

        assert!(check("var values: Vec<string> = [\"one\"]; var bracket: [string] = values; func take(items: Vec<string>): void {} take([\"two\"]); func make(): Vec<string> { return [\"three\"]; } var fromFactory: Vec<int> = Vec.new(); fromFactory.push(1); var map: HashMap<string,int> = HashMap.new(); map.set(\"answer\", 42); var option: Option<int> = Option::Some(7); var result: Result<int,string> = Result::Ok(option.unwrap());").is_ok());
        assert!(check("var argsList: [string] = args(); var nativeList: Vec<string> = nativeArgs();").is_ok());

        let badPush = error("var values: Vec<string> = []; values.push(1);");
        assert!(badPush.message.contains("Vec.push() expected element String but got Int"), "{}", badPush.message);
        let badArgument = error("func take(items: Vec<string>): void {} take([1]);");
        assert!(badArgument.message.contains("array element expected String but got Int"), "{}", badArgument.message);
        let badReturn = error("func make(): Vec<string> { return [1]; }");
        assert!(badReturn.message.contains("array element expected String but got Int"), "{}", badReturn.message);
        let badMapKey = error("var map: HashMap<int,string> = HashMap.new();");
        assert!(badMapKey.message.contains("HashMap key must be string"), "{}", badMapKey.message);
        let badMapValue = error("var map: HashMap<string,int> = {}; map.set(\"key\", \"bad\");");
        assert!(badMapValue.message.contains("HashMap.set() value expected Int but got String"), "{}", badMapValue.message);
        let badOptionPayload = error("var option: Option<int> = Option::Some(\"bad\");");
        assert!(badOptionPayload.message.contains("expected Option(Int) but got Option(String)"), "{}", badOptionPayload.message);
        let badResultPayload = error("var result: Result<int,string> = Result::Err(42);");
        assert!(badResultPayload.message.contains("expected Result(Int, String) but got Result(Int, Int)"), "{}", badResultPayload.message);

        let erasedRuntimeArgs = error("var values: [int] = args();");
        assert!(erasedRuntimeArgs.message.contains("expected Vec(Int) but got Vec(String)"), "{}", erasedRuntimeArgs.message);
    }

    #[test]
    fn nestedGenericFunctionsAndArityAreParsedPrecisely() {
        let nested = Ty::fromName("Vec<(int, string) -> Option<Result<int,string>>?>").unwrap();
        assert!(matches!(nested, Ty::Vec(inner) if matches!(*inner, Ty::Function(_))));
        assert!(matches!(Ty::fromName("Vec<int | string>").unwrap(), Ty::Vec(inner) if matches!(*inner, Ty::Union(_))));

        for (name, expected) in [
            ("Vec<int,string>", "Vec expects 1 type argument"),
            ("HashMap<string>", "HashMap expects 2 type arguments"),
            ("Option<int,string>", "Option expects 1 type argument"),
            ("Result<int>", "Result expects 2 type arguments"),
            ("Vec<int", "malformed type application"),
            ("Box<int>", "generic arguments are only supported for built-in containers"),
        ] {
            let error = Ty::fromName(name).unwrap_err();
            assert!(error.contains(expected), "{}: {}", name, error);
        }
        let unknownArgument = error("var values: Vec<Missing> = [];");
        assert!(unknownArgument.message.contains("unknown type 'Missing'"), "{}", unknownArgument.message);
    }

    #[cfg(unix)]
    #[test]
    fn resolvedActualNativeAstChecksAgainstItsPrivateDefinitions() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let nativeSrc = manifest.join("../../rey-compiler/src");
        let astPath = std::fs::canonicalize(nativeSrc.join("parser/ast/main.rey")).unwrap();
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("rey-native-parser-types-{}-{}", std::process::id(), nonce));
        std::fs::create_dir_all(&root).unwrap();
        std::os::unix::fs::symlink(nativeSrc.join("lexer"), root.join("lexer")).unwrap();
        std::os::unix::fs::symlink(nativeSrc.join("parser/ast"), root.join("ast")).unwrap();
        std::fs::write(root.join("main.rey"), "import ast::{Expr}; func accept(expr: Expr): void {}").unwrap();
        let resolved = crate::imports::resolveEntry(&root.join("main.rey")).unwrap();
        let rootId = std::fs::canonicalize(root.join("main.rey")).unwrap();
        let visibleTypes = &resolved.moduleBindings[&rootId].types;
        assert!(visibleTypes.contains_key("Expr"));
        assert!(!visibleTypes.contains_key("ExprKind"));
        assert!(resolved.sources.contains_key(&astPath));
        let checked = TypeChecker::checkResolvedProgram(&resolved);
        assert!(checked.is_ok(), "{:?}", checked);
    }

    #[test]
    fn specOracleMismatchesAreRejectedOrAccepted() {
        assert!(check("func main(): void { var letter = 'a'; match letter { 'a' => { println(\"letter a\"); }, _ => { println(\"other\"); }, } }").is_ok());
        assert!(check("enum Message { Maybe(Option<int>), } func main(): void { var x = Message::Maybe(Option::None); println(x); }").is_ok());
        assert!(check("struct Counter { value: int, func get(): int { return value; } } func main(): void { var counter = Counter { value: 7 }; println(counter.get()); }").is_ok());
        assert!(check("func main(): void { var nested = ((1, 2), \"z\"); println(nested.0.0); println(nested.0.1); }").is_ok());
        let defaults = error("func bad(a: int = 1, b: int): int { return a + b; }");
        assert!(defaults.message.contains("default parameters must be trailing"), "{}", defaults.message);
        let privateField = error("struct Counter { value: int, func get(): int { return value; } } func main(): void { var counter = Counter { value: 7 }; println(counter.value); }");
        assert!(privateField.message.contains("private field"), "{}", privateField.message);

        let (_, program) = resolveFixture(&[
            ("leaf.rey", "println(\"leaf init\");\nvar seed: int = 40;\npub func value(): int { return seed; }\n"),
            ("main.rey", "import leaf.value;\nfunc main(): void { println(value()); }\n"),
        ]);
        let checked = TypeChecker::checkResolvedProgram(&program);
        assert!(checked.is_ok(), "{:?}", checked);
        assert!(check("/* header */\nfunc main(): void {\n    var hex = 0x10;\n    var bits = 0b1010;\n    println(hex);\n    println(bits);\n}").is_ok());
        assert!(check("func main(): void { println('a' < 'b'); println('\\xFF' > '\\x00'); }").is_ok());
        let mixedChars = error("func main(): void { println('a' < 98); }");
        assert!(mixedChars.message.contains("ordering operator not defined"), "{}", mixedChars.message);
        let mainParams = error("func main(x: int): void { println(x); }");
        assert!(mainParams.message.contains("entry main must take no parameters and return void"), "{}", mainParams.message);
        let mainReturn = error("func main(): int { return 1; }");
        assert!(mainReturn.message.contains("entry main must take no parameters and return void"), "{}", mainReturn.message);

        let (_, importedMain) = resolveFixture(&[
            ("leaf.rey", "pub func main(): int { return 4; }\npub func value(): int { return main(); }\n"),
            ("main.rey", "import leaf.value;\nfunc main(): void { println(value()); }\n"),
        ]);
        assert!(TypeChecker::checkResolvedProgram(&importedMain).is_ok());
    }
}
