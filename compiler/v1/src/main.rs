#![allow(non_snake_case)]

mod ast;
mod interpreter;
mod lexer;
mod parser;

use interpreter::Interpreter;
use lexer::{Lexer, TokenKind};
use parser::Parser;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let filename = if args.len() > 1 {
        args[1].clone()
    } else {
        "".to_string()
    };
    if filename.is_empty() {
        println!("No filename provided");
        return;
    }

    let source = fs::read_to_string(&filename)
        .unwrap_or_else(|_| panic!("Failed to read {} file", filename));

    let mut lexer = Lexer::new(&source);
    let mut tokens = Vec::new();

    let mut has_lexer_error = false;
    loop {
        match lexer.nextToken() {
            Ok(token) => {
                tokens.push(token.clone());
                if token.kind == TokenKind::Eof {
                    break;
                }
            }
            Err(err) => {
                println!("Lexer error: {:?}", err);
                has_lexer_error = true;
                break;
            }
        }
    }
    if has_lexer_error {
        return;
    }
    println!("Parsing Started.");
    let mut parser = Parser::new(tokens);
    match parser.parse() {
        Ok(ast) => {
            let mut interpreter = Interpreter::new();
            match interpreter.interpret(&ast) {
                Ok(()) => {
                    println!("Program executed successfully!");
                }
                Err(err) => {
                    println!("Runtime error: {}", err);
                }
            }
        }
        Err(err) => {
            println!("Parser error: {:?}", err);
        }
    }
}
