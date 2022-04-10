use crate::parse::Expr;
use crate::run::{State, Value};
use std::fmt::Write;

fn tonumber(expr: &Value) -> Result<f64, String> {
    match &expr {
        Value::String(s) => s
            .parse::<f64>()
            .map_err(|_| format!("{} is not a number", s)),
        Value::List(_) => return Err(format!("list is not a number")),
        Value::Number(n) => Ok(*n),
        Value::Quoted(_) => return Err(format!("tried to convert quoted expr")),
    }
}

/*fn tostring(expr: &Value) -> String {
    format!("{}", expr)
}*/

fn frombool(boole: bool) -> Value {
    Value::Number(boole as i32 as f64)
}

pub(crate) fn builtins(state: &mut State, name: &str, args: Vec<Value>) -> Result<Value, String> {
    Ok(match name {
        "add" => Value::Number(
            args.iter()
                .map(tonumber)
                .reduce(|x, y| Ok(x? + y?))
                .unwrap_or(Ok(0f64))?, // returns 0 if argument list is empty
        ),
        "mul" => Value::Number(
            args.iter()
                .map(tonumber)
                .reduce(|x, y| Ok(x? * y?))
                .unwrap_or(Ok(1f64))?, // returns 1 if argument list is empty
        ),
        "sub" => match args[..] {
            [ref x, ref y] => Value::Number(tonumber(x)? - tonumber(y)?),
            _ => return Err("expected 2 arguments".to_string()),
        },
        "div" => match args[..] {
            [ref x, ref y] => Value::Number(tonumber(x)? / tonumber(y)?),
            _ => return Err("expected 2 arguments".to_string()),
        },
        "mod" => match args[..] {
            [ref x, ref y] => Value::Number(tonumber(x)? % tonumber(y)?),
            _ => return Err("expected 2 arguments".to_string()),
        },
        "join" => {
            let mut string = String::new();
            for item in args {
                write!(&mut string, "{}", item)
                    .map_err(|_| "error converting to string".to_string())?;
            }
            Value::String(string)
        }
        "list" => Value::List(args),
        "len" => match &args[..] {
            [Value::List(l)] => Value::Number(l.len() as _),
            [ref l] => Value::Number(format!("{}", l).len() as _),
            _ => return Err("expected one list".to_string()),
        },
        "set" => {
            if args.len() != 2 {
                return Err("expected 2 arguments".to_string());
            } else {
                let mut iter = args.into_iter();
                let (l, r) = (iter.next().unwrap(), iter.next().unwrap());
                if let Value::String(l) = l {
                    state.globals.insert(l, r);
                    Value::default()
                } else {
                    return Err("variable names must be strings".to_string());
                }
            }
        }
        "print" => {
            for i in args {
                print!("{}", i);
            }
            println!();
            Value::default()
        }
        "substr" => match args[..] {
            [ref s, ref x, ref y] => {
                Value::String(format!("{}", s)[tonumber(x)? as _..=tonumber(y)? as _].to_string())
            }
            _ => return Err("expected 3 arguments".to_string()),
        },
        "index" => match args[..] {
            [Value::List(ref l), ref i] => l
                .get(tonumber(i)? as usize + 1)
                .ok_or_else(|| {
                    format!(
                        "tried to get index number {} of a list of length {}",
                        i,
                        l.len()
                    )
                })?
                .clone(),
            _ => return Err("expected 2 arguments".to_string()),
        },
        "not" => match args[..] {
            [ref x] => frombool(tonumber(x)? == 0f64),
            _ => return Err("expected 2 arguments".to_string()),
        },
        "eq" => match args[..] {
            [Value::List(ref l), Value::List(ref m)] => frombool(l == m),
            [Value::Number(x), Value::Number(y)] => frombool(x == y),
            [ref x, ref y] => frombool(format!("{}", x) == format!("{}", y)),
            _ => return Err("expected 2 arguments".to_string()),
        },
        "lt" => match args[..] {
            [ref x, ref y] => frombool(tonumber(x)? < tonumber(y)?),
            _ => return Err("expected 2 arguments".to_string()),
        },
        "gt" => match args[..] {
            [ref x, ref y] => frombool(tonumber(x)? > tonumber(y)?),
            _ => return Err("expected 2 arguments".to_string()),
        },
        "lte" => match args[..] {
            [ref x, ref y] => frombool(tonumber(x)? <= tonumber(y)?),
            _ => return Err("expected 2 arguments".to_string()),
        },
        "gte" => match args[..] {
            [ref x, ref y] => frombool(tonumber(x)? >= tonumber(y)?),
            _ => return Err("expected 2 arguments".to_string()),
        },
        "if" | "while" => match args[..] {
            [ref cond, Value::Quoted(Expr::CodeblockStart(end))] => {
                if tonumber(&cond)? == 0f64 {
                    state.lineno = end;
                }
                Value::default()
            }
            _ => return Err("expected 1 argument".to_string()),
        },
        "end" => match args[..] {
            [Value::Quoted(Expr::CodeblockEnd(ref start, ref stmt))] if stmt == "while" => {
                state.lineno = start - 1;
                Value::default()
            }
            [Value::Quoted(Expr::CodeblockEnd(_, _))] => Value::default(),
            _ => return Err("expected 0 arguments".to_string()),
        },
        s => return Err(format!("{} not implemented", s)),
    })
}
