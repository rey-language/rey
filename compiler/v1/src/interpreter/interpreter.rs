use crate::ast::{Expr, Stmt};
use crate::typecheck::TypeChecker;
use crate::typecheck::TypeError;
use super::environment::Environment;
use super::executor::Executor;
use super::std::StdLib;

pub enum InterpretError {
    Type(TypeError),
    Runtime(String),
}

impl std::fmt::Display for InterpretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpretError::Type(terr) => write!(f, "{}", terr.message),
            InterpretError::Runtime(msg) => write!(f, "{}", msg),
        }
    }
}

impl From<TypeError> for InterpretError {
    fn from(err: TypeError) -> Self {
        InterpretError::Type(err)
    }
}

impl From<String> for InterpretError {
    fn from(err: String) -> Self {
        InterpretError::Runtime(err)
    }
}

pub struct Interpreter {
    environment: Environment,
    executor: Executor,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut environment = Environment::new();

        let globals = StdLib::create_global_environment();
        for (name, value) in globals {
            environment.define(name, value);
        }

        Self {
            environment,
            executor: Executor::new(),
        }
    }

    pub fn interpret(&mut self, statements: &[Stmt]) -> Result<(), InterpretError> {
        let mut checker = TypeChecker::new();
        checker.checkProgram(statements)?;

        self.executor.execute_block(statements, &mut self.environment)?;

        if self.environment.get("main").is_some() {
            let call = Expr::Call {
                callee: Box::new(Expr::Variable {
                    name: "main".to_string(),
                    span: crate::lexer::span::Span { start: 0, end: 0 },
                }),
                args: vec![],
                span: crate::lexer::span::Span { start: 0, end: 0 },
            };
            self.executor.evaluate_expr(&call, &mut self.environment)?;
        }

        Ok(())
    }
}
