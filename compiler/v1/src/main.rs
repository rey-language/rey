#![allow(non_snake_case)]

mod ast;
mod imports;
mod interpreter;
mod lexer;
mod parser;
mod typecheck;

use imports::{resolveEntry, ResolvedProgram};
use interpreter::Interpreter;
use std::env;
use std::path::PathBuf;
use typecheck::TypeChecker;

const USAGE: &str = concat!(
    "usage: rey-v0 run <file> [-- <program-args...>]\n",
    "       rey-v0 check <file>\n",
    "       rey-v0 build <file>\n",
    "       rey-v0 --version\n",
    "       rey-v0 --help"
);

#[derive(Debug, PartialEq, Eq)]
enum CliCommand {
    Run {
        file: String,
        programArgs: Vec<String>,
    },
    Check {
        file: String,
    },
    Build {
        file: String,
    },
    Version,
    Help,
}

fn parse_cli_args(args: &[String]) -> Result<CliCommand, String> {
    match args {
        [flag] if flag == "--version" => Ok(CliCommand::Version),
        [flag] if flag == "--help" => Ok(CliCommand::Help),
        [command, file] if command == "run" => Ok(CliCommand::Run {
            file: file.clone(),
            programArgs: Vec::new(),
        }),
        [command, file, separator, programArgs @ ..] if command == "run" && separator == "--" => {
            Ok(CliCommand::Run {
                file: file.clone(),
                programArgs: programArgs.to_vec(),
            })
        }
        [command, file] if command == "check" => Ok(CliCommand::Check { file: file.clone() }),
        [command, file] if command == "build" => Ok(CliCommand::Build { file: file.clone() }),
        [command] if command == "run" || command == "check" || command == "build" => {
            Err("missing file".to_string())
        }
        [command, ..] if command == "run" || command == "check" || command == "build" => {
            Err("unexpected arguments".to_string())
        }
        [command, ..] if command != "--help" && command != "--version" => {
            Err(format!("unknown command: {}", command))
        }
        [] => Err("missing command".to_string()),
        _ => Err("unexpected arguments".to_string()),
    }
}

fn location(source: &str, span: &crate::lexer::span::Span) -> Option<(usize, usize)> {
    if source.is_empty() {
        return None;
    }
    if span.line > 0 && span.column > 0 {
        return Some((span.line, span.column));
    }
    if span.start > source.len() || !source.is_char_boundary(span.start) {
        return None;
    }

    let mut line = 1;
    let mut column = 1;
    for ch in source[..span.start].chars() {
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    Some((line, column))
}

fn error_line(
    filename: &str,
    source: &str,
    span: &crate::lexer::span::Span,
    message: &str,
) -> String {
    match location(source, span) {
        Some((line, column)) => format!("{}:{}:{}: error: {}", filename, line, column, message),
        None => format!("{}:?:?: error: {}", filename, message),
    }
}

fn report_error(filename: &str, source: &str, span: &crate::lexer::span::Span, message: &str) {
    eprintln!("{}", error_line(filename, source, span, message));
}

struct CliDiagnostic {
    file: PathBuf,
    source: String,
    span: crate::lexer::span::Span,
    message: String,
}

fn resolveAndCheck(path: &PathBuf) -> Result<ResolvedProgram, CliDiagnostic> {
    let program = resolveEntry(path).map_err(|error| CliDiagnostic {
        file: error.file,
        source: error.source,
        span: error.span,
        message: error.message,
    })?;
    TypeChecker::checkResolvedProgram(&program).map_err(|located| CliDiagnostic {
        file: located.file,
        source: located.source,
        span: located.error.span,
        message: located.error.message,
    })?;
    Ok(program)
}

fn report_diagnostic(diagnostic: CliDiagnostic) {
    report_error(
        &diagnostic.file.display().to_string(),
        &diagnostic.source,
        &diagnostic.span,
        &diagnostic.message,
    );
}

fn report_runtime_error(message: &str) {
    eprintln!("<runtime>:?:?: error: {}", message);
}

fn print_usage_error(message: &str) -> i32 {
    eprintln!("error: {}", message);
    eprintln!("{}", USAGE);
    2
}

fn run_file(path: &PathBuf, programArgs: Vec<String>) -> i32 {
    let program = match resolveAndCheck(path) {
        Ok(program) => program,
        Err(diagnostic) => {
            report_diagnostic(diagnostic);
            return 1;
        }
    };
    let mut interpreter = Interpreter::new_with_args(programArgs);
    match interpreter.interpret_resolved(&program, path) {
        Ok(()) => 0,
        Err(message) => {
            report_runtime_error(&message);
            1
        }
    }
}

fn check_file(path: &PathBuf) -> i32 {
    match resolveAndCheck(path) {
        Ok(_) => 0,
        Err(diagnostic) => {
            report_diagnostic(diagnostic);
            1
        }
    }
}

fn run_cli(args: &[String]) -> i32 {
    match parse_cli_args(args) {
        Ok(CliCommand::Version) => {
            println!("rey-v0 0.3.0");
            0
        }
        Ok(CliCommand::Help) => {
            println!("{}", USAGE);
            0
        }
        Ok(CliCommand::Build { file: _ }) => {
            eprintln!(
                "error: the interpreter cannot emit native binaries; use the native compiler rey"
            );
            2
        }
        Ok(CliCommand::Run { file, programArgs }) => {
            let mut interpreterArgs = Vec::with_capacity(programArgs.len() + 1);
            interpreterArgs.push(file.clone());
            interpreterArgs.extend(programArgs);
            run_file(&PathBuf::from(file), interpreterArgs)
        }
        Ok(CliCommand::Check { file }) => check_file(&PathBuf::from(file)),
        Err(message) => print_usage_error(&message),
    }
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    std::process::exit(run_cli(&args));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, Literal};
    use crate::interpreter::environment::Environment;
    use crate::interpreter::executor::Executor;
    use crate::lexer::span::Span;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn uniqueTempDir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let tempRoot = env::temp_dir().canonicalize().unwrap();
        let repoRoot = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .unwrap();
        let path = tempRoot.join(format!("rey-v0-{}-{}-{}", label, std::process::id(), nonce));
        assert!(!path.starts_with(&repoRoot) && !repoRoot.starts_with(&path));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn cliArgs(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn runRey(rel: &str) -> Result<(), String> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
        let program = resolveAndCheck(&path).map_err(|diagnostic| {
            error_line(
                &diagnostic.file.display().to_string(),
                &diagnostic.source,
                &diagnostic.span,
                &diagnostic.message,
            )
        })?;
        let mut interpreter = Interpreter::new_with_args(vec![path.to_string_lossy().into_owned()]);
        interpreter
            .interpret_resolved(&program, &path)
            .map_err(|message| format!("<runtime>:?:?: error: {}", message))
    }

    fn parseErr(rel: &str) -> imports::CompileError {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
        resolveEntry(&path).unwrap_err()
    }

    #[test]
    fn matchStructPatterns() {
        runRey("src/tests/match_struct.rey").unwrap();
    }

    #[test]
    fn matchEnumVariantsQualifiedAndUnqualified() {
        runRey("src/tests/match_enum.rey").unwrap();
    }

    #[test]
    fn fsAndProcessBuiltinsMore() {
        runRey("src/tests/test_fs_process_builtins_more.rey").unwrap();
    }

    #[test]
    fn importGroupedMissingSymbolPointsAtMissingName() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/imports/errors/group_missing_symbol.rey");
        let err = resolveEntry(&path).unwrap_err();
        let got = err
            .source
            .get(err.span.start..err.span.end)
            .unwrap_or("")
            .to_string();
        assert_eq!(got, "nope");
    }

    #[test]
    fn importModuleMainCanImportLocalFile() {
        runRey("../../tests/imports/success/nested_resolution.rey").unwrap();
    }

    #[test]
    fn integerDivisionSemantics() {
        runRey("src/tests/integer_division.rey").unwrap();
    }

    #[test]
    fn structFieldMutationPubWorks() {
        runRey("src/tests/struct_field_mutation.rey").unwrap();
    }

    #[test]
    fn structFieldMutationNestedErrorsClearly() {
        let err = runRey("src/tests/struct_field_mutation_nested_error.rey").unwrap_err();
        assert!(err.contains("nested field assignment"), "got: {}", err);
    }

    #[test]
    fn parserReportsStandardSemicolonError() {
        let err = parseErr("src/tests/syntax_missing_semicolon.rey");
        assert_eq!(err.title, "syntax");
        assert_eq!(err.message, "expected ';'");
    }

    #[test]
    fn matchArmsRequireCommaSeparator() {
        let err = parseErr("src/tests/match_missing_comma.rey");
        assert_eq!(err.title, "syntax");
        assert!(err.message.contains("Expected ',' between match arms."));
    }

    #[test]
    fn enumVariantsRequireCommaSeparator() {
        let err = parseErr("src/tests/enum_missing_comma.rey");
        assert_eq!(err.title, "syntax");
        assert!(err.message.contains("Expected ',' between enum variants."));
    }

    #[test]
    fn runtimeUsesReferenceEqualityForArrayAndStruct() {
        runRey("src/tests/runtime_ref_equality.rey").unwrap();
    }

    #[test]
    fn runtimeReportsNullDereference() {
        let executor = Executor::new();
        let mut env = Environment::new();
        let expr = Expr::Get {
            object: Box::new(Expr::Literal {
                value: Literal::Null,
                span: Span::new(0, 0),
            }),
            name: "field".to_string(),
            span: Span::new(0, 0),
        };
        let err = executor.evaluate_expr(&expr, &mut env).unwrap_err();
        assert!(err.contains("null dereference at line"), "got: {}", err);
    }

    #[test]
    fn runtimeReportsIndexBoundsWithIndexAndLength() {
        let err = runRey("src/tests/runtime_index_oob.rey").unwrap_err();
        assert!(
            err.contains("index out of bounds (i=4, len=3)"),
            "got: {}",
            err
        );
    }

    #[test]
    fn matchReportsNonExhaustivePatterns() {
        let err = runRey("src/tests/match_non_exhaustive.rey").unwrap_err();
        assert!(
            err.contains("TypeError: non-exhaustive match for enum 'Direction'"),
            "got: {}",
            err
        );
    }

    #[test]
    fn runtimeReportsNonExhaustiveMatchFallback() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/match_non_exhaustive.rey");
        let program = resolveEntry(&path).expect("fixture should parse and resolve");
        let executor = Executor::new();
        let mut environment = Environment::new();
        let err = executor
            .execute_block(&program.statements, &mut environment)
            .unwrap_err();
        assert_eq!(err, "error[match]: non-exhaustive patterns");
    }

    #[test]
    fn checkTypechecksWithoutInterpretingProgram() {
        let dir = uniqueTempDir("check-no-run");
        let source = dir.join("program.rey");
        let effect = dir.join("must-not-exist.txt");
        let escaped = effect
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        fs::write(
            &source,
            format!("writeFile(\"{}\", \"executed\");\n", escaped),
        )
        .unwrap();

        assert_eq!(run_cli(&cliArgs(&["check", source.to_str().unwrap()])), 0);
        assert!(
            !effect.exists(),
            "check executed the program's writeFile call"
        );
    }

    #[test]
    fn diagnosticsUseLexerSourceCoordinates() {
        let span = Span::new_at(4, 5, 2, 3);
        assert_eq!(location("a\n  x", &span), Some((2, 3)));
        assert_eq!(location("a\n  x", &Span::new(4, 5)), Some((2, 3)));
        assert_eq!(location("", &Span::new(0, 1)), None);
    }

    #[test]
    fn runtimeStringWithoutLocationDoesNotInventCoordinates() {
        assert_eq!(
            format!("<runtime>:?:?: error: {}", "null dereference at line 45"),
            "<runtime>:?:?: error: null dereference at line 45"
        );
    }

    #[test]
    fn runPassesProgramArgumentsWithoutChangingBoundaries() {
        assert_eq!(
            parse_cli_args(&cliArgs(&[
                "run",
                "compiler main.rey",
                "--",
                "build",
                "file with spaces.rey",
                "--release",
                "",
            ])),
            Ok(CliCommand::Run {
                file: "compiler main.rey".to_string(),
                programArgs: vec![
                    "build".to_string(),
                    "file with spaces.rey".to_string(),
                    "--release".to_string(),
                    "".to_string(),
                ],
            })
        );
        assert_eq!(
            parse_cli_args(&cliArgs(&["run", "app.rey"])),
            Ok(CliCommand::Run {
                file: "app.rey".to_string(),
                programArgs: vec![],
            })
        );
    }

    #[test]
    fn runInjectsScriptAndUserArgsWithoutCliWrapperTokens() {
        let dir = uniqueTempDir("cli-normalized-args");
        let source = dir.join("args.rey");
        let observed = dir.join("observed.txt");
        let escaped = observed
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        let program = format!(
            "writeFile(\"{}\", args()[0] + \"|\" + args()[1] + \"|\" + args()[2] + \"|\" + args()[3] + \"END\");\n",
            escaped
        );
        fs::write(&source, program).unwrap();
        let mut argv = vec![
            "run".to_string(),
            source.to_str().unwrap().to_string(),
            "--".to_string(),
            "arg with spaces".to_string(),
            "--dash".to_string(),
        ];
        argv.push(String::new());

        assert_eq!(run_cli(&argv), 0);
        assert_eq!(
            fs::read_to_string(observed).unwrap(),
            format!("{}|arg with spaces|--dash|END", source.display())
        );
    }

    #[test]
    fn checkAndBuildRejectTrailingArguments() {
        assert_eq!(
            parse_cli_args(&cliArgs(&["check", "app.rey", "--", "arg"])),
            Err("unexpected arguments".to_string())
        );
        assert_eq!(
            parse_cli_args(&cliArgs(&["build", "app.rey", "arg"])),
            Err("unexpected arguments".to_string())
        );
    }

    #[test]
    fn helpUsageDocumentsProgramArgumentSeparator() {
        assert!(USAGE.starts_with("usage: rey-v0 run <file> [-- <program-args...>]\n"));
        assert!(USAGE.lines().all(|line| line.contains("rey-v0")));
    }

    #[test]
    fn missingSourceFileIsACompileErrorNotUsageError() {
        let missing = uniqueTempDir("missing-source").join("absent.rey");
        assert_eq!(run_cli(&cliArgs(&["check", missing.to_str().unwrap()])), 1);
    }

    #[test]
    fn checkRejectsPrivateImportsAndLocatesImportedErrors() {
        let privateDir = uniqueTempDir("private-import");
        let privateEntry = privateDir.join("main.rey");
        let privateSource = "import private_mod.{hidden};\nprintln(hidden());\n";
        fs::write(&privateEntry, privateSource).unwrap();
        fs::write(
            privateDir.join("private_mod.rey"),
            "func hidden(): int {\n    return 9;\n}\n",
        )
        .unwrap();

        assert_eq!(
            run_cli(&cliArgs(&["check", privateEntry.to_str().unwrap()])),
            1
        );
        let privateError = resolveAndCheck(&privateEntry).unwrap_err();
        assert_eq!(privateError.file, privateEntry.canonicalize().unwrap());
        assert_eq!(privateError.source, privateSource);
        let privateLine = error_line(
            &privateError.file.display().to_string(),
            &privateError.source,
            &privateError.span,
            &privateError.message,
        );
        assert!(privateLine.starts_with(&format!("{}:1:", privateEntry.display())));
        assert!(privateLine.contains("private"), "got: {}", privateLine);

        let moduleDir = uniqueTempDir("imported-type-error");
        let importedFile = moduleDir.join("math").join("main.rey");
        let importedSource = "pub func answer(): int {\n    return \"wrong\";\n}\n";
        fs::create_dir_all(importedFile.parent().unwrap()).unwrap();
        fs::write(&importedFile, importedSource).unwrap();
        let entry = moduleDir.join("main.rey");
        fs::write(&entry, "import math;\nprintln(math.answer());\n").unwrap();

        assert_eq!(run_cli(&cliArgs(&["check", entry.to_str().unwrap()])), 1);
        let importedError = resolveAndCheck(&entry).unwrap_err();
        assert_eq!(importedError.file, importedFile.canonicalize().unwrap());
        assert_eq!(importedError.source, importedSource);
        let importedLine = error_line(
            &importedError.file.display().to_string(),
            &importedError.source,
            &importedError.span,
            &importedError.message,
        );
        assert!(importedLine.starts_with(&format!("{}:2:", importedFile.display())));
        assert!(
            importedLine.contains("expected Int"),
            "got: {}",
            importedLine
        );
    }
}
