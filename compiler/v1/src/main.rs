#![allow(non_snake_case)]

mod ast;
mod imports;
mod interpreter;
mod lexer;
mod parser;
mod typecheck;

use imports::resolveEntry;
use interpreter::Interpreter;
use std::env;
use std::path::PathBuf;

fn report_error(
    filename: &str,
    source: &str,
    span: &crate::lexer::span::Span,
    title: &str,
    message: &str,
) {
    let mut line_num = 1;
    let mut line_start = 0;
    for (i, c) in source.char_indices() {
        if i == span.start {
            break;
        }
        if c == '\n' {
            line_num += 1;
            line_start = i + 1;
        }
    }

    let mut line_end = source.len();
    for (i, c) in source[line_start..].char_indices() {
        if c == '\n' {
            line_end = line_start + i;
            break;
        }
    }

    let line_str = &source[line_start..line_end];
    let col = span.start.saturating_sub(line_start);
    let width = span.end.saturating_sub(span.start).max(1);

    let red = "\x1b[1;31m";
    let reset = "\x1b[0m";
    eprintln!("{}error[{}]{}: {}", red, title, reset, message);
    eprintln!(" --> {}:{}:{}", filename, line_num, col + 1);
    eprintln!("  |");
    eprintln!("{:>2} | {}", line_num, line_str);
    eprintln!(
        "   | {}{}{}{}",
        " ".repeat(col),
        red,
        "^".repeat(width),
        reset
    );
    eprintln!();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let filename = if args.len() > 1 {
        args[1].clone()
    } else {
        "".to_string()
    };

    if filename.is_empty() {
        eprintln!("error: no file provided");
        eprintln!("usage: rey <file.rey>");
        std::process::exit(1);
    }

    let entryPath = PathBuf::from(&filename);
    match resolveEntry(&entryPath) {
        Ok(program) => {
            let mut interpreter = Interpreter::new();
            if let Err(err) = interpreter.interpret(&program.statements) {
                eprintln!("\x1b[1;31merror[runtime]\x1b[0m: {}", err);
                std::process::exit(1);
            }
        }
        Err(err) => {
            report_error(
                &err.file.display().to_string(),
                &err.source,
                &err.span,
                &err.title,
                &err.message,
            );
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runRey(rel: &str) -> Result<(), String> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
        let program = resolveEntry(&path).map_err(|e| format!("{}: {}", e.title, e.message))?;
        let mut interpreter = Interpreter::new();
        interpreter
            .interpret(&program.statements)
            .map_err(|e| e.to_string())
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
}
