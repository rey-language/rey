//DO NOT TOUCH THIS CODE AT ALL
//i've spent 6 hours on this, and any changes breaks this piece of shit
//its fragile as hell
//if u do atempt it, add aounts of hours spent = 0

//all the best :)

#![allow(non_snake_case)]

use crate::ast::{Expr, FunctionVisibility, ImportKind, Literal, Parameter, Stmt, Type};
use crate::lexer::{span::Span, Token, TokenKind};
use crate::parser::error::ParserError;

//impl for recursive descent parser
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}
impl Parser {
    pub fn new(mut tokens: Vec<Token>) -> Self {
        if tokens.is_empty() {
            tokens.push(Token {
                kind: TokenKind::Eof,
                span: Span::new(0, 0),
            });
        }
        Self { tokens, current: 0 }
    }
    pub fn parse(&mut self) -> Result<Vec<Stmt>, ParserError> {
        let mut statements = Vec::new();
        while !self.isAtEnd() {
            if let Some(stmt) = self.parseStatement()? {
                statements.push(stmt);
            }
        }
        Ok(statements)
    }

    //parsing statements
    fn parseStatement(&mut self) -> Result<Option<Stmt>, ParserError> {
        if self.isAtEnd() {
            return Ok(None);
        }
        if self.matchToken(&TokenKind::Var) {
            Ok(Some(self.parseVarDeclaration(false)?))
        } else if self.matchToken(&TokenKind::Const) {
            Ok(Some(self.parseVarDeclaration(true)?))
        } else if self.matchToken(&TokenKind::Import) {
            Ok(Some(self.parseImportStatement()?))
        } else if self.matchToken(&TokenKind::Export) {
            self.consume(&TokenKind::Pub, "Expected 'pub' after 'export'.")?;
            self.consume(
                &TokenKind::Func,
                "Expected 'func' after 'export pub' modifier.",
            )?;
            Ok(Some(self.parseFuncDeclaration(FunctionVisibility::ExportPub)?))
        } else if self.matchToken(&TokenKind::Pub) {
            self.consume(&TokenKind::Func, "Expected 'func' after 'pub' modifier.")?;
            Ok(Some(self.parseFuncDeclaration(FunctionVisibility::Pub)?))
        } else if self.matchToken(&TokenKind::Func) {
            Ok(Some(self.parseFuncDeclaration(FunctionVisibility::Private)?))
        } else if self.matchToken(&TokenKind::If) {
            Ok(Some(self.parseIfStatement()?))
        } else if self.matchToken(&TokenKind::While) {
            Ok(Some(self.parseWhileStatement()?))
        } else if self.matchToken(&TokenKind::For) {
            Ok(Some(self.parseForStatement()?))
        } else if self.matchToken(&TokenKind::Break) {
            Ok(Some(self.parseBreakStatement()?))
        } else if self.matchToken(&TokenKind::Continue) {
            Ok(Some(self.parseContinueStatement()?))
        } else if self.matchToken(&TokenKind::Return) {
            Ok(Some(self.parseReturnStatement()?))
        } else {
            Ok(Some(self.parseExpressionStatement()?))
        }
    }
    fn parseVarDeclaration(&mut self, is_const: bool) -> Result<Stmt, ParserError> {
        let name = match &self.peek().kind {
            TokenKind::Identifier(name) => name.clone(),
            _ => return Err(self.error("Expected variable name.")),
        };
        self.advance();

        let ty = self.parseTypeAnnotation()?;

        self.consume(&TokenKind::Equal, "Expected '=' after variable name.")?;
        let initializer = self.parseExpression()?;
        self.consume(
            &TokenKind::Semicolon,
            "Expected ';' after variable declaration.",
        )?;

        Ok(Stmt::VarDecl {
            is_const,
            name,
            ty,
            initializer,
        })
    }

    fn parseFuncDeclaration(&mut self, visibility: FunctionVisibility) -> Result<Stmt, ParserError> {
        let name = match &self.peek().kind {
            TokenKind::Identifier(name) => name.clone(),
            _ => return Err(self.error("Expected function name.")),
        };
        self.advance();

        self.consume(&TokenKind::LeftParen, "Expected '(' after function name.")?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            loop {
                let param_name = match &self.peek().kind {
                    TokenKind::Identifier(name) => name.clone(),
                    _ => return Err(self.error("Expected parameter name.")),
                };
                self.advance();

                let param_ty = self.parseTypeAnnotation()?;

                params.push(Parameter {
                    name: param_name,
                    ty: param_ty,
                });

                if !self.matchToken(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.consume(&TokenKind::RightParen, "Expected ')' after parameters.")?;

        let return_ty = self.parseTypeAnnotation()?;

        self.consume(&TokenKind::LeftBrace, "Expected '{' before function body.")?;

        let mut body = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.isAtEnd() {
            if let Some(stmt) = self.parseStatement()? {
                body.push(stmt);
            }
        }
        self.consume(&TokenKind::RightBrace, "Expected '}' after function body.")?;

        Ok(Stmt::FuncDecl {
            name,
            visibility,
            params,
            return_ty,
            body,
        })
    }

    fn parseImportStatement(&mut self) -> Result<Stmt, ParserError> {
        let import_span = self.previous().span;
        let module = match &self.peek().kind {
            TokenKind::Identifier(name) => name.clone(),
            _ => return Err(self.error("Expected module or file name after 'import'.")),
        };
        self.advance();

        let kind = if self.matchToken(&TokenKind::Dot) {
            let symbols = if self.matchToken(&TokenKind::LeftBrace) {
                let mut values = Vec::new();
                loop {
                    let name = match &self.peek().kind {
                        TokenKind::Identifier(name) => name.clone(),
                        _ => {
                            return Err(
                                self.error("Expected identifier in grouped file import list.")
                            )
                        }
                    };
                    self.advance();
                    values.push(name);
                    if !self.matchToken(&TokenKind::Comma) {
                        break;
                    }
                }
                self.consume(
                    &TokenKind::RightBrace,
                    "Expected '}' after grouped file import list.",
                )?;
                values
            } else {
                let symbol = match &self.peek().kind {
                    TokenKind::Identifier(name) => name.clone(),
                    _ => return Err(self.error("Expected symbol name after file import '.'.")),
                };
                self.advance();
                vec![symbol]
            };
            ImportKind::FileSymbols { module, symbols }
        } else if self.matchDoubleColon() {
            let items = if self.matchToken(&TokenKind::LeftBrace) {
                let mut values = Vec::new();
                loop {
                    let name = match &self.peek().kind {
                        TokenKind::Identifier(name) => name.clone(),
                        _ => return Err(self.error("Expected identifier in grouped module import list.")),
                    };
                    self.advance();
                    values.push(name);
                    if !self.matchToken(&TokenKind::Comma) {
                        break;
                    }
                }
                self.consume(
                    &TokenKind::RightBrace,
                    "Expected '}' after grouped module import list.",
                )?;
                values
            } else {
                let item = match &self.peek().kind {
                    TokenKind::Identifier(name) => name.clone(),
                    _ => return Err(self.error("Expected file name after '::' in module import.")),
                };
                self.advance();
                vec![item]
            };
            ImportKind::ModuleItems { module, items }
        } else {
            ImportKind::ModuleNamespace { module }
        };
        self.consume(&TokenKind::Semicolon, "Expected ';' after import statement.")?;
        Ok(Stmt::Import {
            kind,
            span: import_span,
        })
    }

    fn parseIfStatement(&mut self) -> Result<Stmt, ParserError> {
        self.consume(&TokenKind::LeftParen, "Expected '(' after 'if'.")?;
        let condition = self.parseExpression()?;
        self.consume(&TokenKind::RightParen, "Expected ')' after condition.")?;

        self.consume(&TokenKind::LeftBrace, "Expected '{' after condition.")?;
        let then_branch = self.parseBlock()?;
        self.consume(&TokenKind::RightBrace, "Expected '}' after then branch.")?;

        let else_branch = if self.matchToken(&TokenKind::Else) {
            self.consume(&TokenKind::LeftBrace, "Expected '{' after 'else'.")?;
            let block = self.parseBlock()?;
            self.consume(&TokenKind::RightBrace, "Expected '}' after else branch.")?;
            Some(block)
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parseWhileStatement(&mut self) -> Result<Stmt, ParserError> {
        self.consume(&TokenKind::LeftParen, "Expected '(' after 'while'.")?;
        let condition = self.parseExpression()?;
        self.consume(&TokenKind::RightParen, "Expected ')' after condition.")?;

        self.consume(&TokenKind::LeftBrace, "Expected '{' after condition.")?;
        let body = self.parseBlock()?;
        self.consume(&TokenKind::RightBrace, "Expected '}' after while body.")?;

        Ok(Stmt::While { condition, body })
    }

    fn parseForStatement(&mut self) -> Result<Stmt, ParserError> {
        // Parse variable name
        let variable = match self.peek().kind {
            TokenKind::Identifier(ref name) => name.clone(),
            _ => return Err(self.error("Expected variable name after 'for'.")),
        };
        self.advance();

        self.consume(&TokenKind::In, "Expected 'in' after variable name.")?;
        match self.peek().kind {
            TokenKind::Identifier(ref name) if name == "range" => {
                self.advance();
            }
            _ => return Err(self.error("Expected 'range' after 'in'.")),
        }
        self.consume(&TokenKind::LeftParen, "Expected '(' after 'range'.")?;

        let start = self.parseExpression()?;
        self.consume(&TokenKind::Comma, "Expected ',' after start value.")?;
        let end = self.parseExpression()?;

        self.consume(&TokenKind::RightParen, "Expected ')' after end value.")?;
        self.consume(&TokenKind::LeftBrace, "Expected '{' after range.")?;

        let body = self.parseBlock()?;

        self.consume(&TokenKind::RightBrace, "Expected '}' after for body.")?;

        Ok(Stmt::For {
            variable,
            start,
            end,
            body,
        })
    }

    fn parseBreakStatement(&mut self) -> Result<Stmt, ParserError> {
        self.consume(&TokenKind::Semicolon, "Expected ';' after 'break'.")?;
        Ok(Stmt::Break)
    }

    fn parseContinueStatement(&mut self) -> Result<Stmt, ParserError> {
        self.consume(&TokenKind::Semicolon, "Expected ';' after 'continue'.")?;
        Ok(Stmt::Continue)
    }

    fn parseBlock(&mut self) -> Result<Vec<Stmt>, ParserError> {
        let mut statements = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.isAtEnd() {
            if let Some(stmt) = self.parseStatement()? {
                statements.push(stmt);
            }
        }
        Ok(statements)
    }

    fn parseReturnStatement(&mut self) -> Result<Stmt, ParserError> {
        let expr = self.parseExpression()?;
        self.consume(&TokenKind::Semicolon, "Expected ';' after return value.")?;
        Ok(Stmt::Return(expr))
    }

    fn parseExpressionStatement(&mut self) -> Result<Stmt, ParserError> {
        let expr = self.parseExpression()?;
        self.consume(&TokenKind::Semicolon, "Expected ';' after expression.")?;
        Ok(Stmt::ExprStmt(expr))
    }

    fn parseTypeAnnotation(&mut self) -> Result<Option<Type>, ParserError> {
        if self.matchToken(&TokenKind::Colon) {
            if self.matchToken(&TokenKind::LeftBracket) {
                let inner = match &self.peek().kind {
                    TokenKind::Identifier(name) => name.clone(),
                    _ => return Err(self.error("Expected type name inside '[]'.")),
                };
                self.advance();
                self.consume(&TokenKind::RightBracket, "Expected ']' after array type.")?;
                let mut name = format!("[{}]", inner);
                if self.matchToken(&TokenKind::Question) {
                    name.push('?');
                }
                Ok(Some(Type { name }))
            } else if self.matchToken(&TokenKind::LeftBrace) {
                let key = match &self.peek().kind {
                    TokenKind::Identifier(name) => name.clone(),
                    _ => return Err(self.error("Expected key type name inside '{}'.")),
                };
                self.advance();
                self.consume(
                    &TokenKind::Colon,
                    "Expected ':' between dict key/value types.",
                )?;
                let value = match &self.peek().kind {
                    TokenKind::Identifier(name) => name.clone(),
                    _ => return Err(self.error("Expected value type name inside '{}'.")),
                };
                self.advance();
                self.consume(&TokenKind::RightBrace, "Expected '}' after dict type.")?;
                let mut name = format!("{{{}:{}}}", key, value);
                if self.matchToken(&TokenKind::Question) {
                    name.push('?');
                }
                Ok(Some(Type { name }))
            } else {
                match &self.peek().kind {
                    TokenKind::Identifier(name) => {
                        let mut n = name.clone();
                        self.advance();
                        if self.matchToken(&TokenKind::Question) {
                            n.push('?');
                        }
                        Ok(Some(Type { name: n }))
                    }
                    _ => Err(self.error("Expected type name after ':'")),
                }
            }
        } else {
            Ok(None)
        }
    }

    fn parseUnary(&mut self) -> Result<Expr, ParserError> {
        match &self.peek().kind {
            TokenKind::PlusPlus | TokenKind::MinusMinus => {
                let op = self.peek().kind.clone();
                self.advance();
                let right = self.parseUnary()?;
                match right {
                    Expr::Variable(name) => Ok(Expr::Update {
                        name,
                        op,
                        prefix: true,
                    }),
                    _ => Err(self.error("Invalid ++/-- target.")),
                }
            }
            TokenKind::Minus => {
                self.advance();
                let expr = self.parseUnary()?;
                Ok(Expr::Binary {
                    left: Box::new(Expr::Literal(Literal::Number(0.0))),
                    op: TokenKind::Minus,
                    right: Box::new(expr),
                })
            }
            TokenKind::Not => {
                self.advance();
                let expr = self.parseUnary()?;
                Ok(Expr::Unary {
                    op: TokenKind::Not,
                    right: Box::new(expr),
                })
            }
            _ => self.parsePostfix(),
        }
    }

    fn parsePostfix(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parsePrimary()?;

        loop {
            if self.matchToken(&TokenKind::LeftParen) {
                let mut args = Vec::new();
                if !self.check(&TokenKind::RightParen) {
                    loop {
                        args.push(self.parseExpression()?);
                        if !self.matchToken(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.consume(
                    &TokenKind::RightParen,
                    "Expected ')' after function arguments.",
                )?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                };
                continue;
            }

            if self.matchToken(&TokenKind::LeftBracket) {
                let index = self.parseExpression()?;
                self.consume(&TokenKind::RightBracket, "Expected ']' after index.")?;
                expr = Expr::Index {
                    target: Box::new(expr),
                    index: Box::new(index),
                };
                continue;
            }

            if self.matchToken(&TokenKind::Dot) {
                let name = match &self.peek().kind {
                    TokenKind::Identifier(name) => name.clone(),
                    _ => return Err(self.error("Expected identifier after '.'.")),
                };
                self.advance();

                if self.matchToken(&TokenKind::LeftParen) {
                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RightParen) {
                        loop {
                            args.push(self.parseExpression()?);
                            if !self.matchToken(&TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    self.consume(
                        &TokenKind::RightParen,
                        "Expected ')' after method arguments.",
                    )?;
                    expr = Expr::MethodCall {
                        receiver: Box::new(expr),
                        name,
                        args,
                    };
                    continue;
                }
                expr = Expr::Get {
                    object: Box::new(expr),
                    name,
                };
                continue;
            }

            if self.matchToken(&TokenKind::PlusPlus) {
                match expr {
                    Expr::Variable(name) => {
                        expr = Expr::Update {
                            name,
                            op: TokenKind::PlusPlus,
                            prefix: false,
                        };
                        break;
                    }
                    _ => return Err(self.error("Invalid ++ target.")),
                }
            }

            if self.matchToken(&TokenKind::MinusMinus) {
                match expr {
                    Expr::Variable(name) => {
                        expr = Expr::Update {
                            name,
                            op: TokenKind::MinusMinus,
                            prefix: false,
                        };
                        break;
                    }
                    _ => return Err(self.error("Invalid -- target.")),
                }
            }

            break;
        }

        Ok(expr)
    }

    fn parsePrimary(&mut self) -> Result<Expr, ParserError> {
        match self.peek().kind.clone() {
            TokenKind::Identifier(name) => {
                self.advance();
                Ok(Expr::Variable(name))
            }
            TokenKind::StringLiteral(value) => {
                let span = self.peek().span;
                self.advance();
                if value.contains('{') && value.contains('}') {
                    match Self::parseStringInterpolation(value.clone(), span) {
                        Ok(expr) => return Ok(expr),
                        Err(_) => {} // Fallback to literal if interpolation fails
                    }
                }
                Ok(Expr::Literal(Literal::String(value)))
            }
            TokenKind::CharLiteral(value) => {
                self.advance();
                Ok(Expr::Literal(Literal::Char(value)))
            }
            TokenKind::NumberLiteral(value) => {
                self.advance();
                Ok(Expr::Literal(Literal::Number(value)))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(true)))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(false)))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expr::Literal(Literal::Null))
            }
            TokenKind::LeftParen => {
                self.advance();
                let expr = self.parseExpression()?;
                self.consume(&TokenKind::RightParen, "Expected ')' after expression.")?;
                Ok(expr)
            }
            TokenKind::LeftBracket => {
                self.advance();
                let mut elements = Vec::new();
                if !self.check(&TokenKind::RightBracket) {
                    loop {
                        elements.push(self.parseExpression()?);
                        if !self.matchToken(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.consume(
                    &TokenKind::RightBracket,
                    "Expected ']' after array literal.",
                )?;
                Ok(Expr::ArrayLiteral { elements })
            }
            TokenKind::LeftBrace => {
                self.advance();
                let mut entries = Vec::new();
                if !self.check(&TokenKind::RightBrace) {
                    loop {
                        let key = match self.peek().kind.clone() {
                            TokenKind::Identifier(name) => {
                                self.advance();
                                name
                            }
                            TokenKind::StringLiteral(s) => {
                                self.advance();
                                s
                            }
                            _ => {
                                return Err(
                                    self.error("Expected identifier or string as dictionary key.")
                                )
                            }
                        };
                        self.consume(&TokenKind::Colon, "Expected ':' after dictionary key.")?;
                        let value = self.parseExpression()?;
                        entries.push((key, value));
                        if !self.matchToken(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.consume(
                    &TokenKind::RightBrace,
                    "Expected '}' after dictionary literal.",
                )?;
                Ok(Expr::DictLiteral { entries })
            }
            _ => Err(self.error("Expected expression.")),
        }
    }

    fn parseStringInterpolation(value: String, _span: Span) -> Result<Expr, ParserError> {
        let mut parts: Vec<Expr> = Vec::new();
        let mut current_str = String::new();
        let mut in_expr = false;
        let mut expr_str = String::new();
        let mut brace_depth = 0;

        for c in value.chars() {
            if !in_expr {
                if c == '{' {
                    in_expr = true;
                    brace_depth = 1;
                    if !current_str.is_empty() || parts.is_empty() {
                        parts.push(Expr::Literal(Literal::String(current_str.clone())));
                        current_str.clear();
                    }
                } else {
                    current_str.push(c);
                }
            } else {
                if c == '{' {
                    brace_depth += 1;
                    expr_str.push(c);
                } else if c == '}' {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        in_expr = false;
                        let mut lexer = crate::lexer::Lexer::new(&expr_str);
                        let mut tokens = Vec::new();
                        while let Ok(token) = lexer.nextToken() {
                            if token.kind == TokenKind::Eof {
                                break;
                            }
                            tokens.push(token);
                        }
                        let mut parser = Parser::new(tokens);
                        let inner_expr = parser
                            .parseExpression()
                            .unwrap_or(Expr::Literal(Literal::Null));
                        parts.push(inner_expr);
                        expr_str.clear();
                    } else {
                        expr_str.push(c);
                    }
                } else {
                    expr_str.push(c);
                }
            }
        }
        if !current_str.is_empty() || parts.is_empty() {
            parts.push(Expr::Literal(Literal::String(current_str)));
        }

        let mut iter = parts.into_iter();
        let mut res = iter.next().unwrap();
        for part in iter {
            res = Expr::Binary {
                left: Box::new(res),
                op: TokenKind::Plus,
                right: Box::new(part),
            };
        }
        Ok(res)
    }

    fn parseAssignment(&mut self) -> Result<Expr, ParserError> {
        let expr = self.parseLogicOr()?;

        if self.matchToken(&TokenKind::Equal) {
            let value = self.parseAssignment()?;

            if let Expr::Variable(name) = expr {
                return Ok(Expr::Assign {
                    name,
                    value: Box::new(value),
                });
            }

            return Err(self.error("Invalid assignment target."));
        }

        let compound = if self.matchToken(&TokenKind::PlusEqual) {
            Some(TokenKind::Plus)
        } else if self.matchToken(&TokenKind::MinusEqual) {
            Some(TokenKind::Minus)
        } else if self.matchToken(&TokenKind::StarEqual) {
            Some(TokenKind::Star)
        } else if self.matchToken(&TokenKind::SlashEqual) {
            Some(TokenKind::Slash)
        } else if self.matchToken(&TokenKind::PercentEqual) {
            Some(TokenKind::Percent)
        } else {
            None
        };

        if let Some(op) = compound {
            let value = self.parseAssignment()?;
            if let Expr::Variable(name) = expr {
                let bin = Expr::Binary {
                    left: Box::new(Expr::Variable(name.clone())),
                    op,
                    right: Box::new(value),
                };
                return Ok(Expr::Assign {
                    name,
                    value: Box::new(bin),
                });
            }
            return Err(self.error("Invalid assignment target."));
        }

        Ok(expr)
    }

    fn parseLogicOr(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parseLogicAnd()?;
        while self.matchToken(&TokenKind::OrOr) {
            let op = self.previous().kind.clone();
            let right = self.parseLogicAnd()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parseLogicAnd(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parseEquality()?;
        while self.matchToken(&TokenKind::AndAnd) {
            let op = self.previous().kind.clone();
            let right = self.parseEquality()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }
    fn parseEquality(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parseComparison()?;
        while matches!(
            self.peek().kind,
            TokenKind::EqualEqual | TokenKind::NotEqual
        ) {
            let op = self.peek().kind.clone();
            self.advance();
            let right = self.parseComparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parseComparison(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parseTerm()?;
        while matches!(
            self.peek().kind,
            TokenKind::Greater | TokenKind::GreaterEqual | TokenKind::Less | TokenKind::LessEqual
        ) {
            let op = self.peek().kind.clone();
            self.advance();
            let right = self.parseTerm()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    //expressions
    fn parseExpression(&mut self) -> Result<Expr, ParserError> {
        self.parseAssignment()
    }

    //token utils
    fn matchToken(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn matchDoubleColon(&mut self) -> bool {
        if self.check(&TokenKind::Colon)
            && self
                .tokens
                .get(self.current + 1)
                .is_some_and(|token| matches!(token.kind, TokenKind::Colon))
        {
            self.advance();
            self.advance();
            true
        } else {
            false
        }
    }
    fn consume(&mut self, kind: &TokenKind, message: &str) -> Result<(), ParserError> {
        if self.check(kind) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(message))
        }
    }
    fn check(&self, kind: &TokenKind) -> bool {
        if self.isAtEnd() {
            return false;
        }
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }
    fn advance(&mut self) -> &Token {
        if !self.isAtEnd() {
            self.current += 1;
        }
        self.previous()
    }
    fn isAtEnd(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }
    fn peek(&self) -> &Token {
        self.tokens.get(self.current).unwrap_or_else(|| {
            self.tokens
                .last()
                .expect("parser requires at least one token")
        })
    }
    fn previous(&self) -> &Token {
        if self.current == 0 {
            &self.tokens[0]
        } else {
            &self.tokens[self.current - 1]
        }
    }
    fn parseTerm(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parseFactor()?;
        while matches!(self.peek().kind, TokenKind::Plus | TokenKind::Minus) {
            let op = self.peek().kind.clone();
            self.advance();
            let right = self.parseFactor()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parseFactor(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parseUnary()?;
        while matches!(
            self.peek().kind,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent
        ) {
            let op = self.peek().kind.clone();
            self.advance();
            let right = self.parseUnary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    //error
    fn error(&self, message: &str) -> ParserError {
        ParserError::Custom {
            message: message.to_string(),
            span: self.peek().span,
        }
    }
}
