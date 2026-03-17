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
                    print!("{}", Self::formatValue(arg));
                }
                println!();
                Some(Ok(Value::Null))
            }
            "len" => {
                if args.len() != 1 {
                    return Some(Err(format!("len expects 1 argument, got {}", args.len())));
                }
                match &args[0] {
                    Value::String(s) => Some(Ok(Value::Number(s.chars().count() as f64))),
                    Value::Array(arr) => Some(Ok(Value::Number(arr.borrow().len() as f64))),
                    _ => Some(Err("len expects a string or array".to_string())),
                }
            }
            "push" => {
                if args.len() != 2 {
                    return Some(Err(format!("push expects 2 arguments, got {}", args.len())));
                }
                match &args[0] {
                    Value::Array(arr) => {
                        arr.borrow_mut().push(args[1].clone());
                        Some(Ok(Value::Null))
                    }
                    _ => Some(Err("push expects an array as first argument".to_string())),
                }
            }
            "pop" => {
                if args.len() != 1 {
                    return Some(Err(format!("pop expects 1 argument, got {}", args.len())));
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let v = arr.borrow_mut().pop().unwrap_or(Value::Null);
                        Some(Ok(v))
                    }
                    _ => Some(Err("pop expects an array".to_string())),
                }
            }
            _ => None, // Not a built-in function
        }
    }

    fn formatValue(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::Bool(b) => format!("{}", b),
            Value::Null => "null".to_string(),
            Value::Function(_) => "<function>".to_string(),
            Value::Array(arr) => {
                let items = arr
                    .borrow()
                    .iter()
                    .map(Self::formatValue)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", items)
            }
        }
    }
}
