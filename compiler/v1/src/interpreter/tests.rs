use super::environment::Environment;
use super::executor::Executor;
use super::function::Function;
use super::value::{SharedBuf, SharedStr, StructDef, Value};
use crate::ast::{EnumPayloadDecl, EnumVariantDecl, Expr, FieldDecl, Literal, Parameter, Stmt, Type};
use crate::lexer::span::Span;
use crate::lexer::TokenKind;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn span() -> Span {
    Span::new(0, 0)
}

fn int(value: i64) -> Expr {
    Expr::Literal {
        value: Literal::Int(value),
        span: span(),
    }
}

fn variable(name: &str) -> Expr {
    Expr::Variable {
        name: name.to_string(),
        span: span(),
    }
}

fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::Call {
        callee: Box::new(variable(name)),
        args,
        span: span(),
    }
}

fn method(receiver: Expr, name: &str, args: Vec<Expr>) -> Expr {
    Expr::MethodCall {
        receiver: Box::new(receiver),
        name: name.to_string(),
        args,
        span: span(),
    }
}

fn enumVariant(owner: Option<&str>, variant: &str, args: Vec<Expr>) -> Expr {
    Expr::EnumVariant {
        enum_name: owner.map(str::to_string),
        variant: variant.to_string(),
        args,
        span: span(),
    }
}

fn staticCall(owner: &str, method: &str, args: Vec<Expr>) -> Expr {
    Expr::StaticCall {
        struct_name: owner.to_string(),
        method: method.to_string(),
        args,
        span: span(),
    }
}

fn add(left: Expr, right: Expr) -> Expr {
    Expr::Binary {
        left: Box::new(left),
        op: TokenKind::Plus,
        right: Box::new(right),
        span: span(),
    }
}

fn increment(target: Expr, prefix: bool) -> Expr {
    Expr::UpdateLValue {
        target: Box::new(target),
        op: TokenKind::PlusPlus,
        prefix,
        span: span(),
    }
}

fn function(name: &str, params: Vec<Parameter>, body: Vec<Stmt>) -> Value {
    Value::function(Function::new(name.to_string(), params, body, span(), None))
}

#[test]
fn functionCanAssignGlobalAndLambdasKeepImmutableSnapshots() {
    let executor = Executor::new();
    let mut env = Environment::new();
    env.define("g".to_string(), Value::Int(1));
    env.define(
        "bump".to_string(),
        function(
            "bump",
            vec![],
            vec![Stmt::ExprStmt(Expr::Assign {
                name: "g".to_string(),
                value: Box::new(add(variable("g"), int(1))),
                span: span(),
            })],
        ),
    );

    executor.evaluate_expr(&call("bump", vec![]), &mut env).unwrap();
    executor.evaluate_expr(&call("bump", vec![]), &mut env).unwrap();
    assert_eq!(env.get("g"), Some(Value::Int(3)));

    env.define("captured".to_string(), Value::Int(7));
    let closure = Expr::Lambda {
        params: vec![],
        body: Box::new(variable("captured")),
        span: span(),
    };
    let value = executor.evaluate_expr(&closure, &mut env).unwrap();
    env.assign("captured", Value::Int(9)).unwrap();
    env.define("readCaptured".to_string(), value);
    assert_eq!(executor.evaluate_expr(&call("readCaptured", vec![]), &mut env), Ok(Value::Int(7)));
    executor.evaluate_expr(&call("bump", vec![]), &mut env).unwrap();
    assert_eq!(env.get("g"), Some(Value::Int(4)));

    env.define(
        "items".to_string(),
        Value::Array(Rc::new(RefCell::new(vec![Value::Int(1)]))),
    );
    let aggregate_reader = Expr::Lambda {
        params: vec![],
        body: Box::new(Expr::MethodCall {
            receiver: Box::new(variable("items")),
            name: "push".to_string(),
            args: vec![int(2)],
            span: span(),
        }),
        span: span(),
    };
    let aggregate_reader = executor
        .evaluate_expr(&aggregate_reader, &mut env)
        .unwrap();
    env.define("mutateItems".to_string(), aggregate_reader);
    executor
        .evaluate_expr(&call("mutateItems", vec![]), &mut env)
        .unwrap();
    let items = match env.get("items").unwrap() {
        Value::Array(items) => items,
        _ => panic!("expected captured array"),
    };
    assert_eq!(items.borrow().as_slice(), &[Value::Int(1), Value::Int(2)]);

    let mut write_env = Environment::new();
    write_env.define("captured".to_string(), Value::Int(7));
    let writer = Expr::Lambda {
        params: vec![],
        body: Box::new(Expr::Assign {
            name: "captured".to_string(),
            value: Box::new(int(8)),
            span: span(),
        }),
        span: span(),
    };
    let writer = executor.evaluate_expr(&writer, &mut write_env).unwrap();
    write_env.define("writer".to_string(), writer);
    let err = executor
        .evaluate_expr(&call("writer", vec![]), &mut write_env)
        .unwrap_err();
    assert!(err.contains("captured variable"), "got: {}", err);
    assert_eq!(write_env.get("captured"), Some(Value::Int(7)));
}

#[test]
fn moduleNamespaceCallsBindVariadicParameters() {
    let executor = Executor::new();
    let mut env = Environment::new();
    let params = vec![
        Parameter {
            name: "first".to_string(),
            ty: Some(Type {
                name: "int".to_string(),
            }),
            default: None,
            variadic: false,
        },
        Parameter {
            name: "rest".to_string(),
            ty: Some(Type {
                name: "[int]".to_string(),
            }),
            default: None,
            variadic: true,
        },
    ];
    let sum = function(
        "sum",
        params,
        vec![Stmt::Return(Some(add(
            add(
                add(
                    variable("first"),
                    Expr::Index {
                        target: Box::new(variable("rest")),
                        index: Box::new(int(0)),
                        span: span(),
                    },
                ),
                Expr::Index {
                    target: Box::new(variable("rest")),
                    index: Box::new(int(1)),
                    span: span(),
                },
            ),
            Expr::Index {
                target: Box::new(variable("rest")),
                index: Box::new(int(2)),
                span: span(),
            },
        )))],
    );
    let mut members = HashMap::new();
    members.insert(b"sum".to_vec(), sum.clone());
    env.define("sum".to_string(), sum);
    env.define(
        "math".to_string(),
        Value::Namespace(Rc::new(RefCell::new(members))),
    );
    let namespace_call = Expr::MethodCall {
        receiver: Box::new(variable("math")),
        name: "sum".to_string(),
        args: vec![int(1), int(2), int(3), int(4)],
        span: span(),
    };
    assert_eq!(
        executor.evaluate_expr(&namespace_call, &mut env),
        Ok(Value::Int(10))
    );
    assert_eq!(
        executor.evaluate_expr(&call("sum", vec![int(1), int(2), int(3), int(4)]), &mut env),
        Ok(Value::Int(10))
    );
}

#[test]
fn updateSupportsVariablesFieldsAndIndexesWithCorrectResultOrder() {
    let executor = Executor::new();
    let mut env = Environment::new();
    env.define("x".to_string(), Value::Int(4));
    assert_eq!(
        executor
            .evaluate_expr(&increment(variable("x"), false), &mut env)
            .unwrap(),
        Value::Int(4)
    );
    assert_eq!(env.get("x"), Some(Value::Int(5)));
    assert_eq!(
        executor
            .evaluate_expr(&increment(variable("x"), true), &mut env)
            .unwrap(),
        Value::Int(6)
    );
    assert_eq!(env.get("x"), Some(Value::Int(6)));

    env.register_struct(StructDef {
        name: "Counter".to_string(),
        fields: vec![FieldDecl {
            name: "n".to_string(),
            ty: Type {
                name: "int".to_string(),
            },
            is_pub: true,
        }],
        methods: vec![],
    });
    let mut fields = HashMap::new();
    fields.insert("n".to_string(), Value::Int(10));
    env.define(
        "s".to_string(),
        Value::StructInstance {
            struct_name: SharedStr::from("Counter"),
            fields: Rc::new(RefCell::new(fields)),
        },
    );
    let field = Expr::Get {
        object: Box::new(variable("s")),
        name: "n".to_string(),
        span: span(),
    };
    assert_eq!(
        executor
            .evaluate_expr(&increment(field.clone(), false), &mut env)
            .unwrap(),
        Value::Int(10)
    );
    assert_eq!(executor.evaluate_expr(&field, &mut env).unwrap(), Value::Int(11));

    env.define(
        "v".to_string(),
        Value::Array(Rc::new(RefCell::new(vec![Value::Int(20)]))),
    );
    let index = Expr::Index {
        target: Box::new(variable("v")),
        index: Box::new(int(0)),
        span: span(),
    };
    assert_eq!(
        executor
            .evaluate_expr(&increment(index.clone(), true), &mut env)
            .unwrap(),
        Value::Int(21)
    );
    assert_eq!(executor.evaluate_expr(&index, &mut env).unwrap(), Value::Int(21));
}

#[test]
fn privateStructFieldsRemainImmutableAtRuntime() {
    let executor = Executor::new();
    let mut env = Environment::new();
    env.register_struct(StructDef {
        name: "Profile".to_string(),
        fields: vec![FieldDecl {
            name: "age".to_string(),
            ty: Type {
                name: "int".to_string(),
            },
            is_pub: false,
        }],
        methods: vec![],
    });
    let mut fields = HashMap::new();
    fields.insert("age".to_string(), Value::Int(42));
    env.define(
        "profile".to_string(),
        Value::StructInstance {
            struct_name: SharedStr::from("Profile"),
            fields: Rc::new(RefCell::new(fields)),
        },
    );
    let assignment = Expr::Set {
        object: Box::new(variable("profile")),
        name: "age".to_string(),
        value: Box::new(int(43)),
        span: span(),
    };
    assert!(executor
        .evaluate_expr(&assignment, &mut env)
        .unwrap_err()
        .contains("cannot mutate private field 'age'"));
}

#[test]
fn updateEvaluatesIndexTargetAndIndexExactlyOnce() {
    let executor = Executor::new();
    let mut env = Environment::new();
    env.define("targetCalls".to_string(), Value::Int(0));
    env.define("indexCalls".to_string(), Value::Int(0));
    env.define(
        "values".to_string(),
        Value::Array(Rc::new(RefCell::new(vec![Value::Int(30)]))),
    );
    for (name, counter, result) in [
        ("getValues", "targetCalls", variable("values")),
        ("getIndex", "indexCalls", int(0)),
    ] {
        env.define(
            name.to_string(),
            function(
                name,
                vec![],
                vec![
                    Stmt::ExprStmt(Expr::Assign {
                        name: counter.to_string(),
                        value: Box::new(add(variable(counter), int(1))),
                        span: span(),
                    }),
                    Stmt::Return(Some(result)),
                ],
            ),
        );
    }
    let target = Expr::Index {
        target: Box::new(call("getValues", vec![])),
        index: Box::new(call("getIndex", vec![])),
        span: span(),
    };
    assert_eq!(
        executor
            .evaluate_expr(&increment(target, false), &mut env)
            .unwrap(),
        Value::Int(30)
    );
    assert_eq!(env.get("targetCalls"), Some(Value::Int(1)));
    assert_eq!(env.get("indexCalls"), Some(Value::Int(1)));

    env.register_struct(StructDef {
        name: "Counter".to_string(),
        fields: vec![FieldDecl {
            name: "n".to_string(),
            ty: Type {
                name: "int".to_string(),
            },
            is_pub: true,
        }],
        methods: vec![],
    });
    let mut fields = HashMap::new();
    fields.insert("n".to_string(), Value::Int(40));
    env.define(
        "record".to_string(),
        Value::StructInstance {
            struct_name: SharedStr::from("Counter"),
            fields: Rc::new(RefCell::new(fields)),
        },
    );
    env.define(
        "getRecord".to_string(),
        function(
            "getRecord",
            vec![],
            vec![
                Stmt::ExprStmt(Expr::Assign {
                    name: "targetCalls".to_string(),
                    value: Box::new(add(variable("targetCalls"), int(1))),
                    span: span(),
                }),
                Stmt::Return(Some(variable("record"))),
            ],
        ),
    );
    let field = Expr::Get {
        object: Box::new(call("getRecord", vec![])),
        name: "n".to_string(),
        span: span(),
    };
    assert_eq!(
        executor
            .evaluate_expr(&increment(field, false), &mut env)
            .unwrap(),
        Value::Int(40)
    );
    assert_eq!(env.get("targetCalls"), Some(Value::Int(2)));
    let field_value = match env.get("record").unwrap() {
        Value::StructInstance { fields, .. } => fields.borrow().get("n").cloned(),
        _ => None,
    };
    assert_eq!(field_value, Some(Value::Int(41)));
}

#[test]
fn integerDivisionRemainderAndCharacterRuntimeSemantics() {
    let executor = Executor::new();
    let mut env = Environment::new();
    let binary = |left, op, right| Expr::Binary {
        left: Box::new(left),
        op,
        right: Box::new(right),
        span: span(),
    };
    assert_eq!(
        executor
            .evaluate_expr(&binary(int(-7), TokenKind::Slash, int(3)), &mut env)
            .unwrap(),
        Value::Int(-2)
    );
    assert_eq!(
        executor
            .evaluate_expr(&binary(int(-7), TokenKind::Percent, int(3)), &mut env)
            .unwrap(),
        Value::Int(-1)
    );
    assert_eq!(
        executor
            .evaluate_expr(&binary(int(7), TokenKind::Percent, int(-3)), &mut env)
            .unwrap(),
        Value::Int(1)
    );

    let indexed = Expr::Index {
        target: Box::new(Expr::Literal {
            value: Literal::String("AZ".to_string()),
            span: span(),
        }),
        index: Box::new(int(1)),
        span: span(),
    };
    assert_eq!(
        executor.evaluate_expr(&indexed, &mut env).unwrap(),
        Value::Char(b'Z')
    );
    assert_eq!(
        executor
            .evaluate_expr(
                &binary(
                    Expr::Literal {
                        value: Literal::Char('Z'),
                        span: span(),
                    },
                    TokenKind::EqualEqual,
                    Expr::Literal {
                        value: Literal::Char('Z'),
                        span: span(),
                    },
                ),
                &mut env,
            )
            .unwrap(),
        Value::Bool(true)
    );
    let to_int = Expr::Call {
        callee: Box::new(variable("int")),
        args: vec![Expr::Literal {
            value: Literal::Char('A'),
            span: span(),
        }],
        span: span(),
    };
    assert_eq!(executor.evaluate_expr(&to_int, &mut env), Ok(Value::Int(65)));
    let to_char = Expr::Call {
        callee: Box::new(variable("char")),
        args: vec![int(66)],
        span: span(),
    };
    assert_eq!(executor.evaluate_expr(&to_char, &mut env), Ok(Value::Char(b'B')));

    for (method, expected) in [("toUpper", "AZ"), ("toLower", "az")] {
        let expression = Expr::MethodCall {
            receiver: Box::new(Expr::Literal {
                value: Literal::String("Az".to_string()),
                span: span(),
            }),
            name: method.to_string(),
            args: vec![],
            span: span(),
        };
        assert_eq!(
            executor.evaluate_expr(&expression, &mut env).unwrap(),
            Value::utf8(expected)
        );
    }
}

#[test]
fn byteStringsIndexMeasureCompareAndConcatenateRawBytes() {
    let executor = Executor::new();
    let mut env = Environment::new();
    let raw = Expr::Literal {
        value: Literal::ByteString(vec![0, 0x80, 0xff]),
        span: span(),
    };
    let value = executor.evaluate_expr(&raw, &mut env).unwrap();
    assert_eq!(value.stringBytes(), Some(&[0, 0x80, 0xff][..]));

    let length = Expr::MethodCall {
        receiver: Box::new(raw.clone()),
        name: "len".to_string(),
        args: vec![],
        span: span(),
    };
    assert_eq!(executor.evaluate_expr(&length, &mut env), Ok(Value::Int(3)));
    let index = Expr::Index {
        target: Box::new(raw.clone()),
        index: Box::new(int(1)),
        span: span(),
    };
    assert_eq!(executor.evaluate_expr(&index, &mut env), Ok(Value::Char(0x80)));

    let ascii_case = Expr::MethodCall {
        receiver: Box::new(Expr::Literal {
            value: Literal::ByteString(vec![0x80, b'a', 0xff]),
            span: span(),
        }),
        name: "toUpper".to_string(),
        args: vec![],
        span: span(),
    };
    assert_eq!(
        executor.evaluate_expr(&ascii_case, &mut env).unwrap().stringBytes(),
        Some(&[0x80, b'A', 0xff][..])
    );

    for (method, args, expected) in [
        (
            "contains",
            vec![Expr::Literal {
                value: Literal::ByteString(vec![0x80]),
                span: span(),
            }],
            Value::Bool(true),
        ),
        (
            "startsWith",
            vec![Expr::Literal {
                value: Literal::ByteString(vec![0]),
                span: span(),
            }],
            Value::Bool(true),
        ),
        (
            "endsWith",
            vec![Expr::Literal {
                value: Literal::ByteString(vec![0xff]),
                span: span(),
            }],
            Value::Bool(true),
        ),
    ] {
        let method_call = Expr::MethodCall {
            receiver: Box::new(raw.clone()),
            name: method.to_string(),
            args,
            span: span(),
        };
        assert_eq!(executor.evaluate_expr(&method_call, &mut env), Ok(expected));
    }
    let replaced = Expr::MethodCall {
        receiver: Box::new(raw.clone()),
        name: "replace".to_string(),
        args: vec![
            Expr::Literal {
                value: Literal::ByteString(vec![0x80]),
                span: span(),
            },
            Expr::Literal {
                value: Literal::ByteString(vec![0x81, 0x82]),
                span: span(),
            },
        ],
        span: span(),
    };
    assert_eq!(
        executor.evaluate_expr(&replaced, &mut env).unwrap().stringBytes(),
        Some(&[0, 0x81, 0x82, 0xff][..])
    );
    let split = Expr::MethodCall {
        receiver: Box::new(raw.clone()),
        name: "split".to_string(),
        args: vec![Expr::Literal {
            value: Literal::ByteString(vec![0x80]),
            span: span(),
        }],
        span: span(),
    };
    let split = executor.evaluate_expr(&split, &mut env).unwrap();
    match split {
        Value::Array(parts) => {
            assert_eq!(parts.borrow()[0].stringBytes(), Some(&[0][..]));
            assert_eq!(parts.borrow()[1].stringBytes(), Some(&[0xff][..]));
        }
        other => panic!("expected byte-string array, got {}", other),
    }
    for (name, args, expected) in [
        ("slice", vec![int(1), int(3)], Value::fromStringBytes(vec![0x80, 0xff])),
        (
            "indexOf",
            vec![Expr::Literal {
                value: Literal::ByteString(vec![0x80]),
                span: span(),
            }],
            Value::Int(1),
        ),
    ] {
        let expression = Expr::MethodCall {
            receiver: Box::new(raw.clone()),
            name: name.to_string(),
            args,
            span: span(),
        };
        assert_eq!(executor.evaluate_expr(&expression, &mut env), Ok(expected));
    }

    let concat = Expr::Binary {
        left: Box::new(Expr::Literal {
            value: Literal::String("é".to_string()),
            span: span(),
        }),
        op: TokenKind::Plus,
        right: Box::new(Expr::Literal {
            value: Literal::ByteString(vec![0xff]),
            span: span(),
        }),
        span: Span::new(10, 11),
    };
    assert_eq!(
        executor.evaluate_expr(&concat, &mut env).unwrap().stringBytes(),
        Some(&[0xc3, 0xa9, 0xff][..])
    );
    let char_roundtrip = call(
        "string",
        vec![call("char", vec![int(255)])],
    );
    assert_eq!(
        executor.evaluate_expr(&char_roundtrip, &mut env).unwrap().stringBytes(),
        Some(&[0xff][..])
    );
    let interpolation = Expr::Binary {
        left: Box::new(Expr::Literal {
            value: Literal::String("raw=".to_string()),
            span: span(),
        }),
        op: TokenKind::Plus,
        right: Box::new(Expr::Literal {
            value: Literal::ByteString(vec![0xff]),
            span: span(),
        }),
        span: Span::new(0, 0),
    };
    assert_eq!(
        executor.evaluate_expr(&interpolation, &mut env).unwrap().stringBytes(),
        Some(&[b'r', b'a', b'w', b'=', 0xff][..])
    );

    let decoded_once = Expr::Literal {
        value: Literal::ByteString(vec![0xc3, 0xa9]),
        span: span(),
    };
    let decoded_once = executor.evaluate_expr(&decoded_once, &mut env).unwrap();
    assert!(matches!(&decoded_once, Value::String(text) if text == "é"));
    assert_eq!(decoded_once.stringBytes(), Some(&[0xc3, 0xa9][..]));
    let equal_bytes = Expr::Binary {
        left: Box::new(Expr::Literal {
            value: Literal::ByteString(vec![0xc3, 0xa9]),
            span: span(),
        }),
        op: TokenKind::EqualEqual,
        right: Box::new(Expr::Literal {
            value: Literal::String("é".to_string()),
            span: span(),
        }),
        span: Span::new(10, 11),
    };
    assert_eq!(executor.evaluate_expr(&equal_bytes, &mut env), Ok(Value::Bool(true)));
    let ordered_bytes = Expr::Binary {
        left: Box::new(Expr::Literal {
            value: Literal::ByteString(vec![0xff]),
            span: span(),
        }),
        op: TokenKind::Greater,
        right: Box::new(Expr::Literal {
            value: Literal::ByteString(vec![0x80]),
            span: span(),
        }),
        span: Span::new(10, 11),
    };
    assert_eq!(executor.evaluate_expr(&ordered_bytes, &mut env), Ok(Value::Bool(true)));

    let byte_dict = Expr::DictByteLiteral {
        entries: vec![(vec![0x80], int(1)), (vec![0xc2, 0x80], int(2))],
        span: span(),
    };
    for (key, expected) in [(vec![0x80], 1), (vec![0xc2, 0x80], 2)] {
        let get = Expr::Index {
            target: Box::new(byte_dict.clone()),
            index: Box::new(Expr::Literal {
                value: Literal::ByteString(key),
                span: span(),
            }),
            span: span(),
        };
        assert_eq!(executor.evaluate_expr(&get, &mut env), Ok(Value::Int(expected)));
    }
}

#[test]
fn dataDictionariesKeepAliasIdentityAndBuiltinLength() {
    let executor = Executor::new();
    let mut env = Environment::new();
    let literal = Expr::DictLiteral {
        entries: vec![("x".to_string(), int(1))],
        span: span(),
    };
    let table = executor.evaluate_expr(&literal, &mut env).unwrap();
    env.define("table".to_string(), table.clone());
    env.define("alias".to_string(), table);
    let same = Expr::Binary {
        left: Box::new(variable("table")),
        op: TokenKind::EqualEqual,
        right: Box::new(variable("alias")),
        span: span(),
    };
    assert_eq!(executor.evaluate_expr(&same, &mut env), Ok(Value::Bool(true)));

    let separate = executor.evaluate_expr(&literal, &mut env).unwrap();
    env.define("separate".to_string(), separate);
    let different = Expr::Binary {
        left: Box::new(variable("table")),
        op: TokenKind::EqualEqual,
        right: Box::new(variable("separate")),
        span: span(),
    };
    assert_eq!(
        executor.evaluate_expr(&different, &mut env),
        Ok(Value::Bool(false))
    );

    executor
        .evaluate_expr(
            &Expr::IndexSet {
                target: Box::new(variable("alias")),
                index: Box::new(Expr::Literal {
                    value: Literal::String("x".to_string()),
                    span: span(),
                }),
                value: Box::new(int(8)),
                span: span(),
            },
            &mut env,
        )
        .unwrap();
    let value = Expr::Index {
        target: Box::new(variable("table")),
        index: Box::new(Expr::Literal {
            value: Literal::String("x".to_string()),
            span: span(),
        }),
        span: span(),
    };
    assert_eq!(executor.evaluate_expr(&value, &mut env), Ok(Value::Int(8)));
    let length = Expr::MethodCall {
        receiver: Box::new(variable("table")),
        name: "len".to_string(),
        args: vec![],
        span: span(),
    };
    assert_eq!(executor.evaluate_expr(&length, &mut env), Ok(Value::Int(1)));
    let dict_with_len_key = Expr::DictLiteral {
        entries: vec![("len".to_string(), int(99)), ("x".to_string(), int(1))],
        span: span(),
    };
    let plain = executor.evaluate_expr(&dict_with_len_key, &mut env).unwrap();
    env.define("plain".to_string(), plain);
    let length = Expr::MethodCall {
        receiver: Box::new(variable("plain")),
        name: "len".to_string(),
        args: vec![],
        span: span(),
    };
    assert_eq!(executor.evaluate_expr(&length, &mut env), Ok(Value::Int(2)));
}

#[test]
fn dictionaryLiteralsShareHashMapReceiverMethods() {
    let executor = Executor::new();
    let mut env = Environment::new();
    let values = executor
        .evaluate_expr(
            &Expr::DictLiteral {
                entries: vec![("k".to_string(), int(1))],
                span: span(),
            },
            &mut env,
        )
        .unwrap();
    env.define("values".to_string(), values);

    assert_eq!(executor.evaluate_expr(&method(variable("values"), "set", vec![Expr::Literal { value: Literal::String("k".to_string()), span: span() }, int(9)]), &mut env), Ok(Value::Null));
    assert_eq!(executor.evaluate_expr(&method(variable("values"), "get", vec![Expr::Literal { value: Literal::String("k".to_string()), span: span() }]), &mut env), Ok(Value::Int(9)));
    assert_eq!(executor.evaluate_expr(&method(variable("values"), "has", vec![Expr::Literal { value: Literal::String("k".to_string()), span: span() }]), &mut env), Ok(Value::Bool(true)));
    assert_eq!(executor.evaluate_expr(&method(variable("values"), "len", vec![]), &mut env), Ok(Value::Int(1)));
    assert_eq!(executor.evaluate_expr(&method(variable("values"), "delete", vec![Expr::Literal { value: Literal::String("k".to_string()), span: span() }]), &mut env), Ok(Value::Null));
    assert_eq!(executor.evaluate_expr(&method(variable("values"), "len", vec![]), &mut env), Ok(Value::Int(0)));
}

#[test]
fn signedIntegerOverflowReturnsRuntimeErrorsWithoutChangingLvalues() {
    let executor = Executor::new();
    let mut env = Environment::new();
    let binary = |left, op, right| Expr::Binary {
        left: Box::new(left),
        op,
        right: Box::new(right),
        span: Span::new(10, 11),
    };
    for (left, op, right) in [
        (i64::MAX, TokenKind::Plus, 1),
        (i64::MIN, TokenKind::Minus, 1),
        (i64::MAX, TokenKind::Star, 2),
        (i64::MIN, TokenKind::Slash, -1),
        (i64::MIN, TokenKind::Percent, -1),
    ] {
        assert!(executor
            .evaluate_expr(&binary(int(left), op, int(right)), &mut env)
            .unwrap_err()
            .contains("overflow"));
    }
    let negate_min = Expr::Unary {
        op: TokenKind::Minus,
        right: Box::new(int(i64::MIN)),
        span: Span::new(10, 11),
    };
    assert!(executor
        .evaluate_expr(&negate_min, &mut env)
        .unwrap_err()
        .contains("overflow"));
    assert!(executor
        .evaluate_expr(&call("abs", vec![int(i64::MIN)]), &mut env)
        .unwrap_err()
        .contains("overflow"));

    env.define("n".to_string(), Value::Int(i64::MAX));
    let compound = Expr::Assign {
        name: "n".to_string(),
        value: Box::new(add(variable("n"), int(1))),
        span: span(),
    };
    assert!(executor.evaluate_expr(&compound, &mut env).is_err());
    assert_eq!(env.get("n"), Some(Value::Int(i64::MAX)));
    let update = increment(variable("n"), false);
    assert!(executor.evaluate_expr(&update, &mut env).is_err());
    assert_eq!(env.get("n"), Some(Value::Int(i64::MAX)));
}

#[test]
fn floatToIntRejectsTheRoundedUpperBoundAndNonFiniteValues() {
    let executor = Executor::new();
    let mut env = Environment::new();
    let to_int = |value| Expr::MethodCall {
        receiver: Box::new(Expr::Literal {
            value: Literal::Float(value),
            span: span(),
        }),
        name: "toInt".to_string(),
        args: vec![],
        span: span(),
    };
    assert_eq!(
        executor.evaluate_expr(&to_int(-9_223_372_036_854_775_808.0), &mut env),
        Ok(Value::Int(i64::MIN))
    );
    for value in [9_223_372_036_854_775_808.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        assert!(executor
            .evaluate_expr(&to_int(value), &mut env)
            .unwrap_err()
            .contains("out of int range"));
    }
    for (text, expected) in [
        (b"12.5".to_vec(), Some(12)),
        (b"1_2.5".to_vec(), Some(12)),
        (b"12oops".to_vec(), None),
        (b"1__2".to_vec(), None),
    ] {
        let conversion = Expr::MethodCall {
            receiver: Box::new(Expr::Literal {
                value: Literal::ByteString(text),
                span: span(),
            }),
            name: "toInt".to_string(),
            args: vec![],
            span: span(),
        };
        match expected {
            Some(expected) => assert_eq!(
                executor.evaluate_expr(&conversion, &mut env),
                Ok(Value::Int(expected))
            ),
            None => assert!(executor.evaluate_expr(&conversion, &mut env).is_err()),
        }
    }
    let valid_float_text = Expr::MethodCall {
        receiver: Box::new(Expr::Literal {
            value: Literal::ByteString(b"1_2.5".to_vec()),
            span: span(),
        }),
        name: "toFloat".to_string(),
        args: vec![],
        span: span(),
    };
    assert_eq!(
        executor.evaluate_expr(&valid_float_text, &mut env),
        Ok(Value::Float(12.5))
    );
    let malformed = Expr::MethodCall {
        receiver: Box::new(Expr::Literal {
            value: Literal::ByteString(b"9223372036854775808".to_vec()),
            span: span(),
        }),
        name: "toInt".to_string(),
        args: vec![],
        span: span(),
    };
    assert!(executor
        .evaluate_expr(&malformed, &mut env)
        .unwrap_err()
        .contains("out of int range"));
}

#[test]
fn stringConcatenationRejectsMixedOperandsInBothOrders() {
    let executor = Executor::new();
    let mut env = Environment::new();
    let binary = |left, right| Expr::Binary {
        left: Box::new(left),
        op: TokenKind::Plus,
        right: Box::new(right),
        span: Span::new(10, 11),
    };
    let string = || Expr::Literal {
        value: Literal::String("a".to_string()),
        span: span(),
    };
    assert!(executor
        .evaluate_expr(&binary(string(), int(2)), &mut env)
        .is_err());
    assert!(executor
        .evaluate_expr(&binary(int(2), string()), &mut env)
        .is_err());
    assert_eq!(
        executor
            .evaluate_expr(&binary(string(), string()), &mut env)
            .unwrap(),
        Value::utf8("aa")
    );

    env.define("s".to_string(), Value::utf8("a"));
    let mixed_assignment = Expr::Assign {
        name: "s".to_string(),
        value: Box::new(binary(variable("s"), int(2))),
        span: span(),
    };
    assert!(executor
        .evaluate_expr(&mixed_assignment, &mut env)
        .is_err());
    assert_eq!(env.get("s"), Some(Value::utf8("a")));

    let converted = Expr::MethodCall {
        receiver: Box::new(int(2)),
        name: "toString".to_string(),
        args: vec![],
        span: span(),
    };
    assert_eq!(
        executor
            .evaluate_expr(&binary(string(), converted), &mut env)
            .unwrap(),
        Value::utf8("a2")
    );
}

#[test]
fn tupleDictAndLambdaCaptureReadsRemainAvailable() {
    let executor = Executor::new();
    let mut env = Environment::new();
    env.define("captured".to_string(), Value::Int(12));
    let tuple = Expr::Get {
        object: Box::new(Expr::TupleLiteral {
            elements: vec![int(5), int(6)],
            span: span(),
        }),
        name: "1".to_string(),
        span: span(),
    };
    assert_eq!(executor.evaluate_expr(&tuple, &mut env).unwrap(), Value::Int(6));

    let dict = Expr::Get {
        object: Box::new(Expr::DictLiteral {
            entries: vec![("key".to_string(), int(8))],
            span: span(),
        }),
        name: "key".to_string(),
        span: span(),
    };
    assert_eq!(executor.evaluate_expr(&dict, &mut env).unwrap(), Value::Int(8));

    let lambda = Expr::Lambda {
        params: vec![],
        body: Box::new(variable("captured")),
        span: span(),
    };
    let closure = executor.evaluate_expr(&lambda, &mut env).unwrap();
    env.define("reader".to_string(), closure);
    env.assign("captured", Value::Int(20)).unwrap();
    assert_eq!(
        executor.evaluate_expr(&call("reader", vec![]), &mut env),
        Ok(Value::Int(12))
    );
}

#[test]
fn lexicalBlocksAndNestedLocalsDoNotOverwriteOuterBindings() {
    let executor = Executor::new();
    let mut env = Environment::new();
    env.define("value".to_string(), Value::Int(10));
    env.define("inside".to_string(), Value::Int(0));

    executor
        .execute(
            &Stmt::If {
                condition: Expr::Literal {
                    value: Literal::Bool(true),
                    span: span(),
                },
                then_branch: vec![
                    Stmt::VarDecl {
                        is_const: false,
                        name: "value".to_string(),
                        ty: None,
                        initializer: int(20),
                    },
                    Stmt::ExprStmt(Expr::Assign {
                        name: "inside".to_string(),
                        value: Box::new(variable("value")),
                        span: span(),
                    }),
                ],
                else_branch: None,
            },
            &mut env,
        )
        .unwrap();

    assert_eq!(env.get("inside"), Some(Value::Int(20)));
    assert_eq!(env.get("value"), Some(Value::Int(10)));
}

#[test]
fn parameterShadowingIsLimitedToItsLexicalBlock() {
    let executor = Executor::new();
    let mut env = Environment::new();
    env.define("inside".to_string(), Value::Int(0));
    env.define("after".to_string(), Value::Int(0));
    let show = function(
        "show",
        vec![Parameter {
            name: "value".to_string(),
            ty: Some(Type {
                name: "int".to_string(),
            }),
            default: None,
            variadic: false,
        }],
        vec![
            Stmt::If {
                condition: Expr::Literal {
                    value: Literal::Bool(true),
                    span: span(),
                },
                then_branch: vec![
                    Stmt::VarDecl {
                        is_const: false,
                        name: "value".to_string(),
                        ty: None,
                        initializer: int(99),
                    },
                    Stmt::ExprStmt(Expr::Assign {
                        name: "inside".to_string(),
                        value: Box::new(variable("value")),
                        span: span(),
                    }),
                ],
                else_branch: None,
            },
            Stmt::ExprStmt(Expr::Assign {
                name: "after".to_string(),
                value: Box::new(variable("value")),
                span: span(),
            }),
            Stmt::Return(Some(variable("value"))),
        ],
    );
    env.define("show".to_string(), show);

    assert_eq!(
        executor.evaluate_expr(&call("show", vec![int(7)]), &mut env),
        Ok(Value::Int(7))
    );
    assert_eq!(env.get("inside"), Some(Value::Int(99)));
    assert_eq!(env.get("after"), Some(Value::Int(7)));
}

#[test]
fn booleanOperatorsShortCircuitAndEvaluateRequiredRightSideOnce() {
    let executor = Executor::new();
    let mut env = Environment::new();
    env.define("calls".to_string(), Value::Int(0));
    env.define(
        "boom".to_string(),
        function(
            "boom",
            vec![],
            vec![
                Stmt::ExprStmt(Expr::Assign {
                    name: "calls".to_string(),
                    value: Box::new(add(variable("calls"), int(1))),
                    span: span(),
                }),
                Stmt::Return(Some(Expr::Literal {
                    value: Literal::Bool(true),
                    span: span(),
                })),
            ],
        ),
    );
    let binary = |left, op, right| Expr::Binary {
        left: Box::new(left),
        op,
        right: Box::new(right),
        span: Span::new(10, 11),
    };

    assert_eq!(
        executor.evaluate_expr(
            &binary(
                Expr::Literal {
                    value: Literal::Bool(false),
                    span: span(),
                },
                TokenKind::AndAnd,
                call("boom", vec![]),
            ),
            &mut env,
        ),
        Ok(Value::Bool(false))
    );
    assert_eq!(
        executor.evaluate_expr(
            &binary(
                Expr::Literal {
                    value: Literal::Bool(true),
                    span: span(),
                },
                TokenKind::OrOr,
                call("boom", vec![]),
            ),
            &mut env,
        ),
        Ok(Value::Bool(true))
    );
    assert_eq!(env.get("calls"), Some(Value::Int(0)));
    assert_eq!(
        executor.evaluate_expr(
            &binary(
                Expr::Literal {
                    value: Literal::Bool(true),
                    span: span(),
                },
                TokenKind::AndAnd,
                call("boom", vec![]),
            ),
            &mut env,
        ),
        Ok(Value::Bool(true))
    );
    assert_eq!(env.get("calls"), Some(Value::Int(1)));
}

#[test]
fn integerRangePatternsIncludeBothEndpointsAndRetainFallback() {
    let executor = Executor::new();
    let run_match = |value| {
        let mut env = Environment::new();
        env.define("matched".to_string(), Value::Int(0));
        executor
            .execute(
                &Stmt::Match {
                    expr: int(value),
                    arms: vec![
                        crate::ast::stmt::MatchArm {
                            pattern: crate::ast::stmt::Pattern::Range {
                                start: Literal::Int(1),
                                end: Literal::Int(3),
                            },
                            body: vec![Stmt::ExprStmt(Expr::Assign {
                                name: "matched".to_string(),
                                value: Box::new(int(1)),
                                span: span(),
                            })],
                        },
                        crate::ast::stmt::MatchArm {
                            pattern: crate::ast::stmt::Pattern::Wildcard,
                            body: vec![Stmt::ExprStmt(Expr::Assign {
                                name: "matched".to_string(),
                                value: Box::new(int(2)),
                                span: span(),
                            })],
                        },
                    ],
                },
                &mut env,
            )
            .unwrap();
        env.get("matched").unwrap()
    };
    assert_eq!(run_match(1), Value::Int(1));
    assert_eq!(run_match(3), Value::Int(1));
    assert_eq!(run_match(4), Value::Int(2));
}

#[test]
fn payloadEnumRuntimeConstructsAndBindsPayloads() {
    let executor = Executor::new();
    let mut env = Environment::new();
    executor
        .execute(
            &Stmt::EnumDeclWithPayload {
                name: "ResultValue".to_string(),
                is_pub: false,
                variants: vec![
                    EnumVariantDecl {
                        name: "Success".to_string(),
                        payloads: vec![EnumPayloadDecl {
                            name: Some("value".to_string()),
                            ty: Type {
                                name: "int".to_string(),
                            },
                            span: span(),
                        }],
                        span: span(),
                    },
                    EnumVariantDecl {
                        name: "Failure".to_string(),
                        payloads: vec![],
                        span: span(),
                    },
                ],
            },
            &mut env,
        )
        .unwrap();
    env.define("matched".to_string(), Value::Int(0));
    let success = Expr::EnumVariant {
        enum_name: Some("ResultValue".to_string()),
        variant: "Success".to_string(),
        args: vec![int(7)],
        span: span(),
    };
    executor
        .execute(
            &Stmt::Match {
                expr: success,
                arms: vec![crate::ast::stmt::MatchArm {
                    pattern: crate::ast::stmt::Pattern::EnumVariantPayload {
                        enum_name: Some("ResultValue".to_string()),
                        variant: "Success".to_string(),
                        payloads: vec![crate::ast::stmt::Pattern::Variable(
                            "captured".to_string(),
                        )],
                    },
                    body: vec![Stmt::ExprStmt(Expr::Assign {
                        name: "matched".to_string(),
                        value: Box::new(variable("captured")),
                        span: span(),
                    })],
                }],
            },
            &mut env,
        )
        .unwrap();
    assert_eq!(env.get("matched"), Some(Value::Int(7)));
}

#[test]
fn lowercaseStringInstanceOfAndInterpolationConversionsWork() {
    let executor = Executor::new();
    let mut env = Environment::new();
    let is_string = Expr::InstanceOf {
        value: Box::new(Expr::Literal {
            value: Literal::String("rey".to_string()),
            span: span(),
        }),
        ty: Type {
            name: "string".to_string(),
        },
        span: span(),
    };
    assert_eq!(
        executor.evaluate_expr(&is_string, &mut env),
        Ok(Value::Bool(true))
    );
    let byte_len = Expr::MethodCall {
        receiver: Box::new(Expr::Literal {
            value: Literal::String("é".to_string()),
            span: span(),
        }),
        name: "len".to_string(),
        args: vec![],
        span: span(),
    };
    assert_eq!(executor.evaluate_expr(&byte_len, &mut env), Ok(Value::Int(2)));
    let ascii_case = Expr::MethodCall {
        receiver: Box::new(Expr::Literal {
            value: Literal::String("éA".to_string()),
            span: span(),
        }),
        name: "toUpper".to_string(),
        args: vec![],
        span: span(),
    };
    assert_eq!(
        executor.evaluate_expr(&ascii_case, &mut env),
        Ok(Value::utf8("éA"))
    );

    let interpolation = Expr::Binary {
        left: Box::new(Expr::Literal {
            value: Literal::String("value=".to_string()),
            span: span(),
        }),
        op: TokenKind::Plus,
        right: Box::new(Expr::Binary {
            left: Box::new(int(1)),
            op: TokenKind::Plus,
            right: Box::new(int(2)),
            span: Span::new(10, 11),
        }),
        span: Span::new(0, 0),
    };
    assert_eq!(
        executor.evaluate_expr(&interpolation, &mut env),
        Ok(Value::utf8("value=3"))
    );

    let source = [
        r#"println("hello " + "rey");"#,
        r#"println("value={1 + 2}");"#,
        r#"println("product={2 * 3}");"#,
        r#"println("concat=" + ("a" + "b"));"#,
    ]
    .join("\n");
    let mut lexer = crate::lexer::Lexer::new(&source);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.nextToken().unwrap();
        let done = token.kind == TokenKind::Eof;
        tokens.push(token);
        if done {
            break;
        }
    }
    let mut parser = crate::parser::Parser::new(tokens);
    let statements = parser.parse().unwrap();
    let mut outputs = Vec::new();
    for statement in &statements {
        let expression = match statement {
            Stmt::ExprStmt(Expr::Call { args, .. }) => &args[0],
            other => panic!("expected println expression, got {:?}", other),
        };
        outputs.push(executor.evaluate_expr(expression, &mut env).unwrap());
    }
    assert_eq!(
        outputs,
        vec![
            Value::utf8("hello rey"),
            Value::utf8("value=3"),
            Value::utf8("product=6"),
            Value::utf8("concat=ab"),
        ]
    );
}

#[test]
fn explicitScalarConversionsValidateTheirInputTypes() {
    let executor = Executor::new();
    let mut env = Environment::new();
    assert_eq!(
        executor.evaluate_expr(
            &call(
                "float",
                vec![int(3)],
            ),
            &mut env,
        ),
        Ok(Value::Float(3.0))
    );
    assert_eq!(
        executor.evaluate_expr(&call("string", vec![int(65)]), &mut env),
        Ok(Value::utf8("65"))
    );
    assert_eq!(
        executor.evaluate_expr(
            &call(
                "string",
                vec![Expr::Literal {
                    value: Literal::Bool(false),
                    span: span(),
                }],
            ),
            &mut env,
        ),
        Ok(Value::utf8("false"))
    );
    assert!(executor
        .evaluate_expr(&call("float", vec![Expr::Literal {
            value: Literal::String("3".to_string()),
            span: span(),
        }]), &mut env)
        .unwrap_err()
        .contains("float expects an int or float"));
    assert!(executor
        .evaluate_expr(&call("string", vec![Expr::ArrayLiteral {
            elements: vec![int(1)],
            span: span(),
        }]), &mut env)
        .unwrap_err()
        .contains("string expects a scalar value"));
    assert!(executor
        .evaluate_expr(&call("int", vec![int(65)]), &mut env)
        .unwrap_err()
        .contains("int expects a char"));
    assert!(executor
        .evaluate_expr(
            &call(
                "char",
                vec![Expr::Literal {
                    value: Literal::Char('A'),
                    span: span(),
                }],
            ),
            &mut env,
        )
        .unwrap_err()
        .contains("char expects an int"));
    assert!(executor
        .evaluate_expr(&call("char", vec![int(256)]), &mut env)
        .unwrap_err()
        .contains("range 0..=255"));
    assert!(executor
        .evaluate_expr(
            &call(
                "int",
                vec![Expr::Literal {
                    value: Literal::Char('Ā'),
                    span: span(),
                }],
            ),
            &mut env,
        )
        .unwrap_err()
        .contains("range 0..=255"));
    assert!(executor
        .evaluate_expr(
            &Expr::Literal {
                value: Literal::Char('Ā'),
                span: span(),
            },
            &mut env,
        )
        .unwrap_err()
        .contains("char literal out of range 0..=255"));
}

#[test]
fn resultUnwrapErrReturnsErrValueAndRejectsOk() {
    let executor = Executor::new();
    let mut env = Environment::new();
    env.define(
        "failed".to_string(),
        Value::Result(Rc::new(RefCell::new(Err(Value::utf8("unchanged error"))))),
    );
    let unwrap_err = Expr::MethodCall {
        receiver: Box::new(variable("failed")),
        name: "unwrapErr".to_string(),
        args: vec![],
        span: span(),
    };
    assert_eq!(
        executor.evaluate_expr(&unwrap_err, &mut env),
        Ok(Value::utf8("unchanged error"))
    );

    env.define(
        "succeeded".to_string(),
        Value::Result(Rc::new(RefCell::new(Ok(Value::Int(7))))),
    );
    let misuse = Expr::MethodCall {
        receiver: Box::new(variable("succeeded")),
        name: "unwrapErr".to_string(),
        args: vec![],
        span: span(),
    };
    assert!(executor
        .evaluate_expr(&misuse, &mut env)
        .unwrap_err()
        .contains("Cannot unwrapErr Ok: 7"));
}

#[test]
fn optionAndResultConstructorFormsCreateBuiltinValues() {
    let executor = Executor::new();
    let mut env = Environment::new();

    let some_forms = vec![
        enumVariant(Some("Option"), "Some", vec![int(5)]),
        staticCall("Option", "Some", vec![int(5)]),
        call("Option::Some", vec![int(5)]),
    ];
    for (index, constructor) in some_forms.into_iter().enumerate() {
        let value = executor.evaluate_expr(&constructor, &mut env).unwrap();
        assert!(matches!(&value, Value::Option(_)));
        let name = format!("some{}", index);
        env.define(name.clone(), value);
        assert_eq!(
            executor.evaluate_expr(&method(variable(&name), "isSome", vec![]), &mut env),
            Ok(Value::Bool(true))
        );
        assert_eq!(
            executor.evaluate_expr(&method(variable(&name), "unwrap", vec![]), &mut env),
            Ok(Value::Int(5))
        );
        assert_eq!(
            executor.evaluate_expr(&method(variable(&name), "unwrapOr", vec![int(9)]), &mut env),
            Ok(Value::Int(5))
        );
    }

    let none_forms = vec![
        enumVariant(Some("Option"), "None", vec![]),
        staticCall("Option", "None", vec![]),
        call("Option::None", vec![]),
    ];
    for (index, constructor) in none_forms.into_iter().enumerate() {
        let value = executor.evaluate_expr(&constructor, &mut env).unwrap();
        assert!(matches!(&value, Value::Option(option) if option.borrow().is_none()));
        let name = format!("none{}", index);
        env.define(name.clone(), value);
        assert_eq!(
            executor.evaluate_expr(&method(variable(&name), "isNone", vec![]), &mut env),
            Ok(Value::Bool(true))
        );
        assert_eq!(
            executor.evaluate_expr(&method(variable(&name), "unwrapOr", vec![int(9)]), &mut env),
            Ok(Value::Int(9))
        );
        assert!(executor
            .evaluate_expr(&method(variable(&name), "unwrap", vec![]), &mut env)
            .unwrap_err()
            .contains("Cannot unwrap None"));
    }

    let ok_forms = vec![
        enumVariant(Some("Result"), "Ok", vec![int(7)]),
        staticCall("Result", "Ok", vec![int(7)]),
        call("Result::Ok", vec![int(7)]),
    ];
    for (index, constructor) in ok_forms.into_iter().enumerate() {
        let value = executor.evaluate_expr(&constructor, &mut env).unwrap();
        assert!(matches!(&value, Value::Result(_)));
        let name = format!("ok{}", index);
        env.define(name.clone(), value);
        assert_eq!(
            executor.evaluate_expr(&method(variable(&name), "isOk", vec![]), &mut env),
            Ok(Value::Bool(true))
        );
        assert_eq!(
            executor.evaluate_expr(&method(variable(&name), "unwrap", vec![]), &mut env),
            Ok(Value::Int(7))
        );
        assert_eq!(
            executor.evaluate_expr(&method(variable(&name), "unwrapOr", vec![int(9)]), &mut env),
            Ok(Value::Int(7))
        );
        assert!(executor
            .evaluate_expr(&method(variable(&name), "unwrapErr", vec![]), &mut env)
            .unwrap_err()
            .contains("Cannot unwrapErr Ok: 7"));
    }

    let err_forms = vec![
        enumVariant(Some("Result"), "Err", vec![Expr::Literal {
            value: Literal::String("bad".to_string()),
            span: span(),
        }]),
        staticCall("Result", "Err", vec![Expr::Literal {
            value: Literal::String("bad".to_string()),
            span: span(),
        }]),
        call("Result::Err", vec![Expr::Literal {
            value: Literal::String("bad".to_string()),
            span: span(),
        }]),
    ];
    for (index, constructor) in err_forms.into_iter().enumerate() {
        let value = executor.evaluate_expr(&constructor, &mut env).unwrap();
        assert!(matches!(&value, Value::Result(result) if matches!(result.borrow().as_ref(), Err(Value::String(error)) if error == "bad")));
        let name = format!("err{}", index);
        env.define(name.clone(), value);
        assert_eq!(
            executor.evaluate_expr(&method(variable(&name), "isErr", vec![]), &mut env),
            Ok(Value::Bool(true))
        );
        assert_eq!(
            executor.evaluate_expr(&method(variable(&name), "unwrapErr", vec![]), &mut env),
            Ok(Value::utf8("bad"))
        );
        assert_eq!(
            executor.evaluate_expr(&method(variable(&name), "unwrapOr", vec![int(9)]), &mut env),
            Ok(Value::Int(9))
        );
        assert!(executor
            .evaluate_expr(&method(variable(&name), "unwrap", vec![]), &mut env)
            .unwrap_err()
            .contains("Cannot unwrap Err: bad"));
    }

    let byte_error = Expr::Literal {
        value: Literal::ByteString(vec![0x80, 0xff]),
        span: span(),
    };
    let byte_result = executor
        .evaluate_expr(&enumVariant(Some("Result"), "Err", vec![byte_error]), &mut env)
        .unwrap();
    env.define("byteErr".to_string(), byte_result);
    assert_eq!(
        executor.evaluate_expr(&method(variable("byteErr"), "unwrapErr", vec![]), &mut env),
        Ok(Value::ByteString(SharedBuf::new(vec![0x80, 0xff])))
    );

    let integer_error = executor
        .evaluate_expr(&enumVariant(Some("Result"), "Err", vec![int(42)]), &mut env)
        .unwrap();
    env.define("integerErr".to_string(), integer_error);
    assert_eq!(
        executor.evaluate_expr(&method(variable("integerErr"), "unwrapErr", vec![]), &mut env),
        Ok(Value::Int(42))
    );

    env.register_enum("Option".to_string(), vec!["Some".to_string()]);
    let user_variant = executor
        .evaluate_expr(&enumVariant(None, "Some", vec![int(11)]), &mut env)
        .unwrap();
    assert!(matches!(user_variant, Value::EnumVariantPayload { enum_name, variant, .. } if enum_name == "Option" && variant == "Some"));
}

#[test]
fn vecLiteralReverseMutatesAliasesAndJoinPreservesBytes() {
    let executor = Executor::new();
    let mut env = Environment::new();
    let literal = Expr::ArrayLiteral {
        elements: vec![
            Expr::Literal {
                value: Literal::String("red".to_string()),
                span: span(),
            },
            Expr::Literal {
                value: Literal::ByteString(vec![0xff]),
                span: span(),
            },
            Expr::Literal {
                value: Literal::String("blue".to_string()),
                span: span(),
            },
        ],
        span: span(),
    };
    let Value::Array(values) = executor.evaluate_expr(&literal, &mut env).unwrap() else {
        panic!("expected array-backed Vec literal");
    };
    env.define("values".to_string(), Value::Array(values.clone()));
    env.define("alias".to_string(), Value::Array(values.clone()));
    let Value::Array(alias) = env.get("alias").unwrap() else {
        panic!("expected alias to retain array representation");
    };
    assert!(Rc::ptr_eq(&values, &alias));

    assert_eq!(
        executor.evaluate_expr(&method(variable("alias"), "reverse", vec![]), &mut env),
        Ok(Value::Null)
    );
    assert!(matches!(
        values.borrow().as_slice(),
        [Value::String(blue), Value::ByteString(raw), Value::String(red)]
            if blue == "blue" && raw.asBytes() == [0xff] && red == "red"
    ));
    assert_eq!(
        executor.evaluate_expr(
            &method(
                variable("values"),
                "join",
                vec![Expr::Literal {
                    value: Literal::ByteString(vec![0x80]),
                    span: span(),
                }],
            ),
            &mut env,
        ),
        Ok(Value::ByteString(SharedBuf::new(vec![
            b'b', b'l', b'u', b'e', 0x80, 0xff, 0x80, b'r', b'e', b'd',
        ])))
    );
}

#[test]
fn nullEqualityAndInequalityAreComplements() {
    let executor = Executor::new();
    let mut env = Environment::new();
    let null = || Expr::Literal { value: Literal::Null, span: span() };
    let compare = |left, op, right| Expr::Binary {
        left: Box::new(left),
        op,
        right: Box::new(right),
        span: span(),
    };

    assert_eq!(executor.evaluate_expr(&compare(null(), TokenKind::EqualEqual, null()), &mut env), Ok(Value::Bool(true)));
    assert_eq!(executor.evaluate_expr(&compare(null(), TokenKind::NotEqual, null()), &mut env), Ok(Value::Bool(false)));
    assert_eq!(executor.evaluate_expr(&compare(null(), TokenKind::NotEqual, int(1)), &mut env), Ok(Value::Bool(true)));
    assert_eq!(executor.evaluate_expr(&compare(null(), TokenKind::EqualEqual, int(1)), &mut env), Ok(Value::Bool(false)));
}

#[test]
fn enumAndTupleEqualityUseTheirValueSemantics() {
    let executor = Executor::new();
    let mut env = Environment::new();
    env.register_enum("Kind".to_string(), vec!["String".to_string(), "Int".to_string()]);
    let compare = |left, op, right| Expr::Binary {
        left: Box::new(left),
        op,
        right: Box::new(right),
        span: span(),
    };

    assert_eq!(executor.evaluate_expr(&compare(enumVariant(Some("Kind"), "String", vec![]), TokenKind::EqualEqual, enumVariant(Some("Kind"), "String", vec![])), &mut env), Ok(Value::Bool(true)));
    assert_eq!(executor.evaluate_expr(&compare(enumVariant(Some("Kind"), "String", vec![]), TokenKind::NotEqual, enumVariant(Some("Kind"), "Int", vec![])), &mut env), Ok(Value::Bool(true)));
    let tuple = || Expr::TupleLiteral { elements: vec![int(1), int(2)], span: span() };
    assert_eq!(executor.evaluate_expr(&compare(tuple(), TokenKind::EqualEqual, tuple()), &mut env), Ok(Value::Bool(true)));

    let fields = Rc::new(RefCell::new(HashMap::new()));
    let struct_value = Value::StructInstance { struct_name: SharedStr::from("Box"), fields: fields.clone() };
    let enum_value = Value::EnumVariantPayload { enum_name: SharedStr::from("Data"), variant: SharedStr::from("Box"), payloads: vec![struct_value.clone()] };
    let enum_alias = Value::EnumVariantPayload { enum_name: SharedStr::from("Data"), variant: SharedStr::from("Box"), payloads: vec![struct_value] };
    let enum_distinct = Value::EnumVariantPayload {
        enum_name: SharedStr::from("Data"),
        variant: SharedStr::from("Box"),
        payloads: vec![Value::StructInstance { struct_name: SharedStr::from("Box"), fields: Rc::new(RefCell::new(HashMap::new())) }],
    };
    assert_eq!(enum_value, enum_alias);
    assert_ne!(enum_value, enum_distinct);
}

#[test]
fn variantOnlyPatternsIgnorePayloadsForQualifiedAndBareVariants() {
    let executor = Executor::new();
    let mut env = Environment::new();
    env.register_enum("E".to_string(), vec!["Item".to_string(), "Other".to_string()]);
    env.define(
        "value".to_string(),
        Value::EnumVariantPayload { enum_name: SharedStr::from("E"), variant: SharedStr::from("Item"), payloads: vec![Value::Int(9)] },
    );
    env.define("result".to_string(), Value::Int(0));
    let assignment = |value| Stmt::ExprStmt(Expr::Assign {
        name: "result".to_string(),
        value: Box::new(int(value)),
        span: span(),
    });

    let qualified = Stmt::Match {
        expr: variable("value"),
        arms: vec![
            crate::ast::stmt::MatchArm { pattern: crate::ast::Pattern::EnumVariant("E".to_string(), "Item".to_string()), body: vec![assignment(1)] },
            crate::ast::stmt::MatchArm { pattern: crate::ast::Pattern::EnumVariant("E".to_string(), "Other".to_string()), body: vec![assignment(2)] },
        ],
    };
    executor.execute(&qualified, &mut env).unwrap();
    assert_eq!(env.get("result"), Some(Value::Int(1)));

    let bare = Stmt::Match {
        expr: variable("value"),
        arms: vec![
            crate::ast::stmt::MatchArm { pattern: crate::ast::Pattern::Variable("Item".to_string()), body: vec![assignment(3)] },
            crate::ast::stmt::MatchArm { pattern: crate::ast::Pattern::Variable("Other".to_string()), body: vec![assignment(4)] },
        ],
    };
    executor.execute(&bare, &mut env).unwrap();
    assert_eq!(env.get("result"), Some(Value::Int(3)));
}

#[test]
fn stringUpdatePreservesAliasesCapturesAndByteIdentity() {
    let executor = Executor::new();
    let mut env = Environment::new();
    env.define("a".to_string(), Value::utf8("hi"));
    env.define("b".to_string(), env.get("a").unwrap());
    let assign = Expr::Assign {
        name: "a".to_string(),
        value: Box::new(add(
            variable("a"),
            Expr::Literal {
                value: Literal::String("!".to_string()),
                span: span(),
            },
        )),
        span: span(),
    };
    assert_eq!(
        executor.evaluate_expr(&assign, &mut env).unwrap(),
        Value::utf8("hi!")
    );
    assert_eq!(env.get("b"), Some(Value::utf8("hi")));

    env.define("s".to_string(), Value::utf8("snap"));
    let closure = Expr::Lambda {
        params: vec![],
        body: Box::new(variable("s")),
        span: span(),
    };
    let captured = executor.evaluate_expr(&closure, &mut env).unwrap();
    let assignS = Expr::Assign {
        name: "s".to_string(),
        value: Box::new(add(
            variable("s"),
            Expr::Literal {
                value: Literal::String("X".to_string()),
                span: span(),
            },
        )),
        span: span(),
    };
    executor.evaluate_expr(&assignS, &mut env).unwrap();
    env.define("readS".to_string(), captured);
    assert_eq!(
        executor.evaluate_expr(&call("readS", vec![]), &mut env).unwrap(),
        Value::utf8("snap")
    );
    assert_eq!(env.get("s"), Some(Value::utf8("snapX")));

    let overflow = add(int(i64::MAX), int(1));
    let err = executor.evaluate_expr(&overflow, &mut env).unwrap_err();
    assert!(err.contains("overflow"), "{err}");

    let joined = executor
        .evaluate_expr(
            &add(
                Expr::Literal {
                    value: Literal::ByteString(vec![0xff]),
                    span: span(),
                },
                Expr::Literal {
                    value: Literal::ByteString(vec![0x80]),
                    span: span(),
                },
            ),
            &mut env,
        )
        .unwrap();
    assert_eq!(joined.stringBytes(), Some(&[0xff, 0x80][..]));
    assert!(matches!(joined, Value::ByteString(_)));
}

#[test]
fn optionEqualityNullToStringArrayMapAndCreateDispatch() {
    let executor = Executor::new();
    let mut env = Environment::new();
    let compare = |left, op, right| Expr::Binary {
        left: Box::new(left),
        op,
        right: Box::new(right),
        span: span(),
    };
    assert_eq!(
        executor.evaluate_expr(
            &compare(
                enumVariant(Some("Option"), "Some", vec![int(7)]),
                TokenKind::EqualEqual,
                enumVariant(Some("Option"), "Some", vec![int(7)]),
            ),
            &mut env,
        ),
        Ok(Value::Bool(true))
    );
    assert_eq!(
        executor.evaluate_expr(
            &compare(
                enumVariant(Some("Option"), "None", vec![]),
                TokenKind::EqualEqual,
                enumVariant(Some("Option"), "None", vec![]),
            ),
            &mut env,
        ),
        Ok(Value::Bool(true))
    );
    assert_eq!(
        executor.evaluate_expr(
            &method(
                Expr::Literal {
                    value: Literal::Null,
                    span: span(),
                },
                "toString",
                vec![],
            ),
            &mut env,
        ),
        Ok(Value::utf8("null"))
    );

    let mapped = executor
        .evaluate_expr(
            &method(
                Expr::ArrayLiteral {
                    elements: vec![int(1), int(2), int(3), int(4)],
                    span: span(),
                },
                "map",
                vec![Expr::Lambda {
                    params: vec![Parameter {
                        name: "n".to_string(),
                        ty: None,
                        default: None,
                        variadic: false,
                    }],
                    body: Box::new(add(variable("n"), variable("n"))),
                    span: span(),
                }],
            ),
            &mut env,
        )
        .unwrap();
    match mapped {
        Value::Vec(values) => {
            assert_eq!(
                values.borrow().clone(),
                vec![Value::Int(2), Value::Int(4), Value::Int(6), Value::Int(8)]
            );
        }
        other => panic!("expected vec, got {other}"),
    }

    env.register_struct(StructDef {
        name: "Box".to_string(),
        fields: vec![],
        methods: vec![],
    });
    let create = Function::new(
        "create".to_string(),
        vec![Parameter {
            name: "n".to_string(),
            ty: None,
            default: None,
            variadic: false,
        }],
        vec![Stmt::Return(Some(variable("n")))],
        span(),
        Some(env.clone()),
    );
    env.define("create".to_string(), Value::function(create));
    assert_eq!(
        executor.evaluate_expr(&staticCall("Box", "create", vec![int(3)]), &mut env),
        Ok(Value::Int(3))
    );
}

#[test]
fn charOrderingIsUnsignedBytes() {
    let compare = |left: char, op: TokenKind, right: char| {
        Expr::Binary {
            left: Box::new(Expr::Literal {
                value: Literal::Char(left),
                span: span(),
            }),
            op,
            right: Box::new(Expr::Literal {
                value: Literal::Char(right),
                span: span(),
            }),
            span: span(),
        }
    };
    let mut env = Environment::new();
    let executor = Executor::new();
    assert_eq!(
        executor.evaluate_expr(&compare('a', TokenKind::Less, 'b'), &mut env),
        Ok(Value::Bool(true))
    );
    assert_eq!(
        executor.evaluate_expr(&compare('\u{00ff}', TokenKind::Greater, '\u{0000}'), &mut env),
        Ok(Value::Bool(true))
    );
    assert_eq!(
        executor.evaluate_expr(&compare('\u{00ff}', TokenKind::LessEqual, '\u{00ff}'), &mut env),
        Ok(Value::Bool(true))
    );
}
