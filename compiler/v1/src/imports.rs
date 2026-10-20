#![allow(non_snake_case)]

use crate::ast::{Expr, FunctionVisibility, ImportKind, Stmt};
use crate::ast::stmt::ImportName;
use crate::lexer::{span::Span, Lexer, TokenKind};
use crate::parser::Parser;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CompileError {
    #[cfg_attr(not(test), allow(dead_code))]
    pub title: String,
    pub file: PathBuf,
    pub source: String,
    pub span: Span,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedProgram {
    pub statements: Vec<Stmt>,
    pub statementProvenance: Vec<StatementProvenance>,
    pub sources: HashMap<PathBuf, String>,
    pub moduleBindings: HashMap<PathBuf, ModuleBindings>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResolvedSymbol {
    pub moduleId: PathBuf,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct ModuleBindings {
    pub direct: HashMap<String, ResolvedSymbol>,
    pub types: HashMap<String, ResolvedSymbol>,
    pub namespaces: HashMap<String, HashMap<String, ResolvedSymbol>>,
    pub dependencies: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatementProvenance {
    pub moduleId: PathBuf,
    pub span: Span,
}

#[derive(Debug, Clone)]
struct ResolvedFile {
    statements: Vec<Stmt>,
    statementProvenance: Vec<StatementProvenance>,
    declStatements: Vec<Stmt>,
    declKeys: Vec<DeclKey>,
    declProvenance: Vec<StatementProvenance>,
    localFunctionVisibility: HashMap<String, FunctionVisibility>,
    localTypeVisibility: HashMap<String, bool>,
    moduleBindings: ModuleBindings,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DeclKey {
    Source { file: PathBuf, index: usize },
    Namespace { target: PathBuf, name: String },
}

pub fn resolveEntry(entryPath: &Path) -> Result<ResolvedProgram, CompileError> {
    let canonicalEntry = canonicalPath(entryPath);
    let projectRoot = canonicalEntry
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let mut state = ResolverState::new(projectRoot);
    let resolved = state.resolveFile(&canonicalEntry)?;
    Ok(ResolvedProgram {
        statements: resolved.statements,
        statementProvenance: resolved.statementProvenance,
        sources: state.sources,
        moduleBindings: state.moduleBindings,
    })
}

struct ResolverState {
    cache: HashMap<PathBuf, ResolvedFile>,
    stack: Vec<PathBuf>,
    projectRoot: PathBuf,
    sources: HashMap<PathBuf, String>,
    moduleBindings: HashMap<PathBuf, ModuleBindings>,
}

impl ResolverState {
    fn new(projectRoot: PathBuf) -> Self {
        Self {
            cache: HashMap::new(),
            stack: Vec::new(),
            projectRoot,
            sources: HashMap::new(),
            moduleBindings: HashMap::new(),
        }
    }

    fn resolveFile(&mut self, filePath: &Path) -> Result<ResolvedFile, CompileError> {
        let filePath = canonicalPath(filePath);
        if let Some(cached) = self.cache.get(&filePath) {
            return Ok(cached.clone());
        }

        // cycle checks are reported from import sites to preserve file/line context.
        self.stack.push(filePath.clone());
        let result = (|| -> Result<ResolvedFile, CompileError> {
            let source = fs::read_to_string(&filePath).map_err(|_| CompileError {
                title: "import".to_string(),
                file: filePath.clone(),
                source: String::new(),
                span: Span::new(0, 1),
                message: format!("File not found: '{}'", filePath.display()),
            })?;
            self.sources.insert(filePath.clone(), source.clone());

            let statements = parseSource(&filePath, &source)?;
            let currentDir = filePath
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf();

            let mut localFunctionVisibility = HashMap::new();
            let mut localTypeVisibility = HashMap::new();
            for stmt in &statements {
                if let Stmt::FuncDecl {
                    name, visibility, ..
                } = stmt
                {
                    localFunctionVisibility.insert(name.clone(), *visibility);
                }
                match stmt {
                    Stmt::StructDecl { name, is_pub, .. }
                    | Stmt::EnumDecl { name, is_pub, .. }
                    | Stmt::EnumDeclWithPayload { name, is_pub, .. } => {
                        localTypeVisibility.insert(name.clone(), *is_pub);
                    }
                    _ => {}
                }
            }

            let mut resolved = Vec::new();
            let mut statementProvenance = Vec::new();
            let mut declStatements = Vec::new();
            let mut declKeys = Vec::new();
            let mut declProvenance = Vec::new();
            let mut moduleBindings = ModuleBindings::default();
            let mut injectedNames = HashMap::new();
            let mut includedDeclarations = HashSet::new();
            let mut localDeclIndex = 0;
            for stmt in statements {
                match stmt {
                    Stmt::Import { kind, span } => {
                        self.resolveImport(
                            &filePath,
                            &source,
                            &currentDir,
                            kind,
                            span,
                            &mut resolved,
                            &mut statementProvenance,
                            &mut declStatements,
                            &mut declKeys,
                            &mut declProvenance,
                            &mut moduleBindings,
                            &mut injectedNames,
                            &mut includedDeclarations,
                        )?;
                    }
                    other => {
                        if isDeclaration(&other) {
                            let key = DeclKey::Source {
                                file: filePath.clone(),
                                index: localDeclIndex,
                            };
                            localDeclIndex += 1;
                            includedDeclarations.insert(key.clone());
                            declStatements.push(other.clone());
                            declKeys.push(key);
                            declProvenance.push(StatementProvenance {
                                moduleId: filePath.clone(),
                                span: statementSpan(&other),
                            });
                        }
                        statementProvenance.push(StatementProvenance {
                            moduleId: filePath.clone(),
                            span: statementSpan(&other),
                        });
                        resolved.push(other);
                    }
                }
            }

            Ok(ResolvedFile {
                statements: resolved,
                statementProvenance,
                declStatements,
                declKeys,
                declProvenance,
                localFunctionVisibility,
                localTypeVisibility,
                moduleBindings,
            })
        })();
        self.stack.pop();

        let file = result?;
        self.moduleBindings
            .insert(filePath.clone(), file.moduleBindings.clone());
        self.cache.insert(filePath, file.clone());
        Ok(file)
    }

    #[allow(clippy::too_many_arguments)]
    fn resolveImport(
        &mut self,
        ownerFile: &Path,
        ownerSource: &str,
        currentDir: &Path,
        kind: ImportKind,
        span: Span,
        resolved: &mut Vec<Stmt>,
        statementProvenance: &mut Vec<StatementProvenance>,
        declStatements: &mut Vec<Stmt>,
        declKeys: &mut Vec<DeclKey>,
        declProvenance: &mut Vec<StatementProvenance>,
        moduleBindings: &mut ModuleBindings,
        injectedNames: &mut HashMap<String, PathBuf>,
        includedDeclarations: &mut HashSet<DeclKey>,
    ) -> Result<(), CompileError> {
        match kind {
            ImportKind::FileSymbols { module, symbols } => {
                let importFile = match self.findFileImport(currentDir, &module, ownerFile, ownerSource, span) {
                    Ok(importFile) => importFile,
                    Err(fileError) => {
                        if symbols.is_empty()
                            || self.findModuleDir(currentDir, &module, ownerFile, ownerSource, span).is_err()
                        {
                            return Err(fileError);
                        }
                        for symbol in symbols {
                            self.resolveImport(
                                ownerFile,
                                ownerSource,
                                currentDir,
                                ImportKind::ModuleItems {
                                    module: module.clone(),
                                    items: vec![symbol.clone()],
                                },
                                symbol.span,
                                resolved,
                                statementProvenance,
                                declStatements,
                                declKeys,
                                declProvenance,
                                moduleBindings,
                                injectedNames,
                                includedDeclarations,
                            )?;
                        }
                        return Ok(());
                    }
                };
                if self.stack.contains(&importFile) {
                    return Err(self.circularError(ownerFile, ownerSource, span, &importFile));
                }
                let imported = self.resolveFile(&importFile)?;
                recordDependency(moduleBindings, &importFile);
                appendDeclarations(
                    &imported,
                    resolved,
                    statementProvenance,
                    declStatements,
                    declKeys,
                    declProvenance,
                    includedDeclarations,
                );

                if symbols.is_empty() {
                    for (name, visibility) in &imported.localFunctionVisibility {
                        if !isImportable(*visibility) {
                            continue;
                        }
                        if let Some(existing) = injectedNames.get(name) {
                            if existing != &importFile {
                                return Err(CompileError {
                                    title: "import".to_string(),
                                    file: ownerFile.to_path_buf(),
                                    source: ownerSource.to_string(),
                                    span,
                                    message: format!("Duplicate import: '{}'", name),
                                });
                            }
                        } else {
                            injectedNames.insert(name.clone(), importFile.clone());
                        }
                        moduleBindings.direct.insert(name.clone(), ResolvedSymbol {
                            moduleId: importFile.clone(),
                            name: name.clone(),
                        });
                    }
                    for (name, isPub) in &imported.localTypeVisibility {
                        if !isPub { continue; }
                        if let Some(existing) = injectedNames.get(name) {
                            if existing != &importFile {
                                return Err(CompileError {
                                    title: "import".to_string(),
                                    file: ownerFile.to_path_buf(),
                                    source: ownerSource.to_string(),
                                    span,
                                    message: format!("Duplicate import: '{}'", name),
                                });
                            }
                        } else {
                            injectedNames.insert(name.clone(), importFile.clone());
                        }
                        moduleBindings.types.insert(name.clone(), ResolvedSymbol {
                            moduleId: importFile.clone(),
                            name: name.clone(),
                        });
                    }
                }

                for symbol in symbols {
                    if let Some(existing) = injectedNames.get(&symbol.name) {
                        if existing != &importFile {
                            return Err(CompileError {
                                title: "import".to_string(),
                                file: ownerFile.to_path_buf(),
                                source: ownerSource.to_string(),
                                span: symbol.span,
                                message: format!("Duplicate import: '{}'", symbol.name),
                            });
                        }
                    } else {
                        injectedNames.insert(symbol.name.clone(), importFile.clone());
                    }
                    match imported.localFunctionVisibility.get(&symbol.name) {
                        Some(FunctionVisibility::Pub) => {
                            moduleBindings.direct.insert(symbol.name.clone(), ResolvedSymbol {
                                moduleId: importFile.clone(),
                                name: symbol.name.clone(),
                            });
                        }
                        Some(FunctionVisibility::Private) => {
                            return Err(CompileError {
                                title: "import".to_string(),
                                file: ownerFile.to_path_buf(),
                                source: ownerSource.to_string(),
                                span: symbol.span,
                                message: format!(
                                    "Function '{}' exists in '{}' but is private",
                                    symbol.name,
                                    importFile.display()
                                ),
                            });
                        }
                        None => {
                            match imported.localTypeVisibility.get(&symbol.name) {
                                Some(true) => {
                                    moduleBindings.types.insert(symbol.name.clone(), ResolvedSymbol {
                                        moduleId: importFile.clone(),
                                        name: symbol.name.clone(),
                                    });
                                }
                                Some(false) => return Err(CompileError {
                                    title: "import".to_string(),
                                    file: ownerFile.to_path_buf(),
                                    source: ownerSource.to_string(),
                                    span: symbol.span,
                                    message: format!("Type '{}' exists in '{}' but is private", symbol.name, importFile.display()),
                                }),
                                None => return Err(CompileError {
                                    title: "import".to_string(),
                                    file: ownerFile.to_path_buf(),
                                    source: ownerSource.to_string(),
                                    span: symbol.span,
                                    message: format!("Function or type '{}' not found in file '{}'", symbol.name, importFile.display()),
                                }),
                            }
                        }
                    }
                }
                Ok(())
            }
            ImportKind::ModuleNamespace { module } => {
                if isFilePathImport(&module) {
                    let importFile =
                        self.findPathImport(currentDir, &module, ownerFile, ownerSource, span)?;
                    if self.stack.contains(&importFile) {
                        return Err(self.circularError(ownerFile, ownerSource, span, &importFile));
                    }
                    let imported = self.resolveFile(&importFile)?;
                    recordDependency(moduleBindings, &importFile);
                    appendDeclarations(
                        &imported,
                        resolved,
                        statementProvenance,
                        declStatements,
                        declKeys,
                        declProvenance,
                        includedDeclarations,
                    );
                    for (name, visibility) in &imported.localFunctionVisibility {
                        if !isImportable(*visibility) {
                            continue;
                        }
                        if let Some(existing) = injectedNames.get(name) {
                            if existing != &importFile {
                                return Err(CompileError {
                                    title: "import".to_string(),
                                    file: ownerFile.to_path_buf(),
                                    source: ownerSource.to_string(),
                                    span,
                                    message: format!("Duplicate import: '{}'", name),
                                });
                            }
                        } else {
                            injectedNames.insert(name.clone(), importFile.clone());
                        }
                        moduleBindings.direct.insert(name.clone(), ResolvedSymbol {
                            moduleId: importFile.clone(),
                            name: name.clone(),
                        });
                    }
                    for (name, isPub) in &imported.localTypeVisibility {
                        if !isPub { continue; }
                        if let Some(existing) = injectedNames.get(name) {
                            if existing != &importFile {
                                return Err(CompileError {
                                    title: "import".to_string(),
                                    file: ownerFile.to_path_buf(),
                                    source: ownerSource.to_string(),
                                    span,
                                    message: format!("Duplicate import: '{}'", name),
                                });
                            }
                        } else {
                            injectedNames.insert(name.clone(), importFile.clone());
                        }
                        moduleBindings.types.insert(name.clone(), ResolvedSymbol {
                            moduleId: importFile.clone(),
                            name: name.clone(),
                        });
                    }
                    return Ok(());
                }

                let (moduleDir, moduleMain) = if let Some((parent, item)) = module.rsplit_once("::") {
                    let file = self.findModuleItemFile(
                        currentDir,
                        parent,
                        item,
                        ownerFile,
                        ownerSource,
                        span,
                    )?;
                    (file.clone(), file)
                } else {
                    let moduleDir =
                        self.findModuleDir(currentDir, &module, ownerFile, ownerSource, span)?;
                    let moduleMain = canonicalPath(&moduleDir.join("main.rey"));
                    if !moduleMain.is_file() {
                        return Err(CompileError {
                            title: "import".to_string(),
                            file: ownerFile.to_path_buf(),
                            source: ownerSource.to_string(),
                            span,
                            message: format!(
                                "Folder '{}' is not a module: missing main.rey",
                                moduleDir.display()
                            ),
                        });
                    }
                    (moduleDir, moduleMain)
                };
                let namespaceName = moduleName(&module);
                if let Some(existing) = injectedNames.get(namespaceName) {
                    if existing == &moduleDir {
                        return Ok(());
                    }
                    return Err(CompileError {
                        title: "import".to_string(),
                        file: ownerFile.to_path_buf(),
                        source: ownerSource.to_string(),
                        span,
                        message: format!("Duplicate import: '{}'", namespaceName),
                    });
                }
                injectedNames.insert(namespaceName.to_string(), moduleDir.clone());

                let mut exportedSymbols = HashSet::new();
                let mut namespaceEntries = Vec::new();
                let mut namespaceSymbols = HashMap::new();
                let mut publicTypes = HashMap::new();
                // A directory module is main.rey only. Sibling .rey files are not part of it.
                let moduleFiles = vec![canonicalPath(&moduleMain)];

                for path in moduleFiles {
                    let path = canonicalPath(&path);
                    if self.stack.contains(&path) {
                        return Err(self.circularError(ownerFile, ownerSource, span, &path));
                    }
                    let imported = self.resolveFile(&path)?;
                    recordDependency(moduleBindings, &path);
                    appendDeclarations(
                        &imported,
                        resolved,
                        statementProvenance,
                        declStatements,
                        declKeys,
                        declProvenance,
                        includedDeclarations,
                    );

                    for (name, visibility) in &imported.localFunctionVisibility {
                        if isImportable(*visibility) {
                            if !exportedSymbols.insert(name.clone()) {
                                return Err(CompileError {
                                    title: "import".to_string(),
                                    file: ownerFile.to_path_buf(),
                                    source: ownerSource.to_string(),
                                    span,
                                    message: format!(
                                        "Duplicate exported function '{}' in module '{}'",
                                        name, module
                                    ),
                                });
                            }
                            namespaceEntries.push((
                                name.clone(),
                                Expr::Variable {
                                    name: name.clone(),
                                    span,
                                },
                            ));
                            namespaceSymbols.insert(name.clone(), ResolvedSymbol {
                                moduleId: path.clone(),
                                name: name.clone(),
                            });
                        }
                    }
                    for (name, isPub) in &imported.localTypeVisibility {
                        if !isPub { continue; }
                        if publicTypes.insert(name.clone(), path.clone()).is_some() {
                            return Err(CompileError {
                                title: "import".to_string(),
                                file: ownerFile.to_path_buf(),
                                source: ownerSource.to_string(),
                                span,
                                message: format!("Duplicate exported type '{}' in module '{}'", name, module),
                            });
                        }
                    }
                }

                let namespace = self.namespaceStmt(namespaceName, namespaceEntries, span);
                appendNamespace(
                    namespace,
                    ownerFile,
                    &moduleDir,
                    namespaceName,
                    resolved,
                    statementProvenance,
                    declStatements,
                    declKeys,
                    declProvenance,
                    includedDeclarations,
                );
                moduleBindings.namespaces.insert(namespaceName.to_string(), namespaceSymbols);
                for (name, moduleId) in publicTypes {
                    moduleBindings.types.insert(name.clone(), ResolvedSymbol { moduleId, name });
                }
                Ok(())
            }
            ImportKind::ModuleItems { module, items } => {
                for item in items {
                    if let Ok(moduleDir) = self.findModuleDir(
                        currentDir,
                        &module,
                        ownerFile,
                        ownerSource,
                        item.span,
                    ) {
                        if moduleDir.join("main.rey").is_file() {
                            self.resolveSelectedSymbolFromDirectory(
                                ownerFile,
                                ownerSource,
                                &module,
                                &moduleDir,
                                &item,
                                resolved,
                                statementProvenance,
                                declStatements,
                                declKeys,
                                declProvenance,
                                moduleBindings,
                                injectedNames,
                                includedDeclarations,
                            )?;
                            continue;
                        }
                    }
                    let importFile = self.findModuleItemFile(
                        currentDir,
                        &module,
                        &item.name,
                        ownerFile,
                        ownerSource,
                        item.span,
                    )?;
                    if self.stack.contains(&importFile) {
                        return Err(self.circularError(
                            ownerFile,
                            ownerSource,
                            item.span,
                            &importFile,
                        ));
                    }
                    let imported = self.resolveFile(&importFile)?;
                    recordDependency(moduleBindings, &importFile);
                    if let Some(existing) = injectedNames.get(&item.name) {
                        if existing == &importFile {
                            continue;
                        }
                        return Err(CompileError {
                            title: "import".to_string(),
                            file: ownerFile.to_path_buf(),
                            source: ownerSource.to_string(),
                            span: item.span,
                            message: format!("Duplicate import: '{}'", item.name),
                        });
                    }
                    injectedNames.insert(item.name.clone(), importFile.clone());
                    appendDeclarations(
                        &imported,
                        resolved,
                        statementProvenance,
                        declStatements,
                        declKeys,
                        declProvenance,
                        includedDeclarations,
                    );

                    let mut namespaceEntries = Vec::new();
                    let mut namespaceSymbols = HashMap::new();
                    for (stmt, provenance) in imported.declStatements.iter().zip(&imported.declProvenance) {
                        if let Stmt::FuncDecl {
                            name, visibility, ..
                        } = stmt
                        {
                            if isImportable(*visibility) {
                                namespaceEntries.push((
                                    name.clone(),
                                    Expr::Variable {
                                        name: name.clone(),
                                        span: item.span,
                                    },
                                ));
                                namespaceSymbols.insert(name.clone(), ResolvedSymbol {
                                    moduleId: provenance.moduleId.clone(),
                                    name: name.clone(),
                                });
                            }
                        }
                        let publicType = match stmt {
                            Stmt::StructDecl { name, is_pub, .. }
                            | Stmt::EnumDecl { name, is_pub, .. }
                            | Stmt::EnumDeclWithPayload { name, is_pub, .. } if *is_pub => Some(name),
                            _ => None,
                        };
                        if let Some(name) = publicType {
                            moduleBindings.types.insert(name.clone(), ResolvedSymbol {
                                moduleId: provenance.moduleId.clone(),
                                name: name.clone(),
                            });
                        }
                    }
                    let namespace = self.namespaceStmt(&item.name, namespaceEntries, item.span);
                    appendNamespace(
                        namespace,
                        ownerFile,
                        &importFile,
                        &item.name,
                        resolved,
                        statementProvenance,
                        declStatements,
                        declKeys,
                        declProvenance,
                        includedDeclarations,
                    );
                    moduleBindings.namespaces.insert(item.name.clone(), namespaceSymbols);
                }
                Ok(())
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn resolveSelectedSymbolFromDirectory(
        &mut self,
        ownerFile: &Path,
        ownerSource: &str,
        module: &str,
        moduleDir: &Path,
        selected: &ImportName,
        resolved: &mut Vec<Stmt>,
        statementProvenance: &mut Vec<StatementProvenance>,
        declStatements: &mut Vec<Stmt>,
        declKeys: &mut Vec<DeclKey>,
        declProvenance: &mut Vec<StatementProvenance>,
        moduleBindings: &mut ModuleBindings,
        injectedNames: &mut HashMap<String, PathBuf>,
        includedDeclarations: &mut HashSet<DeclKey>,
    ) -> Result<(), CompileError> {
        let mainFile = canonicalPath(&moduleDir.join("main.rey"));
        if !mainFile.is_file() {
            return Err(CompileError {
                title: "import".to_string(),
                file: ownerFile.to_path_buf(),
                source: ownerSource.to_string(),
                span: selected.span,
                message: format!(
                    "Function or type '{}' not found in module '{}'",
                    selected.name, module
                ),
            });
        }
        let files = vec![mainFile];

        let mut publicFunctions = Vec::new();
        let mut privateFunction = None;
        let mut publicTypes = Vec::new();
        let mut privateType = None;
        for path in &files {
            let source = fs::read_to_string(path).map_err(|_| CompileError {
                title: "import".to_string(),
                file: path.clone(),
                source: String::new(),
                span: selected.span,
                message: format!("File not found: '{}'", path.display()),
            })?;
            for statement in parseSource(path, &source)? {
                match statement {
                    Stmt::FuncDecl { name, visibility, .. } if name == selected.name => {
                        if isImportable(visibility) {
                            publicFunctions.push(path.clone());
                        } else {
                            privateFunction.get_or_insert_with(|| path.clone());
                        }
                    }
                    Stmt::StructDecl { name, is_pub, .. }
                    | Stmt::EnumDecl { name, is_pub, .. }
                    | Stmt::EnumDeclWithPayload { name, is_pub, .. }
                        if name == selected.name =>
                    {
                        if is_pub {
                            publicTypes.push(path.clone());
                        } else {
                            privateType.get_or_insert_with(|| path.clone());
                        }
                    }
                    _ => {}
                }
            }
        }

        let exportedCount = publicFunctions.len() + publicTypes.len();
        if exportedCount > 1 {
            return Err(CompileError {
                title: "import".to_string(),
                file: ownerFile.to_path_buf(),
                source: ownerSource.to_string(),
                span: selected.span,
                message: format!("Duplicate exported symbol '{}' in module '{}'", selected.name, module),
            });
        }
        let (sourceFile, isFunction) = if let Some(path) = publicFunctions.pop() {
            (path, true)
        } else if let Some(path) = publicTypes.pop() {
            (path, false)
        } else if let Some(path) = privateFunction {
            return Err(CompileError {
                title: "import".to_string(),
                file: ownerFile.to_path_buf(),
                source: ownerSource.to_string(),
                span: selected.span,
                message: format!("Function '{}' exists in '{}' but is private", selected.name, path.display()),
            });
        } else if let Some(path) = privateType {
            return Err(CompileError {
                title: "import".to_string(),
                file: ownerFile.to_path_buf(),
                source: ownerSource.to_string(),
                span: selected.span,
                message: format!("Type '{}' exists in '{}' but is private", selected.name, path.display()),
            });
        } else {
            return Err(CompileError {
                title: "import".to_string(),
                file: ownerFile.to_path_buf(),
                source: ownerSource.to_string(),
                span: selected.span,
                message: format!("Function or type '{}' not found in module '{}'", selected.name, module),
            });
        };

        if let Some(existing) = injectedNames.get(&selected.name) {
            if existing != &sourceFile {
                return Err(CompileError {
                    title: "import".to_string(),
                    file: ownerFile.to_path_buf(),
                    source: ownerSource.to_string(),
                    span: selected.span,
                    message: format!("Duplicate import: '{}'", selected.name),
                });
            }
        } else {
            injectedNames.insert(selected.name.clone(), sourceFile.clone());
        }

        let imported = self.resolveFile(&sourceFile)?;
        recordDependency(moduleBindings, &sourceFile);
        appendDeclarations(
            &imported,
            resolved,
            statementProvenance,
            declStatements,
            declKeys,
            declProvenance,
            includedDeclarations,
        );
        let symbol = ResolvedSymbol { moduleId: sourceFile, name: selected.name.clone() };
        if isFunction {
            moduleBindings.direct.insert(selected.name.clone(), symbol);
        } else {
            moduleBindings.types.insert(selected.name.clone(), symbol);
        }
        Ok(())
    }

    fn circularError(
        &self,
        ownerFile: &Path,
        ownerSource: &str,
        span: Span,
        target: &Path,
    ) -> CompileError {
        let mut chain = self
            .stack
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>();
        chain.push(target.display().to_string());
        CompileError {
            title: "import".to_string(),
            file: ownerFile.to_path_buf(),
            source: ownerSource.to_string(),
            span,
            message: format!("Circular import detected: {}", chain.join(" -> ")),
        }
    }

    fn namespaceStmt(&self, name: &str, entries: Vec<(String, Expr)>, span: Span) -> Stmt {
        Stmt::VarDecl {
            is_const: true,
            name: name.to_string(),
            ty: None,
            initializer: Expr::DictLiteral { entries, span },
        }
    }

    fn findFileImport(
        &self,
        currentDir: &Path,
        module: &str,
        ownerFile: &Path,
        ownerSource: &str,
        span: Span,
    ) -> Result<PathBuf, CompileError> {
        let mut candidates = Vec::new();
        candidates.push(currentDir.join(format!("{}.rey", module)));
        candidates.push(currentDir.join("src").join(format!("{}.rey", module)));
        candidates.push(self.projectRoot.join(format!("{}.rey", module)));
        candidates.push(self.projectRoot.join("src").join(format!("{}.rey", module)));
        if let Some(home) = homePath() {
            candidates.push(home.join(".reyc/std/src").join(format!("{}.rey", module)));
        }
        if let Some(home) = homePath() {
            candidates.push(home.join(".reyc/packages").join(format!("{}.rey", module)));
        }

        for candidate in candidates {
            if candidate.exists() {
                return Ok(canonicalPath(&candidate));
            }
        }

        Err(CompileError {
            title: "import".to_string(),
            file: ownerFile.to_path_buf(),
            source: ownerSource.to_string(),
            span,
            message: format!("File not found for import '{}.rey'", module),
        })
    }

    fn findPathImport(
        &self,
        currentDir: &Path,
        path: &str,
        ownerFile: &Path,
        ownerSource: &str,
        span: Span,
    ) -> Result<PathBuf, CompileError> {
        let path = Path::new(path);
        let mut candidates = Vec::new();
        if path.is_absolute() {
            candidates.push(path.to_path_buf());
        } else {
            candidates.push(currentDir.join(path));
            candidates.push(currentDir.join("src").join(path));
            candidates.push(self.projectRoot.join(path));
            candidates.push(self.projectRoot.join("src").join(path));
        }
        for candidate in candidates {
            if candidate.is_file() {
                return Ok(canonicalPath(&candidate));
            }
        }
        Err(CompileError {
            title: "import".to_string(),
            file: ownerFile.to_path_buf(),
            source: ownerSource.to_string(),
            span,
            message: format!("File not found for import path '{}'", path.display()),
        })
    }

    fn findModuleDir(
        &self,
        currentDir: &Path,
        module: &str,
        ownerFile: &Path,
        ownerSource: &str,
        span: Span,
    ) -> Result<PathBuf, CompileError> {
        let modulePath = modulePath(module);
        let mut candidates = Vec::new();
        candidates.push(currentDir.join(&modulePath));
        candidates.push(currentDir.join("src").join(&modulePath));
        candidates.push(self.projectRoot.join(&modulePath));
        candidates.push(self.projectRoot.join("src").join(&modulePath));
        if module == "std" {
            for base in stdBases() {
                candidates.push(base);
            }
        } else if let Some(stdRelative) = module.strip_prefix("std::") {
            for base in stdBases() {
                candidates.push(base.join(stdRelative.replace("::", "/")));
            }
        }
        if let Some(home) = homePath() {
            candidates.push(home.join(".reyc/packages").join(&modulePath));
        }

        for candidate in candidates {
            if candidate.is_dir() {
                return Ok(canonicalPath(&candidate));
            }
        }

        Err(CompileError {
            title: "import".to_string(),
            file: ownerFile.to_path_buf(),
            source: ownerSource.to_string(),
            span,
            message: format!("Module folder not found: '{}'", module),
        })
    }

    fn findModuleItemFile(
        &self,
        currentDir: &Path,
        module: &str,
        item: &str,
        ownerFile: &Path,
        ownerSource: &str,
        span: Span,
    ) -> Result<PathBuf, CompileError> {
        let modulePath = modulePath(module);
        let mut candidates = Vec::new();
        candidates.push(currentDir.join(&modulePath).join(item).join("main.rey"));
        candidates.push(currentDir.join(&modulePath).join(format!("{}.rey", item)));
        candidates.push(
            currentDir
                .join("src")
                .join(&modulePath)
                .join(item)
                .join("main.rey"),
        );
        candidates.push(
            currentDir
                .join("src")
                .join(&modulePath)
                .join(format!("{}.rey", item)),
        );
        candidates.push(
            self.projectRoot
                .join(&modulePath)
                .join(item)
                .join("main.rey"),
        );
        candidates.push(
            self.projectRoot
                .join(&modulePath)
                .join(format!("{}.rey", item)),
        );
        candidates.push(
            self.projectRoot
                .join("src")
                .join(&modulePath)
                .join(item)
                .join("main.rey"),
        );
        candidates.push(
            self.projectRoot
                .join("src")
                .join(&modulePath)
                .join(format!("{}.rey", item)),
        );
        if module == "std" {
            for base in stdBases() {
                candidates.push(base.join(item).join("main.rey"));
                candidates.push(base.join(format!("{}.rey", item)));
            }
        } else if let Some(stdRelative) = module.strip_prefix("std::") {
            for base in stdBases() {
                let target = base.join(stdRelative.replace("::", "/"));
                candidates.push(target.join(item).join("main.rey"));
                candidates.push(target.join(format!("{}.rey", item)));
            }
        }
        if let Some(home) = homePath() {
            candidates.push(
                home.join(".reyc/packages")
                    .join(&modulePath)
                    .join(item)
                    .join("main.rey"),
            );
            candidates.push(
                home.join(".reyc/packages")
                    .join(&modulePath)
                    .join(format!("{}.rey", item)),
            );
        }

        for candidate in candidates {
            if fileExistsWithExactCase(&candidate) {
                return Ok(canonicalPath(&candidate));
            }
        }

        Err(CompileError {
            title: "import".to_string(),
            file: ownerFile.to_path_buf(),
            source: ownerSource.to_string(),
            span,
            message: format!("File not found for module import '{}::{}'", module, item),
        })
    }
}

fn isFilePathImport(value: &str) -> bool {
    let path = Path::new(value);
    path.is_absolute()
        || value.contains('/')
        || path.extension().is_some_and(|extension| extension == "rey")
}

fn fileExistsWithExactCase(path: &Path) -> bool {
    let mut current = if path.is_absolute() {
        PathBuf::new()
    } else {
        match env::current_dir() {
            Ok(current) => current,
            Err(_) => return false,
        }
    };

    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => current.push(prefix.as_os_str()),
            std::path::Component::RootDir => current.push(component.as_os_str()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => current.push(component.as_os_str()),
            std::path::Component::Normal(name) => {
                let Ok(entries) = fs::read_dir(&current) else {
                    return false;
                };
                if !entries
                    .filter_map(Result::ok)
                    .any(|entry| entry.file_name() == name)
                {
                    return false;
                }
                current.push(name);
            }
        }
    }

    current.is_file()
}

fn moduleName(module: &str) -> &str {
    module.rsplit("::").next().unwrap_or(module)
}

fn modulePath(module: &str) -> PathBuf {
    module
        .split("::")
        .filter(|part| !part.is_empty())
        .collect::<PathBuf>()
}

fn isDeclaration(stmt: &Stmt) -> bool {
    matches!(
        stmt,
        Stmt::FuncDecl { .. }
            | Stmt::StructDecl { .. }
            | Stmt::EnumDecl { .. }
            | Stmt::EnumDeclWithPayload { .. }
            | Stmt::VarDecl { .. }
    )
}

fn isImportable(visibility: FunctionVisibility) -> bool {
    matches!(visibility, FunctionVisibility::Pub)
}

fn appendDeclarations(
    imported: &ResolvedFile,
    resolved: &mut Vec<Stmt>,
    statementProvenance: &mut Vec<StatementProvenance>,
    declStatements: &mut Vec<Stmt>,
    declKeys: &mut Vec<DeclKey>,
    declProvenance: &mut Vec<StatementProvenance>,
    includedDeclarations: &mut HashSet<DeclKey>,
) {
    for ((stmt, key), provenance) in imported
        .declStatements
        .iter()
        .zip(&imported.declKeys)
        .zip(&imported.declProvenance)
    {
        if includedDeclarations.insert(key.clone()) {
            resolved.push(stmt.clone());
            statementProvenance.push(provenance.clone());
            declStatements.push(stmt.clone());
            declKeys.push(key.clone());
            declProvenance.push(provenance.clone());
        }
    }
}

fn appendNamespace(
    namespace: Stmt,
    ownerFile: &Path,
    target: &Path,
    name: &str,
    resolved: &mut Vec<Stmt>,
    statementProvenance: &mut Vec<StatementProvenance>,
    declStatements: &mut Vec<Stmt>,
    declKeys: &mut Vec<DeclKey>,
    declProvenance: &mut Vec<StatementProvenance>,
    includedDeclarations: &mut HashSet<DeclKey>,
) {
    let key = DeclKey::Namespace {
        target: canonicalPath(target),
        name: name.to_string(),
    };
    if includedDeclarations.insert(key.clone()) {
        let provenance = StatementProvenance {
            moduleId: canonicalPath(ownerFile),
            span: statementSpan(&namespace),
        };
        resolved.push(namespace.clone());
        statementProvenance.push(provenance.clone());
        declStatements.push(namespace);
        declKeys.push(key);
        declProvenance.push(provenance);
    }
}

fn recordDependency(bindings: &mut ModuleBindings, source: &Path) {
    let moduleId = canonicalPath(source);
    if !bindings.dependencies.contains(&moduleId) {
        bindings.dependencies.push(moduleId);
    }
}

fn statementSpan(stmt: &Stmt) -> Span {
    match stmt {
        Stmt::VarDecl { initializer, .. } => initializer.span(),
        Stmt::FuncDecl { body, .. } => body.first().map(statementSpan).unwrap_or(Span::new(0, 0)),
        Stmt::StructDecl { methods, .. } => methods
            .iter()
            .flat_map(|method| &method.body)
            .next()
            .map(statementSpan)
            .unwrap_or(Span::new(0, 0)),
        Stmt::Import { span, .. } => *span,
        Stmt::If { condition, .. } | Stmt::While { condition, .. } => condition.span(),
        Stmt::Loop { body } => body.first().map(statementSpan).unwrap_or(Span::new(0, 0)),
        Stmt::For { iterator, .. } => match iterator {
            crate::ast::stmt::ForIterator::Range { start, .. } => start.span(),
            crate::ast::stmt::ForIterator::Array(expr) => expr.span(),
        },
        Stmt::EnumDecl { .. } | Stmt::EnumDeclWithPayload { .. } => Span::new(0, 0),
        Stmt::Match { expr, .. } => expr.span(),
        Stmt::Return(Some(expr)) | Stmt::ExprStmt(expr) => expr.span(),
        Stmt::Return(None) | Stmt::Break | Stmt::Continue => Span::new(0, 0),
    }
}

fn parseSource(filePath: &Path, source: &str) -> Result<Vec<Stmt>, CompileError> {
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();
    loop {
        match lexer.nextToken() {
            Ok(token) => {
                tokens.push(token.clone());
                if token.kind == TokenKind::Eof {
                    break;
                }
            }
            Err(err) => {
                return Err(CompileError {
                    title: "lexer".to_string(),
                    file: filePath.to_path_buf(),
                    source: source.to_string(),
                    span: *err.span(),
                    message: err.message(),
                });
            }
        }
    }

    let mut parser = Parser::new(tokens);
    parser.parse().map_err(|err| CompileError {
        title: "syntax".to_string(),
        file: filePath.to_path_buf(),
        source: source.to_string(),
        span: *err.span(),
        message: err.message(),
    })
}

fn canonicalPath(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn homePath() -> Option<PathBuf> {
    env::var("HOME").ok().map(PathBuf::from)
}

fn stdBases() -> Vec<PathBuf> {
    let mut bases = Vec::new();
    if let Ok(val) = env::var("REY_STD_PATH") {
        let p = PathBuf::from(val);
        if p.join("src").is_dir() {
            bases.push(p.join("src"));
        } else {
            bases.push(p);
        }
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            bases.push(parent.join("std/src"));
            bases.push(parent.join("../std/src"));
            bases.push(parent.join("../../std/src"));
            bases.push(parent.join("../lib/rey/std/src"));
        }
    }
    if let Some(home) = homePath() {
        bases.push(home.join(".reyc/std/src"));
        bases.push(home.join(".capy/std/src"));
    }
    bases
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typecheck::TypeChecker;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

    fn fixture(files: &[(&str, &str)]) -> PathBuf {
        let scratch = std::env::var_os("REY_V03_SCRATCH")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let root = scratch.join(format!(
            "rey-provenance-{}-{}",
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

    fn checkEntry(root: &Path) -> Result<(), crate::typecheck::LocatedTypeError> {
        let program = resolveEntry(&root.join("main.rey")).unwrap();
        assert_eq!(program.statements.len(), program.statementProvenance.len());
        TypeChecker::checkResolvedProgram(&program)
    }

    #[test]
    fn declarations_from_canonical_source_are_merged_once() {
        let sourcePath = canonicalPath(Path::new(file!()));
        let alternatePath = canonicalPath(
            &Path::new(file!())
                .parent()
                .unwrap()
                .join("..")
                .join("src/imports.rs"),
        );
        assert_eq!(sourcePath, alternatePath);

        let statement = Stmt::EnumDecl {
            name: "Shared".to_string(),
            variants: vec!["Only".to_string()],
            is_pub: false,
        };
        let imported = |file: PathBuf| {
            ResolvedFile {
                statements: vec![statement.clone()],
                statementProvenance: vec![StatementProvenance {
                    moduleId: file.clone(),
                    span: Span::new(0, 0),
                }],
                declStatements: vec![statement.clone()],
                declKeys: vec![DeclKey::Source { file: file.clone(), index: 0 }],
                declProvenance: vec![StatementProvenance {
                    moduleId: file,
                    span: Span::new(0, 0),
                }],
                localFunctionVisibility: HashMap::new(),
                localTypeVisibility: HashMap::new(),
                moduleBindings: ModuleBindings::default(),
            }
        };

        let mut resolved = Vec::new();
        let mut statementProvenance = Vec::new();
        let mut declStatements = Vec::new();
        let mut declKeys = Vec::new();
        let mut declProvenance = Vec::new();
        let mut included = HashSet::new();
        appendDeclarations(
            &imported(sourcePath.clone()),
            &mut resolved,
            &mut statementProvenance,
            &mut declStatements,
            &mut declKeys,
            &mut declProvenance,
            &mut included,
        );
        appendDeclarations(
            &imported(alternatePath),
            &mut resolved,
            &mut statementProvenance,
            &mut declStatements,
            &mut declKeys,
            &mut declProvenance,
            &mut included,
        );

        assert_eq!(resolved.len(), 1);
        assert_eq!(declStatements.len(), 1);
        assert_eq!(statementProvenance.len(), 1);
    }

    #[test]
    fn resolvedCheckerUsesModulePrivacyAndPublicBindings() {
        let internal = fixture(&[(
            "main.rey",
            "func hidden(): int { return 1; } func call(): int { return hidden(); }",
        )]);
        assert!(checkEntry(&internal).is_ok());

        let barePrivate = fixture(&[
            ("lib/main.rey", "func hidden(): int { return 1; } pub func visible(): int { return 2; }"),
            ("main.rey", "import lib; var value = hidden();"),
        ]);
        let error = checkEntry(&barePrivate).unwrap_err();
        assert_eq!(error.file, canonicalPath(&barePrivate.join("main.rey")));
        assert!(error.error.message.contains("unknown function 'hidden'"));

        let qualified = fixture(&[
            ("lib/main.rey", "pub func visible(): int { return 2; }"),
            ("main.rey", "import lib; var value: int = lib.visible();"),
        ]);
        assert!(checkEntry(&qualified).is_ok());

        let direct = fixture(&[
            ("util.rey", "pub func visible(): int { return 3; }"),
            ("main.rey", "import util.visible; var value: int = visible();"),
        ]);
        assert!(checkEntry(&direct).is_ok());

        let privateImport = fixture(&[
            ("util.rey", "func hidden(): int { return 1; }"),
            ("main.rey", "import util.hidden; hidden();"),
        ]);
        let importError = resolveEntry(&privateImport.join("main.rey")).unwrap_err();
        assert!(importError.message.contains("is private"));

        let typeVisibility = fixture(&[
            ("lib/main.rey", "struct Hidden {} pub struct Visible {} pub func make(): Visible { return Visible {}; }"),
            ("main.rey", "import lib; var value: Visible = lib.make();"),
        ]);
        assert!(checkEntry(&typeVisibility).is_ok());

        let privateTypeLeak = fixture(&[
            ("lib/main.rey", "struct Hidden {}"),
            ("main.rey", "import lib; var value: Hidden = Hidden {};"),
        ]);
        let typeError = checkEntry(&privateTypeLeak).unwrap_err();
        assert_eq!(typeError.file, canonicalPath(&privateTypeLeak.join("main.rey")));
        assert!(typeError.error.message.contains("unknown type 'Hidden'"));

        let explicitPrivateType = fixture(&[
            ("types.rey", "struct Hidden {}"),
            ("main.rey", "import types.Hidden; var value: Hidden = Hidden {};"),
        ]);
        let importError = resolveEntry(&explicitPrivateType.join("main.rey")).unwrap_err();
        assert!(importError.message.contains("Type 'Hidden'"));
    }

    #[test]
    fn resolvedCheckerPreservesModuleIsolationIdempotenceAndSourceErrors() {
        let duplicateImports = fixture(&[
            ("lib/main.rey", "pub func visible(): int { return 1; }"),
            ("main.rey", "import lib; import lib; var value = lib.visible();"),
        ]);
        let program = resolveEntry(&duplicateImports.join("main.rey")).unwrap();
        assert_eq!(program.statements.iter().filter(|stmt| matches!(stmt,
            Stmt::VarDecl { name, .. } if name == "lib"
        )).count(), 1);
        assert!(TypeChecker::checkResolvedProgram(&program).is_ok());

        let duplicatePrivateNames = fixture(&[
            ("a/main.rey", "func helper(): int { return 1; } pub func invoke(): int { return helper(); }"),
            ("b/main.rey", "func helper(): int { return 2; } pub func invoke(): int { return helper(); }"),
            ("main.rey", "import a; import b; var left: int = a.invoke(); var right: int = b.invoke();"),
        ]);
        assert!(checkEntry(&duplicatePrivateNames).is_ok());

        let importedBodyError = fixture(&[
            ("lib/main.rey", "func broken(): int { return \"wrong\"; } pub func run(): int { return broken(); }"),
            ("main.rey", "import lib; var value = lib.run();"),
        ]);
        let error = checkEntry(&importedBodyError).unwrap_err();
        assert_eq!(error.file, canonicalPath(&importedBodyError.join("lib/main.rey")));
        assert!(error.source.contains("wrong"));
        assert!(error.error.message.contains("return expected"));

        let reexportingWrapper = fixture(&[
            ("util.rey", "pub func value(): int { return 4; }"),
            ("lib/main.rey", "import util.value; pub func wrapper(): int { return value(); }"),
            ("main.rey", "import lib; var result: int = lib.wrapper();"),
        ]);
        assert!(checkEntry(&reexportingWrapper).is_ok());
    }

    #[test]
    fn nestedDiamondKeepsImporterNamespaceBindingsAfterDeclarationDedup() {
        let root = fixture(&[
            ("shared/main.rey", "pub func answer(): int { return 42; }"),
            ("left/main.rey", "import shared; pub func value(): int { return shared.answer(); }"),
            ("right/main.rey", "import shared; import shared; pub func value(): int { return shared.answer(); }"),
            ("main.rey", "import left; import right; var a: int = left.value(); var b: int = right.value();"),
        ]);
        let program = resolveEntry(&root.join("main.rey")).unwrap();
        assert!(TypeChecker::checkResolvedProgram(&program).is_ok());
        let rootId = canonicalPath(&root.join("main.rey"));
        let leftId = canonicalPath(&root.join("left/main.rey"));
        let rightId = canonicalPath(&root.join("right/main.rey"));
        let sharedId = canonicalPath(&root.join("shared/main.rey"));
        assert_eq!(program.moduleBindings[&rootId].dependencies, vec![leftId.clone(), rightId.clone()]);
        assert_eq!(program.moduleBindings[&leftId].dependencies, vec![sharedId.clone()]);
        assert_eq!(program.moduleBindings[&rightId].dependencies, vec![sharedId]);
        assert_eq!(program.moduleBindings[&leftId].namespaces["shared"]["answer"].moduleId, canonicalPath(&root.join("shared/main.rey")));
        assert_eq!(program.moduleBindings[&rightId].namespaces["shared"]["answer"].moduleId, canonicalPath(&root.join("shared/main.rey")));
        assert_eq!(program.moduleBindings[&rootId].namespaces["left"]["value"].moduleId, leftId);
        assert_eq!(program.moduleBindings[&rootId].namespaces["right"]["value"].moduleId, rightId);
    }

    #[test]
    fn selectedModulePublicTypesSupportColonColonAndDottedImports() {
        let files = [
            ("lexer/main.rey", "pub struct Span { start: int, end: int, line: int, column: int, } pub struct Token { kind: string, span: Span, }"),
            ("lexer/token.rey", "// compatibility file; Token is declared in main.rey"),
            ("main.rey", "import lexer::{Token, Span}; var span: Span = Span { start: 0, end: 1, line: 1, column: 1 }; var token: Token = Token { kind: \"id\", span: span };")
        ];
        let colonColon = fixture(&files);
        let program = resolveEntry(&colonColon.join("main.rey")).unwrap();
        let checked = TypeChecker::checkResolvedProgram(&program);
        assert!(checked.is_ok(), "{:?}", checked);
        let rootId = canonicalPath(&colonColon.join("main.rey"));
        let lexerId = canonicalPath(&colonColon.join("lexer/main.rey"));
        assert_eq!(program.moduleBindings[&rootId].types["Token"].moduleId, lexerId);
        assert_eq!(program.moduleBindings[&rootId].types["Span"].moduleId, lexerId);
        assert_eq!(program.moduleBindings[&rootId].dependencies, vec![lexerId.clone()]);

        let dotted = fixture(&[
            ("lexer/main.rey", "pub struct Span { start: int, end: int, line: int, column: int, } pub struct Token { kind: string, span: Span, }"),
            ("lexer/token.rey", "// compatibility file; Token is declared in main.rey"),
            ("main.rey", "import lexer.{Token, Span}; var span: Span = Span { start: 0, end: 1, line: 1, column: 1 }; var token: Token = Token { kind: \"id\", span: span };")
        ]);
        let dottedProgram = resolveEntry(&dotted.join("main.rey")).unwrap();
        assert!(TypeChecker::checkResolvedProgram(&dottedProgram).is_ok());
        let dottedRootId = canonicalPath(&dotted.join("main.rey"));
        let dottedLexerId = canonicalPath(&dotted.join("lexer/main.rey"));
        assert_eq!(dottedProgram.moduleBindings[&dottedRootId].types["Token"].moduleId, dottedLexerId);
        assert_eq!(dottedProgram.moduleBindings[&dottedRootId].types["Span"].moduleId, dottedLexerId);
        assert_eq!(dottedProgram.moduleBindings[&dottedRootId].dependencies, vec![dottedLexerId]);

        let direct = fixture(&[
            ("lexer/main.rey", "pub struct Span { start: int, end: int, line: int, column: int, } pub struct Token { kind: string, span: Span, }"),
            ("lexer/token.rey", "// compatibility file; Token is declared in main.rey"),
            ("main.rey", "import lexer.Token; import lexer.Span; import lexer::{Token}; var span: Span = Span { start: 0, end: 1, line: 1, column: 1 }; var token: Token = Token { kind: \"id\", span: span };")
        ]);
        let directProgram = resolveEntry(&direct.join("main.rey")).unwrap();
        let checked = TypeChecker::checkResolvedProgram(&directProgram);
        assert!(checked.is_ok(), "{:?}", checked);
        let directRootId = canonicalPath(&direct.join("main.rey"));
        let directLexerId = canonicalPath(&direct.join("lexer/main.rey"));
        assert_eq!(directProgram.moduleBindings[&directRootId].types["Token"].moduleId, directLexerId);
        assert_eq!(directProgram.moduleBindings[&directRootId].types["Span"].moduleId, directLexerId);
        assert_eq!(directProgram.moduleBindings[&directRootId].dependencies, vec![directLexerId]);
    }

    #[test]
    fn nativeParserGroupedImportsBindToTheirCanonicalOwners() {
        let nativeSrc = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../rey-compiler/src");
        let parserSourcePath = nativeSrc.join("parser/main.rey");
        let parserSource = fs::read_to_string(&parserSourcePath).unwrap();
        let parserStatements = parseSource(&parserSourcePath, &parserSource).unwrap();
        let astItems = parserStatements
            .iter()
            .find_map(|stmt| match stmt {
                Stmt::Import {
                    kind: ImportKind::ModuleItems { module, items },
                    ..
                } if module == "ast" => Some(items),
                _ => None,
            })
            .unwrap();
        let mut astSource = "import lexer::{Span};\n".to_string();
        for item in astItems {
            astSource.push_str(&format!("pub struct {} {{}}\n", item.name));
        }

        let root = fixture(&[(
            "main.rey",
            "import lexer::{Token, Span, LexError}; import parser;",
        )]);
        for relative in [
            "lexer/main.rey",
            "lexer/token.rey",
            "parser/main.rey",
        ] {
            let source = nativeSrc.join(relative);
            let target = root.join(relative);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(source, target).unwrap();
        }
        let astTarget = root.join("parser/ast/main.rey");
        fs::create_dir_all(astTarget.parent().unwrap()).unwrap();
        fs::write(astTarget, astSource).unwrap();

        let program = resolveEntry(&root.join("main.rey")).unwrap();
        let rootId = canonicalPath(&root.join("main.rey"));
        let parserId = canonicalPath(&root.join("parser/main.rey"));
        let astId = canonicalPath(&root.join("parser/ast/main.rey"));
        let lexerId = canonicalPath(&root.join("lexer/main.rey"));

        assert_eq!(program.moduleBindings[&rootId].types["Token"].moduleId, lexerId);
        assert_eq!(program.moduleBindings[&rootId].types["Span"].moduleId, lexerId);
        assert_eq!(program.moduleBindings[&rootId].types["LexError"].moduleId, lexerId);
        assert_eq!(program.moduleBindings[&parserId].types["Token"].moduleId, lexerId);
        assert_eq!(program.moduleBindings[&parserId].types["Span"].moduleId, lexerId);
        assert_eq!(program.moduleBindings[&parserId].types["LiteralKind"].moduleId, astId);
        assert_eq!(program.moduleBindings[&astId].types["Span"].moduleId, lexerId);
        assert_eq!(program.moduleBindings[&parserId].dependencies, vec![lexerId, astId]);
        assert_eq!(program.moduleBindings[&rootId].namespaces["parser"]["parse"].moduleId, parserId);
        assert_eq!(program.moduleBindings[&rootId].dependencies, vec![canonicalPath(&root.join("lexer/main.rey")), parserId]);
    }

    #[test]
    fn moduleItemLookupRejectsCaseInsensitiveFilenameAliases() {
        let root = fixture(&[("token.rey", "pub func legacy(): int { return 1; }")]);
        assert!(fileExistsWithExactCase(&root.join("token.rey")));
        assert!(!fileExistsWithExactCase(&root.join("Token.rey")));
    }

    #[test]
    fn braceImportUsesDeclarationInMainReyInsteadOfASiblingFile() {
        let child = fixture(&[
            ("lexer/main.rey", "pub func tokens(): int { return 0; }"),
            ("lexer/tokens.rey", "pub func value(): int { return 1; }"),
            ("lexer/tokens/main.rey", "pub func value(): int { return 9; }"),
            ("main.rey", "import lexer::{tokens}; var answer: int = tokens();"),
        ]);
        let program = resolveEntry(&child.join("main.rey")).unwrap();
        let checked = TypeChecker::checkResolvedProgram(&program);
        assert!(checked.is_ok(), "{:?}", checked);
        let rootId = canonicalPath(&child.join("main.rey"));
        assert_eq!(
            program.moduleBindings[&rootId].direct["tokens"].moduleId,
            canonicalPath(&child.join("lexer/main.rey"))
        );
        assert!(!program.sources.contains_key(&canonicalPath(&child.join("lexer/tokens/main.rey"))));
        assert!(!program.sources.contains_key(&canonicalPath(&child.join("lexer/tokens.rey"))));

        let privateFunction = fixture(&[
            ("lexer/main.rey", "func hidden(): int { return 0; }"),
            ("main.rey", "import lexer::{hidden};"),
        ]);
        let error = resolveEntry(&privateFunction.join("main.rey")).unwrap_err();
        assert!(error.message.contains("Function 'hidden'") && error.message.contains("private"), "{}", error.message);

        let privateType = fixture(&[
            ("lexer/main.rey", "struct Hidden {}"),
            ("main.rey", "import lexer.{Hidden};"),
        ]);
        let error = resolveEntry(&privateType.join("main.rey")).unwrap_err();
        assert!(error.message.contains("Type 'Hidden'") && error.message.contains("private"), "{}", error.message);
    }

    #[test]
    fn emptyExportFolderNamespaceStillRecordsSideEffectDependencies() {
        let root = fixture(&[
            ("empty/main.rey", "var mainEffect = 1;"),
            ("empty/side.rey", "var sideEffect = 2;"),
            ("main.rey", "import empty;"),
        ]);
        let program = resolveEntry(&root.join("main.rey")).unwrap();
        let rootId = canonicalPath(&root.join("main.rey"));
        let mainId = canonicalPath(&root.join("empty/main.rey"));
        let sideId = canonicalPath(&root.join("empty/side.rey"));
        assert_eq!(program.moduleBindings[&rootId].namespaces["empty"].len(), 0);
        assert_eq!(program.moduleBindings[&rootId].dependencies, vec![mainId]);
        assert!(!program.sources.contains_key(&sideId));
    }

    #[test]
    fn directoryModuleIsMainReyOnlyAndBraceNamesDoNotUseSiblingFiles() {
        let root = fixture(&[
            ("math/main.rey", "pub func answer(): int { return 1; } pub func area(): int { return 2; }"),
            ("math/answer.rey", "pub func answer(): int { return 99; }"),
            ("shapes/solid/main.rey", "pub func area(): int { return 3; }"),
            ("shapes/solid/area.rey", "pub func ignored(): int { return 8; }"),
            ("modules/account.rey", "pub func id(): int { return 4; }"),
            (
                "main.rey",
                "import math;\nimport shapes::solid::{area};\nvar fromNamespace: int = math.answer();\nvar fromBrace: int = area();\nimport modules::{account};\nvar fromChild: int = account.id();\n",
            ),
        ]);
        let program = resolveEntry(&root.join("main.rey")).unwrap();
        assert!(TypeChecker::checkResolvedProgram(&program).is_ok());
        let rootId = canonicalPath(&root.join("main.rey"));
        let mathId = canonicalPath(&root.join("math/main.rey"));
        assert_eq!(program.moduleBindings[&rootId].namespaces["math"]["answer"].moduleId, mathId);
        assert!(!program.sources.contains_key(&canonicalPath(&root.join("math/answer.rey"))));
        assert!(!program.sources.contains_key(&canonicalPath(&root.join("shapes/solid/area.rey"))));
        assert_eq!(
            program.moduleBindings[&rootId].direct["area"].moduleId,
            canonicalPath(&root.join("shapes/solid/main.rey"))
        );
        assert!(program.sources.contains_key(&canonicalPath(&root.join("modules/account.rey"))));
    }

    #[test]
    fn unbracedChildPathPrefersFolderMainThenFlatFile() {
        let fileOnly = fixture(&[
            ("action/main.rey", "pub func entry(): int { return 1; }"),
            ("action/run.rey", "pub func go(): int { return 2; }"),
            ("main.rey", "import action::run;\nvar value: int = run.go();\n"),
        ]);
        let program = resolveEntry(&fileOnly.join("main.rey")).unwrap();
        assert!(TypeChecker::checkResolvedProgram(&program).is_ok());
        let rootId = canonicalPath(&fileOnly.join("main.rey"));
        let runId = canonicalPath(&fileOnly.join("action/run.rey"));
        assert_eq!(program.moduleBindings[&rootId].namespaces["run"]["go"].moduleId, runId);

        let folderWins = fixture(&[
            ("action/main.rey", "pub func entry(): int { return 1; }"),
            ("action/run.rey", "pub func go(): int { return 2; }"),
            ("action/run/main.rey", "pub func go(): int { return 9; }"),
            ("main.rey", "import action::run;\nvar value: int = run.go();\n"),
        ]);
        let folderProgram = resolveEntry(&folderWins.join("main.rey")).unwrap();
        assert!(TypeChecker::checkResolvedProgram(&folderProgram).is_ok());
        let folderRoot = canonicalPath(&folderWins.join("main.rey"));
        assert_eq!(
            folderProgram.moduleBindings[&folderRoot].namespaces["run"]["go"].moduleId,
            canonicalPath(&folderWins.join("action/run/main.rey"))
        );
        assert!(!folderProgram.sources.contains_key(&canonicalPath(&folderWins.join("action/run.rey"))));

        let flat = fixture(&[
            ("math.rey", "pub func answer(): int { return 1; }"),
            ("main.rey", "import math;\n"),
        ]);
        let error = resolveEntry(&flat.join("main.rey")).unwrap_err();
        assert!(error.message.contains("Module folder not found"), "{}", error.message);
    }

    #[test]
    fn interpreterStillRejectsDuplicatePrivateNamesInFlattenedProgram() {
        let root = fixture(&[
            ("a/main.rey", "func helper(): string { return \"a\"; } pub func invoke(): string { return helper(); }"),
            ("b/main.rey", "func helper(): int { return 2; } pub func invoke(): int { return helper(); }"),
            ("main.rey", "import a; import b; var left = a.invoke(); var right = b.invoke();"),
        ]);
        let program = resolveEntry(&root.join("main.rey")).unwrap();
        let error = crate::interpreter::Interpreter::new()
            .interpret(&program.statements)
            .unwrap_err();
        assert!(error.to_string().contains("return expected"), "{}", error);
    }
}
