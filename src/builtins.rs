use crate::parse::Expr;
use crate::run::{RunErrorKind as Error, State, Value};
use std::fmt::Write;

fn tonumber(expr: &Value) -> Result<f64, Error> {
    match &expr {
        Value::String(s) => s
            .parse::<f64>()
            .map_err(|_| Error::IsNotNumber(expr.clone())),
        Value::List(_) => return Err(Error::IsNotNumber(expr.clone())),
        Value::Number(n) => Ok(*n),
        Value::Quoted(_) => panic!(),
    }
}

fn frombool(boole: bool) -> Value {
    Value::Number(boole as i32 as f64)
}

pub(crate) fn builtins(
    state: &mut State,
    name: &str,
    mut args: Vec<Value>,
) -> Result<Value, Error> {
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
        "sub" => match &args[..] {
            [x, y] => Value::Number(tonumber(x)? - tonumber(y)?),
            _ => return Err(Error::ValueError(2)),
        },
        "div" => match &args[..] {
            [x, y] => Value::Number(tonumber(x)? / tonumber(y)?),
            _ => return Err(Error::ValueError(2)),
        },
        "mod" => match &args[..] {
            [x, y] => Value::Number(tonumber(x)? % tonumber(y)?),
            _ => return Err(Error::ValueError(2)),
        },
        "join" => {
            let mut string = String::new();
            for item in args {
                write!(&mut string, "{}", item).unwrap();
            }
            Value::String(string)
        }
        "list" => Value::List(args),
        "len" => match &args[..] {
            [Value::List(l)] => Value::Number(l.len() as _),
            [l] => Value::Number(format!("{}", l).len() as _),
            _ => return Err(Error::ValueError(1)),
        },
        "set" => match &mut args[..] {
            [Value::String(l), r] => {
                state.globals.insert(std::mem::take(l), std::mem::take(r));
                Value::default()
            }
            _ => return Err(Error::ValueError(2)),
        },
        "print" => {
            for i in args {
                print!("{}", i);
            }
            println!();
            Value::default()
        }
        "input" => {
            let mut buffer = String::new();
            std::io::stdin()
                .read_line(&mut buffer)
                .map_err(|e| Error::IOError(e))?;
            Value::String(buffer)
        }
        "substr" => match &args[..] {
            [s, x, y] => {
                Value::String(format!("{}", s)[tonumber(x)? as _..=tonumber(y)? as _].to_string())
            }
            _ => return Err(Error::ValueError(3)),
        },
        "index" => match &args[..] {
            [Value::List(l), i] => {
                let index = tonumber(i)? as usize + 1;
                l.get(index)
                    .ok_or_else(|| Error::IndexError(index, l.len()))?
                    .clone()
            }
            _ => return Err(Error::ValueError(2)),
        },
        "not" => match &args[..] {
            [x] => frombool(tonumber(x)? == 0f64),
            _ => return Err(Error::ValueError(2)),
        },
        "eq" => match &args[..] {
            [Value::List(l), Value::List(m)] => frombool(l == m),
            [Value::Number(x), Value::Number(y)] => frombool(x == y),
            [x, y] => frombool(format!("{}", x) == format!("{}", y)),
            _ => return Err(Error::ValueError(2)),
        },
        "lt" => match &args[..] {
            [x, y] => frombool(tonumber(x)? < tonumber(y)?),
            _ => return Err(Error::ValueError(2)),
        },
        "gt" => match &args[..] {
            [x, y] => frombool(tonumber(x)? > tonumber(y)?),
            _ => return Err(Error::ValueError(2)),
        },
        "lte" => match &args[..] {
            [x, y] => frombool(tonumber(x)? <= tonumber(y)?),
            _ => return Err(Error::ValueError(2)),
        },
        "gte" => match &args[..] {
            [x, y] => frombool(tonumber(x)? >= tonumber(y)?),
            _ => return Err(Error::ValueError(2)),
        },
        "or" => match &args[..] {
            [x, y] => frombool(tonumber(x)? != 0f64 || tonumber(y)? != 0f64),
            _ => return Err(Error::ValueError(2)),
        },
        "and" => match &args[..] {
            [x, y] => frombool(tonumber(x)? != 0f64 && tonumber(y)? != 0f64),
            _ => return Err(Error::ValueError(2)),
        },
        "if" | "while" => match &args[..] {
            [cond, Value::Quoted(Expr::CodeblockStart(end))] => {
                if tonumber(&cond)? == 0f64 {
                    state.lineno = *end;
                }
                Value::default()
            }
            _ => return Err(Error::ValueError(1)),
        },
        "end" => match &args[..] {
            [Value::Quoted(Expr::CodeblockEnd(start, stmt))] if stmt == "while" => {
                state.lineno = start - 1;
                Value::default()
            }
            [Value::Quoted(Expr::CodeblockEnd(_, _))] => Value::default(),
            _ => return Err(Error::ValueError(0)),
        },
        _ => return Err(Error::NotImplemented),
    })
}
