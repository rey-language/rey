use crate::ast::Stmt;
use super::environment::Environment;
use super::evaluator::Evaluator;

pub struct Executor {
    evaluator: Evaluator,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            evaluator: Evaluator::new(),
        }
    }

    pub fn execute(&self, stmt: &Stmt, env: &mut Environment) -> Result<(), String> {
        match stmt {
            Stmt::VarDecl { name, initializer, .. } => {
                let value = self.evaluator.evaluate(initializer, env)?;
                env.define(name.clone(), value);
                Ok(())
            }
            Stmt::ExprStmt(expr) => {
                self.evaluator.evaluate(expr, env)?;
                Ok(())
            }
            Stmt::FuncDecl { .. } => {
                // still a todo item
                Ok(())
            }
        }
    }

    pub fn execute_block(&self, statements: &[Stmt], env: &mut Environment) -> Result<(), String> {
        for stmt in statements {
            self.execute(stmt, env)?;
        }
        Ok(())
    }
}