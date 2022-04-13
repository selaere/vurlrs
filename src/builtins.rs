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
        Value::Lineptr(_) => panic!(),
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
    (tonumber(val)?.floor() as usize)
        .checked_sub(1)
        .ok_or(Error::ZeroIndex)
}

fn tolist(val: &Value) -> Result<RefMut<Vec<Value>>, Error> {
    match val {
        Value::List(l) => Ok(l.borrow_mut()),
        _ => Err(Error::IsNotList(val.clone())),
    }
}

pub fn builtins<'a>(state: &'a mut State, name: &str, args: &'a [Value]) -> Result<Value, Error> {
    let mut args = args;

    // yes i have to specify how many arguments there are. i could use ${count} but that's unstable
    macro_rules! command {
        ($num:literal $($var:ident)* => $code:expr) => {
            match args {
                [$( $var ),*] => $code,
                _ => return Err(Error::ValueError($num)),
            }
        };
    }

    if let Some(Value::Lineptr(lineptr)) = args.last() {
        args = &args[..args.len() - 1];
        return Ok(match name {
            "end _func" | "end define" => match args {
                [] => return Err(Error::Return(Value::default())),
                [v] => return Err(Error::Return(v.clone())),
                _ => return Err(Error::ValueError(1)),
            },
            "end while" => command!(0 => {
                state.lineno = lineptr - 1;
                Value::default()
            }),
            "end if" => command!(0 => Value::default()),
            "if" | "while" => command!(1 cond => {
                if tonumber(cond)? == 0f64 {
                    state.lineno = *lineptr;
                }
                Value::default()
            }),
            "_func" => {
                if args.len() <= 1 {
                    return Err(Error::ValueError(1));
                }
                let name = &args[0];
                let arguments = &args[1..].iter().map(tostr).collect::<Vec<_>>();
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
                state.lineno = *lineptr;
                Value::default()
            }
            "define" => command!(1 name => {
                if (state.functions)
                    .insert(
                        Rc::from("call ".to_string() + &tostr(name)),
                        Function {
                            lineno: state.lineno + 1,
                            arguments: None,
                        },
                    ).is_some()
                {
                    return Err(Error::FuncDefined(tostr(name)));
                };
                state.lineno = *lineptr;
                Value::default()
            }),
            name => run::execute_function(state, name, args)?,
        });
    }

    Ok(match name {
        "set" => command!(2 l r => {
            let l = tostr(l);
            if l.starts_with('%') {
                state.locals.insert(l, r.clone());
            } else {
                state.globals.insert(l, r.clone());
            }
            Value::default()
        }),
        "_get" => command!(1 v => {
            let s = tostr(v);
            let var = if s.starts_with('%') {
                state.locals.get(s.as_ref())
            } else {
                state.globals.get(s.as_ref())
            };
            var.cloned().ok_or(Error::NameError(s))?
        }),
        "call" => command!(1 name => run::execute_function(
            state,
            &("call ".to_string() + &tostr(name)),
            &[],
        )?),
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
        "_pow" => command!(2 x y => Value::Number(tonumber(x)?.powf(tonumber(y)?))),
        "_floor" => command!(1 x => Value::Number(tonumber(x)?.floor())),
        "_sin" => command!(1 x => Value::Number(tonumber(x)?.sin())),
        "_cos" => command!(1 x => Value::Number(tonumber(x)?.cos())),
        "_tan" => command!(1 x => Value::Number(tonumber(x)?.tan())),
        "_asin" => command!(1 x => Value::Number(tonumber(x)?.asin())),
        "_acos" => command!(1 x => Value::Number(tonumber(x)?.acos())),
        "_atan" => command!(1 x => Value::Number(tonumber(x)?.atan())),
        "_ln" => command!(1 x => Value::Number(tonumber(x)?.ln())),
        "len" => command!(1 i => match i {
            Value::List(l) => Value::Number(l.borrow().len() as _),
            l => Value::Number(tostr(l).chars().count() as _),
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
            let (start, stop) = (toindex(x)?, toindex(y)? + 1);
            Value::String(Rc::from(
                tostr(s).chars()
                    .skip(start)
                    .take(stop.saturating_sub(start))
                    .collect::<String>(),
            ))
        }),
        "_chr" => command!(1 x => {
            let num = tonumber(x)?.floor() as u32;
            Value::String(Rc::from(
                char::try_from(num)
                    .map_err(|_| Error::ChrError(num))?
                    .to_string(),
            ))
        }),
        "_ord" => command!(1 x => {
            let string = tostr(x);
            let mut iter = string.chars();
            let chr = iter.next().ok_or_else(|| Error::OrdError(Rc::clone(&string)))?;
            if iter.next().is_some() {
                return Err(Error::OrdError(Rc::clone(&string)));
            };
            Value::Number(chr as u32 as f64)
        }),
        "join" => {
            let mut string = String::new();
            for item in args {
                write!(&mut string, "{}", item).unwrap();
            }
            Value::String(Rc::from(string))
        }
        "list" => Value::List(Rc::from(RefCell::from(args.to_vec()))),
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
        name => run::execute_function(state, name, args)?,
    })
}
