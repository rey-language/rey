use crate::lexer::token;

use super::{
    cursor::Cursor,
    error::LexerError,
    span::Span,
    token::{Token, TokenKind},
};

pub struct Lexer<'a> {
    cursor: Cursor<'a>,}

impl<'a> Lexer<'a> {
    //new lexer
    pub fn new(input: &'a str) -> Self {
        Self {
            cursor: Cursor::new(input),
        } }

    //next token
    pub fn nextToken(&mut self) -> Result<Token, LexerError> {
        // 1.skip whitespace
        while let Some(ch) = self.cursor.peek() {
            if ch.is_whitespace() {
                self.cursor.advance();
            } else {
                break;
            }
        }
        let start = self.cursor.position();

        // 2.end of sc input
        let ch = match self.cursor.advance() {
            Some(c) => c,
            None => {
                return Ok(Token {
                    kind: TokenKind::Eof,
                    span: Span::new(start, start),
                }); }
        };
        match ch {
            '"' => self.lexString(start),

            c if c.is_alphabetic() || c == '_' => {
                Ok(self.lexIdentifier(start, c))
            }

            '(' => Ok(self.simpleToken(TokenKind::LeftParen, start)),
            ')' => Ok(self.simpleToken(TokenKind::RightParen, start)),
            '{' => Ok(self.simpleToken(TokenKind::LeftBrace, start)),
            '}' => Ok(self.simpleToken(TokenKind::RightBrace, start)),
            ';' => Ok(self.simpleToken(TokenKind::Semicolon, start)),
            '+' => Ok(self.simpleToken(TokenKind::Plus, start)),
            '-' => Ok(self.simpleToken(TokenKind::Minus, start)),
            '*' => Ok(self.simpleToken(TokenKind::Star, start)),
            '/' => Ok(self.simpleToken(TokenKind::Slash, start)),
            ':' => Ok(self.simpleToken(TokenKind::Colon, start)),
            '!' => Ok(self.simpleToken(TokenKind::Not, start)),
            '.' => Ok(self.simpleToken(TokenKind::Dot, start)),
            ',' => Ok(self.simpleToken(TokenKind::Comma, start)),
            '%' => Ok(self.simpleToken(TokenKind::Percent, start)),
            

            '=' => {
                let kind = if self.matchNext('=') {
                TokenKind::EqualEqual
                    } else {
                        TokenKind::Equal
                    };
                    Ok(self.simpleToken(kind, start))
            }
            '<' => {
                let kind = if self.matchNext('=') {
                    TokenKind::LessEqual
                } else {
                    TokenKind::Less
                };
                Ok(self.simpleToken(kind, start))}
            '>' => {
                let kind = if self.matchNext('=') {
                    TokenKind::GreaterEqual}
             else {
                    TokenKind::Greater
                };
                Ok(self.simpleToken(kind, start))
            }


            int if int.is_digit(10) => {
                let mut number = String::new();
                number.push(int);

                while let Some(ch) = self.cursor.peek() {
                    if ch.is_digit(10) || ch == '.' {
                        self.cursor.advance();
                        number.push(ch);
                    } else {
                        break;
                    }
                }

                let value: f64 = number.parse().unwrap();
                Ok(Token {
                    kind: TokenKind::NumberLiteral(value),
                    span: Span::new(start, self.cursor.position()),
                })
            }

            _ => Err(LexerError::UnexpectedCharacter {
                found: ch,
                span: Span::new(start, self.cursor.position()),
            }),
        }
    }
    fn matchNext(&mut self, expected: char) -> bool {
     match self.cursor.peek() {
        Some(ch) if ch == expected => {
            self.cursor.advance();
            true
        }
        _ => false,
     }
        }
    fn lexString(&mut self, start: usize) -> Result<Token, LexerError> {
        let mut value = String::new();

        while let Some(ch) = self.cursor.advance() {
            if ch == '"' {
                return Ok(Token {
                    kind: TokenKind::StringLiteral(value),
                    span: Span::new(start, self.cursor.position()),
                });
            }
            value.push(ch);
        }

        Err(LexerError::UnterminatedString {
            span: Span::new(start, self.cursor.position()),
        })
    }
    fn lexIdentifier(&mut self, start: usize, first: char) -> Token {
        let mut ident = String::new();
        ident.push(first);

        while let Some(ch) = self.cursor.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                self.cursor.advance();
                ident.push(ch);
            } else {
                break;
            }
        }

        let kind = match ident.as_str() {
            "var" => TokenKind::Var,
            "func" => TokenKind::Func,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            _ => TokenKind::Identifier(ident),
        };

        Token {
            kind,
            span: Span::new(start, self.cursor.position()),}
    }

    fn simpleToken(&self, kind: TokenKind, start: usize) -> Token {
        Token {
            kind,
            span: Span::new(start, start + 1),}
 }
}
