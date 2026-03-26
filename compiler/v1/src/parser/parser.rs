//DO NOT TOUCH THIS CODE AT ALL
//i've spent 6 hours on this, and any changes breaks this piece of shit
//its fragile as hell
//if u do atempt it, add aounts of hours spent = 0

//all the best :)

#![allow(non_snake_case)]

use crate::ast::{
    Expr, FieldDecl, FunctionVisibility, ImportKind, Literal, MethodDecl, Parameter, Pattern,
    Stmt, Type,
};
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
        } else if self.matchToken(&TokenKind::Struct) {
            Ok(Some(self.parseStructDeclaration()?))
        } else if self.matchToken(&TokenKind::Enum) {
            Ok(Some(self.parseEnumDeclaration()?))
        } else if self.matchToken(&TokenKind::Match) {
            Ok(Some(self.parseMatchStatement()?))
        } else if self.matchToken(&TokenKind::If) {
            Ok(Some(self.parseIfStatement()?))
        } else if self.matchToken(&TokenKind::While) {
            Ok(Some(self.parseWhileStatement()?))
        } else if self.matchToken(&TokenKind::Loop) {
            Ok(Some(self.parseLoopStatement()?))
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

                let mut variadic = false;
                let param_ty = if self.matchToken(&TokenKind::Colon) {
                    if self.matchToken(&TokenKind::Ellipsis) {
                        variadic = true;
                        let inner = self
                            .parseTypeAtom()?
                            .ok_or_else(|| self.error("Expected type name after '...'."))?;
                        Some(Type {
                            name: format!("[{}]", inner.name),
                        })
                    } else {
                        self.parseTypeOnly()?
                    }
                } else {
                    None
                };

                let default = if self.matchToken(&TokenKind::Equal) {
                    Some(self.parseExpression()?)
                } else {
                    None
                };

                params.push(Parameter {
                    name: param_name,
                    ty: param_ty,
                    default,
                    variadic,
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

    fn parseStructDeclaration(&mut self) -> Result<Stmt, ParserError> {
        let name = match &self.peek().kind {
            TokenKind::Identifier(name) => name.clone(),
            _ => return Err(self.error("Expected struct name.")),
        };
        self.advance();

        self.consume(&TokenKind::LeftBrace, "Expected '{' after struct name.")?;

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.isAtEnd() {
            let is_pub = self.matchToken(&TokenKind::Pub);

            if self.matchToken(&TokenKind::Func) {
                // Parse method
                let method_name = match &self.peek().kind {
                    TokenKind::Identifier(n) => n.clone(),
                    _ => return Err(self.error("Expected method name.")),
                };
                self.advance();

                self.consume(&TokenKind::LeftParen, "Expected '(' after method name.")?;
                let mut params = Vec::new();
                if !self.check(&TokenKind::RightParen) {
                    loop {
                        let param_name = match &self.peek().kind {
                            TokenKind::Identifier(n) => n.clone(),
                            _ => return Err(self.error("Expected parameter name.")),
                        };
                        self.advance();

                        let mut variadic = false;
                        let param_ty = if self.matchToken(&TokenKind::Colon) {
                            if self.matchToken(&TokenKind::Ellipsis) {
                                variadic = true;
                                let inner = self
                                    .parseTypeAtom()?
                                    .ok_or_else(|| self.error("Expected type name after '...'."))?;
                                Some(Type {
                                    name: format!("[{}]", inner.name),
                                })
                            } else {
                                self.parseTypeOnly()?
                            }
                        } else {
                            None
                        };

                        let default = if self.matchToken(&TokenKind::Equal) {
                            Some(self.parseExpression()?)
                        } else {
                            None
                        };
                        params.push(Parameter {
                            name: param_name,
                            ty: param_ty,
                            default,
                            variadic,
                        });
                        if !self.matchToken(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.consume(&TokenKind::RightParen, "Expected ')' after parameters.")?;
                let return_ty = self.parseTypeAnnotation()?;
                self.consume(&TokenKind::LeftBrace, "Expected '{' before method body.")?;

                let mut body = Vec::new();
                while !self.check(&TokenKind::RightBrace) && !self.isAtEnd() {
                    if let Some(stmt) = self.parseStatement()? {
                        body.push(stmt);
                    }
                }
                self.consume(&TokenKind::RightBrace, "Expected '}' after method body.")?;

                // Only factory methods like 'create' should be static
                // Instance methods should never be static, even if they return the struct type
                let is_static = is_pub && method_name == "create" && 
                               return_ty.as_ref().map(|t| t.name.as_str()) == Some(&name);

                methods.push(MethodDecl {
                    name: method_name,
                    params,
                    return_ty,
                    body,
                    is_pub,
                    is_static,
                });
            } else {
                // Parse field
                let field_name = match &self.peek().kind {
                    TokenKind::Identifier(n) => n.clone(),
                    _ => return Err(self.error("Expected field name or 'func'.")),
                };
                self.advance();

                self.consume(&TokenKind::Colon, "Expected ':' after field name.")?;
                let ty = self.parseTypeOnly()?.ok_or_else(|| self.error("Expected type name for field."))?;
                self.matchToken(&TokenKind::Comma); // optional trailing comma

                fields.push(FieldDecl {
                    name: field_name,
                    ty,
                    is_pub,
                });
            }
        }

        self.consume(&TokenKind::RightBrace, "Expected '}' after struct body.")?;

        Ok(Stmt::StructDecl {
            name,
            fields,
            methods,
        })
    }

    fn parseImportStatement(&mut self) -> Result<Stmt, ParserError> {
        let import_span = self.previous().span;
        use crate::ast::stmt::ImportName;
        let module = match &self.peek().kind {
            TokenKind::Identifier(name) => name.clone(),
            _ => return Err(self.error("Expected module or file name after 'import'.")),
        };
        self.advance();

        let kind = if self.matchToken(&TokenKind::Dot) {
            let symbols = if self.matchToken(&TokenKind::LeftBrace) {
                let mut values = Vec::new();
                loop {
                    let span = self.peek().span;
                    let name = match &self.peek().kind {
                        TokenKind::Identifier(name) => name.clone(),
                        _ => {
                            return Err(
                                self.error("Expected identifier in grouped file import list.")
                            )
                        }
                    };
                    self.advance();
                    values.push(ImportName { name, span });
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
                let span = self.peek().span;
                let symbol = match &self.peek().kind {
                    TokenKind::Identifier(name) => name.clone(),
                    _ => return Err(self.error("Expected symbol name after file import '.'.")),
                };
                self.advance();
                vec![ImportName { name: symbol, span }]
            };
            ImportKind::FileSymbols { module, symbols }
        } else if self.matchToken(&TokenKind::ColonColon) {
            let items = if self.matchToken(&TokenKind::LeftBrace) {
                let mut values = Vec::new();
                loop {
                    let span = self.peek().span;
                    let name = match &self.peek().kind {
                        TokenKind::Identifier(name) => name.clone(),
                        _ => return Err(self.error("Expected identifier in grouped module import list.")),
                    };
                    self.advance();
                    values.push(ImportName { name, span });
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
                let span = self.peek().span;
                let item = match &self.peek().kind {
                    TokenKind::Identifier(name) => name.clone(),
                    _ => return Err(self.error("Expected file name after '::' in module import.")),
                };
                self.advance();
                vec![ImportName { name: item, span }]
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
        let condition = if self.matchToken(&TokenKind::LeftParen) {
            let condition = self.parseExpression()?;
            self.consume(&TokenKind::RightParen, "Expected ')' after condition.")?;
            condition
        } else {
            self.parseExpression()?
        };

        self.consume(&TokenKind::LeftBrace, "Expected '{' after condition.")?;
        let then_branch = self.parseBlock()?;
        self.consume(&TokenKind::RightBrace, "Expected '}' after then branch.")?;

        let else_branch = if self.matchToken(&TokenKind::Else) {
            if self.matchToken(&TokenKind::If) {
                // else if chaining - consume If and parse nested if
                let nested_if = self.parseIfStatement()?;
                Some(vec![nested_if])
            } else {
                self.consume(&TokenKind::LeftBrace, "Expected '{' after 'else'.")?;
                let block = self.parseBlock()?;
                self.consume(&TokenKind::RightBrace, "Expected '}' after else branch.")?;
                Some(block)
            }
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
        let condition = if self.matchToken(&TokenKind::LeftParen) {
            let condition = self.parseExpression()?;
            self.consume(&TokenKind::RightParen, "Expected ')' after condition.")?;
            condition
        } else {
            self.parseExpression()?
        };

        self.consume(&TokenKind::LeftBrace, "Expected '{' after condition.")?;
        let body = self.parseBlock()?;
        self.consume(&TokenKind::RightBrace, "Expected '}' after while body.")?;

        Ok(Stmt::While { condition, body })
    }

    fn parseLoopStatement(&mut self) -> Result<Stmt, ParserError> {
        self.consume(&TokenKind::LeftBrace, "Expected '{' after 'loop'.")?;
        let body = self.parseBlock()?;
        self.consume(&TokenKind::RightBrace, "Expected '}' after loop body.")?;

        Ok(Stmt::Loop { body })
    }

    fn parseEnumDeclaration(&mut self) -> Result<Stmt, ParserError> {
        let name = match &self.peek().kind {
            TokenKind::Identifier(name) => name.clone(),
            _ => return Err(self.error("Expected enum name.")),
        };
        self.advance();

        self.consume(&TokenKind::LeftBrace, "Expected '{' after enum name.")?;

        let mut variants = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.isAtEnd() {
            let variant = match &self.peek().kind {
                TokenKind::Identifier(name) => name.clone(),
                _ => return Err(self.error("Expected variant name.")),
            };
            self.advance();
            variants.push(variant);

            if !self.matchToken(&TokenKind::Comma) {
                break;
            }
        }

        self.consume(&TokenKind::RightBrace, "Expected '}' after enum variants.")?;

        Ok(Stmt::EnumDecl { name, variants })
    }

    fn parseMatchStatement(&mut self) -> Result<Stmt, ParserError> {
        use crate::ast::stmt::MatchArm;

        let expr = self.parseExpression()?;

        self.consume(&TokenKind::LeftBrace, "Expected '{' after match expression.")?;

        let mut arms = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.isAtEnd() {
            let pattern = self.parsePattern()?;
            self.consume(&TokenKind::Arrow, "Expected '=>' after pattern.")?;

            // Parse body - can be single expression or block
            let body = if self.check(&TokenKind::LeftBrace) {
                self.advance();
                let block = self.parseBlock()?;
                self.consume(&TokenKind::RightBrace, "Expected '}' after match arm body.")?;
                block
            } else {
                let expr = self.parseExpression()?;
                vec![Stmt::Return(expr)]
            };

            arms.push(MatchArm { pattern, body });

            if !self.matchToken(&TokenKind::Comma) {
                break;
            }
        }

        self.consume(&TokenKind::RightBrace, "Expected '}' after match arms.")?;

        Ok(Stmt::Match { expr, arms })
    }

    fn parsePattern(&mut self) -> Result<Pattern, ParserError> {
        use crate::ast::stmt::Pattern;

        match &self.peek().kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();

                // Struct pattern: StructName { field: <pattern>, ... }
                if name
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                    && self.check(&TokenKind::LeftBrace)
                {
                    self.advance(); // consume '{'
                    let mut fields = Vec::new();
                    if !self.check(&TokenKind::RightBrace) {
                        loop {
                            let field_name = match &self.peek().kind {
                                TokenKind::Identifier(f) => f.clone(),
                                _ => return Err(self.error("Expected field name in struct pattern.")),
                            };
                            self.advance();
                            self.consume(&TokenKind::Colon, "Expected ':' after field name in struct pattern.")?;
                            let field_pat = self.parsePattern()?;
                            fields.push((field_name, field_pat));
                            if !self.matchToken(&TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    self.consume(&TokenKind::RightBrace, "Expected '}' after struct pattern fields.")?;
                    return Ok(Pattern::Struct {
                        struct_name: name,
                        fields,
                    });
                }

                // Check for Enum::Variant pattern
                if self.matchToken(&TokenKind::ColonColon) {
                    let variant = match &self.peek().kind {
                        TokenKind::Identifier(v) => v.clone(),
                        _ => return Err(self.error("Expected variant name after '::'.")),
                    };
                    self.advance();
                    Ok(Pattern::EnumVariant(name, variant))
                } else if name == "_" {
                    Ok(Pattern::Wildcard)
                } else {
                    Ok(Pattern::Variable(name))
                }
            }
            TokenKind::NumberLiteral(n) => {
                let val = n.clone();
                self.advance();
                let lit = if val.contains('.') {
                    crate::ast::Literal::Float(val.parse().unwrap())
                } else {
                    crate::ast::Literal::Int(val.parse().unwrap())
                };
                Ok(Pattern::Literal(lit))
            }
            TokenKind::StringLiteral(s) => {
                let val = s.clone();
                self.advance();
                Ok(Pattern::Literal(crate::ast::Literal::String(val)))
            }
            TokenKind::True => {
                self.advance();
                Ok(Pattern::Literal(crate::ast::Literal::Bool(true)))
            }
            TokenKind::False => {
                self.advance();
                Ok(Pattern::Literal(crate::ast::Literal::Bool(false)))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Pattern::Literal(crate::ast::Literal::Null))
            }
            _ => Err(self.error("Expected pattern.")),
        }
    }

    fn parseForStatement(&mut self) -> Result<Stmt, ParserError> {
        use crate::ast::stmt::ForIterator;

        // Parse variable name
        let variable = match self.peek().kind {
            TokenKind::Identifier(ref name) => name.clone(),
            _ => return Err(self.error("Expected variable name after 'for'.")),
        };
        self.advance();

        self.consume(&TokenKind::In, "Expected 'in' after variable name.")?;

        // Check if it's range(start, end) or an array expression
        let iterator = match self.peek().kind {
            TokenKind::Identifier(ref name) if name == "range" => {
                self.advance();
                self.consume(&TokenKind::LeftParen, "Expected '(' after 'range'.")?;

                let start = self.parseExpression()?;
                self.consume(&TokenKind::Comma, "Expected ',' after start value.")?;
                let end = self.parseExpression()?;

                self.consume(&TokenKind::RightParen, "Expected ')' after end value.")?;
                ForIterator::Range { start, end }
            }
            _ => {
                // Parse any expression as array
                let expr = self.parseExpression()?;
                ForIterator::Array(expr)
            }
        };

        self.consume(&TokenKind::LeftBrace, "Expected '{' after for iterator.")?;
        let body = self.parseBlock()?;
        self.consume(&TokenKind::RightBrace, "Expected '}' after for body.")?;

        Ok(Stmt::For {
            variable,
            iterator,
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
            self.parseTypeOnly()
        } else {
            Ok(None)
        }
    }

    fn parseTypeOnly(&mut self) -> Result<Option<Type>, ParserError> {
        let first = self
            .parseTypeAtom()?
            .ok_or_else(|| self.error("Expected type name"))?;
        let mut parts = vec![first.name];
        while self.matchToken(&TokenKind::Pipe) {
            let next = self
                .parseTypeAtom()?
                .ok_or_else(|| self.error("Expected type name after '|'."))?;
            parts.push(next.name);
        }
        Ok(Some(Type {
            name: parts.join(" | "),
        }))
    }

    fn parseTypeAtom(&mut self) -> Result<Option<Type>, ParserError> {
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
                _ => Err(self.error("Expected type name")),
            }
        }
    }

    fn parseUnary(&mut self) -> Result<Expr, ParserError> {
        match &self.peek().kind {
            TokenKind::PlusPlus | TokenKind::MinusMinus => {
                let op = self.peek().kind.clone();
                let span = self.peek().span;
                self.advance();
                let right = self.parseUnary()?;
                match right {
                    Expr::Variable { name, .. } => Ok(Expr::Update {
                        name,
                        op,
                        prefix: true,
                        span,
                    }),
                    _ => Err(self.error("Invalid ++/-- target.")),
                }
            }
            TokenKind::Minus => {
                let span = self.peek().span;
                self.advance();
                let expr = self.parseUnary()?;
                Ok(Expr::Binary {
                    left: Box::new(Expr::Literal {
                        value: Literal::Int(0),
                        span,
                    }),
                    op: TokenKind::Minus,
                    right: Box::new(expr),
                    span: self.previous().span,
                })
            }
            TokenKind::Not => {
                let span = self.peek().span;
                self.advance();
                let expr = self.parseUnary()?;
                Ok(Expr::Unary {
                    op: TokenKind::Not,
                    right: Box::new(expr),
                    span,
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
                    span: self.previous().span,
                };
                continue;
            }

            if self.matchToken(&TokenKind::LeftBracket) {
                let index = self.parseExpression()?;
                self.consume(&TokenKind::RightBracket, "Expected ']' after index.")?;
                expr = Expr::Index {
                    target: Box::new(expr),
                    index: Box::new(index),
                    span: self.previous().span,
                };
                continue;
            }

            if self.matchToken(&TokenKind::Dot) {
                let member_name = match &self.peek().kind {
                    TokenKind::Identifier(name) => name.clone(),
                    TokenKind::NumberLiteral(raw) => {
                        let n: f64 = raw.parse().unwrap_or(0.0);
                        if n.fract() != 0.0 {
                            return Err(self.error("Tuple index after '.' must be an integer."));
                        }
                        (n as i64).to_string()
                    }
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
                    // If the receiver is a Variable with uppercase first char, it's a static call
                    if let Expr::Variable { ref name, .. } = expr {
                        let struct_name = name;
                        if struct_name
                            .chars()
                            .next()
                            .map(|c| c.is_uppercase())
                            .unwrap_or(false)
                        {
                            expr = Expr::StaticCall {
                                struct_name: struct_name.clone(),
                                method: member_name.clone(),
                                args,
                                span: self.previous().span,
                            };
                            continue;
                        }
                    }
                    expr = Expr::MethodCall {
                        receiver: Box::new(expr),
                        name: member_name,
                        args,
                        span: self.previous().span,
                    };
                    continue;
                }
                expr = Expr::Get {
                    object: Box::new(expr),
                    name: member_name,
                    span: self.previous().span,
                };
                continue;
            }

            if self.matchToken(&TokenKind::PlusPlus) {
                match expr {
                    Expr::Variable { name, .. } => {
                        expr = Expr::Update {
                            name,
                            op: TokenKind::PlusPlus,
                            prefix: false,
                            span: self.previous().span,
                        };
                        break;
                    }
                    _ => return Err(self.error("Invalid ++ target.")),
                }
            }

            if self.matchToken(&TokenKind::MinusMinus) {
                match expr {
                    Expr::Variable { name, .. } => {
                        expr = Expr::Update {
                            name,
                            op: TokenKind::MinusMinus,
                            prefix: false,
                            span: self.previous().span,
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
                // Check if this is a struct literal: Identifier { field: value, ... }
                if name
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                    && self.check(&TokenKind::LeftBrace)
                {
                    let start = self.previous().span.start;
                    self.advance(); // consume '{'
                    let mut fields = Vec::new();
                    if !self.check(&TokenKind::RightBrace) {
                        loop {
                            let field_name = match self.peek().kind.clone() {
                                TokenKind::Identifier(n) => {
                                    self.advance();
                                    n
                                }
                                _ => {
                                    return Err(self.error("Expected field name in struct literal."))
                                }
                            };
                            self.consume(
                                &TokenKind::Colon,
                                "Expected ':' after field name in struct literal.",
                            )?;
                            let value = self.parseExpression()?;
                            fields.push((field_name, value));
                            if !self.matchToken(&TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    self.consume(
                        &TokenKind::RightBrace,
                        "Expected '}' after struct literal fields.",
                    )?;
                    Ok(Expr::StructLiteral {
                        name,
                        fields,
                        span: Span::new(start, self.previous().span.end),
                    })
                } else {
                    Ok(Expr::Variable {
                        name,
                        span: self.previous().span,
                    })
                }
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
                Ok(Expr::Literal {
                    value: Literal::String(value),
                    span,
                })
            }
            TokenKind::CharLiteral(value) => {
                let span = self.peek().span;
                self.advance();
                Ok(Expr::Literal {
                    value: Literal::Char(value),
                    span,
                })
            }
            TokenKind::NumberLiteral(value) => {
                let span = self.peek().span;
                self.advance();
                let is_float = value.contains('.');
                if is_float {
                    Ok(Expr::Literal {
                        value: Literal::Float(value.parse().unwrap()),
                        span,
                    })
                } else {
                    Ok(Expr::Literal {
                        value: Literal::Int(value.parse().unwrap()),
                        span,
                    })
                }
            }
            TokenKind::True => {
                let span = self.peek().span;
                self.advance();
                Ok(Expr::Literal {
                    value: Literal::Bool(true),
                    span,
                })
            }
            TokenKind::False => {
                let span = self.peek().span;
                self.advance();
                Ok(Expr::Literal {
                    value: Literal::Bool(false),
                    span,
                })
            }
            TokenKind::Null => {
                let span = self.peek().span;
                self.advance();
                Ok(Expr::Literal {
                    value: Literal::Null,
                    span,
                })
            }
            TokenKind::LeftParen => {
                let start = self.peek().span.start;
                self.advance();

                // try parse lambda: (x: int, y: int) => expr
                let saved = self.current;
                let mut lambdaParams: Vec<Parameter> = Vec::new();
                let mut lambdaOk = true;
                if !self.check(&TokenKind::RightParen) {
                    loop {
                        let param_name = match &self.peek().kind {
                            TokenKind::Identifier(name) => name.clone(),
                            _ => {
                                lambdaOk = false;
                                break;
                            }
                        };
                        self.advance();

                        let param_ty = if self.matchToken(&TokenKind::Colon) {
                            self.parseTypeOnly()?
                        } else {
                            lambdaOk = false;
                            break;
                        };

                        lambdaParams.push(Parameter {
                            name: param_name,
                            ty: param_ty,
                            default: None,
                            variadic: false,
                        });

                        if !self.matchToken(&TokenKind::Comma) {
                            break;
                        }
                    }
                }

                if lambdaOk {
                    if self.consume(&TokenKind::RightParen, "Expected ')' after lambda parameters.").is_ok()
                        && self.matchToken(&TokenKind::Arrow)
                    {
                        let body = self.parseExpression()?;
                        let span = Span::new(start, body.span().end);
                        return Ok(Expr::Lambda {
                            params: lambdaParams,
                            body: Box::new(body),
                            span,
                        });
                    }
                }

                // backtrack and parse as grouping / tuple literal
                self.current = saved;

                if self.check(&TokenKind::RightParen) {
                    return Err(self.error("Expected expression."));
                }

                let first = self.parseExpression()?;
                if self.matchToken(&TokenKind::Comma) {
                    let mut elements = vec![first];
                    if !self.check(&TokenKind::RightParen) {
                        loop {
                            elements.push(self.parseExpression()?);
                            if !self.matchToken(&TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    self.consume(&TokenKind::RightParen, "Expected ')' after tuple literal.")?;
                    let span = self.previous().span;
                    Ok(Expr::TupleLiteral { elements, span })
                } else {
                    self.consume(&TokenKind::RightParen, "Expected ')' after expression.")?;
                    Ok(first)
                }
            }
            TokenKind::LeftBracket => {
                let start = self.peek().span.start;
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
                Ok(Expr::ArrayLiteral {
                    elements,
                    span: Span::new(start, self.previous().span.end),
                })
            }
            TokenKind::LeftBrace => {
                let start = self.peek().span.start;
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
                Ok(Expr::DictLiteral {
                    entries,
                    span: Span::new(start, self.previous().span.end),
                })
            }
            _ => Err(self.error("Expected expression.")),
        }
    }

    fn parseStringInterpolation(value: String, span: Span) -> Result<Expr, ParserError> {
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
                        parts.push(Expr::Literal {
                            value: Literal::String(current_str.clone()),
                            span,
                        });
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
                            .unwrap_or(Expr::Literal {
                                value: Literal::Null,
                                span,
                            });
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
            parts.push(Expr::Literal {
                value: Literal::String(current_str),
                span,
            });
        }

        let mut iter = parts.into_iter();
        let mut res = iter.next().unwrap();
        for part in iter {
            res = Expr::Binary {
                left: Box::new(res),
                op: TokenKind::Plus,
                right: Box::new(part),
                span: Span::new(0, 0),
            };
        }
        Ok(res)
    }

    fn parseAssignment(&mut self) -> Result<Expr, ParserError> {
        let expr = self.parseLogicOr()?;

        if self.matchToken(&TokenKind::Equal) {
            let span = self.previous().span;
            let value = self.parseAssignment()?;

            if let Expr::Variable { name, .. } = expr {
                return Ok(Expr::Assign {
                    name,
                    value: Box::new(value),
                    span,
                });
            }

            if let Expr::Get { object, name: field_name, .. } = expr {
                return Ok(Expr::Set {
                    object,
                    name: field_name,
                    value: Box::new(value),
                    span,
                });
            }

            if let Expr::Index { target, index, .. } = expr {
                return Ok(Expr::IndexSet {
                    target,
                    index,
                    value: Box::new(value),
                    span,
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
            if let Expr::Variable { name, .. } = expr {
                let bin = Expr::Binary {
                    left: Box::new(Expr::Variable {
                        name: name.clone(),
                        span: self.previous().span,
                    }),
                    op,
                    right: Box::new(value),
                    span: self.previous().span,
                };
                return Ok(Expr::Assign {
                    name,
                    value: Box::new(bin),
                    span: self.previous().span,
                });
            }
            if let Expr::Get { object, name: field_name, span: get_span, .. } = expr {
                let bin = Expr::Binary {
                    left: Box::new(Expr::Get { object: object.clone(), name: field_name.clone(), span: get_span }),
                    op,
                    right: Box::new(value),
                    span: self.previous().span,
                };
                return Ok(Expr::Set {
                    object,
                    name: field_name,
                    value: Box::new(bin),
                    span: self.previous().span,
                });
            }
            if let Expr::Index { target, index, span: idx_span, .. } = expr {
                let bin = Expr::Binary {
                    left: Box::new(Expr::Index { target: target.clone(), index: index.clone(), span: idx_span }),
                    op,
                    right: Box::new(value),
                    span: self.previous().span,
                };
                return Ok(Expr::IndexSet {
                    target,
                    index,
                    value: Box::new(bin),
                    span: self.previous().span,
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
                span: self.previous().span,
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
                span: self.previous().span,
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
                span: self.previous().span,
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
                span: self.previous().span,
            };
        }
        while self.matchToken(&TokenKind::InstanceOf) {
            let start = expr.span().start;
            let ty = self
                .parseTypeAtom()?
                .ok_or_else(|| self.error("Expected type name after 'instanceof'."))?;
            expr = Expr::InstanceOf {
                value: Box::new(expr),
                ty,
                span: Span::new(start, self.previous().span.end),
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
                span: self.previous().span,
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
                span: self.previous().span,
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
