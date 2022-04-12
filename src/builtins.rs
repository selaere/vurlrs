use crate::parse::Expr;
use crate::run::{self, Function, RunErrorKind as Error, State, Value};
use std::cell::{RefCell, RefMut};
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

fn tolist(val: &Value) -> Result<RefMut<Vec<Value>>, Error> {
    match val {
        Value::List(l) => Ok(l.borrow_mut()),
        _ => Err(Error::IsNotList(val.clone())),
    }
}

pub fn builtins(state: &mut State, name: &str, args: &[Value]) -> Result<Value, Error> {
    // yes i have to specify how many arguments there are. i could use ${count} but that's unstable
    macro_rules! command {
        ($num:literal $($var:ident)* => $code:expr) => {
            match args {
                [$( $var ),*] => $code,
                _ => return Err(Error::ValueError($num)),
            }
        };
    }

    Ok(match name {
        "add" => Value::Number(
            args.iter()
                .map(tonumber)
                .reduce(|x, y| Ok(x? + y?))
                .unwrap_or(Ok(0f64))?,
        ),
        "mul" => Value::Number(
            args.iter()
                .map(tonumber)
                .reduce(|x, y| Ok(x? * y?))
                .unwrap_or(Ok(1f64))?,
        ),
        "sub" => command!(2 x y => Value::Number(tonumber(x)? - tonumber(y)?)),
        "div" => command!(2 x y => Value::Number(tonumber(x)? / tonumber(y)?)),
        "mod" => command!(2 x y => Value::Number(tonumber(x)? % tonumber(y)?)),
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
        "set" => command!(2 l r => {
            let l = tostr(l);
            if l.starts_with('%') {
                state.locals.insert(l, r.clone());
            } else {
                state.globals.insert(l, r.clone());
            }
            Value::default()
        }),
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
        "_printraw" => {
            for v in args.iter() {
                print!("{}", v)
            }
            Value::default()
        }
        "input" => command!(0 => {
            let mut buffer = String::new();
            std::io::stdin()
                .read_line(&mut buffer)
                .map_err(Error::IOError)?;
            Value::String(Rc::from(buffer))
        }),
        "substr" => command!(3 s x y => {
            let (start, stop) = (tonumber(x)? as usize, tonumber(y)? as usize);
            Value::String(Rc::from(
                tostr(s)
                    .chars()
                    .skip(start - 1)
                    .take(stop.saturating_sub(start - 1))
                    .collect::<String>(),
            ))
        }),
        "eq" => frombool(match args {
            [Value::List(l), Value::List(m)] => l == m,
            [Value::Number(x), Value::Number(y)] => x == y,
            [x, y] => tostr(x) == tostr(y),
            _ => return Err(Error::ValueError(2)),
        }),
        "not" => command!(1 x => frombool(tonumber(x)? == 0f64)),
        "lt" => command!(2 x y => frombool(tonumber(x)? < tonumber(y)?)),
        "gt" => command!(2 x y => frombool(tonumber(x)? > tonumber(y)?)),
        "lte" => command!(2 x y => frombool(tonumber(x)? <= tonumber(y)?)),
        "gte" => command!(2 x y => frombool(tonumber(x)? >= tonumber(y)?)),
        "or" => command!(2 x y => frombool(tonumber(x)? != 0f64 || tonumber(y)? != 0f64)),
        "and" => command!(2 x y => frombool(tonumber(x)? != 0f64 && tonumber(y)? != 0f64)),
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
            [Value::Quoted(Expr::CodeblockEnd(_, stmt))] if stmt == "_func" || stmt == "define" => {
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
        "_ord" => command!(1 x => {
            let string = tostr(x);
            let mut iter = string.chars();
            let chr = (iter.next())
                .ok_or_else(|| Error::OrdError(Rc::clone(&string)))?;
            if iter.next().is_some() {
                return Err(Error::OrdError(Rc::clone(&string)));
            };
            Value::Number(chr as u32 as f64)
        }),
        "_chr" => command!(1 x => {
            let num = tonumber(x)? as u32;
            Value::String(Rc::from(
                char::try_from(num)
                    .map_err(|_| Error::ChrError(num))?
                    .to_string(),
            ))
        }),
        "index" => command!(2 l i => {
            let list = tolist(l)?;
            let index = toindex(i)?;
            list.get(index)
                .ok_or_else(|| Error::IndexError(index, list.len()))?
                .clone()
        }),
        "push" => command!(2 l v => {
            let mut borrow = tolist(l)?;
            borrow.push(v.clone());
            Value::default()
        }),
        "pop" => command!(1 l => tolist(l)?.pop().ok_or(Error::PopError)?),
        "insert" => command!(3 l i v => {
            let mut borrow = tolist(l)?;
            let index = borrow.len().min(toindex(i)?);
            borrow.insert(index, v.clone());
            Value::default()
        }),
        "remove" => command!(2 l i => {
            let mut borrow = tolist(l)?;
            let len = borrow.len();
            let index = len.min(toindex(i)?);
            borrow.remove(index)
        }),
        "replace" => command!(3 l i v => {
            let mut borrow = tolist(l)?;
            let index = toindex(i)?;
            let len = borrow.len();
            *borrow.get_mut(index).ok_or(Error::IndexError(index, len))? = v.clone();
            Value::default()
        }),
        "_func" => {
            if args.len() <= 1 {
                return Err(Error::ValueError(1));
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
        "define" => match args {
            [name, Value::Quoted(Expr::CodeblockStart(lineno))] => {
                if (state.functions)
                    .insert(
                        Rc::from("call ".to_string() + &tostr(name)),
                        Function {
                            lineno: state.lineno + 1,
                            arguments: None,
                        },
                    )
                    .is_some()
                {
                    return Err(Error::FuncDefined(tostr(name)));
                };
                state.lineno = *lineno;
                Value::default()
            }
            _ => return Err(Error::ValueError(1)),
        },
        "call" => match args {
            [name] => run::execute_function(state, &("call ".to_string() + &tostr(name)), &[])?,
            _ => return Err(Error::ValueError(1)),
        },
        name => run::execute_function(state, name, args)?,
    })
}
