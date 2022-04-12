use crate::parse::Expr;
use crate::run::{self, Function, RunErrorKind as Error, State, Value};
use std::cell::RefCell;
use std::fmt::Write;
use std::rc::Rc;

fn tonumber(val: &Value) -> Result<f64, Error> {
    match &val {
        Value::String(s) => s
            .parse::<f64>()
            .map_err(|_| Error::IsNotNumber(val.clone())),
        Value::List(_) => Err(Error::IsNotNumber(val.clone())),
        Value::Number(n) => Ok(*n),
        Value::Quoted(_) => panic!(),
    }
}

fn tostr(val: &Value) -> Rc<str> {
    match val {
        Value::String(s) => Rc::clone(s),
        other => Rc::from(format!("{}", other).as_str()),
    }
}

fn frombool(boole: bool) -> Value {
    Value::Number(boole as i32 as f64)
}

fn toindex(val: &Value) -> Result<usize, Error> {
    (tonumber(val)? as usize)
        .checked_sub(1)
        .ok_or(Error::ZeroIndex)
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
            Value::String(Rc::from(string))
        }
        "list" => Value::List(Rc::from(RefCell::from(args.to_vec()))),
        "len" => match args {
            [Value::List(l)] => Value::Number(l.borrow().len() as _),
            [l] => Value::Number(tostr(l).chars().count() as _),
            _ => return Err(Error::ValueError(1)),
        },
        "set" => match args {
            [Value::String(l), r] => {
                if l.starts_with('%') {
                    state.locals.insert(Rc::clone(l), r.clone());
                } else {
                    state.globals.insert(Rc::clone(l), r.clone());
                }
                Value::default()
            }
            _ => return Err(Error::ValueError(2)),
        },
        "print" => {
            for (n, v) in args.iter().enumerate() {
                if n == args.len() - 1 {
                    println!("{}", v);
                } else {
                    print!("{} ", v);
                }
            }
            Value::default()
        }
        "input" => {
            let mut buffer = String::new();
            std::io::stdin()
                .read_line(&mut buffer)
                .map_err(Error::IOError)?;
            Value::String(Rc::from(buffer))
        }
        "substr" => match args {
            [s, x, y] => {
                let (start, stop) = (tonumber(x)? as usize, tonumber(y)? as usize);
                Value::String(Rc::from(
                    tostr(s)
                        .chars()
                        .skip(start - 1)
                        .take(stop.saturating_sub(start - 1))
                        .collect::<String>(),
                ))
            }
            _ => return Err(Error::ValueError(3)),
        },
        "not" => match args {
            [x] => frombool(tonumber(x)? == 0f64),
            _ => return Err(Error::ValueError(2)),
        },
        "eq" => frombool(match args {
            [Value::List(l), Value::List(m)] => l == m,
            [Value::Number(x), Value::Number(y)] => x == y,
            [x, y] => tostr(x) == tostr(y),
            _ => return Err(Error::ValueError(2)),
        }),
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
            [Value::Quoted(Expr::CodeblockEnd(_, stmt))] if stmt == "_func" => {
                return Err(Error::Return(Value::default()))
            }
            [v, Value::Quoted(Expr::CodeblockEnd(_, stmt))] if stmt == "_func" => {
                return Err(Error::Return(v.clone()))
            }
            [Value::Quoted(Expr::CodeblockEnd(start, stmt))] if stmt == "while" => {
                state.lineno = start - 1;
                Value::default()
            }
            [Value::Quoted(Expr::CodeblockEnd(_, _))] => Value::default(),
            _ => return Err(Error::ValueError(0)),
        },
        "_ord" => match args {
            [x] => {
                let string = tostr(x);
                let mut iter = string.chars();
                let chr = iter
                    .next()
                    .ok_or_else(|| Error::OrdError(Rc::clone(&string)))?;
                if iter.next().is_some() {
                    return Err(Error::OrdError(Rc::clone(&string)));
                };
                Value::Number(chr as u32 as f64)
            }
            _ => return Err(Error::ValueError(1)),
        },
        "_chr" => match args {
            [x] => {
                let num = tonumber(x)? as u32;
                Value::String(Rc::from(
                    char::try_from(num)
                        .map_err(|_| Error::ChrError(num))?
                        .to_string(),
                ))
            }
            _ => return Err(Error::ValueError(1)),
        },
        "index" => match args {
            [Value::List(l), i] => {
                let index = toindex(i)?;
                let list = l.borrow();
                list.get(index)
                    .ok_or_else(|| Error::IndexError(index, list.len()))?
                    .clone()
            }
            _ => return Err(Error::ValueError(2)),
        },
        "push" => match args {
            [Value::List(l), i] => {
                let mut borrow = l.borrow_mut();
                borrow.push(i.clone());
                Value::default()
            }
            _ => return Err(Error::ValueError(2)),
        },
        "pop" => match args {
            [Value::List(l)] => {
                let mut borrow = l.borrow_mut();
                borrow.pop().ok_or(Error::PopError)? // this is fine apparently??
            }
            _ => return Err(Error::ValueError(1)),
        },
        "insert" => match args {
            [Value::List(l), i, v] => {
                let mut borrow = l.borrow_mut();
                let index = borrow.len().min(toindex(i)?);
                borrow.insert(index, v.clone());
                Value::default()
            }
            _ => return Err(Error::ValueError(3)),
        },
        "remove" => match args {
            [Value::List(l), i] => {
                let mut borrow = l.borrow_mut();
                let len = borrow.len();
                let index = len.min(toindex(i)?);
                borrow.remove(index)
            }
            _ => return Err(Error::ValueError(2)),
        },
        "replace" => match args {
            [Value::List(l), i, v] => {
                let mut borrow = l.borrow_mut();
                let index = toindex(i)?;
                let len = borrow.len();
                *borrow.get_mut(index).ok_or(Error::IndexError(index, len))? = v.clone();
                Value::default()
            }
            _ => return Err(Error::ValueError(3)),
        },
        "_func" => {
            if args.len() <= 1 {
                return Err(Error::ValueError(2));
            }
            let name = &args[0];
            let arguments = &args[1..args.len() - 1]
                .iter()
                .map(tostr)
                .collect::<Vec<_>>();
            let arguments = arguments
                .first()
                .map_or(false, |x| !str::eq(x, "...")) // ?????
                .then(|| Rc::from(&arguments[..]));
            if (state.functions)
                .insert(
                    tostr(name),
                    Function {
                        lineno: state.lineno + 1,
                        arguments,
                    },
                )
                .is_some()
            {
                return Err(Error::FuncDefined(tostr(name)));
            }
            state.lineno = match args[args.len() - 1] {
                Value::Quoted(Expr::CodeblockStart(lineno)) => lineno,
                _ => return Err(Error::ValueError(1)),
            };
            Value::default()
        }
        name => {
            run::execute_function(state, name, args)?
        }
    })
}
