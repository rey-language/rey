use super::Literal;
use super::Parameter;
use crate::lexer::span::Span;
use crate::lexer::TokenKind;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal {
        value: Literal,
        span: Span,
    },
    TupleLiteral {
        elements: Vec<Expr>,
        span: Span,
    },
    Binary {
        left: Box<Expr>,
        op: TokenKind,
        right: Box<Expr>,
        span: Span,
    },
    Variable {
        name: String,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    ArrayLiteral {
        elements: Vec<Expr>,
        span: Span,
    },
    DictLiteral {
        entries: Vec<(String, Expr)>,
        span: Span,
    },
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    Get {
        object: Box<Expr>,
        name: String,
        span: Span,
    },
    MethodCall {
        receiver: Box<Expr>,
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    StructLiteral {
        name: String,
        fields: Vec<(String, Expr)>,
        span: Span,
    },
    StaticCall {
        struct_name: String,
        method: String,
        args: Vec<Expr>,
        span: Span,
    },
    Unary {
        op: TokenKind,
        right: Box<Expr>,
        span: Span,
    },

    InstanceOf {
        value: Box<Expr>,
        ty: super::Type,
        span: Span,
    },

    Lambda {
        params: Vec<Parameter>,
        body: Box<Expr>,
        span: Span,
    },

    Assign {
        name: String,
        value: Box<Expr>,
        span: Span,
    },

    Set {
        object: Box<Expr>,
        name: String,
        value: Box<Expr>,
        span: Span,
    },

    IndexSet {
        target: Box<Expr>,
        index: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },

    Update {
        name: String,
        op: TokenKind,
        prefix: bool,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal { span, .. } => *span,
            Expr::TupleLiteral { span, .. } => *span,
            Expr::Binary { span, .. } => *span,
            Expr::Variable { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::ArrayLiteral { span, .. } => *span,
            Expr::DictLiteral { span, .. } => *span,
            Expr::Index { span, .. } => *span,
            Expr::Get { span, .. } => *span,
            Expr::MethodCall { span, .. } => *span,
            Expr::StructLiteral { span, .. } => *span,
            Expr::StaticCall { span, .. } => *span,
            Expr::Unary { span, .. } => *span,
            Expr::InstanceOf { span, .. } => *span,
            Expr::Lambda { span, .. } => *span,
            Expr::Assign { span, .. } => *span,
            Expr::Set { span, .. } => *span,
            Expr::IndexSet { span, .. } => *span,
            Expr::Update { span, .. } => *span,
        }
    }
}
