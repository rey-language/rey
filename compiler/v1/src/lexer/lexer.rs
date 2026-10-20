use super::{
    cursor::Cursor,
    error::LexerError,
    span::Span,
    token::{StringPart, Token, TokenKind},
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
        let startLine = self.cursor.line();
        let startColumn = self.cursor.column();
        let start = self.cursor.position();

        // 2.end of sc input
        let ch = match self.cursor.advance() {
            Some(c) => c,
            None => {
                return Ok(Token {
                    kind: TokenKind::Eof,
                    span: Span::new_at(start, start, startLine, startColumn),
                });
            }
        };
        match ch {
            '"' => {
                if self.cursor.peek() == Some('"') && self.cursor.peekN(1) == Some('"') {
                    self.cursor.advance();
                    self.cursor.advance();
                    self.lexMultilineString(start, startLine, startColumn)
                } else {
                    self.lexString(start, startLine, startColumn)
                }
            }
            '\'' => self.lexChar(start, startLine, startColumn),

            c if c.is_alphabetic() || c == '_' => {
                Ok(self.lexIdentifier(start, startLine, startColumn, c))
            }

            '(' => Ok(self.simpleToken(TokenKind::LeftParen, start, startLine, startColumn)),
            ')' => Ok(self.simpleToken(TokenKind::RightParen, start, startLine, startColumn)),
            '{' => Ok(self.simpleToken(TokenKind::LeftBrace, start, startLine, startColumn)),
            '}' => Ok(self.simpleToken(TokenKind::RightBrace, start, startLine, startColumn)),
            '[' => Ok(self.simpleToken(TokenKind::LeftBracket, start, startLine, startColumn)),
            ']' => Ok(self.simpleToken(TokenKind::RightBracket, start, startLine, startColumn)),
            ';' => Ok(self.simpleToken(TokenKind::Semicolon, start, startLine, startColumn)),
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
                    span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
                })
            }
            '-' => {
                let kind = if self.matchNext('-') {
                    TokenKind::MinusMinus
                } else if self.matchNext('>') {
                    TokenKind::ThinArrow
                } else if self.matchNext('=') {
                    TokenKind::MinusEqual
                } else {
                    TokenKind::Minus
                };
                Ok(Token {
                    kind,
                    span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
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
                    span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
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
                if let Some('*') = self.cursor.peek() {
                    self.cursor.advance();
                    loop {
                        match self.cursor.advance() {
                            Some('*') if self.cursor.peek() == Some('/') => {
                                self.cursor.advance();
                                return self.nextToken();
                            }
                            Some(_) => {}
                            None => {
                                return Err(LexerError::UnterminatedBlockComment {
                                    span: Span::new_at(
                                        start,
                                        self.cursor.position(),
                                        startLine,
                                        startColumn,
                                    ),
                                });
                            }
                        }
                    }
                }
                let kind = if self.matchNext('=') {
                    TokenKind::SlashEqual
                } else {
                    TokenKind::Slash
                };
                Ok(Token {
                    kind,
                    span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
                })
            }
            ':' => {
                let kind = if self.matchNext(':') {
                    TokenKind::ColonColon
                } else {
                    TokenKind::Colon
                };
                Ok(self.simpleToken(kind, start, startLine, startColumn))
            }
            '?' => Ok(self.simpleToken(TokenKind::Question, start, startLine, startColumn)),
            '.' => {
                if self.cursor.peek() == Some('.') && self.cursor.peekN(1) == Some('.') {
                    self.cursor.advance();
                    self.cursor.advance();
                    Ok(Token {
                        kind: TokenKind::Ellipsis,
                        span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
                    })
                } else if self.matchNext('.') {
                    Ok(Token {
                        kind: TokenKind::DotDot,
                        span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
                    })
                } else {
                    Ok(self.simpleToken(TokenKind::Dot, start, startLine, startColumn))
                }
            }
            ',' => Ok(self.simpleToken(TokenKind::Comma, start, startLine, startColumn)),
            '%' => {
                let kind = if self.matchNext('=') {
                    TokenKind::PercentEqual
                } else {
                    TokenKind::Percent
                };
                Ok(Token {
                    kind,
                    span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
                })
            }

            '&' => {
                if self.matchNext('&') {
                    Ok(Token {
                        kind: TokenKind::AndAnd,
                        span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
                    })
                } else {
                    Err(LexerError::UnexpectedCharacter {
                        found: ch,
                        span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
                    })
                }
            }
            '|' => {
                if self.matchNext('|') {
                    Ok(Token {
                        kind: TokenKind::OrOr,
                        span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
                    })
                } else {
                    Ok(Token {
                        kind: TokenKind::Pipe,
                        span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
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
                Ok(self.simpleToken(kind, start, startLine, startColumn))
            }
            '<' => {
                let kind = if self.matchNext('=') {
                    TokenKind::LessEqual
                } else {
                    TokenKind::Less
                };
                Ok(self.simpleToken(kind, start, startLine, startColumn))
            }
            '>' => {
                let kind = if self.matchNext('=') {
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::Greater
                };
                Ok(self.simpleToken(kind, start, startLine, startColumn))
            }
            '!' => {
                let kind = if self.matchNext('=') {
                    TokenKind::NotEqual
                } else {
                    TokenKind::Not
                };
                Ok(self.simpleToken(kind, start, startLine, startColumn))
            }

            int if int.is_digit(10) => {
                let mut number = String::new();
                number.push(int);
                if number == "0" {
                    if let Some(prefix) = self.cursor.peek() {
                        if matches!(prefix, 'x' | 'X' | 'b' | 'B') {
                            return self.lexPrefixedInteger(
                                prefix,
                                number,
                                start,
                                startLine,
                                startColumn,
                            );
                        }
                    }
                }
                let followsMemberDot = start > 0 && self.cursor.byteAt(start - 1) == Some(b'.');

                while let Some(ch) = self.cursor.peek() {
                    if ch.is_digit(10) {
                        self.cursor.advance();
                        number.push(ch);
                    } else if ch == '_'
                        && number.chars().last().is_some_and(|last| last.is_digit(10))
                        && self.cursor.peekN(1).is_some_and(|next| next.is_digit(10))
                    {
                        self.cursor.advance();
                        number.push(ch);
                    } else if ch == '.'
                        && self.cursor.peekN(1) != Some('.')
                        && !number.contains('.')
                        && !followsMemberDot
                    {
                        self.cursor.advance();
                        number.push(ch);
                    } else {
                        break;
                    }
                }

                Ok(Token {
                    kind: TokenKind::NumberLiteral(number),
                    span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
                })
            }

            _ => Err(LexerError::UnexpectedCharacter {
                found: ch,
                span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
            }),
        }
    }
    fn lexPrefixedInteger(
        &mut self,
        prefix: char,
        mut number: String,
        start: usize,
        startLine: usize,
        startColumn: usize,
    ) -> Result<Token, LexerError> {
        self.cursor.advance();
        number.push(prefix);
        let binary = prefix == 'b' || prefix == 'B';
        let mut digits = 0usize;
        let validDigit = |ch: char| {
            if binary {
                ch == '0' || ch == '1'
            } else {
                ch.is_ascii_hexdigit()
            }
        };
        loop {
            let Some(ch) = self.cursor.peek() else { break };
            if validDigit(ch) {
                self.cursor.advance();
                number.push(ch);
                digits += 1;
            } else if ch == '_'
                && digits > 0
                && self.cursor.peekN(1).is_some_and(validDigit)
            {
                self.cursor.advance();
                number.push(ch);
            } else if ch.is_ascii_digit() || (ch == '.' && self.cursor.peekN(1) != Some('.')) {
                self.cursor.advance();
                return Err(LexerError::InvalidNumber {
                    span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
                });
            } else {
                break;
            }
        }
        if digits == 0 {
            return Err(LexerError::InvalidNumber {
                span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
            });
        }
        Ok(Token {
            kind: TokenKind::NumberLiteral(number),
            span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
        })
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
    fn lexString(
        &mut self,
        start: usize,
        startLine: usize,
        startColumn: usize,
    ) -> Result<Token, LexerError> {
        self.lexStringContents(start, startLine, startColumn, false)
    }

    fn lexMultilineString(
        &mut self,
        start: usize,
        startLine: usize,
        startColumn: usize,
    ) -> Result<Token, LexerError> {
        if self.cursor.peek() == Some('\r') && self.cursor.peekN(1) == Some('\n') {
            self.cursor.advance();
            self.cursor.advance();
        } else if self.cursor.peek() == Some('\n') {
            self.cursor.advance();
        }
        self.lexStringContents(start, startLine, startColumn, true)
    }

    fn lexStringContents(
        &mut self,
        start: usize,
        startLine: usize,
        startColumn: usize,
        multiline: bool,
    ) -> Result<Token, LexerError> {
        let mut text = Vec::new();
        let mut parts = Vec::new();

        loop {
            let ch = match self.cursor.advance() {
                Some(ch) => ch,
                None => {
                    return Err(LexerError::UnterminatedString {
                        span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
                    })
                }
            };

            if ch == '"' {
                if !multiline {
                    return Ok(self.stringToken(text, parts, start, startLine, startColumn));
                }
                if self.cursor.peek() == Some('"') && self.cursor.peekN(1) == Some('"') {
                    self.cursor.advance();
                    self.cursor.advance();
                    return Ok(self.stringToken(text, parts, start, startLine, startColumn));
                }
            }

            if !multiline && ch == '\\' {
                let escaped =
                    self.cursor
                        .advance()
                        .ok_or_else(|| LexerError::UnterminatedString {
                            span: Span::new_at(
                                start,
                                self.cursor.position(),
                                startLine,
                                startColumn,
                            ),
                        })?;
                let byte = match escaped {
                    '"' => b'"',
                    '\\' => b'\\',
                    'n' => b'\n',
                    'r' => b'\r',
                    't' => b'\t',
                    '\'' => b'\'',
                    '0' => 0,
                    'x' => self.lexHexEscape(start, startLine, startColumn)?,
                    other => {
                        return Err(LexerError::InvalidEscape {
                            escape: format!("\\{}", other),
                            span: Span::new_at(
                                start,
                                self.cursor.position(),
                                startLine,
                                startColumn,
                            ),
                        })
                    }
                };
                text.push(byte);
                continue;
            }

            match ch {
                '{' if self.matchNext('{') => text.push(b'{'),
                '{' => {
                    parts.push(StringPart::Text(std::mem::take(&mut text)));
                    let (source, span) = self.lexInterpolation(start, startLine, startColumn)?;
                    parts.push(StringPart::Expression { source, span });
                }
                '}' if self.matchNext('}') => text.push(b'}'),
                '}' => {
                    return Err(LexerError::UnmatchedStringBrace {
                        span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
                    })
                }
                other => pushUtf8Bytes(&mut text, other),
            }
        }
    }

    fn lexInterpolation(
        &mut self,
        start: usize,
        startLine: usize,
        startColumn: usize,
    ) -> Result<(String, Span), LexerError> {
        let expressionStart = self.cursor.position();
        let expressionLine = self.cursor.line();
        let expressionColumn = self.cursor.column();
        let mut expression = String::new();
        let mut depth = 1usize;
        while let Some(ch) = self.cursor.advance() {
            match ch {
                '{' => {
                    depth += 1;
                    expression.push(ch);
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((
                            expression,
                            Span::new_at(
                                expressionStart,
                                self.cursor.position() - 1,
                                expressionLine,
                                expressionColumn,
                            ),
                        ));
                    }
                    expression.push(ch);
                }
                other => expression.push(other),
            }
        }
        Err(LexerError::UnterminatedInterpolation {
            span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
        })
    }

    fn stringToken(
        &self,
        mut text: Vec<u8>,
        mut parts: Vec<StringPart>,
        start: usize,
        startLine: usize,
        startColumn: usize,
    ) -> Token {
        let kind = if parts.is_empty() {
            match String::from_utf8(text) {
                Ok(value) => TokenKind::StringLiteral(value),
                Err(error) => TokenKind::ByteStringLiteral(error.into_bytes()),
            }
        } else {
            parts.push(StringPart::Text(std::mem::take(&mut text)));
            TokenKind::InterpolatedStringLiteral(parts)
        };
        Token {
            kind,
            span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
        }
    }

    fn lexChar(
        &mut self,
        start: usize,
        startLine: usize,
        startColumn: usize,
    ) -> Result<Token, LexerError> {
        let ch = match self.cursor.advance() {
            Some(c) => c,
            None => {
                return Err(LexerError::UnterminatedChar {
                    span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
                });
            }
        };

        let escapedInput = ch == '\\';
        let value = if escapedInput {
            let esc = self
                .cursor
                .advance()
                .ok_or_else(|| LexerError::UnterminatedChar {
                    span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
                })?;
            match esc {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '\\' => '\\',
                '\'' => '\'',
                '"' => '"',
                '0' => '\0',
                'x' => char::from(self.lexHexEscape(start, startLine, startColumn)?),
                other => {
                    return Err(LexerError::InvalidEscape {
                        escape: format!("\\{}", other),
                        span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
                    })
                }
            }
        } else {
            ch
        };

        if (!escapedInput && (ch.len_utf8() != 1 || ch == '\n' || ch == '\r'))
            || value as u32 > u8::MAX as u32
        {
            return Err(LexerError::InvalidCharacterValue {
                value: value as u32,
                span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
            });
        }

        match self.cursor.advance() {
            Some('\'') => Ok(Token {
                kind: TokenKind::CharLiteral(value),
                span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
            }),
            Some(other) => Err(LexerError::UnexpectedCharacter {
                found: other,
                span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
            }),
            None => Err(LexerError::UnterminatedChar {
                span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
            }),
        }
    }
    fn lexIdentifier(
        &mut self,
        start: usize,
        startLine: usize,
        startColumn: usize,
        first: char,
    ) -> Token {
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
            "import" => TokenKind::Import,
            "export" => TokenKind::Export,
            "pub" => TokenKind::Pub,
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
            "self" => TokenKind::SelfKw,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            _ => TokenKind::Identifier(ident),
        };

        Token {
            kind,
            span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
        }
    }

    fn lexHexEscape(
        &mut self,
        start: usize,
        startLine: usize,
        startColumn: usize,
    ) -> Result<u8, LexerError> {
        let first = self.cursor.advance();
        let second = self.cursor.advance();
        let digits = match (first, second) {
            (Some(a), Some(b)) if a.is_ascii_hexdigit() && b.is_ascii_hexdigit() => {
                format!("{}{}", a, b)
            }
            (a, b) => {
                let escape = format!("\\x{}{}", a.unwrap_or('\0'), b.unwrap_or('\0'));
                return Err(LexerError::InvalidEscape {
                    escape,
                    span: Span::new_at(start, self.cursor.position(), startLine, startColumn),
                });
            }
        };
        let value = u8::from_str_radix(&digits, 16).expect("validated hexadecimal escape");
        Ok(value)
    }

    fn simpleToken(&self, kind: TokenKind, start: usize, line: usize, column: usize) -> Token {
        Token {
            kind,
            span: Span::new_at(start, self.cursor.position(), line, column),
        }
    }
}

fn pushUtf8Bytes(output: &mut Vec<u8>, ch: char) {
    let mut encoded = [0; 4];
    output.extend_from_slice(ch.encode_utf8(&mut encoded).as_bytes());
}

#[cfg(test)]
mod tests {
    use super::Lexer;
    use crate::lexer::error::LexerError;
    use crate::lexer::{StringPart, TokenKind};

    fn lexOne(source: &str) -> TokenKind {
        Lexer::new(source).nextToken().unwrap().kind
    }

    fn stringParts(source: &str) -> Vec<StringPart> {
        match lexOne(source) {
            TokenKind::InterpolatedStringLiteral(parts) => parts,
            other => panic!("expected interpolated string, got {other:?}"),
        }
    }

    #[test]
    fn string_and_char_hex_escapes_and_numeric_separators() {
        assert_eq!(
            lexOne(r#""a\x41\0\'""#),
            TokenKind::StringLiteral("aA\0'".to_string())
        );
        assert_eq!(lexOne(r#"'\x41'"#), TokenKind::CharLiteral('A'));
        assert_eq!(lexOne(r#"'\xFF'"#), TokenKind::CharLiteral('\u{FF}'));
        assert_eq!(
            lexOne("12_345"),
            TokenKind::NumberLiteral("12_345".to_string())
        );
    }

    #[test]
    fn integer_boundaries_and_range_separator_stay_integer_tokens() {
        let mut lexer = Lexer::new("9_223_372_036_854_775_807..0 9_223_372_036_854_775_808");
        let mut kinds = Vec::new();
        loop {
            let token = lexer.nextToken().unwrap();
            let atEnd = token.kind == TokenKind::Eof;
            kinds.push(token.kind);
            if atEnd {
                break;
            }
        }
        assert_eq!(
            kinds,
            vec![
                TokenKind::NumberLiteral("9_223_372_036_854_775_807".to_string()),
                TokenKind::DotDot,
                TokenKind::NumberLiteral("0".to_string()),
                TokenKind::NumberLiteral("9_223_372_036_854_775_808".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn malformed_hex_and_unknown_escapes_are_rejected() {
        for source in [
            r#""\x""#,
            r#""\x1""#,
            r#""\xG0""#,
            r#""\q""#,
            r#"'\xGG'"#,
            r#"'\q'"#,
        ] {
            assert!(
                matches!(
                    Lexer::new(source).nextToken(),
                    Err(LexerError::InvalidEscape { .. })
                ),
                "accepted malformed escape in {source:?}"
            );
        }
    }

    #[test]
    fn source_utf8_string_bytes_round_trip() {
        let sourceText = "cafe\u{301}";
        let source = format!("\"{}\"", sourceText);
        let TokenKind::StringLiteral(value) = lexOne(&source) else {
            panic!("expected string literal")
        };
        assert_eq!(value.as_bytes(), sourceText.as_bytes());
        assert_eq!(lexOne(r#"'\xE9'"#), TokenKind::CharLiteral('é'));
        assert!(matches!(
            Lexer::new("'é'").nextToken(),
            Err(LexerError::InvalidCharacterValue { value: 0xE9, .. })
        ));
        assert!(matches!(
            Lexer::new("'Ā'").nextToken(),
            Err(LexerError::InvalidCharacterValue { value: 0x100, .. })
        ));
    }

    #[test]
    fn escaped_bytes_are_decoded_before_utf8_selection() {
        assert_eq!(
            lexOne(r#""\xC3\xA9""#),
            TokenKind::StringLiteral("é".to_string())
        );
        assert_eq!(
            lexOne(r#""\x80""#),
            TokenKind::ByteStringLiteral(vec![0x80])
        );
        assert_eq!(
            lexOne(r#""\x7Bnot interpolation\x7D""#),
            TokenKind::StringLiteral("{not interpolation}".to_string())
        );
        assert_eq!(
            lexOne(r#""{{literal}}""#),
            TokenKind::StringLiteral("{literal}".to_string())
        );
        let parts = stringParts(r#""\xC3\xA9{x}""#);
        assert_eq!(parts[0], StringPart::Text(vec![0xC3, 0xA9]));
        assert!(matches!(&parts[1], StringPart::Expression { source, .. } if source == "x"));
        assert_eq!(parts[2], StringPart::Text(Vec::new()));
    }

    #[test]
    fn interpolation_keeps_byte_chunks_and_source_expression_text() {
        let parts = stringParts(r#""A\x80{'\xE9'}B""#);
        assert_eq!(parts[0], StringPart::Text(vec![b'A', 0x80]));
        assert!(matches!(&parts[1], StringPart::Expression { source, .. } if source == "'\\xE9'"));
        assert_eq!(parts[2], StringPart::Text(vec![b'B']));

        let parts = stringParts("\"\"\"\nraw {x} text\"\"\"");
        assert_eq!(parts[0], StringPart::Text(b"raw ".to_vec()));
        assert!(matches!(&parts[1], StringPart::Expression { source, .. } if source == "x"));
        assert_eq!(parts[2], StringPart::Text(b" text".to_vec()));
    }

    #[test]
    fn frozen_operator_tokens_are_lexed() {
        let mut lexer = Lexer::new("loop match instanceof null pub => -> :: ..");
        let mut kinds = Vec::new();
        loop {
            let token = lexer.nextToken().unwrap();
            if token.kind == TokenKind::Eof {
                break;
            }
            kinds.push(token.kind);
        }
        assert_eq!(
            kinds,
            vec![
                TokenKind::Loop,
                TokenKind::Match,
                TokenKind::InstanceOf,
                TokenKind::Null,
                TokenKind::Pub,
                TokenKind::Arrow,
                TokenKind::ThinArrow,
                TokenKind::ColonColon,
                TokenKind::DotDot,
            ]
        );
    }

    #[test]
    fn chainedTupleIndexesStaySeparateFromFloats() {
        let mut lexer = Lexer::new("nested.0.1 nested.0.0 1.5");
        let mut kinds = Vec::new();
        loop {
            let token = lexer.nextToken().unwrap();
            if token.kind == TokenKind::Eof {
                break;
            }
            kinds.push(token.kind);
        }
        assert_eq!(
            kinds,
            vec![
                TokenKind::Identifier("nested".to_string()),
                TokenKind::Dot,
                TokenKind::NumberLiteral("0".to_string()),
                TokenKind::Dot,
                TokenKind::NumberLiteral("1".to_string()),
                TokenKind::Identifier("nested".to_string()),
                TokenKind::Dot,
                TokenKind::NumberLiteral("0".to_string()),
                TokenKind::Dot,
                TokenKind::NumberLiteral("0".to_string()),
                TokenKind::NumberLiteral("1.5".to_string()),
            ]
        );
    }

    #[test]
    fn hexBinaryAndBlockCommentsLex() {
        let mut lexer = Lexer::new("/* header */ 0x10 0Xff 0b1010 0B1_1 0x10..2");
        let mut kinds = Vec::new();
        loop {
            let token = lexer.nextToken().unwrap();
            if token.kind == TokenKind::Eof {
                break;
            }
            kinds.push(token.kind);
        }
        assert_eq!(
            kinds,
            vec![
                TokenKind::NumberLiteral("0x10".to_string()),
                TokenKind::NumberLiteral("0Xff".to_string()),
                TokenKind::NumberLiteral("0b1010".to_string()),
                TokenKind::NumberLiteral("0B1_1".to_string()),
                TokenKind::NumberLiteral("0x10".to_string()),
                TokenKind::DotDot,
                TokenKind::NumberLiteral("2".to_string()),
            ]
        );
        assert!(matches!(
            Lexer::new("/*").nextToken(),
            Err(LexerError::UnterminatedBlockComment { .. })
        ));
        assert!(matches!(
            Lexer::new("0b2").nextToken(),
            Err(LexerError::InvalidNumber { .. })
        ));
        assert!(matches!(
            Lexer::new("0x").nextToken(),
            Err(LexerError::InvalidNumber { .. })
        ));
    }
}
