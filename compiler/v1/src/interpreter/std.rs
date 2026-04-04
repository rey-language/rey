use super::function::Function;
use super::value::Value;
use crate::lexer::span::Span;
use std::cell::RefCell;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;
use std::rc::Rc;

pub struct StdLib;

impl StdLib {
    pub fn create_global_environment() -> std::collections::HashMap<String, Value> {
        let mut globals = std::collections::HashMap::new();

        // Add println function
        let println_func = Function::new(
            "println".to_string(),
            vec![], // No parameters - accepts any number of arguments
            vec![], // Empty body - handled specially
            Span::new(0, 0),
            None,
        );
        globals.insert("println".to_string(), Value::Function(println_func));

        let print_func = Function::new(
            "print".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("print".to_string(), Value::Function(print_func));

        let abs_func = Function::new(
            "abs".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("abs".to_string(), Value::Function(abs_func));

        let max_func = Function::new(
            "max".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("max".to_string(), Value::Function(max_func));

        let min_func = Function::new(
            "min".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("min".to_string(), Value::Function(min_func));

        let random_func = Function::new(
            "random".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("random".to_string(), Value::Function(random_func));

        let vec_new = Function::new("Vec.new".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("Vec.new".to_string(), Value::Function(vec_new));

        let linkedlist_new = Function::new(
            "LinkedList.new".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert(
            "LinkedList.new".to_string(),
            Value::Function(linkedlist_new),
        );

        let hashmap_new = Function::new(
            "HashMap.new".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("HashMap.new".to_string(), Value::Function(hashmap_new));

        let stack_new = Function::new(
            "Stack.new".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("Stack.new".to_string(), Value::Function(stack_new));

        let queue_new = Function::new(
            "Queue.new".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("Queue.new".to_string(), Value::Function(queue_new));

        let option_some = Function::new(
            "Option::Some".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("Option::Some".to_string(), Value::Function(option_some));

        let option_none = Function::new(
            "Option::None".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("Option::None".to_string(), Value::Function(option_none));

        let result_ok = Function::new(
            "Result::Ok".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("Result::Ok".to_string(), Value::Function(result_ok));

        let result_err = Function::new(
            "Result::Err".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("Result::Err".to_string(), Value::Function(result_err));

        // FS builtins
        let readFile = Function::new(
            "readFile".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("readFile".to_string(), Value::Function(readFile));

        let writeFile = Function::new(
            "writeFile".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("writeFile".to_string(), Value::Function(writeFile));

        let appendFile = Function::new(
            "appendFile".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("appendFile".to_string(), Value::Function(appendFile));

        let fileExists = Function::new(
            "fileExists".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("fileExists".to_string(), Value::Function(fileExists));

        let deleteFile = Function::new(
            "deleteFile".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("deleteFile".to_string(), Value::Function(deleteFile));

        let mkdir = Function::new("mkdir".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("mkdir".to_string(), Value::Function(mkdir));

        let listDir = Function::new("listDir".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("listDir".to_string(), Value::Function(listDir));

        let getEnv = Function::new("getEnv".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("getEnv".to_string(), Value::Function(getEnv));

        let args = Function::new("args".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("args".to_string(), Value::Function(args));

        let exit = Function::new("exit".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("exit".to_string(), Value::Function(exit));

        let exec = Function::new("exec".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("exec".to_string(), Value::Function(exec));

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
                    Value::Int(n) => Some(Ok(Value::Int(n.abs()))),
                    Value::Float(n) => Some(Ok(Value::Float(n.abs()))),
                    _ => Some(Err("abs expects a number".to_string())),
                }
            }
            "max" => {
                if args.len() != 2 {
                    return Some(Err(format!("max expects 2 arguments, got {}", args.len())));
                }
                match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => {
                        Some(Ok(Value::Int(if a > b { *a } else { *b })))
                    }
                    (Value::Float(a), Value::Float(b)) => {
                        Some(Ok(Value::Float(if a > b { *a } else { *b })))
                    }
                    (Value::Int(a), Value::Float(b)) => {
                        Some(Ok(Value::Float(if (*a as f64) > *b {
                            *a as f64
                        } else {
                            *b
                        })))
                    }
                    (Value::Float(a), Value::Int(b)) => {
                        Some(Ok(Value::Float(if *a > (*b as f64) {
                            *a
                        } else {
                            *b as f64
                        })))
                    }
                    _ => Some(Err("max expects two numbers".to_string())),
                }
            }
            "min" => {
                if args.len() != 2 {
                    return Some(Err(format!("min expects 2 arguments, got {}", args.len())));
                }
                match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => {
                        Some(Ok(Value::Int(if a < b { *a } else { *b })))
                    }
                    (Value::Float(a), Value::Float(b)) => {
                        Some(Ok(Value::Float(if a < b { *a } else { *b })))
                    }
                    (Value::Int(a), Value::Float(b)) => {
                        Some(Ok(Value::Float(if (*a as f64) < *b {
                            *a as f64
                        } else {
                            *b
                        })))
                    }
                    (Value::Float(a), Value::Int(b)) => {
                        Some(Ok(Value::Float(if *a < (*b as f64) {
                            *a
                        } else {
                            *b as f64
                        })))
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
                Some(Ok(Value::Float(rand_val)))
            }
            "floor" => {
                if args.len() != 1 {
                    return Some(Err(format!("floor expects 1 argument, got {}", args.len())));
                }
                let n = match &args[0] {
                    Value::Int(v) => *v as f64,
                    Value::Float(v) => *v,
                    _ => return Some(Err("floor expects a number".to_string())),
                };
                Some(Ok(Value::Int(n.floor() as i64)))
            }
            "ceil" => {
                if args.len() != 1 {
                    return Some(Err(format!("ceil expects 1 argument, got {}", args.len())));
                }
                let n = match &args[0] {
                    Value::Int(v) => *v as f64,
                    Value::Float(v) => *v,
                    _ => return Some(Err("ceil expects a number".to_string())),
                };
                Some(Ok(Value::Int(n.ceil() as i64)))
            }
            "round" => {
                if args.len() != 1 {
                    return Some(Err(format!("round expects 1 argument, got {}", args.len())));
                }
                let n = match &args[0] {
                    Value::Int(v) => *v as f64,
                    Value::Float(v) => *v,
                    _ => return Some(Err("round expects a number".to_string())),
                };
                Some(Ok(Value::Int(n.round() as i64)))
            }
            "sqrt" => {
                if args.len() != 1 {
                    return Some(Err(format!("sqrt expects 1 argument, got {}", args.len())));
                }
                let n = match &args[0] {
                    Value::Int(v) => *v as f64,
                    Value::Float(v) => *v,
                    _ => return Some(Err("sqrt expects a number".to_string())),
                };
                if n < 0.0 {
                    return Some(Err("sqrt expects non-negative input".to_string()));
                }
                Some(Ok(Value::Float(n.sqrt())))
            }
            "pow" => {
                if args.len() != 2 {
                    return Some(Err(format!("pow expects 2 arguments, got {}", args.len())));
                }
                let base = match &args[0] {
                    Value::Int(v) => *v as f64,
                    Value::Float(v) => *v,
                    _ => return Some(Err("pow expects numeric base".to_string())),
                };
                let exp = match &args[1] {
                    Value::Int(v) => *v as f64,
                    Value::Float(v) => *v,
                    _ => return Some(Err("pow expects numeric exponent".to_string())),
                };
                Some(Ok(Value::Float(base.powf(exp))))
            }
            "log" => {
                if args.len() != 1 {
                    return Some(Err(format!("log expects 1 argument, got {}", args.len())));
                }
                let n = match &args[0] {
                    Value::Int(v) => *v as f64,
                    Value::Float(v) => *v,
                    _ => return Some(Err("log expects a number".to_string())),
                };
                if n <= 0.0 {
                    return Some(Err("log expects positive input".to_string()));
                }
                Some(Ok(Value::Float(n.ln())))
            }
            "sin" => {
                if args.len() != 1 {
                    return Some(Err(format!("sin expects 1 argument, got {}", args.len())));
                }
                let n = match &args[0] {
                    Value::Int(v) => *v as f64,
                    Value::Float(v) => *v,
                    _ => return Some(Err("sin expects a number".to_string())),
                };
                Some(Ok(Value::Float(n.sin())))
            }
            "cos" => {
                if args.len() != 1 {
                    return Some(Err(format!("cos expects 1 argument, got {}", args.len())));
                }
                let n = match &args[0] {
                    Value::Int(v) => *v as f64,
                    Value::Float(v) => *v,
                    _ => return Some(Err("cos expects a number".to_string())),
                };
                Some(Ok(Value::Float(n.cos())))
            }
            "tan" => {
                if args.len() != 1 {
                    return Some(Err(format!("tan expects 1 argument, got {}", args.len())));
                }
                let n = match &args[0] {
                    Value::Int(v) => *v as f64,
                    Value::Float(v) => *v,
                    _ => return Some(Err("tan expects a number".to_string())),
                };
                Some(Ok(Value::Float(n.tan())))
            }
            "len" => {
                if args.len() != 1 {
                    return Some(Err(format!("len expects 1 argument, got {}", args.len())));
                }
                match &args[0] {
                    Value::String(s) => Some(Ok(Value::Int(s.chars().count() as i64))),
                    Value::Array(arr) => Some(Ok(Value::Int(arr.borrow().len() as i64))),
                    Value::Dict(d) => Some(Ok(Value::Int(d.borrow().len() as i64))),
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
            // File system builtins
            "readFile" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "readFile expects 1 argument, got {}",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::String(path) => match fs::read_to_string(path) {
                        Ok(content) => Some(Ok(Value::String(content))),
                        Err(e) => Some(Err(format!("{}", e))),
                    },
                    _ => Some(Err("readFile expects a string path".to_string())),
                }
            }
            "writeFile" => {
                if args.len() != 2 {
                    return Some(Err(format!(
                        "writeFile expects 2 arguments, got {}",
                        args.len()
                    )));
                }
                match (&args[0], &args[1]) {
                    (Value::String(path), Value::String(content)) => {
                        match fs::write(path, content) {
                            Ok(_) => Some(Ok(Value::Null)),
                            Err(e) => Some(Err(format!("{}", e))),
                        }
                    }
                    _ => Some(Err("writeFile expects (path, content)".to_string())),
                }
            }
            "appendFile" => {
                if args.len() != 2 {
                    return Some(Err(format!(
                        "appendFile expects 2 arguments, got {}",
                        args.len()
                    )));
                }
                match (&args[0], &args[1]) {
                    (Value::String(path), Value::String(content)) => {
                        match fs::write(path, content) {
                            Ok(_) => Some(Ok(Value::Null)),
                            Err(e) => Some(Err(format!("{}", e))),
                        }
                    }
                    _ => Some(Err("appendFile expects (path, content)".to_string())),
                }
            }
            "fileExists" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "fileExists expects 1 argument, got {}",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::String(path) => Some(Ok(Value::Bool(fs::metadata(path).is_ok()))),
                    _ => Some(Err("fileExists expects a string path".to_string())),
                }
            }
            "deleteFile" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "deleteFile expects 1 argument, got {}",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::String(path) => match fs::remove_file(path) {
                        Ok(_) => Some(Ok(Value::Null)),
                        Err(e) => Some(Err(format!("{}", e))),
                    },
                    _ => Some(Err("deleteFile expects a string path".to_string())),
                }
            }
            "mkdir" => {
                if args.len() != 1 {
                    return Some(Err(format!("mkdir expects 1 argument, got {}", args.len())));
                }
                match &args[0] {
                    Value::String(path) => match fs::create_dir(path) {
                        Ok(_) => Some(Ok(Value::Null)),
                        Err(e) => Some(Err(format!("{}", e))),
                    },
                    _ => Some(Err("mkdir expects a string path".to_string())),
                }
            }
            "listDir" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "listDir expects 1 argument, got {}",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::String(path) => match fs::read_dir(path) {
                        Ok(entries) => {
                            let mut names = Vec::new();
                            for entry in entries.flatten() {
                                if let Ok(name) = entry.file_name().into_string() {
                                    names.push(Value::String(name));
                                }
                            }
                            Some(Ok(Value::Array(Rc::new(RefCell::new(names)))))
                        }
                        Err(e) => Some(Err(format!("{}", e))),
                    },
                    _ => Some(Err("listDir expects a string path".to_string())),
                }
            }
            "getEnv" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "getEnv expects 1 argument, got {}",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::String(key) => match env::var(key) {
                        Ok(val) => Some(Ok(Value::Option(Rc::new(RefCell::new(Some(
                            Value::String(val),
                        )))))),
                        Err(_) => Some(Ok(Value::Option(Rc::new(RefCell::new(None))))),
                    },
                    _ => Some(Err("getEnv expects a string key".to_string())),
                }
            }
            "args" => {
                let args_vec: Vec<Value> = env::args().map(Value::String).collect();
                Some(Ok(Value::Array(Rc::new(RefCell::new(args_vec)))))
            }
            "exit" => {
                if args.len() != 1 {
                    return Some(Err(format!("exit expects 1 argument, got {}", args.len())));
                }
                match args[0] {
                    Value::Int(code) => process::exit(code as i32),
                    Value::Float(n) => process::exit(n as i32),
                    _ => Some(Err("exit expects an integer code".to_string())),
                }
            }
            "exec" => {
                if args.len() != 1 {
                    return Some(Err(format!("exec expects 1 argument, got {}", args.len())));
                }
                match &args[0] {
                    Value::String(cmd) => {
                        match std::process::Command::new("sh").arg("-c").arg(cmd).output() {
                            Ok(output) => {
                                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                                if output.status.success() {
                                    Some(Ok(Value::Result(Rc::new(RefCell::new(Ok(
                                        Value::String(stdout),
                                    ))))))
                                } else {
                                    let msg = if stderr.trim().is_empty() {
                                        format!("command failed: {}", output.status)
                                    } else {
                                        stderr
                                    };
                                    Some(Ok(Value::Result(Rc::new(RefCell::new(Err(msg))))))
                                }
                            }
                            Err(e) => Some(Ok(Value::Result(Rc::new(RefCell::new(Err(
                                format!("{}", e),
                            )))))),
                        }
                    }
                    _ => Some(Err("exec expects a string command".to_string())),
                }
            }
            _ => None, // Not a built-in function
        }
    }

    pub fn formatValue(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Char(c) => c.to_string(),
            Value::Int(n) => format!("{}", n),
            Value::Float(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::Bool(b) => format!("{}", b),
            Value::Null => "null".to_string(),
            Value::EnumVariant { enum_name, variant } => format!("{}::{}", enum_name, variant),
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
            Value::Vec(v) => {
                let items = v
                    .borrow()
                    .iter()
                    .map(Self::formatValue)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", items)
            }
            Value::LinkedList(l) => {
                let items = l
                    .borrow()
                    .iter()
                    .map(Self::formatValue)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("LinkedList([{}])", items)
            }
            Value::HashMap(m) => {
                let m = m.borrow();
                let mut keys = m.keys().cloned().collect::<Vec<_>>();
                keys.sort();
                let items = keys
                    .into_iter()
                    .map(|k| format!("{}: {}", k, Self::formatValue(m.get(&k).unwrap())))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{}}}", items)
            }
            Value::Stack(s) => {
                format!("Stack(len: {})", s.borrow().len())
            }
            Value::Queue(q) => {
                format!("Queue(len: {})", q.borrow().len())
            }
            Value::Option(o) => match o.borrow().as_ref() {
                Some(v) => format!("Some({})", Self::formatValue(v)),
                None => "None".to_string(),
            },
            Value::Result(r) => match r.borrow().as_ref() {
                Ok(v) => format!("Ok({})", Self::formatValue(v)),
                Err(e) => format!("Err({})", e),
            },
        }
    }
}
