use super::function::Function;
use super::value::{displayBytes, escapeBytes, floatToInt, formatFloat, Value};
use crate::lexer::span::Span;
use std::cell::RefCell;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

pub struct StdLib;

#[cfg(unix)]
fn osStringToBytes(value: &OsStr) -> Result<Vec<u8>, String> {
    use std::os::unix::ffi::OsStrExt;
    Ok(value.as_bytes().to_vec())
}

#[cfg(not(unix))]
fn osStringToBytes(value: &OsStr) -> Result<Vec<u8>, String> {
    value
        .to_str()
        .map(|value| value.as_bytes().to_vec())
        .ok_or_else(|| "OS string is not valid UTF-8 on this platform".to_string())
}

#[cfg(unix)]
fn bytesToOsString(bytes: &[u8]) -> Result<OsString, String> {
    use std::os::unix::ffi::OsStringExt;
    Ok(OsString::from_vec(bytes.to_vec()))
}

#[cfg(not(unix))]
fn bytesToOsString(bytes: &[u8]) -> Result<OsString, String> {
    String::from_utf8(bytes.to_vec())
        .map(OsString::from)
        .map_err(|_| "OS path must be valid UTF-8 on this platform".to_string())
}

fn pathFromBytes(bytes: &[u8]) -> Result<PathBuf, String> {
    Ok(PathBuf::from(bytesToOsString(bytes)?))
}

fn pathToBytes(path: &Path) -> Result<Vec<u8>, String> {
    osStringToBytes(path.as_os_str())
}

fn pathArg(value: &Value, builtin: &str) -> Result<PathBuf, String> {
    let bytes = value
        .stringBytes()
        .ok_or_else(|| format!("{} expects a string path", builtin))?;
    pathFromBytes(bytes)
}

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
        globals.insert("println".to_string(), Value::function(println_func));

        let print_func = Function::new(
            "print".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("print".to_string(), Value::function(print_func));

        let abs_func = Function::new(
            "abs".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("abs".to_string(), Value::function(abs_func));

        let max_func = Function::new(
            "max".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("max".to_string(), Value::function(max_func));

        let min_func = Function::new(
            "min".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("min".to_string(), Value::function(min_func));

        let random_func = Function::new(
            "random".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("random".to_string(), Value::function(random_func));

        let vec_new = Function::new("Vec.new".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("Vec.new".to_string(), Value::function(vec_new));

        let linkedlist_new = Function::new(
            "LinkedList.new".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert(
            "LinkedList.new".to_string(),
            Value::function(linkedlist_new),
        );

        let hashmap_new = Function::new(
            "HashMap.new".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("HashMap.new".to_string(), Value::function(hashmap_new));

        let stack_new = Function::new(
            "Stack.new".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("Stack.new".to_string(), Value::function(stack_new));

        let queue_new = Function::new(
            "Queue.new".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("Queue.new".to_string(), Value::function(queue_new));

        let option_some = Function::new(
            "Option::Some".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("Option::Some".to_string(), Value::function(option_some));

        let option_none = Function::new(
            "Option::None".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("Option::None".to_string(), Value::function(option_none));

        let result_ok = Function::new(
            "Result::Ok".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("Result::Ok".to_string(), Value::function(result_ok));

        let result_err = Function::new(
            "Result::Err".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("Result::Err".to_string(), Value::function(result_err));

        // FS builtins
        let readFile = Function::new(
            "readFile".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("readFile".to_string(), Value::function(readFile));

        let writeFile = Function::new(
            "writeFile".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("writeFile".to_string(), Value::function(writeFile));

        let appendFile = Function::new(
            "appendFile".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("appendFile".to_string(), Value::function(appendFile));

        let fileExists = Function::new(
            "fileExists".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("fileExists".to_string(), Value::function(fileExists));

        let deleteFile = Function::new(
            "deleteFile".to_string(),
            vec![],
            vec![],
            Span::new(0, 0),
            None,
        );
        globals.insert("deleteFile".to_string(), Value::function(deleteFile));

        let mkdir = Function::new("mkdir".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("mkdir".to_string(), Value::function(mkdir));

        // aliased natives so std wrappers can call them without recursing into themselves
        let nativeMkdir = Function::new("nativeMkdir".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeMkdir".to_string(), Value::function(nativeMkdir));

        let listDir = Function::new("listDir".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("listDir".to_string(), Value::function(listDir));

        let getEnv = Function::new("getEnv".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("getEnv".to_string(), Value::function(getEnv));

        let args = Function::new("args".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("args".to_string(), Value::function(args));

        let nativeArgs = Function::new("nativeArgs".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeArgs".to_string(), Value::function(nativeArgs));

        let exit = Function::new("exit".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("exit".to_string(), Value::function(exit));

        let nativeExit = Function::new("nativeExit".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeExit".to_string(), Value::function(nativeExit));

        let nativeSqrt = Function::new("nativeSqrt".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeSqrt".to_string(), Value::function(nativeSqrt));

        let nativePow = Function::new("nativePow".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativePow".to_string(), Value::function(nativePow));

        let nativeLog = Function::new("nativeLog".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeLog".to_string(), Value::function(nativeLog));

        let nativeRandom = Function::new("nativeRandom".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeRandom".to_string(), Value::function(nativeRandom));

        let exec = Function::new("exec".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("exec".to_string(), Value::function(exec));

        // More FS/process builtins (aliased with native* for std wrappers)
        let cwd = Function::new("cwd".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("cwd".to_string(), Value::function(cwd));
        let nativeCwd = Function::new("nativeCwd".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeCwd".to_string(), Value::function(nativeCwd));

        let chdir = Function::new("chdir".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("chdir".to_string(), Value::function(chdir));
        let nativeChdir = Function::new("nativeChdir".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeChdir".to_string(), Value::function(nativeChdir));

        let tempDir = Function::new("tempDir".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("tempDir".to_string(), Value::function(tempDir));
        let nativeTempDir = Function::new("nativeTempDir".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeTempDir".to_string(), Value::function(nativeTempDir));

        let mkdirAll = Function::new("mkdirAll".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("mkdirAll".to_string(), Value::function(mkdirAll));
        let nativeMkdirAll = Function::new("nativeMkdirAll".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeMkdirAll".to_string(), Value::function(nativeMkdirAll));

        let removeDir = Function::new("removeDir".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("removeDir".to_string(), Value::function(removeDir));
        let nativeRemoveDir = Function::new("nativeRemoveDir".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeRemoveDir".to_string(), Value::function(nativeRemoveDir));
        let removeDirAll = Function::new("removeDirAll".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("removeDirAll".to_string(), Value::function(removeDirAll));
        let nativeRemoveDirAll = Function::new("nativeRemoveDirAll".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeRemoveDirAll".to_string(), Value::function(nativeRemoveDirAll));

        let copyFile = Function::new("copyFile".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("copyFile".to_string(), Value::function(copyFile));
        let nativeCopyFile = Function::new("nativeCopyFile".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeCopyFile".to_string(), Value::function(nativeCopyFile));
        let moveFile = Function::new("moveFile".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("moveFile".to_string(), Value::function(moveFile));
        let nativeMoveFile = Function::new("nativeMoveFile".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeMoveFile".to_string(), Value::function(nativeMoveFile));

        let isFile = Function::new("isFile".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("isFile".to_string(), Value::function(isFile));
        let nativeIsFile = Function::new("nativeIsFile".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeIsFile".to_string(), Value::function(nativeIsFile));
        let isDir = Function::new("isDir".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("isDir".to_string(), Value::function(isDir));
        let nativeIsDir = Function::new("nativeIsDir".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeIsDir".to_string(), Value::function(nativeIsDir));

        let listDirPaths = Function::new("listDirPaths".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("listDirPaths".to_string(), Value::function(listDirPaths));
        let nativeListDirPaths = Function::new("nativeListDirPaths".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeListDirPaths".to_string(), Value::function(nativeListDirPaths));

        let pathJoin = Function::new("pathJoin".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("pathJoin".to_string(), Value::function(pathJoin));
        let nativePathJoin = Function::new("nativePathJoin".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativePathJoin".to_string(), Value::function(nativePathJoin));
        let basename = Function::new("basename".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("basename".to_string(), Value::function(basename));
        let nativeBasename = Function::new("nativeBasename".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeBasename".to_string(), Value::function(nativeBasename));
        let dirname = Function::new("dirname".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("dirname".to_string(), Value::function(dirname));
        let nativeDirname = Function::new("nativeDirname".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeDirname".to_string(), Value::function(nativeDirname));
        let extname = Function::new("extname".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("extname".to_string(), Value::function(extname));
        let nativeExtname = Function::new("nativeExtname".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeExtname".to_string(), Value::function(nativeExtname));

        let setEnv = Function::new("setEnv".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("setEnv".to_string(), Value::function(setEnv));
        let nativeSetEnv = Function::new("nativeSetEnv".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeSetEnv".to_string(), Value::function(nativeSetEnv));
        let unsetEnv = Function::new("unsetEnv".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("unsetEnv".to_string(), Value::function(unsetEnv));
        let nativeUnsetEnv = Function::new("nativeUnsetEnv".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeUnsetEnv".to_string(), Value::function(nativeUnsetEnv));
        let envAll = Function::new("envAll".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("envAll".to_string(), Value::function(envAll));
        let nativeEnvAll = Function::new("nativeEnvAll".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeEnvAll".to_string(), Value::function(nativeEnvAll));

        let sleepMs = Function::new("sleepMs".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("sleepMs".to_string(), Value::function(sleepMs));
        let nativeSleepMs = Function::new("nativeSleepMs".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeSleepMs".to_string(), Value::function(nativeSleepMs));

        let execDetailed = Function::new("execDetailed".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("execDetailed".to_string(), Value::function(execDetailed));
        let nativeExecDetailed = Function::new("nativeExecDetailed".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeExecDetailed".to_string(), Value::function(nativeExecDetailed));

        let which = Function::new("which".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("which".to_string(), Value::function(which));
        let nativeWhich = Function::new("nativeWhich".to_string(), vec![], vec![], Span::new(0, 0), None);
        globals.insert("nativeWhich".to_string(), Value::function(nativeWhich));

        globals
    }

    pub fn call_builtin_function(
        name: &str,
        args: &[Value],
        program_args: &[String],
    ) -> Option<Result<Value, String>> {
        match name {
            "int" => {
                if args.len() != 1 {
                    return Some(Err(format!("int expects 1 argument, got {}", args.len())));
                }
                match &args[0] {
                    Value::Char(byte) => Some(Ok(Value::Int(*byte as i64))),
                    _ => Some(Err("int expects a char".to_string())),
                }
            }
            "char" => {
                if args.len() != 1 {
                    return Some(Err(format!("char expects 1 argument, got {}", args.len())));
                }
                match &args[0] {
                    Value::Int(n) if (0..=u8::MAX as i64).contains(n) => {
                        Some(Ok(Value::Char(*n as u8)))
                    }
                    Value::Int(n) => Some(Err(format!("char value out of range 0..=255: {}", n))),
                    _ => Some(Err("char expects an int".to_string())),
                }
            }
            "float" => {
                if args.len() != 1 {
                    return Some(Err(format!("float expects 1 argument, got {}", args.len())));
                }
                match &args[0] {
                    Value::Int(n) => Some(Ok(Value::Float(*n as f64))),
                    Value::Float(n) => Some(Ok(Value::Float(*n))),
                    _ => Some(Err("float expects an int or float".to_string())),
                }
            }
            "string" => {
                if args.len() != 1 {
                    return Some(Err(format!("string expects 1 argument, got {}", args.len())));
                }
                let bytes = match &args[0] {
                    Value::Char(byte) => vec![*byte],
                    Value::Int(n) => n.to_string().into_bytes(),
                    Value::Float(n) => Self::formatValue(&Value::Float(*n)).into_bytes(),
                    Value::Bool(b) => b.to_string().into_bytes(),
                    value if value.stringBytes().is_some() => {
                        value.stringBytes().unwrap().to_vec()
                    }
                    _ => return Some(Err("string expects a scalar value".to_string())),
                };
                Some(Ok(Value::fromStringBytes(bytes)))
            }
            "println" => {
                let stdout = io::stdout();
                let mut stdout = stdout.lock();
                Some(Self::writeOutput(&mut stdout, args, true))
            }
            "print" => {
                let stdout = io::stdout();
                let mut stdout = stdout.lock();
                Some(Self::writeOutput(&mut stdout, args, false))
            }
            "eprint" | "eprintln" => {
                let stderr = io::stderr();
                let mut stderr = stderr.lock();
                Some(Self::writeStderr(&mut stderr, args, name == "eprintln"))
            }
            "abs" => {
                if args.len() != 1 {
                    return Some(Err(format!("abs expects 1 argument, got {}", args.len())));
                }
                match &args[0] {
                    Value::Int(n) => match n.checked_abs() {
                        Some(value) => Some(Ok(Value::Int(value))),
                        None => Some(Err("Integer overflow in abs".to_string())),
                    },
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
            "random" | "nativeRandom" => {
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
                    Value::Int(v) => return Some(Ok(Value::Int(*v))),
                    Value::Float(v) => *v,
                    _ => return Some(Err("floor expects a number".to_string())),
                };
                Some(floatToInt(n.floor()).map(Value::Int))
            }
            "ceil" => {
                if args.len() != 1 {
                    return Some(Err(format!("ceil expects 1 argument, got {}", args.len())));
                }
                let n = match &args[0] {
                    Value::Int(v) => return Some(Ok(Value::Int(*v))),
                    Value::Float(v) => *v,
                    _ => return Some(Err("ceil expects a number".to_string())),
                };
                Some(floatToInt(n.ceil()).map(Value::Int))
            }
            "round" => {
                if args.len() != 1 {
                    return Some(Err(format!("round expects 1 argument, got {}", args.len())));
                }
                let n = match &args[0] {
                    Value::Int(v) => return Some(Ok(Value::Int(*v))),
                    Value::Float(v) => *v,
                    _ => return Some(Err("round expects a number".to_string())),
                };
                Some(floatToInt(n.round()).map(Value::Int))
            }
            "sqrt" | "nativeSqrt" => {
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
            "pow" | "nativePow" => {
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
            "log" | "nativeLog" => {
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
                    value if value.stringBytes().is_some() => {
                        Some(Ok(Value::Int(value.stringBytes().unwrap().len() as i64)))
                    }
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
                    let Some(prompt) = args[0].stringBytes() else {
                        return Some(Err("input prompt must be a string".to_string()));
                    };
                    let stdout = io::stdout();
                    let mut stdout = stdout.lock();
                    if let Err(err) = stdout.write_all(prompt) {
                        return Some(Err(format!("failed to write prompt: {}", err)));
                    }
                    let _ = stdout.flush();
                }

                let stdin = io::stdin();
                let mut stdin = stdin.lock();
                let mut line = Vec::new();
                match stdin.read_until(b'\n', &mut line) {
                    Ok(_) => {
                        while line.last().is_some_and(|byte| *byte == b'\n' || *byte == b'\r') {
                            line.pop();
                        }
                        Some(Ok(Value::fromStringBytes(line)))
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
                let path_bytes = match args[0].stringBytes() {
                    Some(path) => path,
                    None => return Some(Err("readFile expects a string path".to_string())),
                };
                let path = match pathFromBytes(path_bytes) {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                match fs::read(path) {
                    Ok(content) => Some(Ok(Value::fromStringBytes(content))),
                    Err(e) => Some(Err(format!("{}", e))),
                }
            }
            "writeFile" => {
                if args.len() != 2 {
                    return Some(Err(format!(
                        "writeFile expects 2 arguments, got {}",
                        args.len()
                    )));
                }
                let path = match pathArg(&args[0], "writeFile") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                let content = match args[1].stringBytes() {
                    Some(content) => content,
                    None => return Some(Err("writeFile expects string content".to_string())),
                };
                match fs::write(path, content) {
                    Ok(_) => Some(Ok(Value::Null)),
                    Err(e) => Some(Err(format!("{}", e))),
                }
            }
            "appendFile" => {
                if args.len() != 2 {
                    return Some(Err(format!(
                        "appendFile expects 2 arguments, got {}",
                        args.len()
                    )));
                }
                let path = match pathArg(&args[0], "appendFile") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                let content = match args[1].stringBytes() {
                    Some(content) => content,
                    None => return Some(Err("appendFile expects string content".to_string())),
                };
                match OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .and_then(|mut file| std::io::Write::write_all(&mut file, content))
                {
                    Ok(_) => Some(Ok(Value::Null)),
                    Err(e) => Some(Err(format!("{}", e))),
                }
            }
            "fileExists" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "fileExists expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let path = match pathArg(&args[0], "fileExists") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                Some(Ok(Value::Bool(fs::metadata(path).is_ok())))
            }
            "deleteFile" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "deleteFile expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let path = match pathArg(&args[0], "deleteFile") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                match fs::remove_file(path) {
                    Ok(_) => Some(Ok(Value::Null)),
                    Err(e) => Some(Err(format!("{}", e))),
                }
            }
            "mkdir" | "nativeMkdir" => {
                if args.len() != 1 {
                    return Some(Err(format!("mkdir expects 1 argument, got {}", args.len())));
                }
                let path = match pathArg(&args[0], "mkdir") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                match fs::create_dir(path) {
                    Ok(_) => Some(Ok(Value::Null)),
                    Err(e) => Some(Err(format!("{}", e))),
                }
            }
            "listDir" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "listDir expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let path = match pathArg(&args[0], "listDir") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                match fs::read_dir(path) {
                        Ok(entries) => {
                            let mut names = Vec::new();
                            for entry in entries.flatten() {
                                if let Ok(name) = osStringToBytes(&entry.file_name()) {
                                    names.push(Value::fromStringBytes(name));
                                }
                            }
                            Some(Ok(Value::Array(Rc::new(RefCell::new(names)))))
                        }
                        Err(e) => Some(Err(format!("{}", e))),
                }
            }
            "getEnv" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "getEnv expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let key = match args[0].stringBytes().and_then(|bytes| bytesToOsString(bytes).ok()) {
                    Some(key) => key,
                    None => return Some(Err("getEnv expects a valid string key".to_string())),
                };
                let value = env::var_os(key).map(|value| {
                    osStringToBytes(&value).map(Value::fromStringBytes)
                });
                Some(match value {
                    Some(Ok(value)) => Ok(Value::Option(Rc::new(RefCell::new(Some(value))))),
                    Some(Err(err)) => Err(err),
                    None => Ok(Value::Option(Rc::new(RefCell::new(None)))),
                })
            }
            "args" | "nativeArgs" => {
                let args_vec = program_args
                    .iter()
                    .cloned()
                    .map(|arg| Value::fromStringBytes(arg.into_bytes()))
                    .collect();
                Some(Ok(Value::Array(Rc::new(RefCell::new(args_vec)))))
            }
            "exit" | "nativeExit" => {
                if args.len() != 1 {
                    return Some(Err(format!("exit expects 1 argument, got {}", args.len())));
                }
                match args[0] {
                    Value::Int(code)
                        if (i32::MIN as i64..=i32::MAX as i64).contains(&code) =>
                    {
                        process::exit(code as i32)
                    }
                    Value::Int(_) => Some(Err("exit expects an in-range integer code".to_string())),
                    Value::Float(n) => match floatToInt(n) {
                        Ok(code) if (i32::MIN as i64..=i32::MAX as i64).contains(&code) => {
                            process::exit(code as i32)
                        }
                        _ => Some(Err("exit expects an in-range integer code".to_string())),
                    },
                    _ => Some(Err("exit expects an integer code".to_string())),
                }
            }
            "exec" => {
                if args.len() != 1 {
                    return Some(Err(format!("exec expects 1 argument, got {}", args.len())));
                }
                let cmd = match args[0].stringBytes().and_then(|b| std::str::from_utf8(b).ok()) {
                    Some(cmd) => cmd,
                    None => return Some(Err("exec expects a UTF-8 command string".to_string())),
                };
                {
                        let mut command = if cfg!(windows) {
                            let mut c = std::process::Command::new("cmd");
                            c.arg("/C").arg(cmd);
                            c
                        } else {
                            let mut c = std::process::Command::new("sh");
                            c.arg("-c").arg(cmd);
                            c
                        };
                        match command.output() {
                            Ok(output) => {
                                let stdout = output.stdout;
                                let stderr = output.stderr;
                                if output.status.success() {
                                    Some(Ok(Value::Result(Rc::new(RefCell::new(Ok(
                                        Value::fromStringBytes(stdout),
                                    ))))))
                                } else {
                                    let msg = if stderr.iter().all(|byte| byte.is_ascii_whitespace()) {
                                        format!("command failed: {}", output.status)
                                    } else {
                                        escapeBytes(&stderr)
                                    };
                                    Some(Ok(Value::Result(Rc::new(RefCell::new(Err(
                                        Value::String(crate::interpreter::value::SharedStr::from(msg)),
                                    ))))))
                                }
                            }
                            Err(e) => Some(Ok(Value::Result(Rc::new(RefCell::new(Err(
                                Value::String(crate::interpreter::value::SharedStr::from(format!("{}", e))),
                            )))))),
                        }
                }
            }
            "execDetailed" | "nativeExecDetailed" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "execDetailed expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let cmd = match args[0].stringBytes().and_then(|b| std::str::from_utf8(b).ok()) {
                    Some(cmd) => cmd,
                    None => return Some(Err("execDetailed expects a UTF-8 command string".to_string())),
                };
                {
                        let mut command = if cfg!(windows) {
                            let mut c = std::process::Command::new("cmd");
                            c.arg("/C").arg(cmd);
                            c
                        } else {
                            let mut c = std::process::Command::new("sh");
                            c.arg("-c").arg(cmd);
                            c
                        };
                        match command.output() {
                            Ok(output) => {
                                let code = output.status.code().unwrap_or(-1) as i64;
                                let stdout = output.stdout;
                                let stderr = output.stderr;
                                let mut m = std::collections::HashMap::new();
                                m.insert(b"code".to_vec(), Value::Int(code));
                                m.insert(b"stdout".to_vec(), Value::fromStringBytes(stdout));
                                m.insert(b"stderr".to_vec(), Value::fromStringBytes(stderr));
                                Some(Ok(Value::Dict(Rc::new(RefCell::new(m)))))
                            }
                            Err(e) => Some(Err(format!("{}", e))),
                        }
                }
            }
            "cwd" | "nativeCwd" => {
                if !args.is_empty() {
                    return Some(Err(format!("cwd expects 0 arguments, got {}", args.len())));
                }
                match env::current_dir() {
                    Ok(path) => Some(pathToBytes(&path).map(Value::fromStringBytes)),
                    Err(e) => Some(Err(format!("{}", e))),
                }
            }
            "chdir" | "nativeChdir" => {
                if args.len() != 1 {
                    return Some(Err(format!("chdir expects 1 argument, got {}", args.len())));
                }
                let path = match pathArg(&args[0], "chdir") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                Some(env::set_current_dir(path).map(|_| Value::Null).map_err(|e| format!("{}", e)))
            }
            "tempDir" | "nativeTempDir" => {
                if !args.is_empty() {
                    return Some(Err(format!(
                        "tempDir expects 0 arguments, got {}",
                        args.len()
                    )));
                }
                Some(pathToBytes(&env::temp_dir()).map(Value::fromStringBytes))
            }
            "mkdirAll" | "nativeMkdirAll" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "mkdirAll expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let path = match pathArg(&args[0], "mkdirAll") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                Some(fs::create_dir_all(path).map(|_| Value::Null).map_err(|e| format!("{}", e)))
            }
            "removeDir" | "nativeRemoveDir" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "removeDir expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let path = match pathArg(&args[0], "removeDir") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                Some(fs::remove_dir(path).map(|_| Value::Null).map_err(|e| format!("{}", e)))
            }
            "removeDirAll" | "nativeRemoveDirAll" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "removeDirAll expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let path = match pathArg(&args[0], "removeDirAll") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                Some(fs::remove_dir_all(path).map(|_| Value::Null).map_err(|e| format!("{}", e)))
            }
            "copyFile" | "nativeCopyFile" => {
                if args.len() != 2 {
                    return Some(Err(format!(
                        "copyFile expects 2 arguments, got {}",
                        args.len()
                    )));
                }
                let src = match pathArg(&args[0], "copyFile") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                let dst = match pathArg(&args[1], "copyFile") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                Some(fs::copy(src, dst).map(|_| Value::Null).map_err(|e| format!("{}", e)))
            }
            "moveFile" | "nativeMoveFile" => {
                if args.len() != 2 {
                    return Some(Err(format!(
                        "moveFile expects 2 arguments, got {}",
                        args.len()
                    )));
                }
                let src = match pathArg(&args[0], "moveFile") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                let dst = match pathArg(&args[1], "moveFile") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                Some(fs::rename(src, dst).map(|_| Value::Null).map_err(|e| format!("{}", e)))
            }
            "isFile" | "nativeIsFile" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "isFile expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let path = match pathArg(&args[0], "isFile") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                Some(Ok(Value::Bool(fs::metadata(path).map(|m| m.is_file()).unwrap_or(false))))
            }
            "isDir" | "nativeIsDir" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "isDir expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let path = match pathArg(&args[0], "isDir") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                Some(Ok(Value::Bool(fs::metadata(path).map(|m| m.is_dir()).unwrap_or(false))))
            }
            "listDirPaths" | "nativeListDirPaths" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "listDirPaths expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let path = match pathArg(&args[0], "listDirPaths") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                match fs::read_dir(path) {
                        Ok(entries) => {
                            let mut paths = Vec::new();
                            for entry in entries.flatten() {
                                match pathToBytes(&entry.path()) {
                                    Ok(bytes) => paths.push(Value::fromStringBytes(bytes)),
                                    Err(err) => return Some(Err(err)),
                                }
                            }
                            Some(Ok(Value::Array(Rc::new(RefCell::new(paths)))))
                        }
                        Err(e) => Some(Err(format!("{}", e))),
                }
            }
            "pathJoin" | "nativePathJoin" => {
                if args.len() != 2 {
                    return Some(Err(format!(
                        "pathJoin expects 2 arguments, got {}",
                        args.len()
                    )));
                }
                let mut joined = match args[0]
                    .stringBytes()
                    .ok_or_else(|| "pathJoin expects string paths".to_string())
                    .and_then(pathFromBytes)
                {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                let child = match args[1]
                    .stringBytes()
                    .ok_or_else(|| "pathJoin expects string paths".to_string())
                    .and_then(pathFromBytes)
                {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                joined.push(child);
                Some(pathToBytes(&joined).map(Value::fromStringBytes))
            }
            "basename" | "nativeBasename" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "basename expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let path = match pathArg(&args[0], "basename") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                let name = path.file_name().unwrap_or_else(|| OsStr::new(""));
                Some(osStringToBytes(name).map(Value::fromStringBytes))
            }
            "dirname" | "nativeDirname" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "dirname expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let path = match pathArg(&args[0], "dirname") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                Some(path.parent().map_or_else(
                    || Ok(Value::fromStringBytes(Vec::new())),
                    |parent| pathToBytes(parent).map(Value::fromStringBytes),
                ))
            }
            "extname" | "nativeExtname" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "extname expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let path = match pathArg(&args[0], "extname") {
                    Ok(path) => path,
                    Err(err) => return Some(Err(err)),
                };
                Some(path.extension().map_or_else(
                    || Ok(Value::fromStringBytes(Vec::new())),
                    |extension| osStringToBytes(extension).map(Value::fromStringBytes),
                ))
            }
            "setEnv" | "nativeSetEnv" => {
                if args.len() != 2 {
                    return Some(Err(format!(
                        "setEnv expects 2 arguments, got {}",
                        args.len()
                    )));
                }
                let key = match args[0].stringBytes().and_then(|bytes| bytesToOsString(bytes).ok()) {
                    Some(value) => value,
                    None => return Some(Err("setEnv expects a string key".to_string())),
                };
                let value = match args[1].stringBytes().and_then(|bytes| bytesToOsString(bytes).ok()) {
                    Some(value) => value,
                    None => return Some(Err("setEnv expects a string value".to_string())),
                };
                env::set_var(key, value);
                Some(Ok(Value::Null))
            }
            "unsetEnv" | "nativeUnsetEnv" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "unsetEnv expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let key = match args[0].stringBytes().and_then(|bytes| bytesToOsString(bytes).ok()) {
                    Some(value) => value,
                    None => return Some(Err("unsetEnv expects a string key".to_string())),
                };
                env::remove_var(key);
                Some(Ok(Value::Null))
            }
            "envAll" | "nativeEnvAll" => {
                if !args.is_empty() {
                    return Some(Err(format!(
                        "envAll expects 0 arguments, got {}",
                        args.len()
                    )));
                }
                let mut m = std::collections::HashMap::new();
                for (k, v) in env::vars_os() {
                    let key = match osStringToBytes(&k) {
                        Ok(bytes) => bytes,
                        Err(err) => return Some(Err(err)),
                    };
                    let value = match osStringToBytes(&v) {
                        Ok(bytes) => Value::fromStringBytes(bytes),
                        Err(err) => return Some(Err(err)),
                    };
                    m.insert(key, value);
                }
                Some(Ok(Value::Dict(Rc::new(RefCell::new(m)))))
            }
            "sleepMs" | "nativeSleepMs" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "sleepMs expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let ms = match &args[0] {
                    Value::Int(v) => *v,
                    Value::Float(v) if v.fract() == 0.0 => match floatToInt(*v) {
                        Ok(value) => value,
                        Err(err) => return Some(Err(err)),
                    },
                    Value::Float(_) => {
                        return Some(Err("sleepMs expects an integer milliseconds value".to_string()))
                    }
                    _ => return Some(Err("sleepMs expects an integer milliseconds value".to_string())),
                };
                if ms < 0 {
                    return Some(Err("sleepMs expects a non-negative value".to_string()));
                }
                thread::sleep(Duration::from_millis(ms as u64));
                Some(Ok(Value::Null))
            }
            "which" | "nativeWhich" => {
                if args.len() != 1 {
                    return Some(Err(format!(
                        "which expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let name = match args[0].stringBytes().and_then(|bytes| bytesToOsString(bytes).ok()) {
                    Some(name) => name,
                    None => return Some(Err("which expects a string command name".to_string())),
                };
                let mut candidates = vec![name.clone()];
                if cfg!(windows) {
                    for extension in [".exe", ".cmd", ".bat"] {
                        let mut candidate = name.clone();
                        candidate.push(extension);
                        candidates.push(candidate);
                    }
                }
                let path_var = env::var_os("PATH").unwrap_or_default();
                for dir in env::split_paths(&path_var) {
                    for candidate in &candidates {
                        let path = dir.join(candidate);
                        if fs::metadata(&path).map(|metadata| metadata.is_file()).unwrap_or(false) {
                            let path_value = match pathToBytes(&path) {
                                Ok(bytes) => Value::fromStringBytes(bytes),
                                Err(err) => return Some(Err(err)),
                            };
                            return Some(Ok(Value::Option(Rc::new(RefCell::new(Some(path_value))))));
                        }
                    }
                }
                Some(Ok(Value::Option(Rc::new(RefCell::new(None)))))
            }
            _ => None, // Not a built-in function
        }
    }

    fn writeBytes(
        writer: &mut impl Write,
        args: &[Value],
        newline: bool,
    ) -> std::io::Result<()> {
        for (index, value) in args.iter().enumerate() {
            if index > 0 {
                writer.write_all(b" ")?;
            }
            writer.write_all(&Self::formatValueBytes(value))?;
        }
        if newline {
            writer.write_all(b"\n")?;
        }
        writer.flush()
    }

    fn writeOutput(
        writer: &mut impl Write,
        args: &[Value],
        newline: bool,
    ) -> Result<Value, String> {
        Self::writeBytes(writer, args, newline)
            .map_err(|err| format!("failed to write output: {}", err))?;
        Ok(Value::Null)
    }

    fn writeStderr(
        writer: &mut impl Write,
        args: &[Value],
        newline: bool,
    ) -> Result<Value, String> {
        Self::writeBytes(writer, args, newline)
            .map_err(|err| format!("failed to write stderr: {}", err))?;
        Ok(Value::Null)
    }

    pub fn formatValueBytes(value: &Value) -> Vec<u8> {
        match value {
            Value::String(string) => string.asBytes().to_vec(),
            Value::ByteString(bytes) => bytes.asBytes().to_vec(),
            Value::Char(byte) => vec![*byte],
            Value::Int(_) | Value::Float(_) | Value::Bool(_) | Value::Null => {
                Self::formatValue(value).into_bytes()
            }
            _ => Self::formatValue(value).into_bytes(),
        }
    }

    pub fn formatValue(value: &Value) -> String {
        match value {
            Value::String(s) => s.asStr().to_string(),
            Value::ByteString(bytes) => format!("b\"{}\"", escapeBytes(bytes.asBytes())),
            Value::Char(byte) => {
                if byte.is_ascii_graphic() || *byte == b' ' {
                    (*byte as char).to_string()
                } else {
                    format!("\\x{:02X}", byte)
                }
            }
            Value::Int(n) => format!("{}", n),
            Value::Float(n) => formatFloat(*n),
            Value::Bool(b) => format!("{}", b),
            Value::Null => "null".to_string(),
            Value::EnumVariant { enum_name, variant } => format!("{}::{}", enum_name, variant),
            Value::EnumVariantPayload {
                enum_name,
                variant,
                payloads,
            } => format!(
                "{}::{}({})",
                enum_name,
                variant,
                payloads
                    .iter()
                    .map(Self::formatValue)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
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
                        format!("{}: {}", displayBytes(&k), Self::formatValue(v))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{}}}", items)
            }
            Value::Namespace(_) => "<namespace>".to_string(),
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
                    .map(|k| {
                        format!(
                            "{}: {}",
                            displayBytes(&k),
                            Self::formatValue(m.get(&k).unwrap())
                        )
                    })
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
                Err(e) => format!("Err({})", Self::formatValue(e)),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StdLib;
    use crate::interpreter::value::{formatFloat, SharedBuf, Value};

    #[test]
    fn stderrBuiltinsWriteOnlyTheirFormattedPayload() {
        let stdout: Vec<u8> = Vec::new();
        let mut stderr: Vec<u8> = Vec::new();
        let args = vec![
            Value::utf8("diagnostic:"),
            Value::utf8("é"),
            Value::Int(7),
        ];

        StdLib::writeStderr(&mut stderr, &args, false).unwrap();
        StdLib::writeStderr(&mut stderr, &[Value::utf8(" done")], true).unwrap();
        StdLib::writeStderr(
            &mut stderr,
            &[Value::ByteString(SharedBuf::new(vec![0, 0x80, 0xff])), Value::Char(0xfe)],
            true,
        )
        .unwrap();

        assert!(stdout.is_empty());
        assert_eq!(
            stderr,
            [
                "diagnostic: é 7 done\n".as_bytes(),
                &[0, 0x80, 0xff, b' ', 0xfe, b'\n'],
            ]
            .concat()
        );
    }

    #[test]
    fn floatSpellingUsesDebugWithoutIntegralZero() {
        assert_eq!(formatFloat(0.0), "0");
        assert_eq!(formatFloat(-0.0), "-0");
        assert_eq!(formatFloat(1.0), "1");
        assert_eq!(formatFloat(-1.0), "-1");
        assert_eq!(formatFloat(1.5), "1.5");
        assert_eq!(formatFloat(0.1), "0.1");
        assert_eq!(formatFloat(1e-4), "0.0001");
        assert_eq!(formatFloat(1e-5), "1e-5");
        assert_eq!(formatFloat(1e16), "1e16");
        assert_eq!(formatFloat(9007199254740992.0), "9007199254740992");
        assert_eq!(formatFloat(2f64.powi(70)), "1.1805916207174113e21");
        assert_eq!(formatFloat(f64::MAX), "1.7976931348623157e308");
        assert_eq!(formatFloat(f64::from_bits(1)), "5e-324");
        assert_eq!(formatFloat(f64::INFINITY), "inf");
        assert_eq!(formatFloat(f64::NEG_INFINITY), "-inf");
        assert_eq!(formatFloat(f64::NAN), "NaN");
        assert_eq!(StdLib::formatValue(&Value::Float(10.0)), "10");
    }
}
