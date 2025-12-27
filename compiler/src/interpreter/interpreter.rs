use crate::ast::Stmt;
use super::environment::Environment;
use super::executor::Executor;

pub struct Interpreter {
    environment: Environment,
    executor: Executor,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            environment: Environment::new(),
            executor: Executor::new(),
        }
    }

    pub fn interpret(&mut self, statements: &[Stmt]) -> Result<(), String> {
        self.executor.execute_block(statements, &mut self.environment)
    }
}