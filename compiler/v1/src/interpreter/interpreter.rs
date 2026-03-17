use crate::ast::{Expr, Stmt};
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

        if self.environment.get("main").is_some() {
            let call = Expr::Call {
                callee: Box::new(Expr::Variable("main".to_string())),
                args: vec![],
            };
            self.executor.evaluate_expr(&call, &mut self.environment)?;
        }

        Ok(())
    }
}
