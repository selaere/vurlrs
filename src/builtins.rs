use crate::parse::Expr;
use crate::run::{RunErrorKind as Error, State, Value};
use std::fmt::Write;

fn tonumber(expr: &Value) -> Result<f64, Error> {
    match &expr {
        Value::String(s) => s
            .parse::<f64>()
            .map_err(|_| Error::IsNotNumber(expr.clone())),
        Value::List(_) => Err(Error::IsNotNumber(expr.clone())),
        Value::Number(n) => Ok(*n),
        Value::Quoted(_) => panic!(),
    }
}

fn frombool(boole: bool) -> Value {
    Value::Number(boole as i32 as f64)
}

pub(crate) fn builtins(state: &mut State, name: &str, args: &[Value]) -> Result<Value, Error> {
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
        "sub" => match args {
            [x, y] => Value::Number(tonumber(x)? - tonumber(y)?),
            _ => return Err(Error::ValueError(2)),
        },
        "div" => match args {
            [x, y] => Value::Number(tonumber(x)? / tonumber(y)?),
            _ => return Err(Error::ValueError(2)),
        },
        "mod" => match args {
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
        "list" => Value::List(args.to_vec()),
        "len" => match args {
            [Value::List(l)] => Value::Number(l.len() as _),
            [l] => Value::Number(format!("{}", l).chars().count() as _),
            _ => return Err(Error::ValueError(1)),
        },
        "set" => match args {
            [Value::String(l), r] => {
                state.globals.insert(l.clone(), r.clone());
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
                .map_err(Error::IOError)?;
            Value::String(buffer)
        }
        "substr" => match args {
            [s, x, y] => {
                let (start, stop) = (tonumber(x)? as usize, tonumber(y)? as usize);
                Value::String(
                    format!("{}", s)
                        .chars()
                        .skip(start - 1)
                        .take(stop.saturating_sub(start - 1))
                        .collect(),
                )
            }
            _ => return Err(Error::ValueError(3)),
        },
        "index" => match args {
            [Value::List(l), i] => {
                let index = tonumber(i)? as usize + 1;
                l.get(index)
                    .ok_or_else(|| Error::IndexError {
                        index,
                        len: l.len(),
                    })?
                    .clone()
            }
            _ => return Err(Error::ValueError(2)),
        },
        "not" => match args {
            [x] => frombool(tonumber(x)? == 0f64),
            _ => return Err(Error::ValueError(2)),
        },
        "eq" => match args {
            [Value::List(l), Value::List(m)] => frombool(l == m),
            [Value::Number(x), Value::Number(y)] => frombool(x == y),
            [x, y] => frombool(format!("{}", x) == format!("{}", y)),
            _ => return Err(Error::ValueError(2)),
        },
        "lt" => match args {
            [x, y] => frombool(tonumber(x)? < tonumber(y)?),
            _ => return Err(Error::ValueError(2)),
        },
        "gt" => match args {
            [x, y] => frombool(tonumber(x)? > tonumber(y)?),
            _ => return Err(Error::ValueError(2)),
        },
        "lte" => match args {
            [x, y] => frombool(tonumber(x)? <= tonumber(y)?),
            _ => return Err(Error::ValueError(2)),
        },
        "gte" => match args {
            [x, y] => frombool(tonumber(x)? >= tonumber(y)?),
            _ => return Err(Error::ValueError(2)),
        },
        "or" => match args {
            [x, y] => frombool(tonumber(x)? != 0f64 || tonumber(y)? != 0f64),
            _ => return Err(Error::ValueError(2)),
        },
        "and" => match args {
            [x, y] => frombool(tonumber(x)? != 0f64 && tonumber(y)? != 0f64),
            _ => return Err(Error::ValueError(2)),
        },
        "if" | "while" => match args {
            [cond, Value::Quoted(Expr::CodeblockStart(end))] => {
                if tonumber(cond)? == 0f64 {
                    state.lineno = *end;
                }
                Value::default()
            }
            _ => return Err(Error::ValueError(1)),
        },
        "end" => match args {
            [Value::Quoted(Expr::CodeblockEnd(start, stmt))] if stmt == "while" => {
                state.lineno = start - 1;
                Value::default()
            }
            [Value::Quoted(Expr::CodeblockEnd(_, _))] => Value::default(),
            _ => return Err(Error::ValueError(0)),
        },
        "_ord" => match args {
            [x] => {
                let string = format!("{}", x);
                let mut iter = string.chars();
                let chr = iter
                    .next()
                    .ok_or_else(|| Error::OrdError(format!("{}", x)))?;
                if iter.next().is_some() {
                    return Err(Error::OrdError(format!("{}", x)));
                };
                Value::Number(chr as u32 as f64)
            }
            _ => return Err(Error::ValueError(1)),
        },
        "_chr" => match args {
            [x] => {
                let num = tonumber(x)? as u32;
                Value::String(
                    char::try_from(num)
                        .map_err(|_| Error::ChrError(num))?
                        .to_string(),
                )
            }
            _ => return Err(Error::ValueError(1)),
        },
        _ => return Err(Error::NotImplemented),
    })
}
