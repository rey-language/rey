#![allow(non_snake_case)]

use crate::ast::{Expr, FunctionVisibility, ImportKind, Stmt};
use crate::lexer::{span::Span, Lexer, TokenKind};
use crate::parser::Parser;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CompileError {
    pub title: String,
    pub file: PathBuf,
    pub source: String,
    pub span: Span,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedProgram {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone)]
struct ResolvedFile {
    statements: Vec<Stmt>,
    functionStatements: Vec<Stmt>,
    declStatements: Vec<Stmt>,
    localFunctionVisibility: HashMap<String, FunctionVisibility>,
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
    })
}

struct ResolverState {
    cache: HashMap<PathBuf, ResolvedFile>,
    stack: Vec<PathBuf>,
    projectRoot: PathBuf,
}

impl ResolverState {
    fn new(projectRoot: PathBuf) -> Self {
        Self {
            cache: HashMap::new(),
            stack: Vec::new(),
            projectRoot,
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

            let statements = parseSource(&filePath, &source)?;
            let currentDir = filePath
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf();

            let mut localFunctionVisibility = HashMap::new();
            for stmt in &statements {
                if let Stmt::FuncDecl {
                    name, visibility, ..
                } = stmt
                {
                    localFunctionVisibility.insert(name.clone(), *visibility);
                }
            }

            let mut resolved = Vec::new();
            let mut injectedNames = HashSet::new();
            let mut includedFiles = HashSet::new();
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
                            &mut injectedNames,
                            &mut includedFiles,
                        )?;
                    }
                    other => resolved.push(other),
                }
            }

            let functionStatements = resolved
                .iter()
                .filter(|stmt| matches!(stmt, Stmt::FuncDecl { .. }))
                .cloned()
                .collect::<Vec<_>>();
            let declStatements = resolved
                .iter()
                .filter(|stmt| {
                    matches!(
                        stmt,
                        Stmt::FuncDecl { .. } | Stmt::StructDecl { .. } | Stmt::EnumDecl { .. }
                    )
                })
                .cloned()
                .collect::<Vec<_>>();

            Ok(ResolvedFile {
                statements: resolved,
                functionStatements,
                declStatements,
                localFunctionVisibility,
            })
        })();
        self.stack.pop();

        let file = result?;
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
        injectedNames: &mut HashSet<String>,
        includedFiles: &mut HashSet<PathBuf>,
    ) -> Result<(), CompileError> {
        match kind {
            ImportKind::FileSymbols { module, symbols } => {
                let importFile =
                    self.findFileImport(currentDir, &module, ownerFile, ownerSource, span)?;
                if self.stack.contains(&importFile) {
                    return Err(self.circularError(ownerFile, ownerSource, span, &importFile));
                }
                let imported = self.resolveFile(&importFile)?;
                if !includedFiles.contains(&importFile) {
                    resolved.extend(imported.functionStatements.clone());
                    includedFiles.insert(importFile.clone());
                }

                for symbol in symbols {
                    if !injectedNames.insert(symbol.name.clone()) {
                        return Err(CompileError {
                            title: "import".to_string(),
                            file: ownerFile.to_path_buf(),
                            source: ownerSource.to_string(),
                            span: symbol.span,
                            message: format!("Duplicate import: '{}'", symbol.name),
                        });
                    }
                    match imported.localFunctionVisibility.get(&symbol.name) {
                        Some(FunctionVisibility::ExportPub) => {}
                        Some(FunctionVisibility::Pub) => {
                            return Err(CompileError {
                                title: "import".to_string(),
                                file: ownerFile.to_path_buf(),
                                source: ownerSource.to_string(),
                                span: symbol.span,
                                message: format!(
                                    "Function '{}' exists in '{}' but is 'pub', not 'export pub'",
                                    symbol.name,
                                    importFile.display()
                                ),
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
                            return Err(CompileError {
                                title: "import".to_string(),
                                file: ownerFile.to_path_buf(),
                                source: ownerSource.to_string(),
                                span: symbol.span,
                                message: format!(
                                    "Function '{}' not found in file '{}'",
                                    symbol.name,
                                    importFile.display()
                                ),
                            });
                        }
                    }
                }
                Ok(())
            }
            ImportKind::ModuleNamespace { module } => {
                if !injectedNames.insert(module.clone()) {
                    return Err(CompileError {
                        title: "import".to_string(),
                        file: ownerFile.to_path_buf(),
                        source: ownerSource.to_string(),
                        span,
                        message: format!("Duplicate import: '{}'", module),
                    });
                }
                let moduleDir =
                    self.findModuleDir(currentDir, &module, ownerFile, ownerSource, span)?;
                let moduleMain = moduleDir.join("main.rey");
                if !moduleMain.exists() {
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

                let mut exportedSymbols = HashSet::new();
                let mut namespaceEntries = Vec::new();
                let mut moduleFiles = fs::read_dir(&moduleDir)
                    .map_err(|_| CompileError {
                        title: "import".to_string(),
                        file: ownerFile.to_path_buf(),
                        source: ownerSource.to_string(),
                        span,
                        message: format!("Could not read module folder '{}'", moduleDir.display()),
                    })?
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .filter(|path| path.extension().is_some_and(|ext| ext == "rey"))
                    .collect::<Vec<_>>();
                moduleFiles.sort();

                for path in moduleFiles {
                    let path = canonicalPath(&path);
                    if self.stack.contains(&path) {
                        return Err(self.circularError(ownerFile, ownerSource, span, &path));
                    }
                    let imported = self.resolveFile(&path)?;
                    if !includedFiles.contains(&path) {
                        resolved.extend(imported.declStatements.clone());
                        includedFiles.insert(path.clone());
                    }

                    for (name, visibility) in &imported.localFunctionVisibility {
                        if *visibility == FunctionVisibility::ExportPub {
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
                        }
                    }
                }

                resolved.push(self.namespaceStmt(&module, namespaceEntries, span));
                Ok(())
            }
            ImportKind::ModuleItems { module, items } => {
                for item in items {
                    if !injectedNames.insert(item.name.clone()) {
                        return Err(CompileError {
                            title: "import".to_string(),
                            file: ownerFile.to_path_buf(),
                            source: ownerSource.to_string(),
                            span: item.span,
                            message: format!("Duplicate import: '{}'", item.name),
                        });
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
                    if !includedFiles.contains(&importFile) {
                        resolved.extend(imported.declStatements.clone());
                        includedFiles.insert(importFile.clone());
                    }

                    let mut namespaceEntries = Vec::new();
                    for (name, visibility) in imported.localFunctionVisibility {
                        if visibility == FunctionVisibility::ExportPub {
                            namespaceEntries.push((
                                name.clone(),
                                Expr::Variable {
                                    name,
                                    span: item.span,
                                },
                            ));
                        }
                    }
                    resolved.push(self.namespaceStmt(&item.name, namespaceEntries, item.span));
                }
                Ok(())
            }
        }
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

    fn findModuleDir(
        &self,
        currentDir: &Path,
        module: &str,
        ownerFile: &Path,
        ownerSource: &str,
        span: Span,
    ) -> Result<PathBuf, CompileError> {
        let mut candidates = Vec::new();
        candidates.push(currentDir.join(module));
        candidates.push(currentDir.join("src").join(module));
        candidates.push(self.projectRoot.join(module));
        candidates.push(self.projectRoot.join("src").join(module));
        if module == "std" {
            if let Some(home) = homePath() {
                candidates.push(home.join(".reyc/std/src"));
            }
        }
        if let Some(home) = homePath() {
            candidates.push(home.join(".reyc/packages").join(module));
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
        let mut candidates = Vec::new();
        candidates.push(currentDir.join(module).join(item).join("main.rey"));
        candidates.push(currentDir.join(module).join(format!("{}.rey", item)));
        candidates.push(currentDir.join("src").join(module).join(item).join("main.rey"));
        candidates.push(currentDir.join("src").join(module).join(format!("{}.rey", item)));
        candidates.push(self.projectRoot.join(module).join(item).join("main.rey"));
        candidates.push(self.projectRoot.join(module).join(format!("{}.rey", item)));
        candidates.push(
            self.projectRoot
                .join("src")
                .join(module)
                .join(item)
                .join("main.rey"),
        );
        candidates.push(
            self.projectRoot
                .join("src")
                .join(module)
                .join(format!("{}.rey", item)),
        );
        if module == "std" {
            if let Some(home) = homePath() {
                candidates.push(home.join(".reyc/std/src").join(item).join("main.rey"));
                candidates.push(home.join(".reyc/std/src").join(format!("{}.rey", item)));
            }
        }
        if let Some(home) = homePath() {
            candidates.push(
                home.join(".reyc/packages")
                    .join(module)
                    .join(item)
                    .join("main.rey"),
            );
            candidates.push(
                home.join(".reyc/packages")
                    .join(module)
                    .join(format!("{}.rey", item)),
            );
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
            message: format!("File not found for module import '{}::{}'", module, item),
        })
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
