mod ast;
mod lexer;
mod parser;

use lexer::{Lexer, TokenKind};
use parser::Parser;
use std::fs;

fn main() {
    let source = fs::read_to_string("../scripts/tests/hello.rey")
        .expect("Failed to read hello.rey file");

    let mut lexer = Lexer::new(&source);
    let mut tokens = Vec::new();

    loop {
        match lexer.nextToken() {
            Ok(token) => {
                println!("{:?}", token);
                tokens.push(token.clone());
                if token.kind == TokenKind::Eof {
                    break;
                }
            }
            Err(err) => {
                println!("Lexer error: {:?}", err);
                break;
            }
        }
    }
    println!("Parsing Started.");
    let mut parser = Parser::new(tokens);
    match parser.parse() {
        Ok(ast) => {
            println!("Program parsed!");
            println!("AST:");
            for stmt in &ast {
                println!("{:?}", stmt);
            }
        }
        Err(err) => {
            println!("Parser error: {:?}", err);
        }
    }
}
