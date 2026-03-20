use super::{
    cursor::Cursor,
    error::LexerError,
    span::Span,
    token::{Token, TokenKind},
};

pub struct Lexer<'a> {
    cursor: Cursor<'a>,
}

impl<'a> Lexer<'a> {
    //new lexer
    pub fn new(input: &'a str) -> Self {
        Self {
            cursor: Cursor::new(input),
        }
    }

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
                });
            }
        };
        match ch {
            '"' => {
                if self.cursor.peek() == Some('"') && self.cursor.peekN(1) == Some('"') {
                    self.cursor.advance();
                    self.cursor.advance();
                    self.lexMultilineString(start)
                } else {
                    self.lexString(start)
                }
            }
            '\'' => self.lexChar(start),

            c if c.is_alphabetic() || c == '_' => Ok(self.lexIdentifier(start, c)),

            '(' => Ok(self.simpleToken(TokenKind::LeftParen, start)),
            ')' => Ok(self.simpleToken(TokenKind::RightParen, start)),
            '{' => Ok(self.simpleToken(TokenKind::LeftBrace, start)),
            '}' => Ok(self.simpleToken(TokenKind::RightBrace, start)),
            '[' => Ok(self.simpleToken(TokenKind::LeftBracket, start)),
            ']' => Ok(self.simpleToken(TokenKind::RightBracket, start)),
            ';' => Ok(self.simpleToken(TokenKind::Semicolon, start)),
            '+' => {
                let kind = if self.matchNext('+') {
                    TokenKind::PlusPlus
                } else if self.matchNext('=') {
                    TokenKind::PlusEqual
                } else {
                    TokenKind::Plus
                };
                Ok(Token {
                    kind,
                    span: Span::new(start, self.cursor.position()),
                })
            }
            '-' => {
                let kind = if self.matchNext('-') {
                    TokenKind::MinusMinus
                } else if self.matchNext('=') {
                    TokenKind::MinusEqual
                } else {
                    TokenKind::Minus
                };
                Ok(Token {
                    kind,
                    span: Span::new(start, self.cursor.position()),
                })
            }
            '*' => {
                let kind = if self.matchNext('=') {
                    TokenKind::StarEqual
                } else {
                    TokenKind::Star
                };
                Ok(Token {
                    kind,
                    span: Span::new(start, self.cursor.position()),
                })
            }
            '/' => {
                // check for comment
                if let Some('/') = self.cursor.peek() {
                    // skip until end of line
                    self.cursor.advance();
                    while let Some(ch) = self.cursor.peek() {
                        if ch == '\n' {
                            self.cursor.advance();
                            break;
                        }
                        self.cursor.advance();
                    }
                    // recurse to get next token
                    return self.nextToken();
                }
                let kind = if self.matchNext('=') {
                    TokenKind::SlashEqual
                } else {
                    TokenKind::Slash
                };
                Ok(Token {
                    kind,
                    span: Span::new(start, self.cursor.position()),
                })
            }
            ':' => {
                let kind = if self.matchNext(':') {
                    TokenKind::ColonColon
                } else {
                    TokenKind::Colon
                };
                Ok(self.simpleToken(kind, start))
            }
            '?' => Ok(self.simpleToken(TokenKind::Question, start)),
            '.' => {
                if self.cursor.peek() == Some('.') && self.cursor.peekN(1) == Some('.') {
                    self.cursor.advance();
                    self.cursor.advance();
                    Ok(Token {
                        kind: TokenKind::Ellipsis,
                        span: Span::new(start, self.cursor.position()),
                    })
                } else {
                    Ok(self.simpleToken(TokenKind::Dot, start))
                }
            }
            ',' => Ok(self.simpleToken(TokenKind::Comma, start)),
            '%' => {
                let kind = if self.matchNext('=') {
                    TokenKind::PercentEqual
                } else {
                    TokenKind::Percent
                };
                Ok(Token {
                    kind,
                    span: Span::new(start, self.cursor.position()),
                })
            }

            '&' => {
                if self.matchNext('&') {
                    Ok(Token {
                        kind: TokenKind::AndAnd,
                        span: Span::new(start, self.cursor.position()),
                    })
                } else {
                    Err(LexerError::UnexpectedCharacter {
                        found: ch,
                        span: Span::new(start, self.cursor.position()),
                    })
                }
            }
            '|' => {
                if self.matchNext('|') {
                    Ok(Token {
                        kind: TokenKind::OrOr,
                        span: Span::new(start, self.cursor.position()),
                    })
                } else {
                    Ok(Token {
                        kind: TokenKind::Pipe,
                        span: Span::new(start, self.cursor.position()),
                    })
                }
            }

            '=' => {
                let kind = if self.matchNext('=') {
                    TokenKind::EqualEqual
                } else if self.matchNext('>') {
                    TokenKind::Arrow
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
                Ok(self.simpleToken(kind, start))
            }
            '>' => {
                let kind = if self.matchNext('=') {
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::Greater
                };
                Ok(self.simpleToken(kind, start))
            }
            '!' => {
                let kind = if self.matchNext('=') {
                    TokenKind::NotEqual
                } else {
                    TokenKind::Not
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
            if ch == '\\' {
                // Handle escaped characters
                if let Some(escaped) = self.cursor.advance() {
                    match escaped {
                        '"' => value.push('"'),
                        '\\' => value.push('\\'),
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        _ => value.push(escaped),
                    }
                } else {
                    return Err(LexerError::UnterminatedString {
                        span: Span::new(start, self.cursor.position()),
                    });
                }
            } else if ch == '"' {
                return Ok(Token {
                    kind: TokenKind::StringLiteral(value),
                    span: Span::new(start, self.cursor.position()),
                });
            } else {
                value.push(ch);
            }
        }

        Err(LexerError::UnterminatedString {
            span: Span::new(start, self.cursor.position()),
        })
    }

    fn lexMultilineString(&mut self, start: usize) -> Result<Token, LexerError> {
        let mut value = String::new();

        while let Some(ch) = self.cursor.advance() {
            if ch == '"' && self.cursor.peek() == Some('"') && self.cursor.peekN(1) == Some('"') {
                self.cursor.advance();
                self.cursor.advance();
                if value.starts_with("\r\n") {
                    value.drain(..2);
                } else if value.starts_with('\n') {
                    value.drain(..1);
                }
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

    fn lexChar(&mut self, start: usize) -> Result<Token, LexerError> {
        let ch = match self.cursor.advance() {
            Some(c) => c,
            None => {
                return Err(LexerError::UnterminatedChar {
                    span: Span::new(start, self.cursor.position()),
                });
            }
        };

        let value = if ch == '\\' {
            let esc = self
                .cursor
                .advance()
                .ok_or_else(|| LexerError::UnterminatedChar {
                    span: Span::new(start, self.cursor.position()),
                })?;
            match esc {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '\\' => '\\',
                '\'' => '\'',
                other => other,
            }
        } else {
            ch
        };

        match self.cursor.advance() {
            Some('\'') => Ok(Token {
                kind: TokenKind::CharLiteral(value),
                span: Span::new(start, self.cursor.position()),
            }),
            Some(other) => Err(LexerError::UnexpectedCharacter {
                found: other,
                span: Span::new(start, self.cursor.position()),
            }),
            None => Err(LexerError::UnterminatedChar {
                span: Span::new(start, self.cursor.position()),
            }),
        }
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
            "const" => TokenKind::Const,
            "func" => TokenKind::Func,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "loop" => TokenKind::Loop,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "enum" => TokenKind::Enum,
            "match" => TokenKind::Match,
            "instanceof" => TokenKind::InstanceOf,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "struct" => TokenKind::Struct,
            "pub" => TokenKind::Pub,
            "self" => TokenKind::SelfKw,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            _ => TokenKind::Identifier(ident),
        };

        Token {
            kind,
            span: Span::new(start, self.cursor.position()),
        }
    }

    fn simpleToken(&self, kind: TokenKind, start: usize) -> Token {
        Token {
            kind,
            span: Span::new(start, start + 1),
        }
    }
}
