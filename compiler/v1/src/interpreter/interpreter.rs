use crate::ast::Stmt;
use super::environment::Environment;
use super::executor::Executor;
use super::std::StdLib;

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

    pub fn interpret(&mut self, statements: &[Stmt]) -> Result<(), String> {
        self.executor.execute_block(statements, &mut self.environment)?;
        Ok(())
    }
}