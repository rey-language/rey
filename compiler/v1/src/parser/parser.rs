//DO NOT TOUCH THIS CODE AT ALL
//i've spent 6 hours on this, and any changes breaks this piece of shit
//its fragile as hell
//if u do atempt it, add aounts of hours spent = 0

//all the best :)

#![allow(non_snake_case)]

use crate::ast::{
    EnumPayloadDecl, EnumVariantDecl, Expr, FieldDecl, FunctionVisibility, ImportKind, Literal,
    MethodDecl, Parameter, Pattern, Stmt, Type,
};
use crate::lexer::{span::Span, StringPart, Token, TokenKind};
use crate::parser::error::ParserError;
use std::collections::{HashMap, HashSet};

//impl for recursive descent parser
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    enumVariantOwners: HashMap<String, Option<String>>,
    payloadEnumVariants: HashSet<(String, String)>,
}

fn indexEnumVariants(
    tokens: &[Token],
) -> (HashMap<String, Option<String>>, HashSet<(String, String)>) {
    let mut owners: HashMap<String, Option<String>> = HashMap::new();
    let mut payloadVariants: HashSet<(String, String)> = HashSet::new();
    let mut i = 0;
    while i + 2 < tokens.len() {
        if !matches!(tokens[i].kind, TokenKind::Enum) {
            i += 1;
            continue;
        }
        let enumName = match &tokens[i + 1].kind {
            TokenKind::Identifier(name) if tokens[i + 2].kind == TokenKind::LeftBrace => {
                name.clone()
            }
            _ => {
                i += 1;
                continue;
            }
        };
        let mut variantIndex = i + 3;
        while variantIndex < tokens.len()
            && !matches!(
                tokens[variantIndex].kind,
                TokenKind::RightBrace | TokenKind::Eof
            )
        {
            let variant = match &tokens[variantIndex].kind {
                TokenKind::Identifier(name) => name.clone(),
                _ => break,
            };
            variantIndex += 1;
            let hasPayload = matches!(
                tokens.get(variantIndex).map(|token| &token.kind),
                Some(TokenKind::LeftParen)
            );
            if hasPayload {
                payloadVariants.insert((enumName.clone(), variant.clone()));
                variantIndex = skipParenGroup(tokens, variantIndex);
            }
            match owners.get_mut(&variant) {
                Some(owner) if owner.as_deref() != Some(enumName.as_str()) => *owner = None,
                Some(_) => {}
                None => {
                    owners.insert(variant, Some(enumName.clone()));
                }
            }
            if matches!(
                tokens.get(variantIndex).map(|token| &token.kind),
                Some(TokenKind::Comma)
            ) {
                variantIndex += 1;
            } else if !matches!(
                tokens.get(variantIndex).map(|token| &token.kind),
                Some(TokenKind::RightBrace)
            ) {
                break;
            }
        }
        i = variantIndex.saturating_add(1);
    }
    (owners, payloadVariants)
}

fn skipParenGroup(tokens: &[Token], start: usize) -> usize {
    let mut depth = 0usize;
    for i in start..tokens.len() {
        match tokens[i].kind {
            TokenKind::LeftParen => depth += 1,
            TokenKind::RightParen => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            TokenKind::Eof => break,
            _ => {}
        }
    }
    tokens.len()
}

fn rebaseSpan(local: Span, source: Span) -> Span {
    let line = source.line + local.line.saturating_sub(1);
    let column = if local.line <= 1 {
        source.column + local.column.saturating_sub(1)
    } else {
        local.column
    };
    Span::new_at(
        source.start + local.start,
        source.start + local.end,
        line,
        column,
    )
}

fn rebaseToken(token: &mut Token, source: Span) {
    token.span = rebaseSpan(token.span, source);
    if let TokenKind::InterpolatedStringLiteral(parts) = &mut token.kind {
        for part in parts {
            if let StringPart::Expression { span, .. } = part {
                *span = rebaseSpan(*span, source);
            }
        }
    }
}

impl Parser {
    pub fn new(mut tokens: Vec<Token>) -> Self {
        if tokens.is_empty() {
            tokens.push(Token {
                kind: TokenKind::Eof,
                span: Span::new(0, 0),
            });
        }
        let (enumVariantOwners, payloadEnumVariants) = indexEnumVariants(&tokens);
        Self {
            tokens,
            current: 0,
            enumVariantOwners,
            payloadEnumVariants,
        }
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
            Err(ParserError::Custom {
                message: "'export' was removed in v0.3; use 'pub'".to_string(),
                span: self.previous().span,
            })
        } else if self.matchToken(&TokenKind::Pub) {
            if self.matchToken(&TokenKind::Func) {
                Ok(Some(self.parseFuncDeclaration(FunctionVisibility::Pub)?))
            } else if self.matchToken(&TokenKind::Struct) {
                Ok(Some(self.parseStructDeclaration(true)?))
            } else if self.matchToken(&TokenKind::Enum) {
                Ok(Some(self.parseEnumDeclaration(true)?))
            } else {
                Err(self.error("Expected 'func', 'struct', or 'enum' after 'pub'."))
            }
        } else if self.matchToken(&TokenKind::Func) {
            Ok(Some(
                self.parseFuncDeclaration(FunctionVisibility::Private)?,
            ))
        } else if self.matchToken(&TokenKind::Struct) {
            Ok(Some(self.parseStructDeclaration(false)?))
        } else if self.matchToken(&TokenKind::Enum) {
            Ok(Some(self.parseEnumDeclaration(false)?))
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

    fn parseFuncDeclaration(
        &mut self,
        visibility: FunctionVisibility,
    ) -> Result<Stmt, ParserError> {
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

    fn parseStructDeclaration(&mut self, is_pub: bool) -> Result<Stmt, ParserError> {
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

                // A function written inside a struct is always an instance method.
                let is_static = false;

                methods.push(MethodDecl {
                    name: method_name,
                    params,
                    return_ty,
                    body,
                    is_pub,
                    is_static,
                });
                self.matchToken(&TokenKind::Comma);
            } else {
                // Parse field
                let field_name = match &self.peek().kind {
                    TokenKind::Identifier(n) => n.clone(),
                    _ => return Err(self.error("Expected field name or 'func'.")),
                };
                self.advance();

                self.consume(&TokenKind::Colon, "Expected ':' after field name.")?;
                let ty = self
                    .parseTypeOnly()?
                    .ok_or_else(|| self.error("Expected type name for field."))?;
                if !self.check(&TokenKind::RightBrace) {
                    self.consume(
                        &TokenKind::Comma,
                        "Expected ',' after struct field declaration.",
                    )?;
                }

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
            is_pub,
        })
    }

    fn parseImportStatement(&mut self) -> Result<Stmt, ParserError> {
        let import_span = self.previous().span;
        use crate::ast::stmt::ImportName;
        let kind = match self.peek().kind.clone() {
            TokenKind::StringLiteral(path) => {
                self.advance();
                let module = path.strip_suffix(".rey").unwrap_or(&path).to_string();
                ImportKind::FileSymbols {
                    module,
                    symbols: Vec::new(),
                }
            }
            TokenKind::Identifier(module) => {
                self.advance();
                if self.matchToken(&TokenKind::Dot) {
                    let symbols = if self.check(&TokenKind::LeftBrace) {
                        self.parseImportNames("file")?
                    } else {
                        let span = self.peek().span;
                        let name =
                            self.consumeImportName("Expected symbol name after file import '.'.")?;
                        vec![ImportName { name, span }]
                    };
                    ImportKind::FileSymbols { module, symbols }
                } else if self.matchToken(&TokenKind::ColonColon) {
                    let mut path = module;
                    if self.check(&TokenKind::LeftBrace) {
                        let items = self.parseImportNames("module")?;
                        ImportKind::ModuleItems {
                            module: path,
                            items,
                        }
                    } else {
                        path.push_str("::");
                        path.push_str(&self.consumeImportName("Expected module name after '::'.")?);
                        let mut items = None;
                        while self.matchToken(&TokenKind::ColonColon) {
                            if self.check(&TokenKind::LeftBrace) {
                                items = Some(self.parseImportNames("module")?);
                                break;
                            }
                            path.push_str("::");
                            path.push_str(
                                &self.consumeImportName("Expected module name after '::'.")?,
                            );
                        }
                        if let Some(items) = items {
                            ImportKind::ModuleItems {
                                module: path,
                                items,
                            }
                        } else {
                            ImportKind::ModuleNamespace { module: path }
                        }
                    }
                } else {
                    ImportKind::ModuleNamespace { module }
                }
            }
            _ => return Err(self.error("Expected module or file name after 'import'.")),
        };
        self.consume(
            &TokenKind::Semicolon,
            "Expected ';' after import statement.",
        )?;
        Ok(Stmt::Import {
            kind,
            span: import_span,
        })
    }

    fn parseImportNames(
        &mut self,
        kind: &str,
    ) -> Result<Vec<crate::ast::stmt::ImportName>, ParserError> {
        use crate::ast::stmt::ImportName;
        self.consume(&TokenKind::LeftBrace, "Expected '{' before import list.")?;
        let mut names = Vec::new();
        if !self.check(&TokenKind::RightBrace) {
            loop {
                let span = self.peek().span;
                let name = self.consumeImportName(&format!(
                    "Expected identifier in grouped {} import list.",
                    kind
                ))?;
                names.push(ImportName { name, span });
                if !self.matchToken(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.consume(
            &TokenKind::RightBrace,
            "Expected '}' after grouped import list.",
        )?;
        Ok(names)
    }

    fn consumeImportName(&mut self, message: &str) -> Result<String, ParserError> {
        match self.peek().kind.clone() {
            TokenKind::Identifier(name) => {
                self.advance();
                Ok(name)
            }
            _ => Err(self.error(message)),
        }
    }

    fn parseIfStatement(&mut self) -> Result<Stmt, ParserError> {
        let condition = self.parseExpression()?;

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
        let condition = self.parseExpression()?;

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

    fn parseEnumDeclaration(&mut self, is_pub: bool) -> Result<Stmt, ParserError> {
        let name = match &self.peek().kind {
            TokenKind::Identifier(name) => name.clone(),
            _ => return Err(self.error("Expected enum name.")),
        };
        self.advance();

        self.consume(&TokenKind::LeftBrace, "Expected '{' after enum name.")?;

        let mut unitVariants = Vec::new();
        let mut variants = Vec::new();
        let mut hasPayloads = false;
        while !self.check(&TokenKind::RightBrace) && !self.isAtEnd() {
            let variantSpan = self.peek().span;
            let variant = match &self.peek().kind {
                TokenKind::Identifier(name) => name.clone(),
                _ => return Err(self.error("Expected variant name.")),
            };
            self.advance();
            unitVariants.push(variant.clone());
            let payloads = if self.matchToken(&TokenKind::LeftParen) {
                hasPayloads = true;
                self.parseEnumPayloads()?
            } else {
                Vec::new()
            };
            variants.push(EnumVariantDecl {
                name: variant,
                payloads,
                span: Span::new_at(
                    variantSpan.start,
                    self.previous().span.end,
                    variantSpan.line,
                    variantSpan.column,
                ),
            });
            if !self.check(&TokenKind::RightBrace) {
                self.consume(&TokenKind::Comma, "Expected ',' between enum variants.")?;
            }
        }

        self.consume(&TokenKind::RightBrace, "Expected '}' after enum variants.")?;

        if hasPayloads {
            Ok(Stmt::EnumDeclWithPayload {
                name,
                variants,
                is_pub,
            })
        } else {
            Ok(Stmt::EnumDecl {
                name,
                variants: unitVariants,
                is_pub,
            })
        }
    }

    fn parseEnumPayloads(&mut self) -> Result<Vec<EnumPayloadDecl>, ParserError> {
        let mut payloads = Vec::new();
        let mut namedMode = None;
        if self.check(&TokenKind::RightParen) {
            return Err(self.error("Expected payload type after '(' in enum variant."));
        }
        loop {
            let payloadStart = self.peek().span;
            let payloadName = match (&self.peek().kind, self.tokens.get(self.current + 1)) {
                (TokenKind::Identifier(name), Some(next)) if next.kind == TokenKind::Colon => {
                    let name = name.clone();
                    self.advance();
                    self.advance();
                    Some(name)
                }
                _ => None,
            };
            if namedMode.is_some_and(|named| named != payloadName.is_some()) {
                return Err(
                    self.error("Enum variant payloads must be either all named or all positional.")
                );
            }
            namedMode = Some(payloadName.is_some());
            let ty = self
                .parseTypeOnly()?
                .ok_or_else(|| self.error("Expected payload type in enum variant."))?;
            payloads.push(EnumPayloadDecl {
                name: payloadName,
                ty,
                span: Span::new_at(
                    payloadStart.start,
                    self.previous().span.end,
                    payloadStart.line,
                    payloadStart.column,
                ),
            });
            if self.matchToken(&TokenKind::Comma) {
                if self.check(&TokenKind::RightParen) {
                    break;
                }
            } else {
                break;
            }
        }
        self.consume(
            &TokenKind::RightParen,
            "Expected ')' after enum variant payload types.",
        )?;
        Ok(payloads)
    }

    fn parseMatchStatement(&mut self) -> Result<Stmt, ParserError> {
        use crate::ast::stmt::MatchArm;

        let expr = self.parseExpression()?;

        self.consume(
            &TokenKind::LeftBrace,
            "Expected '{' after match expression.",
        )?;

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
                vec![Stmt::Return(Some(expr))]
            };

            arms.push(MatchArm { pattern, body });
            if !self.check(&TokenKind::RightBrace) {
                self.consume(&TokenKind::Comma, "Expected ',' between match arms.")?;
            }
        }

        self.consume(&TokenKind::RightBrace, "Expected '}' after match arms.")?;

        Ok(Stmt::Match { expr, arms })
    }

    fn parsePattern(&mut self) -> Result<Pattern, ParserError> {
        let start = self.parsePatternAtom()?;
        if self.matchToken(&TokenKind::DotDot) {
            let end = self.parsePatternAtom()?;
            let start = match start {
                Pattern::Literal(literal) => literal,
                _ => return Err(self.error("Range pattern start must be a literal.")),
            };
            let end = match end {
                Pattern::Literal(literal) => literal,
                _ => return Err(self.error("Range pattern end must be a literal.")),
            };
            return Ok(Pattern::Range { start, end });
        }
        Ok(start)
    }

    fn parsePatternAtom(&mut self) -> Result<Pattern, ParserError> {
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
                                _ => {
                                    return Err(self.error("Expected field name in struct pattern."))
                                }
                            };
                            self.advance();
                            self.consume(
                                &TokenKind::Colon,
                                "Expected ':' after field name in struct pattern.",
                            )?;
                            let field_pat = self.parsePattern()?;
                            fields.push((field_name, field_pat));
                            if !self.matchToken(&TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    self.consume(
                        &TokenKind::RightBrace,
                        "Expected '}' after struct pattern fields.",
                    )?;
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
                    if self.check(&TokenKind::LeftParen) {
                        let payloads = self.parseEnumPayloadPatterns()?;
                        Ok(Pattern::EnumVariantPayload {
                            enum_name: Some(name),
                            variant,
                            payloads,
                        })
                    } else {
                        Ok(Pattern::EnumVariant(name, variant))
                    }
                } else if name == "_" {
                    Ok(Pattern::Wildcard)
                } else if self.check(&TokenKind::LeftParen) {
                    let payloads = self.parseEnumPayloadPatterns()?;
                    Ok(Pattern::EnumVariantPayload {
                        enum_name: None,
                        variant: name,
                        payloads,
                    })
                } else {
                    Ok(Pattern::Variable(name))
                }
            }
            TokenKind::NumberLiteral(n) => {
                let span = self.peek().span;
                let val = n.clone();
                self.advance();
                Ok(Pattern::Literal(self.parseNumericLiteral(&val, span)?))
            }
            TokenKind::Minus => {
                let minusSpan = self.peek().span;
                self.advance();
                let (val, numberSpan) = match self.peek().kind.clone() {
                    TokenKind::NumberLiteral(value) => (value, self.peek().span),
                    _ => return Err(self.error("Expected number after '-' in pattern.")),
                };
                self.advance();
                let span = Span::new_at(
                    minusSpan.start,
                    numberSpan.end,
                    minusSpan.line,
                    minusSpan.column,
                );
                Ok(Pattern::Literal(
                    self.parseNumericLiteral(&format!("-{}", val), span)?,
                ))
            }
            TokenKind::StringLiteral(s) => {
                let val = s.clone();
                self.advance();
                Ok(Pattern::Literal(crate::ast::Literal::String(val)))
            }
            TokenKind::ByteStringLiteral(bytes) => {
                let bytes = bytes.clone();
                self.advance();
                Ok(Pattern::Literal(Literal::ByteString(bytes)))
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
            TokenKind::CharLiteral(value) => {
                let value = *value;
                self.advance();
                Ok(Pattern::Literal(Literal::Char(value)))
            }
            _ => Err(self.error("Expected pattern.")),
        }
    }

    fn parseEnumPayloadPatterns(&mut self) -> Result<Vec<Pattern>, ParserError> {
        self.consume(
            &TokenKind::LeftParen,
            "Expected '(' before enum payload patterns.",
        )?;
        let mut payloads = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            loop {
                payloads.push(self.parsePattern()?);
                if self.matchToken(&TokenKind::Comma) {
                    if self.check(&TokenKind::RightParen) {
                        return Err(self.error("Expected pattern after ',' in enum payload list."));
                    }
                } else {
                    break;
                }
            }
        }
        self.consume(
            &TokenKind::RightParen,
            "Expected ')' after enum payload patterns.",
        )?;
        Ok(payloads)
    }

    fn parseNumericLiteral(&self, raw: &str, span: Span) -> Result<Literal, ParserError> {
        let value = raw.replace('_', "");
        if let Some(hex) = value.strip_prefix("0x").or_else(|| value.strip_prefix("0X")) {
            return i64::from_str_radix(hex, 16).map(Literal::Int).map_err(|_| {
                ParserError::Custom {
                    message: "Integer literal out of range.".to_string(),
                    span,
                }
            });
        }
        if let Some(bits) = value.strip_prefix("0b").or_else(|| value.strip_prefix("0B")) {
            return i64::from_str_radix(bits, 2).map(Literal::Int).map_err(|_| {
                ParserError::Custom {
                    message: "Integer literal out of range.".to_string(),
                    span,
                }
            });
        }
        if value.contains('.') {
            value
                .parse::<f64>()
                .map(Literal::Float)
                .map_err(|_| ParserError::Custom {
                    message: "Invalid numeric literal.".to_string(),
                    span,
                })
        } else {
            value
                .parse::<i64>()
                .map(Literal::Int)
                .map_err(|_| ParserError::Custom {
                    message: "Integer literal out of range.".to_string(),
                    span,
                })
        }
    }

    fn parseTupleIndex(&self, raw: &str, span: Span) -> Result<String, ParserError> {
        let value = raw.replace('_', "");
        let mut parts = value.split('.');
        let integer = parts.next().unwrap_or_default();
        if parts
            .next()
            .is_some_and(|fraction| !fraction.chars().all(|digit| digit == '0'))
        {
            return Err(ParserError::Custom {
                message: "Tuple index after '.' must be an integer.".to_string(),
                span,
            });
        }
        integer
            .parse::<i64>()
            .map(|index| index.to_string())
            .map_err(|_| ParserError::Custom {
                message: "Tuple index is out of range.".to_string(),
                span,
            })
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
        if self.check(&TokenKind::Semicolon) {
            self.advance();
            return Ok(Stmt::Return(None));
        }
        let expr = self.parseExpression()?;
        self.consume(&TokenKind::Semicolon, "Expected ';' after return value.")?;
        Ok(Stmt::Return(Some(expr)))
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

    fn parseGenericTypeArgs(&mut self, name: &str) -> Result<String, ParserError> {
        if !self.matchToken(&TokenKind::Less) {
            return Ok(name.to_string());
        }
        let mut args = vec![
            self.parseRequiredType("Expected type argument after '<'.")?
                .name,
        ];
        while self.matchToken(&TokenKind::Comma) {
            args.push(
                self.parseRequiredType("Expected type argument after ','.")?
                    .name,
            );
        }
        self.consume(
            &TokenKind::Greater,
            "Expected '>' after generic type arguments.",
        )?;
        Ok(format!("{}<{}>", name, args.join(",")))
    }

    fn parseRequiredType(&mut self, message: &str) -> Result<Type, ParserError> {
        if !matches!(
            self.peek().kind,
            TokenKind::Identifier(_)
                | TokenKind::LeftParen
                | TokenKind::LeftBracket
                | TokenKind::LeftBrace
        ) {
            return Err(self.error(message));
        }
        self.parseTypeOnly()?
            .ok_or_else(|| self.error(message))
    }

    fn parseTypeAtom(&mut self) -> Result<Option<Type>, ParserError> {
        if self.matchToken(&TokenKind::LeftParen) {
            Ok(Some(self.parseTupleOrFunctionType()?))
        } else if self.matchToken(&TokenKind::LeftBracket) {
            let inner = self.parseRequiredType("Expected type name inside '[]'.")?;
            self.consume(&TokenKind::RightBracket, "Expected ']' after array type.")?;
            let mut name = format!("[{}]", inner.name);
            if self.matchToken(&TokenKind::Question) {
                name.push('?');
            }
            Ok(Some(Type { name }))
        } else if self.matchToken(&TokenKind::LeftBrace) {
            let key = self.parseRequiredType("Expected key type name inside '{}'.")?;
            self.consume(
                &TokenKind::Colon,
                "Expected ':' between dict key/value types.",
            )?;
            let value = self.parseRequiredType("Expected value type name inside '{}'.")?;
            self.consume(&TokenKind::RightBrace, "Expected '}' after dict type.")?;
            let mut name = format!("{{{}:{}}}", key.name, value.name);
            if self.matchToken(&TokenKind::Question) {
                name.push('?');
            }
            Ok(Some(Type { name }))
        } else {
            match &self.peek().kind {
                TokenKind::Identifier(name) => {
                    let mut n = name.clone();
                    self.advance();
                    n = self.parseGenericTypeArgs(&n)?;
                    if self.matchToken(&TokenKind::Question) {
                        n.push('?');
                    }
                    Ok(Some(Type { name: n }))
                }
                _ => Err(self.error("Expected type name")),
            }
        }
    }

    fn parseTupleOrFunctionType(&mut self) -> Result<Type, ParserError> {
        if self.check(&TokenKind::RightParen) {
            self.advance();
            self.consume(
                &TokenKind::ThinArrow,
                "Expected '->' after empty function parameter list.",
            )?;
            let return_ty = self
                .parseTypeOnly()?
                .ok_or_else(|| self.error("Expected function return type."))?;
            return Ok(Type {
                name: format!("() -> {}", return_ty.name),
            });
        }

        let mut elements = vec![
            self.parseTypeOnly()?
                .ok_or_else(|| self.error("Expected type inside parentheses."))?
                .name,
        ];
        let mut sawComma = false;
        let mut trailingComma = false;
        while self.matchToken(&TokenKind::Comma) {
            sawComma = true;
            if self.check(&TokenKind::RightParen) {
                trailingComma = true;
                break;
            }
            elements.push(
                self.parseTypeOnly()?
                    .ok_or_else(|| self.error("Expected type after ',' in type list."))?
                    .name,
            );
        }
        self.consume(
            &TokenKind::RightParen,
            "Expected ')' after parenthesized type list.",
        )?;

        if self.matchToken(&TokenKind::ThinArrow) {
            if trailingComma {
                return Err(
                    self.error("Function type parameter lists cannot have a trailing comma.")
                );
            }
            let return_ty = self
                .parseTypeOnly()?
                .ok_or_else(|| self.error("Expected function return type."))?;
            return Ok(Type {
                name: format!("({}) -> {}", elements.join(", "), return_ty.name),
            });
        }

        if sawComma {
            if trailingComma && elements.len() != 1 {
                return Err(self.error("Only a one-element tuple type may trail with a comma."));
            }
            let name = if elements.len() == 1 {
                format!("({},)", elements[0])
            } else {
                format!("({})", elements.join(", "))
            };
            return Ok(Type { name });
        }

        Err(self.error("Expected ',' for tuple type or '->' for function type."))
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
                    target @ (Expr::Get { .. } | Expr::Index { .. }) => Ok(Expr::UpdateLValue {
                        target: Box::new(target),
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
                if let TokenKind::NumberLiteral(raw) = self.peek().kind.clone() {
                    let magnitude = raw.replace('_', "");
                    if magnitude.trim_start_matches('0') == "9223372036854775808" {
                        let numberSpan = self.peek().span;
                        self.advance();
                        return Ok(Expr::Literal {
                            value: Literal::Int(i64::MIN),
                            span: Span::new_at(span.start, numberSpan.end, span.line, span.column),
                        });
                    }
                }
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
                if let Expr::Variable { name, .. } = &expr {
                    if self
                        .payloadEnumVariants
                        .iter()
                        .any(|(_, variant)| variant == name)
                    {
                        expr = Expr::EnumVariant {
                            enum_name: self.enumVariantOwners.get(name).cloned().flatten(),
                            variant: name.clone(),
                            args,
                            span: self.previous().span,
                        };
                        continue;
                    }
                }
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

            let isQualified = if self.matchToken(&TokenKind::Dot) {
                Some(false)
            } else if self.matchToken(&TokenKind::ColonColon) {
                Some(true)
            } else {
                None
            };
            if let Some(isQualified) = isQualified {
                let member_name = match &self.peek().kind {
                    TokenKind::Identifier(name) => name.clone(),
                    TokenKind::NumberLiteral(raw) => {
                        self.parseTupleIndex(&raw, self.peek().span)?
                    }
                    _ => return Err(self.error("Expected identifier after '.' or '::'.")),
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
                    if isQualified {
                        if let Expr::Variable { name, .. } = &expr {
                            expr = Expr::EnumVariant {
                                enum_name: Some(name.clone()),
                                variant: member_name,
                                args,
                                span: self.previous().span,
                            };
                            continue;
                        }
                    } else if matches!(&expr, Expr::Variable { .. })
                        && self
                            .payloadEnumVariants
                            .iter()
                            .any(|(_, variant)| variant == &member_name)
                    {
                        expr = Expr::EnumVariant {
                            enum_name: self.enumVariantOwners.get(&member_name).cloned().flatten(),
                            variant: member_name,
                            args,
                            span: self.previous().span,
                        };
                        continue;
                    }
                    // If the receiver is a Variable with uppercase first char, it's a static call
                    if !isQualified {
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
                    }
                    expr = Expr::MethodCall {
                        receiver: Box::new(expr),
                        name: member_name,
                        args,
                        span: self.previous().span,
                    };
                    continue;
                }
                if isQualified {
                    if let Expr::Variable { name, .. } = &expr {
                        if name == "Option"
                            || name == "Result"
                            || self
                                .payloadEnumVariants
                                .contains(&(name.clone(), member_name.clone()))
                        {
                            expr = Expr::EnumVariant {
                                enum_name: Some(name.clone()),
                                variant: member_name,
                                args: Vec::new(),
                                span: self.previous().span,
                            };
                            continue;
                        }
                    }
                }
                expr = Expr::Get {
                    object: Box::new(expr),
                    name: member_name,
                    span: self.previous().span,
                };
                continue;
            }

            if self.matchToken(&TokenKind::PlusPlus) || self.matchToken(&TokenKind::MinusMinus) {
                let op = self.previous().kind.clone();
                let span = self.previous().span;
                expr = match expr {
                    Expr::Variable { name, .. } => Expr::Update {
                        name,
                        op,
                        prefix: false,
                        span,
                    },
                    target @ (Expr::Get { .. } | Expr::Index { .. }) => Expr::UpdateLValue {
                        target: Box::new(target),
                        op,
                        prefix: false,
                        span,
                    },
                    _ => return Err(self.error("Invalid ++/-- target.")),
                };
                break;
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
                            if self.check(&TokenKind::RightBrace) {
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
                Ok(Expr::Literal {
                    value: Literal::String(value),
                    span,
                })
            }
            TokenKind::ByteStringLiteral(bytes) => {
                let span = self.peek().span;
                self.advance();
                Ok(Expr::Literal {
                    value: Literal::ByteString(bytes),
                    span,
                })
            }
            TokenKind::InterpolatedStringLiteral(parts) => {
                let span = self.peek().span;
                self.advance();
                self.parseInterpolatedString(parts, span)
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
                Ok(Expr::Literal {
                    value: self.parseNumericLiteral(&value, span)?,
                    span,
                })
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
                    if self
                        .consume(
                            &TokenKind::RightParen,
                            "Expected ')' after lambda parameters.",
                        )
                        .is_ok()
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
                let mut hasByteKey = false;
                if !self.check(&TokenKind::RightBrace) {
                    loop {
                        let (key, keyBytes) = match self.peek().kind.clone() {
                            TokenKind::Identifier(name) => {
                                self.advance();
                                let bytes = name.as_bytes().to_vec();
                                (Some(name), bytes)
                            }
                            TokenKind::StringLiteral(s) => {
                                self.advance();
                                let bytes = s.as_bytes().to_vec();
                                (Some(s), bytes)
                            }
                            TokenKind::ByteStringLiteral(bytes) => {
                                self.advance();
                                hasByteKey = true;
                                (None, bytes)
                            }
                            _ => {
                                return Err(
                                    self.error("Expected identifier or string as dictionary key.")
                                )
                            }
                        };
                        self.consume(&TokenKind::Colon, "Expected ':' after dictionary key.")?;
                        let value = self.parseExpression()?;
                        entries.push((key, keyBytes, value));
                        if !self.matchToken(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.consume(
                    &TokenKind::RightBrace,
                    "Expected '}' after dictionary literal.",
                )?;
                let span = Span::new(start, self.previous().span.end);
                if hasByteKey {
                    Ok(Expr::DictByteLiteral {
                        entries: entries
                            .into_iter()
                            .map(|(_, keyBytes, value)| (keyBytes, value))
                            .collect(),
                        span,
                    })
                } else {
                    let mut textEntries = Vec::new();
                    for (key, _, value) in entries {
                        let key =
                            key.ok_or_else(|| self.error("Expected UTF-8 dictionary key."))?;
                        textEntries.push((key, value));
                    }
                    Ok(Expr::DictLiteral {
                        entries: textEntries,
                        span,
                    })
                }
            }
            _ => Err(self.error("Expected expression.")),
        }
    }

    fn parseInterpolatedString(
        &mut self,
        parts: Vec<StringPart>,
        span: Span,
    ) -> Result<Expr, ParserError> {
        let mut expressions = Vec::with_capacity(parts.len());
        for part in parts {
            expressions.push(match part {
                StringPart::Text(bytes) => {
                    let value = match String::from_utf8(bytes) {
                        Ok(value) => Literal::String(value),
                        Err(error) => Literal::ByteString(error.into_bytes()),
                    };
                    Expr::Literal { value, span }
                }
                StringPart::Expression {
                    source,
                    span: expressionSpan,
                } => {
                    let expression = Self::parseInterpolationExpression(&source, expressionSpan)?;
                    let callSpan = expression.span();
                    Expr::MethodCall {
                        receiver: Box::new(expression),
                        name: "toString".to_string(),
                        args: Vec::new(),
                        span: callSpan,
                    }
                }
            });
        }

        let mut expressions = expressions.into_iter();
        let mut result = expressions.next().unwrap_or(Expr::Literal {
            value: Literal::String(String::new()),
            span,
        });
        for expression in expressions {
            result = Expr::Binary {
                left: Box::new(result),
                op: TokenKind::Plus,
                right: Box::new(expression),
                span,
            };
        }
        Ok(result)
    }

    fn parseInterpolationExpression(source: &str, span: Span) -> Result<Expr, ParserError> {
        let mut lexer = crate::lexer::Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            let mut token = lexer.nextToken().map_err(|error| ParserError::Custom {
                message: error.message(),
                span: rebaseSpan(*error.span(), span),
            })?;
            let atEnd = token.kind == TokenKind::Eof;
            rebaseToken(&mut token, span);
            tokens.push(token);
            if atEnd {
                break;
            }
        }
        let mut parser = Parser::new(tokens);
        let expression = parser.parseExpression()?;
        if !parser.isAtEnd() {
            return Err(parser.error("Expected end of interpolation expression."));
        }
        Ok(expression)
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

            if let Expr::Get {
                object,
                name: field_name,
                ..
            } = expr
            {
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
            if let Expr::Get {
                object,
                name: field_name,
                span: get_span,
                ..
            } = expr
            {
                let bin = Expr::Binary {
                    left: Box::new(Expr::Get {
                        object: object.clone(),
                        name: field_name.clone(),
                        span: get_span,
                    }),
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
            if let Expr::Index {
                target,
                index,
                span: idx_span,
                ..
            } = expr
            {
                let bin = Expr::Binary {
                    left: Box::new(Expr::Index {
                        target: target.clone(),
                        index: index.clone(),
                        span: idx_span,
                    }),
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
            if std::mem::discriminant(kind) == std::mem::discriminant(&TokenKind::Semicolon) {
                Err(self.error("expected ';'"))
            } else {
                Err(self.error(message))
            }
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

#[cfg(test)]
mod tests {
    use super::Parser;
    use crate::ast::{Expr, ImportKind, Literal, Pattern, Stmt};
    use crate::lexer::{Lexer, TokenKind};

    fn parseSource(source: &str) -> Result<Vec<Stmt>, crate::parser::error::ParserError> {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            let token = lexer.nextToken().unwrap();
            let atEnd = token.kind == TokenKind::Eof;
            tokens.push(token);
            if atEnd {
                break;
            }
        }
        Parser::new(tokens).parse()
    }

    fn countStringConversions(expr: &Expr) -> usize {
        match expr {
            Expr::MethodCall {
                receiver,
                name,
                args,
                ..
            } => {
                (if name == "toString" { 1 } else { 0 })
                    + countStringConversions(receiver)
                    + args.iter().map(countStringConversions).sum::<usize>()
            }
            Expr::Binary { left, right, .. } => {
                countStringConversions(left) + countStringConversions(right)
            }
            Expr::Call { callee, args, .. } => {
                countStringConversions(callee)
                    + args.iter().map(countStringConversions).sum::<usize>()
            }
            _ => 0,
        }
    }

    #[test]
    fn increment_and_decrement_parse_in_all_lvalue_positions() {
        let statements = parseSource(
            "var x = 0; var old = x++; var new = ++x; var a = [0]; a[0]--; ++a[0]; var s = Thing{}; s.field++; --s.field;",
        )
        .unwrap();
        assert!(matches!(
            statements[1],
            Stmt::VarDecl {
                initializer: Expr::Update { prefix: false, .. },
                ..
            }
        ));
        assert!(matches!(
            statements[2],
            Stmt::VarDecl {
                initializer: Expr::Update { prefix: true, .. },
                ..
            }
        ));
        assert!(
            matches!(&statements[4], Stmt::ExprStmt(Expr::UpdateLValue { prefix: false, target, .. }) if matches!(target.as_ref(), Expr::Index { .. }))
        );
        assert!(
            matches!(&statements[5], Stmt::ExprStmt(Expr::UpdateLValue { prefix: true, target, .. }) if matches!(target.as_ref(), Expr::Index { .. }))
        );
        assert!(
            matches!(&statements[7], Stmt::ExprStmt(Expr::UpdateLValue { prefix: false, target, .. }) if matches!(target.as_ref(), Expr::Get { .. }))
        );
        assert!(
            matches!(&statements[8], Stmt::ExprStmt(Expr::UpdateLValue { prefix: true, target, .. }) if matches!(target.as_ref(), Expr::Get { .. }))
        );
    }

    #[test]
    fn match_accepts_ranges_and_qualified_or_bare_enum_patterns() {
        let statements = parseSource(
            "enum Color { Red, Blue } var c = Color::Red; match c { 1_0..2_0 => {}, Red => {}, Color::Blue => {}, _ => {}, }",
        )
        .unwrap();
        assert!(matches!(
            statements[1],
            Stmt::VarDecl {
                initializer: Expr::Get { .. },
                ..
            }
        ));
        let Stmt::Match { arms, .. } = &statements[2] else {
            panic!("expected match")
        };
        assert!(matches!(
            arms[0].pattern,
            Pattern::Range {
                start: Literal::Int(10),
                end: Literal::Int(20)
            }
        ));
        assert!(matches!(&arms[1].pattern, Pattern::Variable(name) if name == "Red"));
        assert!(
            matches!(&arms[2].pattern, Pattern::EnumVariant(enumName, variant) if enumName == "Color" && variant == "Blue")
        );
    }

    #[test]
    fn signed64_boundaries_parse_exactly_and_overflow_is_diagnostic() {
        let statements = parseSource(
            "var max = 9_223_372_036_854_775_807; var min = -9_223_372_036_854_775_808; match 0 { -9_223_372_036_854_775_808..9_223_372_036_854_775_807 => {}, _ => {}, }",
        )
        .unwrap();
        assert!(matches!(
            &statements[0],
            Stmt::VarDecl {
                initializer: Expr::Literal {
                    value: Literal::Int(i64::MAX),
                    ..
                },
                ..
            }
        ));
        assert!(matches!(
            &statements[1],
            Stmt::VarDecl {
                initializer: Expr::Literal {
                    value: Literal::Int(i64::MIN),
                    ..
                },
                ..
            }
        ));
        let Stmt::Match { arms, .. } = &statements[2] else {
            panic!("expected match")
        };
        assert!(matches!(
            &arms[0].pattern,
            Pattern::Range {
                start: Literal::Int(i64::MIN),
                end: Literal::Int(i64::MAX),
            }
        ));

        for source in [
            "var high = 9223372036854775808;",
            "var low = -9223372036854775809;",
            "match 0 { 0..9223372036854775808 => {}, _ => {}, }",
        ] {
            let error = parseSource(source).unwrap_err();
            assert_eq!(error.message(), "Integer literal out of range.");
        }
        assert!(parseSource("var index = tuple.9223372036854775808;")
            .unwrap_err()
            .message()
            .contains("Tuple index is out of range."));
    }

    #[test]
    fn tuple_and_function_types_are_disambiguated_by_arrow() {
        let source = r#"
func makePair(): (int, string) { return (12, "dozen"); }
func getOne(): (int,) { return (12,); }
func accepts(f: (int, string) -> bool): ((int, string), bool) -> void { return f; }
func zeroArgs(): () -> int { return () => 1; }
"#;
        let statements = parseSource(source).unwrap();
        let Stmt::FuncDecl {
            return_ty: Some(pair),
            ..
        } = &statements[0]
        else {
            panic!("expected tuple-return function")
        };
        assert_eq!(pair.name, "(int, string)");
        let Stmt::FuncDecl {
            return_ty: Some(one),
            ..
        } = &statements[1]
        else {
            panic!("expected one-element tuple-return function")
        };
        assert_eq!(one.name, "(int,)");
        let Stmt::FuncDecl {
            params,
            return_ty: Some(nested),
            ..
        } = &statements[2]
        else {
            panic!("expected nested tuple/function types")
        };
        assert_eq!(params[0].ty.as_ref().unwrap().name, "(int, string) -> bool");
        assert_eq!(nested.name, "((int, string), bool) -> void");
        let Stmt::FuncDecl {
            return_ty: Some(zero),
            ..
        } = &statements[3]
        else {
            panic!("expected zero-parameter function type")
        };
        assert_eq!(zero.name, "() -> int");

        let tupleReturnFixture = r#"
func makePair(): (int, string) {
    return (12, "dozen");
}
var pair = makePair();
println(pair.0);
println(pair.1);
"#;
        assert_eq!(parseSource(tupleReturnFixture).unwrap().len(), 4);
    }

    #[test]
    fn conditional_headers_parse_grouping_and_full_expressions() {
        let statements = parseSource(
            "if (a) && b {} if (a || b) && c {} if (a && b) {} if (((a))) {} if (a + 1) < (b * 2) {} while (a || b) && c {} while false {} if (a) {} else if (b) || c {} else {} if (Foo { x: 1 }).x == 1 {}",
        )
        .unwrap();
        assert_eq!(statements.len(), 9);
        assert!(matches!(
            &statements[0],
            Stmt::If {
                condition: Expr::Binary { op: TokenKind::AndAnd, .. },
                ..
            }
        ));
        assert!(matches!(
            &statements[1],
            Stmt::If {
                condition: Expr::Binary {
                    op: TokenKind::AndAnd,
                    left,
                    ..
                },
                ..
            } if matches!(left.as_ref(), Expr::Binary { op: TokenKind::OrOr, .. })
        ));
        assert!(matches!(
            &statements[2],
            Stmt::If {
                condition: Expr::Binary { op: TokenKind::AndAnd, .. },
                ..
            }
        ));
        assert!(matches!(
            &statements[3],
            Stmt::If {
                condition: Expr::Variable { name, .. },
                ..
            } if name == "a"
        ));
        assert!(matches!(
            &statements[4],
            Stmt::If {
                condition: Expr::Binary {
                    op: TokenKind::Less,
                    left,
                    right,
                    ..
                },
                ..
            } if matches!(left.as_ref(), Expr::Binary { op: TokenKind::Plus, .. })
                && matches!(right.as_ref(), Expr::Binary { op: TokenKind::Star, .. })
        ));
        assert!(matches!(
            &statements[5],
            Stmt::While {
                condition: Expr::Binary { op: TokenKind::AndAnd, .. },
                ..
            }
        ));
        assert!(matches!(
            &statements[6],
            Stmt::While {
                condition: Expr::Literal { value: Literal::Bool(false), .. },
                ..
            }
        ));
        assert!(matches!(
            &statements[7],
            Stmt::If {
                else_branch: Some(branch),
                ..
            } if matches!(branch.as_slice(), [Stmt::If { else_branch: Some(_), .. }])
        ));
        assert!(matches!(
            &statements[8],
            Stmt::If {
                condition: Expr::Binary { op: TokenKind::EqualEqual, .. },
                ..
            }
        ));
    }

    #[test]
    fn malformed_conditional_headers_report_errors_at_the_blocker() {
        for (source, expected, marker) in [
            ("if () {}", "Expected expression.", ")"),
            ("while () {}", "Expected expression.", ")"),
            (
                "if (a && b {}",
                "Expected ')' after expression.",
                "{",
            ),
            (
                "while (a || b {}",
                "Expected ')' after expression.",
                "{",
            ),
            (
                "if (a) && b",
                "Expected '{' after condition.",
                "",
            ),
            (
                "while false",
                "Expected '{' after condition.",
                "",
            ),
            (
                "if (a",
                "Expected ')' after expression.",
                "",
            ),
        ] {
            let error = parseSource(source).unwrap_err();
            assert_eq!(error.message(), expected);
            let expectedStart = if marker.is_empty() {
                source.len()
            } else {
                source.find(marker).unwrap()
            };
            assert_eq!(error.span().start, expectedStart);
            assert_eq!(error.span().line, 1);
            assert!(error.span().column > 0);
        }
    }

    #[test]
    fn container_types_parse_recursive_members_and_keep_canonical_names() {
        let statements = parseSource(
            "struct TypeFields { nullable: [TypeExpr?], nested: [[int?]], unioned: [int | string], mapping: {string:[int?]}, recursiveKey: {[int?]:[string | int]}, tupleArray: [(int, string)], functionArray: [(int, string) -> bool], appliedArray: [Vec<int>], outerNullable: [int]?, single: [int], }",
        )
        .unwrap();
        let Stmt::StructDecl { fields, .. } = &statements[0] else {
            panic!("expected struct declaration")
        };
        assert_eq!(
            fields
                .iter()
                .map(|field| field.ty.name.as_str())
                .collect::<Vec<_>>(),
            [
                "[TypeExpr?]",
                "[[int?]]",
                "[int | string]",
                "{string:[int?]}",
                "{[int?]:[string | int]}",
                "[(int, string)]",
                "[(int, string) -> bool]",
                "[Vec<int>]",
                "[int]?",
                "[int]",
            ]
        );
    }

    #[test]
    fn container_types_report_missing_members_and_delimiters() {
        for (source, expected) in [
            (
                "struct Broken { values: [] }",
                "Expected type name inside '[]'.",
            ),
            (
                "struct Broken { values: [int",
                "Expected ']' after array type.",
            ),
            (
                "struct Broken { values: [int | ] }",
                "Expected type name",
            ),
            (
                "struct Broken { values: {string:} }",
                "Expected value type name inside '{}'.",
            ),
            (
                "struct Broken { values: {string:int",
                "Expected '}' after dict type.",
            ),
            (
                "struct Broken { values: [(int, string] }",
                "Expected ')' after parenthesized type list.",
            ),
        ] {
            let error = parseSource(source).unwrap_err();
            assert!(
                error.message().contains(expected),
                "expected {:?} in error {:?}",
                expected,
                error.message()
            );
        }
    }

    #[test]
    fn generic_type_arguments_keep_recursive_canonical_names() {
        let statements = parseSource(
            "func types(vector: Vec<string>, map: HashMap<string,int>, option: Option<Result<int,string>>, array: [Vec<int>], nullable: Vec<int>?, nested: Vec<[int?]>, tuple: Vec<(int, string)>, function: Vec<(int, string) -> bool>, union: Vec<int | string>) {} func arities(vector: Vec<int, string>, map: HashMap<string>, option: Option<int, string>) {}",
        )
        .unwrap();
        let Stmt::FuncDecl { params, .. } = &statements[0] else {
            panic!("expected typed function")
        };
        assert_eq!(
            params
                .iter()
                .map(|parameter| parameter.ty.as_ref().unwrap().name.as_str())
                .collect::<Vec<_>>(),
            [
                "Vec<string>",
                "HashMap<string,int>",
                "Option<Result<int,string>>",
                "[Vec<int>]",
                "Vec<int>?",
                "Vec<[int?]>",
                "Vec<(int, string)>",
                "Vec<(int, string) -> bool>",
                "Vec<int | string>",
            ]
        );
        let Stmt::FuncDecl { params, .. } = &statements[1] else {
            panic!("expected syntactically applied types")
        };
        assert_eq!(
            params
                .iter()
                .map(|parameter| parameter.ty.as_ref().unwrap().name.as_str())
                .collect::<Vec<_>>(),
            ["Vec<int,string>", "HashMap<string>", "Option<int,string>"]
        );
    }

    #[test]
    fn generic_type_arguments_reject_empty_malformed_and_truncated_lists() {
        for (source, expected) in [
            (
                "func broken(value: Vec<>) {}",
                "Expected type argument after '<'.",
            ),
            (
                "func broken(value: Vec<int,>) {}",
                "Expected type argument after ','.",
            ),
            (
                "func broken(value: Vec<int,,string>) {}",
                "Expected type argument after ','.",
            ),
            (
                "func broken(value: Vec<int) {}",
                "Expected '>' after generic type arguments.",
            ),
            (
                "func broken(value: Option<Result<int,string>) {}",
                "Expected '>' after generic type arguments.",
            ),
        ] {
            let error = parseSource(source).unwrap_err();
            assert!(
                error.message().contains(expected),
                "expected {:?} in error {:?}",
                expected,
                error.message()
            );
        }
    }

    #[test]
    fn struct_literals_accept_trailing_commas_in_nested_contexts() {
        let statements = parseSource(
            "func build() { var empty = Foo{}; var plain = Foo { x: 1 }; var trailing = Foo{x:1,}; var values = [Foo{}, Foo{x:1,}]; if true { var outer = Outer{inner:Inner{x:1,},}; } }",
        )
        .unwrap();
        let Stmt::FuncDecl { body, .. } = &statements[0] else {
            panic!("expected function declaration")
        };
        assert!(matches!(
            &body[0],
            Stmt::VarDecl {
                initializer: Expr::StructLiteral { name, fields, .. },
                ..
            } if name == "Foo" && fields.is_empty()
        ));
        assert!(matches!(
            &body[1],
            Stmt::VarDecl {
                initializer: Expr::StructLiteral { name, fields, .. },
                ..
            } if name == "Foo" && fields.len() == 1
        ));
        assert!(matches!(
            &body[2],
            Stmt::VarDecl {
                initializer: Expr::StructLiteral { name, fields, .. },
                ..
            } if name == "Foo" && fields.len() == 1
        ));
        assert!(matches!(
            &body[3],
            Stmt::VarDecl {
                initializer: Expr::ArrayLiteral { elements, .. },
                ..
            } if elements.len() == 2
                && matches!(&elements[0], Expr::StructLiteral { fields, .. } if fields.is_empty())
                && matches!(&elements[1], Expr::StructLiteral { fields, .. } if fields.len() == 1)
        ));
        let Stmt::If { then_branch, .. } = &body[4] else {
            panic!("expected if statement")
        };
        assert!(matches!(
            &then_branch[0],
            Stmt::VarDecl {
                initializer: Expr::StructLiteral { name, fields, .. },
                ..
            } if name == "Outer"
                && matches!(&fields[0].1, Expr::StructLiteral { name, fields, .. }
                    if name == "Inner" && fields.len() == 1)
        ));
    }

    #[test]
    fn malformed_struct_literal_commas_and_truncation_are_rejected() {
        for (source, expected) in [
            (
                "var broken = Foo { x: 1,, };",
                "Expected field name in struct literal.",
            ),
            (
                "var broken = Foo { : 1 };",
                "Expected field name in struct literal.",
            ),
            ("var broken = Foo { x: };", "Expected expression."),
            (
                "var broken = Foo { x: 1,",
                "Expected field name in struct literal.",
            ),
            (
                "var broken = Foo { x: 1",
                "Expected '}' after struct literal fields.",
            ),
        ] {
            let error = parseSource(source).unwrap_err();
            assert!(
                error.message().contains(expected),
                "expected {:?} in error {:?}",
                expected,
                error.message()
            );
        }
    }

    #[test]
    fn type_parentheses_follow_tuple_and_function_trailing_comma_rules() {
        assert!(parseSource("func emptyTuple(): () { return; }")
            .unwrap_err()
            .message()
            .contains("Expected '->' after empty function parameter list."));
        assert!(parseSource("func grouped(): (int) { return; }")
            .unwrap_err()
            .message()
            .contains("Expected ',' for tuple type or '->' for function type."));
        assert!(parseSource("func trailing(): (int, string,) { return; }")
            .unwrap_err()
            .message()
            .contains("Only a one-element tuple type may trail with a comma."));
        assert!(
            parseSource("func functionTrailing(): (int,) -> string { return; }")
                .unwrap_err()
                .message()
                .contains("Function type parameter lists cannot have a trailing comma.")
        );
    }

    #[test]
    fn enum_payload_declarations_preserve_unit_enum_shape_and_payload_types() {
        let statements = parseSource(
            "enum Unit { Red, Blue } enum Message { Count(value: int), Label(value: string), Pair(int, string), }",
        )
        .unwrap();
        assert!(matches!(
            &statements[0],
            Stmt::EnumDecl { name, variants, is_pub }
                if name == "Unit"
                    && variants.len() == 2
                    && variants[0] == "Red"
                    && variants[1] == "Blue"
                    && !is_pub
        ));
        let Stmt::EnumDeclWithPayload {
            name,
            variants,
            is_pub,
        } = &statements[1]
        else {
            panic!("expected structured payload enum")
        };
        assert_eq!(name, "Message");
        assert!(!is_pub);
        assert_eq!(variants.len(), 3);
        assert_eq!(variants[0].name, "Count");
        assert_eq!(variants[0].payloads.len(), 1);
        assert_eq!(variants[0].payloads[0].name.as_deref(), Some("value"));
        assert_eq!(variants[0].payloads[0].ty.name, "int");
        assert_eq!(variants[1].payloads[0].name.as_deref(), Some("value"));
        assert_eq!(variants[1].payloads[0].ty.name, "string");
        assert_eq!(variants[2].payloads.len(), 2);
        assert_eq!(variants[2].payloads[0].name, None);
        assert_eq!(variants[2].payloads[1].ty.name, "string");
    }

    #[test]
    fn pub_struct_and_enums_retain_visibility_with_private_defaults() {
        let statements = parseSource(
            "struct Local {} pub struct Public {} enum LocalKind { Only } pub enum PublicKind { Unit, Count(value: int) } pub func api() {}",
        )
        .unwrap();
        assert!(matches!(
            &statements[0],
            Stmt::StructDecl { name, is_pub: false, .. } if name == "Local"
        ));
        assert!(matches!(
            &statements[1],
            Stmt::StructDecl { name, is_pub: true, .. } if name == "Public"
        ));
        assert!(matches!(
            &statements[2],
            Stmt::EnumDecl { name, is_pub: false, .. } if name == "LocalKind"
        ));
        assert!(matches!(
            &statements[3],
            Stmt::EnumDeclWithPayload { name, is_pub: true, .. } if name == "PublicKind"
        ));
        assert!(matches!(
            &statements[4],
            Stmt::FuncDecl { name, visibility: crate::ast::FunctionVisibility::Pub, .. }
                if name == "api"
        ));
    }

    #[test]
    fn pub_does_not_accept_global_bindings() {
        for source in ["pub var value = 1;", "pub const value = 1;"] {
            let error = parseSource(source).unwrap_err();
            assert_eq!(
                error.message(),
                "Expected 'func', 'struct', or 'enum' after 'pub'."
            );
        }
    }

    #[test]
    fn payload_constructors_and_binding_patterns_keep_positional_arity() {
        let statements = parseSource(
            "enum Message { Count(value: int), Pair(int, int) } var bare = Count(8); var qualified = Message::Count(9); var pair = Message::Pair(1, 2); match pair { Message::Count(value) => {}, Count(value) => {}, Message::Pair(left, right) => {}, Pair(left, right) => {}, Message::Count => {}, }",
        )
        .unwrap();
        assert!(matches!(
            &statements[1],
            Stmt::VarDecl {
                initializer: Expr::EnumVariant {
                    enum_name: Some(enumName),
                    variant,
                    args,
                    ..
                },
                ..
            } if enumName == "Message" && variant == "Count" && args.len() == 1
        ));
        assert!(matches!(
            &statements[2],
            Stmt::VarDecl {
                initializer: Expr::EnumVariant {
                    enum_name: Some(enumName),
                    variant,
                    args,
                    ..
                },
                ..
            } if enumName == "Message" && variant == "Count" && args.len() == 1
        ));
        assert!(matches!(
            &statements[3],
            Stmt::VarDecl {
                initializer: Expr::EnumVariant { variant, args, .. },
                ..
            } if variant == "Pair" && args.len() == 2
        ));
        let Stmt::Match { arms, .. } = &statements[4] else {
            panic!("expected match")
        };
        assert!(matches!(
            &arms[0].pattern,
            Pattern::EnumVariantPayload {
                enum_name: Some(enumName),
                variant,
                payloads,
            } if enumName == "Message" && variant == "Count" && payloads.len() == 1
        ));
        assert!(matches!(
            &arms[1].pattern,
            Pattern::EnumVariantPayload {
                enum_name: None,
                variant,
                payloads,
            } if variant == "Count" && payloads.len() == 1
        ));
        assert!(matches!(
            &arms[2].pattern,
            Pattern::EnumVariantPayload { payloads, .. } if payloads.len() == 2
        ));
        assert!(matches!(
            &arms[3].pattern,
            Pattern::EnumVariantPayload { enum_name: None, payloads, .. } if payloads.len() == 2
        ));
        assert!(matches!(
            &arms[4].pattern,
            Pattern::EnumVariant(enumName, variant) if enumName == "Message" && variant == "Count"
        ));
    }

    #[test]
    fn malformed_enum_payload_syntax_is_a_parser_error() {
        assert!(parseSource("enum E { Count() }")
            .unwrap_err()
            .message()
            .contains("Expected payload type"));
        assert!(parseSource("enum E { Count(value:) }")
            .unwrap_err()
            .message()
            .contains("Expected type name"));
        assert!(
            parseSource("enum E { Count(int) } match e { E::Count(value => {}, }")
                .unwrap_err()
                .message()
                .contains("Expected ')' after enum payload patterns.")
        );
        assert!(parseSource("enum E { Count(int) } var x = E::Count(1;")
            .unwrap_err()
            .message()
            .contains("Expected ')' after method arguments."));
    }

    #[test]
    fn import_spellings_share_the_import_statement_node() {
        let statements = parseSource(
            "import a.b; import a.{b, c}; import std::math; import std::math::{sin, cos}; import \"path/file.rey\";",
        )
        .unwrap();
        assert_eq!(statements.len(), 5);
        assert!(statements
            .iter()
            .all(|stmt| matches!(stmt, Stmt::Import { .. })));
        assert!(
            matches!(&statements[0], Stmt::Import { kind: ImportKind::FileSymbols { module, symbols }, .. } if module == "a" && symbols.len() == 1)
        );
        assert!(
            matches!(&statements[1], Stmt::Import { kind: ImportKind::FileSymbols { module, symbols }, .. } if module == "a" && symbols.len() == 2)
        );
        assert!(
            matches!(&statements[2], Stmt::Import { kind: ImportKind::ModuleNamespace { module }, .. } if module == "std::math")
        );
        assert!(
            matches!(&statements[3], Stmt::Import { kind: ImportKind::ModuleItems { module, items }, .. } if module == "std::math" && items.len() == 2)
        );
        assert!(
            matches!(&statements[4], Stmt::Import { kind: ImportKind::FileSymbols { module, symbols }, .. } if module == "path/file" && symbols.is_empty())
        );
    }

    #[test]
    fn removed_export_has_actionable_parse_error_and_types_keep_case() {
        let error = parseSource("export pub func f() {};").unwrap_err();
        assert_eq!(error.message(), "'export' was removed in v0.3; use 'pub'");

        let statements = parseSource("func f(value: String): (Int) -> Void {}").unwrap();
        let Stmt::FuncDecl {
            params, return_ty, ..
        } = &statements[0]
        else {
            panic!("expected function")
        };
        assert_eq!(params[0].ty.as_ref().unwrap().name, "String");
        assert_eq!(return_ty.as_ref().unwrap().name, "(Int) -> Void");
    }

    #[test]
    fn escaped_string_and_char_literals_reach_ast() {
        let statements = parseSource(r#"var text = "\x41"; var letter = '\x42';"#).unwrap();
        assert!(matches!(
            &statements[0],
            Stmt::VarDecl {
                initializer: Expr::Literal {
                    value: Literal::String(value),
                    ..
                },
                ..
            } if value == "A"
        ));
        assert!(matches!(
            &statements[1],
            Stmt::VarDecl {
                initializer: Expr::Literal {
                    value: Literal::Char('B'),
                    ..
                },
                ..
            }
        ));

        let utf8Statements = parseSource("var text = \"café\";").unwrap();
        let Stmt::VarDecl {
            initializer:
                Expr::Literal {
                    value: Literal::String(value),
                    ..
                },
            ..
        } = &utf8Statements[0]
        else {
            panic!("expected string literal")
        };
        assert_eq!(value.as_bytes(), "café".as_bytes());
    }

    #[test]
    fn byte_strings_nul_patterns_and_interpolation_chunks_reach_ast() {
        let statements = parseSource(
            r#"var bytes = "\x80"; var nul = "A\0B"; var joined = "A\x80{'\xE9'}\x81"; match bytes { "\x80" => {}, _ => {}, }"#,
        )
        .unwrap();
        assert!(matches!(
            &statements[0],
            Stmt::VarDecl {
                initializer: Expr::Literal {
                    value: Literal::ByteString(bytes),
                    ..
                },
                ..
            } if bytes == &[0x80]
        ));
        assert!(matches!(
            &statements[1],
            Stmt::VarDecl {
                initializer: Expr::Literal {
                    value: Literal::String(value),
                    ..
                },
                ..
            } if value.as_bytes() == &[b'A', 0, b'B']
        ));
        let Stmt::VarDecl { initializer, .. } = &statements[2] else {
            panic!("expected interpolated variable")
        };
        let Expr::Binary { left, right, .. } = initializer else {
            panic!("expected concatenated interpolation")
        };
        assert!(matches!(
            right.as_ref(),
            Expr::Literal { value: Literal::ByteString(bytes), .. } if bytes == &[0x81]
        ));
        let Expr::Binary { left, right, .. } = left.as_ref() else {
            panic!("expected byte text followed by expression")
        };
        assert!(matches!(
            left.as_ref(),
            Expr::Literal { value: Literal::ByteString(bytes), .. } if bytes == &[b'A', 0x80]
        ));
        assert!(matches!(
            right.as_ref(),
            Expr::MethodCall { name, args, receiver, .. }
                if name == "toString"
                    && args.is_empty()
                    && matches!(receiver.as_ref(), Expr::Literal { value: Literal::Char('é'), .. })
        ));
        let Stmt::Match { arms, .. } = &statements[3] else {
            panic!("expected match")
        };
        assert!(matches!(
            &arms[0].pattern,
            Pattern::Literal(Literal::ByteString(bytes)) if bytes == &[0x80]
        ));

        let dictionary = parseSource(r#"var table = {"\x80": 1, "ok": 2};"#).unwrap();
        assert!(matches!(
            &dictionary[0],
            Stmt::VarDecl {
                initializer: Expr::DictByteLiteral { entries, .. },
                ..
            } if entries.len() == 2
                && entries[0].0 == [0x80]
                && entries[1].0 == b"ok"
        ));
    }

    #[test]
    fn interpolation_stringifies_adjacent_values_once_and_rebases_source_spans() {
        let values = parseSource("var text = \"{1}{true}{'A'}\";").unwrap();
        let Stmt::VarDecl { initializer, .. } = &values[0] else {
            panic!("expected interpolation variable")
        };
        assert!(matches!(initializer, Expr::Binary { .. }));
        assert_eq!(countStringConversions(initializer), 3);

        let source = "func next(): int { return 1; }\nvar text = \"v={next()}\";";
        let statements = parseSource(source).unwrap();
        let Stmt::VarDecl { initializer, .. } = &statements[1] else {
            panic!("expected interpolation variable")
        };
        let Expr::Binary { left, .. } = initializer else {
            panic!("expected string concatenation")
        };
        let Expr::Binary { right, .. } = left.as_ref() else {
            panic!("expected interpolation expression")
        };
        let Expr::MethodCall {
            name,
            receiver,
            args,
            ..
        } = right.as_ref()
        else {
            panic!("expected explicit string conversion")
        };
        assert_eq!(name, "toString");
        assert!(args.is_empty());
        let Expr::Call { callee, .. } = receiver.as_ref() else {
            panic!("expected one evaluation of next()")
        };
        let Expr::Variable { name, span } = callee.as_ref() else {
            panic!("expected next function name")
        };
        assert_eq!(name, "next");
        assert_eq!(span.start, source.rfind("next()").unwrap());
        assert_eq!(span.line, 2);
        assert_eq!(
            span.column,
            source.lines().nth(1).unwrap().find("next()").unwrap() + 1
        );
    }

    #[test]
    fn charPatternsOptionNoneAndMethodCommasParse() {
        let matched = parseSource(
            "func main(): void { var letter = 'a'; match letter { 'a' => { println(letter); }, _ => { println(letter); }, } }",
        )
        .expect("char pattern");
        let Stmt::FuncDecl { body, .. } = &matched[0] else {
            panic!("expected function");
        };
        let Stmt::Match { arms, .. } = &body[1] else {
            panic!("expected match");
        };
        assert!(matches!(
            arms[0].pattern,
            Pattern::Literal(Literal::Char('a'))
        ));

        let structured = parseSource(
            "struct Counter { pub value: int, pub func get(): int { return value; }, }",
        )
        .expect("trailing method comma");
        let Stmt::StructDecl { methods, .. } = &structured[0] else {
            panic!("expected struct");
        };
        assert_eq!(methods[0].name, "get");

        parseSource(
            "enum Message { Maybe(Option<int>), } func main(): void { var x = Message::Maybe(Option::None); }",
        )
        .expect("Option::None");

        let created = parseSource(
            "struct Box { pub n: int, pub func create(n: int): Box { return Box { n: n }; }, }",
        )
        .expect("instance create");
        let Stmt::StructDecl { methods, .. } = &created[0] else {
            panic!("expected struct");
        };
        assert_eq!(methods[0].name, "create");
        assert!(!methods[0].is_static);
    }

    #[test]
    fn hexAndBinaryIntegersParseAsInts() {
        let stmts = parseSource("var hex = 0x10; var bits = 0b1010; var wide = 0x7FFF_FFFF_FFFF_FFFF;")
            .expect("hex and binary");
        let ints: Vec<i64> = stmts
            .iter()
            .map(|stmt| match stmt {
                Stmt::VarDecl { initializer, .. } => match initializer {
                    Expr::Literal { value: Literal::Int(n), .. } => *n,
                    other => panic!("expected int, got {other:?}"),
                },
                other => panic!("expected var, got {other:?}"),
            })
            .collect();
        assert_eq!(ints, vec![16, 10, i64::MAX]);
    }
}
