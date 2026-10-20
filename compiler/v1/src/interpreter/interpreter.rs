use super::environment::Environment;
use super::executor::Executor;
use super::std::StdLib;
use super::value::{StructDef, Value};
use crate::ast::{Expr, Stmt};
use crate::imports::ResolvedProgram;
use crate::typecheck::TypeChecker;
use crate::typecheck::TypeError;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[cfg_attr(not(test), allow(dead_code))]
pub enum InterpretError {
    Type(TypeError),
    Runtime(String),
}

impl std::fmt::Display for InterpretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpretError::Type(terr) => write!(f, "{}", terr.message),
            InterpretError::Runtime(msg) => write!(f, "{}", msg),
        }
    }
}

impl From<TypeError> for InterpretError {
    fn from(err: TypeError) -> Self {
        InterpretError::Type(err)
    }
}

impl From<String> for InterpretError {
    fn from(err: String) -> Self {
        InterpretError::Runtime(err)
    }
}

pub struct Interpreter {
    environment: Environment,
    builtins: Environment,
    executor: Executor,
}

impl Interpreter {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn new() -> Self {
        Self::new_with_args(Vec::new())
    }

    // args are normalized as [program, ...user_args]
    pub fn new_with_args(program_args: Vec<String>) -> Self {
        let mut builtins = Environment::new();
        let globals = StdLib::create_global_environment();
        for (name, value) in globals {
            builtins.define(name, value);
        }
        let environment = Environment::with_parent(builtins.clone());

        Self {
            environment,
            builtins,
            executor: Executor::with_program_args(program_args),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn interpret(&mut self, statements: &[Stmt]) -> Result<(), InterpretError> {
        let mut checker = TypeChecker::new();
        checker.checkProgram(statements)?;

        self.executor
            .execute_block(statements, &mut self.environment)?;

        if self.environment.get("main").is_some() {
                let call = Expr::Call {
                    callee: Box::new(Expr::Variable {
                        name: "main".to_string(),
                        span: crate::lexer::span::Span {
                            start: 0,
                            end: 0,
                            line: 0,
                            column: 0,
                        },
                    }),
                    args: vec![],
                    span: crate::lexer::span::Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        column: 0,
                    },
                };
            self.executor.evaluate_expr(&call, &mut self.environment)?;
        }

        Ok(())
    }

    // runtime-only; caller must run TypeChecker::checkResolvedProgram first
    pub fn interpret_resolved(
        &mut self,
        program: &ResolvedProgram,
        entry_module: &Path,
    ) -> Result<(), String> {
        if program.statements.len() != program.statementProvenance.len() {
            return Err("Resolved statement provenance is not aligned".to_string());
        }
        let entry_id = if program.sources.contains_key(entry_module) {
            entry_module.to_path_buf()
        } else {
            std::fs::canonicalize(entry_module).map_err(|err| format!("{}", err))?
        };
        if !program.sources.contains_key(&entry_id) {
            return Err(format!("Resolved entry module not found: {}", entry_id.display()));
        }
        let module_ids = program.sources.keys().cloned().collect::<HashSet<_>>();
        let dependencies = program
            .moduleBindings
            .iter()
            .map(|(module_id, bindings)| (module_id.clone(), bindings.dependencies.clone()))
            .collect::<HashMap<_, _>>();
        let module_order = dependencyFirstOrder(&entry_id, &dependencies, &module_ids)?;
        let mut module_statements = HashMap::new();
        for (module_id, source) in &program.sources {
            module_statements.insert(module_id.clone(), parseResolvedSource(source)?);
        }
        self.execute_resolved_modules(program, &entry_id, &module_order, module_statements)
    }

    fn execute_resolved_modules(
        &mut self,
        program: &ResolvedProgram,
        entry_id: &PathBuf,
        module_order: &[PathBuf],
        module_statements: HashMap<PathBuf, Vec<Stmt>>,
    ) -> Result<(), String> {
        let mut module_environments = module_order
            .iter()
            .cloned()
            .map(|module_id| (module_id, Environment::with_parent(self.builtins.clone())))
            .collect::<HashMap<_, _>>();
        for module_id in module_order {
            if !module_statements.contains_key(module_id) {
                return Err(format!("Missing parsed statements for module {}", module_id.display()));
            }
        }

        // Register every local type before function closures capture their module scope.
        for module_id in module_order {
            for statement in &module_statements[module_id] {
                if !isTypeDeclaration(statement) {
                    continue;
                }
                let module_env = module_environments
                    .get_mut(module_id)
                    .ok_or_else(|| "Missing runtime module environment".to_string())?;
                self.executor.execute(statement, module_env)?;
            }
        }

        // Install type aliases in dependency order before named functions capture environments.
        for owner_id in module_order {
            let Some(bindings) = program.moduleBindings.get(owner_id) else {
                continue;
            };
            let mut direct_types: Vec<(String, Option<StructDef>, Option<Vec<String>>)> =
                Vec::new();
            for (alias, symbol) in &bindings.types {
                let source = module_environments
                    .get(&symbol.moduleId)
                    .ok_or_else(|| format!("Missing module {}", symbol.moduleId.display()))?;
                let struct_def = source.get_struct(&symbol.name).cloned();
                let enum_variants = source.enum_defs.get(&symbol.name).cloned();
                direct_types.push((alias.clone(), struct_def, enum_variants));
            }
            let owner_env = module_environments
                .get_mut(owner_id)
                .ok_or_else(|| format!("Missing owner module {}", owner_id.display()))?;
            for (alias, struct_def, enum_variants) in direct_types {
                if let Some(def) = struct_def {
                    Rc::make_mut(&mut owner_env.struct_defs).insert(alias.clone(), def);
                }
                if let Some(variants) = enum_variants {
                    owner_env.register_enum(alias, variants);
                }
            }
        }

        // Methods use their defining module's lexical scope, not the caller's imports.
        for module_id in module_order {
            if let Some(bindings) = program.moduleBindings.get(module_id) {
                for (alias, symbol) in &bindings.types {
                    let source = module_environments
                        .get(&symbol.moduleId)
                        .ok_or_else(|| format!("Missing module {}", symbol.moduleId.display()))?;
                    if let Some(context) = source.get_struct_context(&symbol.name) {
                        module_environments
                            .get_mut(module_id)
                            .ok_or_else(|| format!("Missing owner module {}", module_id.display()))?
                            .register_struct_context(alias.clone(), context);
                    }
                }
            }

            let context = Rc::new(
                module_environments
                    .get(module_id)
                    .ok_or_else(|| {
                        format!(
                            "Missing runtime module environment: {}",
                            module_id.display()
                        )
                    })?
                    .clone(),
            );
            let module_env = module_environments
                .get_mut(module_id)
                .ok_or_else(|| "Missing runtime module environment".to_string())?;
            for statement in &module_statements[module_id] {
                if let Stmt::StructDecl { name, .. } = statement {
                    module_env.register_struct_context(name.clone(), context.clone());
                }
            }
        }

        // Function closures now see local forward types and all imported type aliases.
        for module_id in module_order {
            for statement in &module_statements[module_id] {
                if !isFunctionDeclaration(statement) {
                    continue;
                }
                let module_env = module_environments
                    .get_mut(module_id)
                    .ok_or_else(|| "Missing runtime module environment".to_string())?;
                self.executor.execute(statement, module_env)?;
            }
        }

        // Bind functions and namespaces after all module-local function identities exist.
        for owner_id in module_order {
            let Some(bindings) = program.moduleBindings.get(owner_id) else {
                continue;
            };
            let mut direct_functions = Vec::new();
            let mut namespaces = Vec::new();
            for (alias, symbol) in &bindings.direct {
                let source = module_environments
                    .get(&symbol.moduleId)
                    .ok_or_else(|| format!("Missing module {}", symbol.moduleId.display()))?;
                if let Some(Value::Function(function)) = source.get(&symbol.name) {
                    direct_functions.push((alias.clone(), function));
                }
            }
            for (namespace_name, symbols) in &bindings.namespaces {
                let mut members = HashMap::new();
                for (member_name, symbol) in symbols {
                    let source = module_environments
                        .get(&symbol.moduleId)
                        .ok_or_else(|| format!("Missing module {}", symbol.moduleId.display()))?;
                    match source.get(&symbol.name) {
                        Some(Value::Function(function)) => {
                            members
                                .insert(member_name.as_bytes().to_vec(), Value::Function(function));
                        }
                        _ => {
                            return Err(format!(
                                "Resolved function '{}::{}' is unavailable",
                                symbol.moduleId.display(),
                                symbol.name
                            ));
                        }
                    }
                }
                namespaces.push((namespace_name.clone(), members));
            }
            let owner_env = module_environments
                .get_mut(owner_id)
                .ok_or_else(|| format!("Missing owner module {}", owner_id.display()))?;
            for (alias, function) in direct_functions {
                owner_env.define(alias, Value::Function(function));
            }
            for (name, members) in namespaces {
                owner_env.define(name, Value::Namespace(Rc::new(RefCell::new(members))));
            }
        }

        self.environment = module_environments
            .get(entry_id)
            .cloned()
            .ok_or_else(|| "Missing entry module environment".to_string())?;

        // Initialize every imported source once after all function identities and imports exist.
        for module_id in module_order {
            if module_id == entry_id {
                continue;
            }
            let module_env = module_environments
                .get_mut(module_id)
                .ok_or_else(|| "Missing runtime module environment".to_string())?;
            for statement in &module_statements[module_id] {
                if matches!(statement, Stmt::Import { .. }) || isRuntimeDeclaration(statement) {
                    continue;
                }
                self.executor.execute(statement, module_env)?;
            }
        }

        // The entry module's top-level statements run in source order after imported modules.
        for statement in &module_statements[entry_id] {
            if matches!(statement, Stmt::Import { .. }) || isRuntimeDeclaration(statement) {
                continue;
            }
            self.executor.execute(statement, &mut self.environment)?;
        }

        if self.environment.get("main").is_some() {
            let call = Expr::Call {
                callee: Box::new(Expr::Variable {
                    name: "main".to_string(),
                    span: crate::lexer::span::Span::new(0, 0),
                }),
                args: vec![],
                span: crate::lexer::span::Span::new(0, 0),
            };
            self.executor.evaluate_expr(&call, &mut self.environment)?;
        }
        Ok(())
    }
}

fn isRuntimeDeclaration(statement: &Stmt) -> bool {
    matches!(
        statement,
        Stmt::FuncDecl { .. }
            | Stmt::StructDecl { .. }
            | Stmt::EnumDecl { .. }
            | Stmt::EnumDeclWithPayload { .. }
    )
}

fn isTypeDeclaration(statement: &Stmt) -> bool {
    matches!(
        statement,
        Stmt::StructDecl { .. } | Stmt::EnumDecl { .. } | Stmt::EnumDeclWithPayload { .. }
    )
}

fn isFunctionDeclaration(statement: &Stmt) -> bool {
    matches!(statement, Stmt::FuncDecl { .. })
}

fn parseResolvedSource(source: &str) -> Result<Vec<Stmt>, String> {
    let mut lexer = crate::lexer::Lexer::new(source);
    let mut tokens = Vec::new();
    loop {
        let token = lexer
            .nextToken()
            .map_err(|error| error.message())?;
        let at_end = token.kind == crate::lexer::TokenKind::Eof;
        tokens.push(token);
        if at_end {
            break;
        }
    }
    crate::parser::Parser::new(tokens)
        .parse()
        .map_err(|error| format!("resolved module parse failed: {:?}", error))
}

fn dependencyFirstOrder(
    entry: &PathBuf,
    dependencies: &HashMap<PathBuf, Vec<PathBuf>>,
    known_modules: &HashSet<PathBuf>,
) -> Result<Vec<PathBuf>, String> {
    fn visit(
        module_id: &PathBuf,
        dependencies: &HashMap<PathBuf, Vec<PathBuf>>,
        known_modules: &HashSet<PathBuf>,
        active: &mut HashSet<PathBuf>,
        visited: &mut HashSet<PathBuf>,
        ordered: &mut Vec<PathBuf>,
    ) -> Result<(), String> {
        if active.contains(module_id) {
            return Err(format!("Circular runtime module dependency: {}", module_id.display()));
        }
        if visited.contains(module_id) {
            return Ok(());
        }
        if !known_modules.contains(module_id) {
            return Err(format!(
                "Resolved module dependency target missing: {}",
                module_id.display()
            ));
        }
        active.insert(module_id.clone());
        let module_dependencies = dependencies.get(module_id).ok_or_else(|| {
            format!(
                "Resolved module dependency metadata missing for {}",
                module_id.display()
            )
        })?;
        for dependency in module_dependencies {
            visit(
                dependency,
                dependencies,
                known_modules,
                active,
                visited,
                ordered,
            )?;
        }
        active.remove(module_id);
        visited.insert(module_id.clone());
        ordered.push(module_id.clone());
        Ok(())
    }

    let mut active = HashSet::new();
    let mut visited = HashSet::new();
    let mut ordered = Vec::new();
    visit(
        entry,
        dependencies,
        known_modules,
        &mut active,
        &mut visited,
        &mut ordered,
    )?;
    if visited.len() != known_modules.len() {
        let missing = known_modules
            .difference(&visited)
            .map(|module| module.display().to_string())
            .collect::<Vec<_>>();
        return Err(format!(
            "Resolved module dependency metadata missing for: {}",
            missing.join(", ")
        ));
    }
    Ok(ordered)
}

#[cfg(test)]
mod args_tests {
    use super::*;
    use crate::interpreter::value::Value;
    use crate::lexer::span::Span;

    fn args_call() -> Expr {
        Expr::Call {
            callee: Box::new(Expr::Variable {
                name: "args".to_string(),
                span: Span::new(0, 0),
            }),
            args: vec![],
            span: Span::new(0, 0),
        }
    }

    fn args_values(interpreter: &mut Interpreter) -> Vec<String> {
        match interpreter
            .executor
            .evaluate_expr(&args_call(), &mut interpreter.environment)
            .unwrap()
        {
            Value::Array(values) => values
                .borrow()
                .iter()
                .map(|value| match value {
                    Value::String(value) => value.asStr().to_string(),
                    other => panic!("args() returned non-string: {}", other),
                })
                .collect(),
            other => panic!("args() returned non-array: {}", other),
        }
    }

    #[test]
    fn applicationArgsAreNormalizedAndInstanceScoped() {
        let mut first = Interpreter::new_with_args(vec![
            "/tmp/source with spaces.rey".to_string(),
            "--verbose".to_string(),
            "two words".to_string(),
            "-x".to_string(),
            "".to_string(),
        ]);
        let mut second = Interpreter::new_with_args(vec!["/tmp/other.rey".to_string()]);

        assert_eq!(
            args_values(&mut first),
            vec![
                "/tmp/source with spaces.rey",
                "--verbose",
                "two words",
                "-x",
                "",
            ]
        );
        assert_eq!(args_values(&mut second), vec!["/tmp/other.rey"]);
        assert!(args_values(&mut Interpreter::new()).is_empty());
    }
}

#[cfg(test)]
mod resolved_tests {
    use super::*;
    use crate::ast::{FunctionVisibility, Literal, Type};
    use crate::imports::{ModuleBindings, ResolvedProgram, ResolvedSymbol, StatementProvenance};
    use crate::lexer::span::Span;
    use std::collections::HashMap;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

    fn fixture(files: &[(&str, &str)]) -> PathBuf {
        let scratch = std::env::var_os("REY_V03_SCRATCH")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let root = scratch.join(format!(
            "rey-runtime-types-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        for (relative, contents) in files {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
        root
    }

    fn span() -> Span {
        Span::new(0, 0)
    }

    fn int(value: i64) -> Expr {
        Expr::Literal {
            value: Literal::Int(value),
            span: span(),
        }
    }

    fn variable(name: &str) -> Expr {
        Expr::Variable {
            name: name.to_string(),
            span: span(),
        }
    }

    fn call(name: &str) -> Expr {
        Expr::Call {
            callee: Box::new(variable(name)),
            args: vec![],
            span: span(),
        }
    }

    fn function(name: &str, visibility: FunctionVisibility, body: Vec<Stmt>) -> Stmt {
        Stmt::FuncDecl {
            name: name.to_string(),
            visibility,
            params: vec![],
            return_ty: Some(Type {
                name: "int".to_string(),
            }),
            body,
        }
    }

    fn get_function() -> Stmt {
        function(
            "get",
            FunctionVisibility::Pub,
            vec![Stmt::Return(Some(call("helper")))],
        )
    }

    #[test]
    fn resolvedRuntimeCapturesForwardImportedTypesAndDefiningMethodScopes() {
        let root = fixture(&[
            (
                "model/main.rey",
                "pub func make(): Parcel { return Parcel { value: 5 }; } pub func selected(): Flag { return Flag::Ready; } func secret(): int { return 20; } pub struct Parcel { value: int, pub func total(): int { return secret() + value; } } pub enum Flag { Ready, }",
            ),
            (
                "a/main.rey",
                "func secret(): int { return 100; } struct Local { value: int, pub func read(): int { return secret() + value; } } pub func score(): int { var local = Local { value: 1 }; return local.read(); }",
            ),
            (
                "b/main.rey",
                "func secret(): int { return 200; } struct Local { value: int, pub func read(): int { return secret() + value; } } pub func score(): int { var local = Local { value: 2 }; return local.read(); }",
            ),
            (
                "main.rey",
                "import model; import a; import b; var aggregate: int = 0; var enumMatches: int = 0; func main(): void { var local: Parcel = Parcel { value: 2 }; var fromNamespace: Parcel = model.make(); aggregate = local.total(); aggregate = aggregate + fromNamespace.total(); aggregate = aggregate + a.score(); aggregate = aggregate + b.score(); var localChoice: Flag = Flag::Ready; match localChoice { Flag::Ready => { enumMatches = enumMatches + 1; }, _ => {}, } var selected: Flag = model.selected(); match selected { Flag::Ready => { enumMatches = enumMatches + 1; }, _ => {}, } }",
            ),
        ]);
        let entry = root.join("main.rey");
        let program = crate::imports::resolveEntry(&entry).unwrap();
        TypeChecker::checkResolvedProgram(&program).unwrap();

        let mut interpreter = Interpreter::new();
        interpreter.interpret_resolved(&program, &entry).unwrap();
        assert_eq!(
            interpreter.environment.get("aggregate"),
            Some(Value::Int(350))
        );
        assert_eq!(
            interpreter.environment.get("enumMatches"),
            Some(Value::Int(2))
        );

        let parcel = interpreter.environment.get_struct("Parcel").unwrap();
        assert_eq!(parcel.name, "Parcel");
        let method_context = interpreter
            .environment
            .get_struct_context("Parcel")
            .unwrap();
        assert!(method_context.get("secret").is_some());
    }

    #[test]
    fn resolvedRuntimeDoesNotImportPrivateTypes() {
        let root = fixture(&[
            ("private.rey", "struct Hidden { value: int, }"),
            (
                "main.rey",
                "import private.Hidden; var hidden: Hidden = Hidden { value: 1 };",
            ),
        ]);
        let error = crate::imports::resolveEntry(&root.join("main.rey")).unwrap_err();
        assert!(error.message.contains("Type 'Hidden'") && error.message.contains("private"));
    }

    #[test]
    fn resolvedModulesKeepPrivateFunctionIdentityAndInitializeOnce() {
        let entry = PathBuf::from("/virtual/app/main.rey");
        let module_a = PathBuf::from("/virtual/app/a/main.rey");
        let module_b = PathBuf::from("/virtual/app/b/main.rey");

        let statements = vec![
            function(
                "helper",
                FunctionVisibility::Private,
                vec![Stmt::Return(Some(int(10)))],
            ),
            get_function(),
            Stmt::VarDecl {
                is_const: true,
                name: "counter".to_string(),
                ty: Some(Type {
                    name: "int".to_string(),
                }),
                initializer: int(0),
            },
            function(
                "increment",
                FunctionVisibility::Pub,
                vec![
                    Stmt::ExprStmt(Expr::Assign {
                        name: "counter".to_string(),
                        value: Box::new(Expr::Binary {
                            left: Box::new(variable("counter")),
                            op: crate::lexer::TokenKind::Plus,
                            right: Box::new(int(1)),
                            span: span(),
                        }),
                        span: span(),
                    }),
                    Stmt::Return(Some(variable("counter"))),
                ],
            ),
            function(
                "helper",
                FunctionVisibility::Private,
                vec![Stmt::Return(Some(int(20)))],
            ),
            get_function(),
            Stmt::VarDecl {
                is_const: false,
                name: "aValue".to_string(),
                ty: None,
                initializer: Expr::MethodCall {
                    receiver: Box::new(variable("a")),
                    name: "get".to_string(),
                    args: vec![],
                    span: span(),
                },
            },
            Stmt::VarDecl {
                is_const: false,
                name: "bValue".to_string(),
                ty: None,
                initializer: Expr::MethodCall {
                    receiver: Box::new(variable("b")),
                    name: "get".to_string(),
                    args: vec![],
                    span: span(),
                },
            },
            Stmt::VarDecl {
                is_const: false,
                name: "directValue".to_string(),
                ty: None,
                initializer: call("directGet"),
            },
            Stmt::VarDecl {
                is_const: false,
                name: "aFirst".to_string(),
                ty: None,
                initializer: Expr::MethodCall {
                    receiver: Box::new(variable("a")),
                    name: "increment".to_string(),
                    args: vec![],
                    span: span(),
                },
            },
            Stmt::VarDecl {
                is_const: false,
                name: "aSecond".to_string(),
                ty: None,
                initializer: Expr::MethodCall {
                    receiver: Box::new(variable("a")),
                    name: "increment".to_string(),
                    args: vec![],
                    span: span(),
                },
            },
        ];
        let statement_provenance = vec![
            module_provenance(&module_a),
            module_provenance(&module_a),
            module_provenance(&module_a),
            module_provenance(&module_a),
            module_provenance(&module_b),
            module_provenance(&module_b),
            module_provenance(&entry),
            module_provenance(&entry),
            module_provenance(&entry),
            module_provenance(&entry),
            module_provenance(&entry),
        ];

        let mut root_bindings = ModuleBindings::default();
        root_bindings
            .direct
            .insert("directGet".to_string(), symbol(&module_a, "get"));
        root_bindings.namespaces.insert(
            "a".to_string(),
            HashMap::from([
                ("get".to_string(), symbol(&module_a, "get")),
                ("increment".to_string(), symbol(&module_a, "increment")),
            ]),
        );
        root_bindings.namespaces.insert(
            "b".to_string(),
            HashMap::from([("get".to_string(), symbol(&module_b, "get"))]),
        );
        let module_bindings = HashMap::from([
            (entry.clone(), root_bindings),
            (module_a.clone(), ModuleBindings::default()),
            (module_b.clone(), ModuleBindings::default()),
        ]);
        let sources = HashMap::from([
            (
                entry.clone(),
                "var aValue = a.get(); var bValue = b.get(); var directValue = directGet(); var aFirst = a.increment(); var aSecond = a.increment();".to_string(),
            ),
            (
                module_a.clone(),
                "func helper(): int { return 10; } pub func get(): int { return helper(); } const counter: int = 0; pub func increment(): int { counter = counter + 1; return counter; }".to_string(),
            ),
            (
                module_b.clone(),
                "func helper(): int { return 20; } pub func get(): int { return helper(); }".to_string(),
            ),
        ]);
        let program = ResolvedProgram {
            statements,
            statementProvenance: statement_provenance,
            sources,
            moduleBindings: module_bindings,
        };

        let mut interpreter = Interpreter::new();
        let mut runtime_statements: HashMap<PathBuf, Vec<Stmt>> = HashMap::new();
        for (statement, provenance) in program
            .statements
            .iter()
            .cloned()
            .zip(&program.statementProvenance)
        {
            runtime_statements
                .entry(provenance.moduleId.clone())
                .or_default()
                .push(statement);
        }
        let module_order = vec![module_a.clone(), module_b.clone(), entry.clone()];
        interpreter
            .execute_resolved_modules(&program, &entry, &module_order, runtime_statements)
            .unwrap();
        assert_eq!(interpreter.environment.get("aValue"), Some(Value::Int(10)));
        assert_eq!(interpreter.environment.get("bValue"), Some(Value::Int(20)));
        assert_eq!(interpreter.environment.get("directValue"), Some(Value::Int(10)));
        assert_eq!(interpreter.environment.get("aFirst"), Some(Value::Int(1)));
        assert_eq!(interpreter.environment.get("aSecond"), Some(Value::Int(2)));
    }

    #[test]
    fn resolvedRuntimeRejectsAQueriedSourceWithNoDependencyMetadata() {
        let entry = PathBuf::from("/virtual/app/main.rey");
        let empty_module = PathBuf::from("/virtual/app/empty/main.rey");
        let mut root_bindings = ModuleBindings::default();
        root_bindings
            .namespaces
            .insert("empty".to_string(), HashMap::new());
        let program = ResolvedProgram {
            statements: vec![],
            statementProvenance: vec![],
            sources: HashMap::from([
                (entry.clone(), String::new()),
                (empty_module.clone(), String::new()),
            ]),
            moduleBindings: HashMap::from([
                (entry.clone(), root_bindings),
                (empty_module.clone(), ModuleBindings::default()),
            ]),
        };
        let mut interpreter = Interpreter::new();
        let error = interpreter.interpret_resolved(&program, &entry).unwrap_err();
        assert!(
            error.contains(&empty_module.display().to_string()),
            "got: {}",
            error
        );
    }

    #[test]
    fn dependencyFirstOrderKeepsResolverOrderForEmptyModulesAndDiamonds() {
        let entry = PathBuf::from("/project/app/main.rey");
        let module_a = PathBuf::from("/project/app/a/main.rey");
        let module_b = PathBuf::from("/project/app/b/main.rey");
        let leaf = PathBuf::from("/project/app/leaf/main.rey");
        let empty_imports = HashMap::from([
            (
                entry.clone(),
                ModuleBindings {
                    dependencies: vec![module_b.clone(), module_a.clone()],
                    ..ModuleBindings::default()
                },
            ),
            (module_a.clone(), ModuleBindings::default()),
            (module_b.clone(), ModuleBindings::default()),
        ]);
        let empty_dependencies = empty_imports
            .iter()
            .map(|(module, bindings)| (module.clone(), bindings.dependencies.clone()))
            .collect::<HashMap<_, _>>();
        assert_eq!(
            dependencyFirstOrder(
                &entry,
                &empty_dependencies,
                &HashSet::from([entry.clone(), module_a.clone(), module_b.clone()]),
            )
            .unwrap(),
            vec![module_b.clone(), module_a.clone(), entry.clone()]
        );

        let dependencies = HashMap::from([
            (entry.clone(), vec![module_b.clone(), module_a.clone(), module_b.clone()]),
            (module_a.clone(), vec![leaf.clone()]),
            (module_b.clone(), vec![leaf.clone()]),
            (leaf.clone(), vec![]),
        ]);
        let known = HashSet::from([
            entry.clone(),
            module_a.clone(),
            module_b.clone(),
            leaf.clone(),
        ]);
        assert_eq!(
            dependencyFirstOrder(&entry, &dependencies, &known).unwrap(),
            vec![leaf.clone(), module_b.clone(), module_a.clone(), entry.clone()]
        );

        let cyclic = HashMap::from([
            (entry.clone(), vec![module_a.clone()]),
            (module_a.clone(), vec![entry.clone()]),
        ]);
        let cycle_error = dependencyFirstOrder(
            &entry,
            &cyclic,
            &HashSet::from([entry.clone(), module_a.clone()]),
        )
        .unwrap_err();
        assert!(cycle_error.contains("Circular runtime module dependency"));

        let missing = HashMap::from([(entry.clone(), vec![leaf.clone()])]);
        let missing_error =
            dependencyFirstOrder(&entry, &missing, &HashSet::from([entry.clone()]))
            .unwrap_err();
        assert!(missing_error.contains("dependency target missing"));
    }

    #[test]
    fn resolvedDiamondInitializesLeafOnceInImportOrder() {
        let entry = PathBuf::from("/project/app/main.rey");
        let module_a = PathBuf::from("/project/app/a/main.rey");
        let module_b = PathBuf::from("/project/app/b/main.rey");
        let leaf = PathBuf::from("/project/app/leaf/main.rey");
        let module_bindings = HashMap::from([
            (
                entry.clone(),
                ModuleBindings {
                    dependencies: vec![module_b.clone(), module_a.clone(), module_b.clone()],
                    ..ModuleBindings::default()
                },
            ),
            (
                module_a.clone(),
                ModuleBindings {
                    dependencies: vec![leaf.clone()],
                    ..ModuleBindings::default()
                },
            ),
            (
                module_b.clone(),
                ModuleBindings {
                    dependencies: vec![leaf.clone()],
                    ..ModuleBindings::default()
                },
            ),
            (leaf.clone(), ModuleBindings::default()),
        ]);
        let dependencies = module_bindings
            .iter()
            .map(|(module, bindings)| (module.clone(), bindings.dependencies.clone()))
            .collect::<HashMap<_, _>>();
        let known = HashSet::from([
            entry.clone(),
            module_a.clone(),
            module_b.clone(),
            leaf.clone(),
        ]);
        let order = dependencyFirstOrder(&entry, &dependencies, &known).unwrap();
        assert_eq!(order, vec![leaf.clone(), module_b.clone(), module_a.clone(), entry.clone()]);

        let program = ResolvedProgram {
            statements: vec![],
            statementProvenance: vec![],
            sources: known
                .iter()
                .cloned()
                .map(|module| (module, String::new()))
                .collect(),
            moduleBindings: module_bindings,
        };
        let push = |value| {
            Stmt::ExprStmt(Expr::MethodCall {
                receiver: Box::new(variable("initLog")),
                name: "push".to_string(),
                args: vec![int(value)],
                span: span(),
            })
        };
        let module_statements = HashMap::from([
            (entry.clone(), vec![]),
            (module_a, vec![push(1)]),
            (module_b, vec![push(2)]),
            (leaf, vec![push(0)]),
        ]);
        let mut interpreter = Interpreter::new();
        interpreter.builtins.define(
            "initLog".to_string(),
            Value::Array(Rc::new(RefCell::new(vec![]))),
        );
        interpreter
            .execute_resolved_modules(&program, &entry, &order, module_statements)
            .unwrap();
        let values = match interpreter.builtins.get("initLog").unwrap() {
            Value::Array(values) => values,
            other => panic!("expected init log array, got {}", other),
        };
        assert_eq!(values.borrow().as_slice(), &[Value::Int(0), Value::Int(2), Value::Int(1)]);
    }

    #[test]
    fn importedInitsRunOnceInSourceOrderAndSkipImportedMain() {
        let leaf = "var steps: int = 1;\nvar ranMain: int = 0;\npub func stamp(): int { steps = steps * 10; return steps; }\npub func mainFlag(): int { return ranMain; }\npub func main(): void { ranMain = 1; }\n";
        let left = "import leaf;\nvar mark: int = leaf.stamp();\npub func fromLeft(): int { return mark; }\n";
        let right = "import leaf;\nvar mark: int = leaf.stamp();\npub func fromRight(): int { return mark; }\n";
        let runCase = |entrySource: &str| {
            let root = fixture(&[
                ("leaf/main.rey", leaf),
                ("left/main.rey", left),
                ("right/main.rey", right),
                ("main.rey", entrySource),
            ]);
            let entry = root.join("main.rey");
            let program = crate::imports::resolveEntry(&entry).unwrap();
            TypeChecker::checkResolvedProgram(&program).unwrap();
            let mut interpreter = Interpreter::new();
            interpreter.interpret_resolved(&program, &entry).unwrap();
            interpreter
        };
        let forward = runCase("import left;\nimport right;\nimport leaf;\nvar leftMark: int = left.fromLeft();\nvar rightMark: int = right.fromRight();\nvar flag: int = leaf.mainFlag();\nvar entryRan: int = 0;\nfunc main(): void { entryRan = 1; }\n");
        assert_eq!(forward.environment.get("leftMark"), Some(Value::Int(10)));
        assert_eq!(forward.environment.get("rightMark"), Some(Value::Int(100)));
        assert_eq!(forward.environment.get("flag"), Some(Value::Int(0)));
        assert_eq!(forward.environment.get("entryRan"), Some(Value::Int(1)));

        let reversed = runCase("import right;\nimport left;\nimport leaf;\nvar leftMark: int = left.fromLeft();\nvar rightMark: int = right.fromRight();\nvar flag: int = leaf.mainFlag();\nvar entryRan: int = 0;\nfunc main(): void { entryRan = 1; }\n");
        assert_eq!(reversed.environment.get("leftMark"), Some(Value::Int(100)));
        assert_eq!(reversed.environment.get("rightMark"), Some(Value::Int(10)));
        assert_eq!(reversed.environment.get("flag"), Some(Value::Int(0)));
        assert_eq!(reversed.environment.get("entryRan"), Some(Value::Int(1)));
    }

    fn module_provenance(module_id: &PathBuf) -> StatementProvenance {
        StatementProvenance {
            moduleId: module_id.clone(),
            span: span(),
        }
    }

    fn symbol(module_id: &PathBuf, name: &str) -> ResolvedSymbol {
        ResolvedSymbol {
            moduleId: module_id.clone(),
            name: name.to_string(),
        }
    }
}
