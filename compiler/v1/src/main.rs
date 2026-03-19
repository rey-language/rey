#![allow(non_snake_case)]

mod ast;
mod interpreter;
mod lexer;
mod parser;
mod typecheck;

use interpreter::Interpreter;
use lexer::{Lexer, TokenKind};
use parser::Parser;
use std::env;
use std::fs;

fn report_error(source: &str, span: &crate::lexer::span::Span, title: &str, message: &str) {
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
    eprintln!(" --> line {}:{}", line_num, col + 1);
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

    let source = match fs::read_to_string(&filename) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("error: could not read file '{}'", filename);
            std::process::exit(1);
        }
    };

    let mut lexer = Lexer::new(&source);
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
                report_error(&source, err.span(), "lexer", &err.message());
                std::process::exit(1);
            }
        }
    }

    let mut parser = Parser::new(tokens);
    match parser.parse() {
        Ok(ast) => {
            let mut interpreter = Interpreter::new();
            if let Err(err) = interpreter.interpret(&ast) {
                eprintln!("\x1b[1;31merror[runtime]\x1b[0m: {}", err);
                std::process::exit(1);
            }
        }
        Err(err) => {
            report_error(&source, err.span(), "syntax", &err.message());
            std::process::exit(1);
        }
    }
}
