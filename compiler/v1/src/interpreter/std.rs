use super::function::Function;
use super::value::Value;
use crate::lexer::span::Span;
use std::io::{self, Write};

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

        let print_func = Function::new(
            "print".to_string(),
            vec![],
            vec![],
            Span { start: 0, end: 0 },
        );
        globals.insert("print".to_string(), Value::Function(print_func));

        let abs_func = Function::new("abs".to_string(), vec![], vec![], Span { start: 0, end: 0 });
        globals.insert("abs".to_string(), Value::Function(abs_func));

        let max_func = Function::new("max".to_string(), vec![], vec![], Span { start: 0, end: 0 });
        globals.insert("max".to_string(), Value::Function(max_func));

        let min_func = Function::new("min".to_string(), vec![], vec![], Span { start: 0, end: 0 });
        globals.insert("min".to_string(), Value::Function(min_func));

        let random_func = Function::new(
            "random".to_string(),
            vec![],
            vec![],
            Span { start: 0, end: 0 },
        );
        globals.insert("random".to_string(), Value::Function(random_func));

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
            "print" => {
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        print!(" ");
                    }
                    print!("{}", Self::formatValue(arg));
                }
                let _ = io::stdout().flush();
                Some(Ok(Value::Null))
            }
            "abs" => {
                if args.len() != 1 {
                    return Some(Err(format!("abs expects 1 argument, got {}", args.len())));
                }
                match &args[0] {
                    Value::Number(n) => Some(Ok(Value::Number(n.abs()))),
                    _ => Some(Err("abs expects a number".to_string())),
                }
            }
            "max" => {
                if args.len() != 2 {
                    return Some(Err(format!("max expects 2 arguments, got {}", args.len())));
                }
                match (&args[0], &args[1]) {
                    (Value::Number(a), Value::Number(b)) => {
                        Some(Ok(Value::Number(if a > b { *a } else { *b })))
                    }
                    _ => Some(Err("max expects two numbers".to_string())),
                }
            }
            "min" => {
                if args.len() != 2 {
                    return Some(Err(format!("min expects 2 arguments, got {}", args.len())));
                }
                match (&args[0], &args[1]) {
                    (Value::Number(a), Value::Number(b)) => {
                        Some(Ok(Value::Number(if a < b { *a } else { *b })))
                    }
                    _ => Some(Err("min expects two numbers".to_string())),
                }
            }
            "random" => {
                if args.len() != 0 {
                    return Some(Err(format!(
                        "random expects 0 arguments, got {}",
                        args.len()
                    )));
                }
                let time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_micros();
                // Simple pseudo-random using time and a bit of math to avoid contiguous values
                let rand_val =
                    ((time.wrapping_mul(1103515245).wrapping_add(12345)) % 10000) as f64 / 10000.0;
                Some(Ok(Value::Number(rand_val)))
            }
            "len" => {
                if args.len() != 1 {
                    return Some(Err(format!("len expects 1 argument, got {}", args.len())));
                }
                match &args[0] {
                    Value::String(s) => Some(Ok(Value::Number(s.chars().count() as f64))),
                    Value::Array(arr) => Some(Ok(Value::Number(arr.borrow().len() as f64))),
                    Value::Dict(d) => Some(Ok(Value::Number(d.borrow().len() as f64))),
                    _ => Some(Err("len expects a string, array, or dictionary".to_string())),
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
            "input" => {
                if args.len() > 1 {
                    return Some(Err(format!(
                        "input expects 0 or 1 arguments, got {}",
                        args.len()
                    )));
                }
                if args.len() == 1 {
                    match &args[0] {
                        Value::String(s) => {
                            print!("{}", s);
                            let _ = io::stdout().flush();
                        }
                        _ => return Some(Err("input prompt must be a string".to_string())),
                    }
                }

                let mut line = String::new();
                match io::stdin().read_line(&mut line) {
                    Ok(_) => {
                        while line.ends_with('\n') || line.ends_with('\r') {
                            line.pop();
                        }
                        Some(Ok(Value::String(line)))
                    }
                    Err(e) => Some(Err(format!("failed to read input: {}", e))),
                }
            }
            _ => None, // Not a built-in function
        }
    }

    pub fn formatValue(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Char(c) => c.to_string(),
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
            Value::Tuple(items) => {
                let items = items
                    .borrow()
                    .iter()
                    .map(Self::formatValue)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", items)
            }
            Value::Array(arr) => {
                let items = arr
                    .borrow()
                    .iter()
                    .map(Self::formatValue)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", items)
            }
            Value::Dict(d) => {
                let d = d.borrow();
                let mut keys = d.keys().cloned().collect::<Vec<_>>();
                keys.sort();
                let items = keys
                    .into_iter()
                    .map(|k| {
                        let v = d.get(&k).expect("key came from map");
                        format!("{}: {}", k, Self::formatValue(v))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{}}}", items)
            }
            Value::StructInstance {
                struct_name,
                fields,
            } => {
                let fields = fields.borrow();
                let items: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, Self::formatValue(v)))
                    .collect();
                format!("{} {{ {} }}", struct_name, items.join(", "))
            }
        }
    }
}
