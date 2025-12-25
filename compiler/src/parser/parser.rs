use crate::ast::{Expr, Literal, Parameter, Stmt, Type};
use crate::lexer::{Token, TokenKind};
use crate::parser::error::ParserError;

//impl for recursive descent parser
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}
impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
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
            Ok(Some(self.parseVarDeclaration()?))
        } else if self.matchToken(&TokenKind::Func) {
            Ok(Some(self.parseFuncDeclaration()?))
        } else {
            Ok(Some(self.parseExpressionStatement()?))
        }
    }
    fn parseVarDeclaration(&mut self) -> Result<Stmt, ParserError> {
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

        Ok(Stmt::VarDecl { name, ty, initializer })
    }

    fn parseFuncDeclaration(&mut self) -> Result<Stmt, ParserError> {
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
            params,
            return_ty,
            body,
        })
    }

    fn parseExpressionStatement(&mut self) -> Result<Stmt, ParserError> {
        let expr = self.parseExpression()?;
        self.consume(&TokenKind::Semicolon, "Expected ';' after expression.")?;
        Ok(Stmt::ExprStmt(expr))
    }

    fn parseTypeAnnotation(&mut self) -> Result<Option<Type>, ParserError> {
        if self.matchToken(&TokenKind::Colon) {
            match &self.peek().kind {
                TokenKind::Identifier(name) => {
                    let ty = Type { name: name.clone() };
                    self.advance();
                    Ok(Some(ty))
                }
                _ => Err(self.error("Expected type name after ':'")),
            }
        } else {
            Ok(None)
        }
    }

    fn parseUnary(&mut self) -> Result<Expr, ParserError> {
        match &self.peek().kind {
            TokenKind::Minus => {
                self.advance();
                let expr = self.parseUnary()?;
                Ok(Expr::Binary {
                    left: Box::new(Expr::Literal(Literal::Number(0.0))),
                    op: TokenKind::Minus,
                    right: Box::new(expr),
                })
            }
            _ => self.parsePrimary(),
        }
    }

    fn parsePrimary(&mut self) -> Result<Expr, ParserError> {
        match self.peek().kind.clone() {
            TokenKind::Identifier(name) => {
                self.advance();
                if self.matchToken(&TokenKind::LeftParen) {
                    // Function call
                    let callee = Expr::Variable(name);
                    let mut args = Vec::new();

                    if !self.check(&TokenKind::RightParen) {
                        loop {
                            args.push(self.parseExpression()?);
                            if !self.matchToken(&TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    self.consume(&TokenKind::RightParen, "Expected ')' after function arguments.")?;
                    Ok(Expr::Call {
                        callee: Box::new(callee),
                        args,
                    })
                } else {
                    Ok(Expr::Variable(name))
                }
            }
            TokenKind::StringLiteral(value) => {
                self.advance();
                Ok(Expr::Literal(Literal::String(value)))
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
            _ => Err(self.error("Expected expression.")),
        }
    }

    fn parseAdditive(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parseUnary()?;

        while matches!(self.peek().kind, TokenKind::Plus | TokenKind::Minus) {
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

    //expressions
    fn parseExpression(&mut self) -> Result<Expr, ParserError> {
        self.parseAdditive()
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
    fn consume(&mut self, kind: &TokenKind, message: &str) -> Result<(), ParserError> {
        if self.check(kind) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(message))
        }
    }
    fn consumeIdentifier(&mut self, message: &str) -> Result<(), ParserError> {
        match &self.peek().kind {
            TokenKind::Identifier(_) => {
                self.advance();
                Ok(())
            }
            _ => Err(self.error(message)),
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
        &self.tokens[self.current]
    }
    fn previous(&self) -> &Token {
        if self.current == 0 {
            &self.tokens[0]
        } else {
            &self.tokens[self.current - 1]
        }
    }

    //error
    fn error(&self, message: &str) -> ParserError {
        ParserError::Custom {
            message: message.to_string(),
            span: self.peek().span,
        }
    }
    
}
