#![allow(non_snake_case)]

use crate::ast::{FunctionVisibility, ImportKind, Stmt};
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

        if self.stack.contains(&filePath) {
            let mut chain = self
                .stack
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>();
            chain.push(filePath.display().to_string());
            let source = fs::read_to_string(&filePath).unwrap_or_default();
            return Err(CompileError {
                title: "import".to_string(),
                file: filePath.clone(),
                source,
                span: Span::new(0, 1),
                message: format!("Circular import detected: {}", chain.join(" -> ")),
            });
        }

        self.stack.push(filePath.clone());
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
        self.stack.pop();

        let file = ResolvedFile {
            statements: resolved,
            functionStatements,
            localFunctionVisibility,
        };
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
                    self.findFileWithOrder(currentDir, &module, Some(ownerFile), ownerSource, span)?;
                let imported = self.resolveFile(&importFile)?;
                if !includedFiles.contains(&importFile) {
                    resolved.extend(imported.functionStatements.clone());
                    includedFiles.insert(importFile.clone());
                }

                for symbol in symbols {
                    if !injectedNames.insert(symbol.clone()) {
                        return Err(CompileError {
                            title: "import".to_string(),
                            file: ownerFile.to_path_buf(),
                            source: ownerSource.to_string(),
                            span,
                            message: format!("Duplicate import: '{}'", symbol),
                        });
                    }
                    match imported.localFunctionVisibility.get(&symbol) {
                        Some(FunctionVisibility::ExportPub) => {}
                        Some(FunctionVisibility::Pub) => {
                            return Err(CompileError {
                                title: "import".to_string(),
                                file: ownerFile.to_path_buf(),
                                source: ownerSource.to_string(),
                                span,
                                message: format!(
                                    "Function '{}' exists in '{}' but is 'pub', not 'export pub'",
                                    symbol,
                                    importFile.display()
                                ),
                            });
                        }
                        Some(FunctionVisibility::Private) | None => {
                            return Err(CompileError {
                                title: "import".to_string(),
                                file: ownerFile.to_path_buf(),
                                source: ownerSource.to_string(),
                                span,
                                message: format!(
                                    "Function '{}' not found in file '{}'",
                                    symbol,
                                    importFile.display()
                                ),
                            });
                        }
                    }
                }
                Ok(())
            }
            ImportKind::ModuleNamespace { module } => Err(CompileError {
                title: "import".to_string(),
                file: ownerFile.to_path_buf(),
                source: ownerSource.to_string(),
                span,
                message: format!(
                    "Module imports are not enabled yet for '{}'. Use file imports for now.",
                    module
                ),
            }),
            ImportKind::ModuleItems { module, .. } => Err(CompileError {
                title: "import".to_string(),
                file: ownerFile.to_path_buf(),
                source: ownerSource.to_string(),
                span,
                message: format!(
                    "Module imports are not enabled yet for '{}'. Use file imports for now.",
                    module
                ),
            }),
        }
    }

    fn findFileWithOrder(
        &self,
        currentDir: &Path,
        module: &str,
        ownerFile: Option<&Path>,
        ownerSource: &str,
        span: Span,
    ) -> Result<PathBuf, CompileError> {
        let mut candidates = Vec::new();
        candidates.push(currentDir.join(format!("{}.rey", module)));
        candidates.push(self.projectRoot.join(format!("{}.rey", module)));
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
            file: ownerFile
                .map(|f| f.to_path_buf())
                .unwrap_or_else(|| PathBuf::from(module)),
            source: ownerSource.to_string(),
            span,
            message: format!("File not found for import '{}.{}'", module, "<symbol>"),
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
