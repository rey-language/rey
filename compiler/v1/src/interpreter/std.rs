use super::value::Value;
use super::function::Function;
use crate::lexer::span::Span;

pub struct StdLib;

impl StdLib {
    pub fn create_global_environment() -> std::collections::HashMap<String, Value> {
        let mut globals = std::collections::HashMap::new();

        // Add println function
        let println_func = Function::new(
            "println".to_string(),
            vec![], // No parameters - accepts any number of arguments
            vec![], // Empty body - handled specially
            Span { start: 0, end: 0 },
        );
        globals.insert("println".to_string(), Value::Function(println_func));

        globals
    }

    pub fn call_builtin_function(name: &str, args: &[Value]) -> Option<Result<Value, String>> {
        match name {
            "println" => {
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        print!(" ");
                    }
                    match arg {
                        Value::String(s) => print!("{}", s),
                        Value::Number(n) => print!("{}", n),
                        Value::Bool(b) => print!("{}", b),
                        Value::Null => print!("null"),
                        Value::Function(_) => print!("<function>"),
                    }
                }
                println!();
                Some(Ok(Value::Null))
            }
            _ => None, // Not a built-in function
        }
    }
}