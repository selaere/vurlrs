use crate::run::{self, Function, RunErrorKind as Error, State, Value};
use std::cell::{RefCell, RefMut};
use std::fmt::Write;
use std::rc::Rc;
use Value::{Lineptr, List, Number, String as StringVal};

fn tonumber(val: &Value) -> Result<f64, Error> {
    match &val {
        StringVal(s) => s
            .parse::<f64>()
            .map_err(|_| Error::IsNotNumber(val.clone())),
        List(_) => Err(Error::IsNotNumber(val.clone())),
        Number(n) => Ok(*n),
        Lineptr(_) => panic!(),
    }
}

fn tostr(val: &Value) -> Rc<str> {
    match val {
        StringVal(s) => Rc::clone(s),
        other => Rc::from(format!("{}", other).as_str()),
    }
}

fn frombool(boole: bool) -> Value {
    Number(boole as i32 as f64)
}

fn toindex(val: &Value) -> Result<usize, Error> {
    (tonumber(val)?.floor() as usize)
        .checked_sub(1)
        .ok_or(Error::ZeroIndex)
}

fn tolist(val: &Value) -> Result<RefMut<Vec<Value>>, Error> {
    match val {
        List(l) => Ok(l.borrow_mut()),
        _ => Err(Error::IsNotList(val.clone())),
    }
}

fn eq(a: &Value, b: &Value) -> bool {
    match &[a, b] {
        [List(l), List(m)] => {
            let (l, m) = (l.borrow(), m.borrow());
            l.iter().zip(m.iter()).all(|(x, y)| eq(x, y))
        }
        [Number(x), Number(y)] => x == y,
        [x, y] => tostr(x) == tostr(y),
    }
}

pub fn builtins<'a>(state: &'a mut State, name: &str, args: &'a [Value]) -> Result<Value, Error> {
    let mut args = args;
    // a command with a fixed (as in, not variadic) number of arguments.
    macro_rules! fixed {
        ([$( $var:ident ),*] , $code:expr) => {
            match args {
                [$( $var ),*] => $code,
                _ => return Err(Error::ValueError(
                    <[&str]>::len(&[$(stringify!($var)),*])
                    // evil trick to get how many arguments there are. rust can probably
                    // optimize this away. i would use ${count} but that's unstable
                )),
            }
        };
    }

    if let Some(Lineptr(lineptr)) = args.last() {
        args = &args[..args.len() - 1];
        return Ok(match name {
            "end _func" | "end define" => match args {
                [] => return Err(Error::Return(Value::default())),
                [v] => return Err(Error::Return(v.clone())),
                _ => return Err(Error::ValueError(1)),
            },
            "end while" => fixed!([], {
                state.lineno = lineptr - 1;
                Value::default()
            }),
            "end if" => fixed!([], Value::default()),
            "if" | "while" => fixed!([cond], {
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
            "define" => fixed!([name], {
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
                state.lineno = *lineptr;
                Value::default()
            }),
            name => run::execute_function(state, name, args)?,
        });
    }

    Ok(match name {
        "add" => Number(
            args.iter()
                .map(tonumber)
                .reduce(|x, y| Ok(x? + y?))
                .unwrap_or(Ok(0f64))?,
        ),
        "mul" => Number(
            args.iter()
                .map(tonumber)
                .reduce(|x, y| Ok(x? * y?))
                .unwrap_or(Ok(1f64))?,
        ),
        "sub" => fixed!([x, y], Number(tonumber(x)? - tonumber(y)?)),
        "div" => fixed!([x, y], Number(tonumber(x)? / tonumber(y)?)),
        "mod" => fixed!([x, y], Number(tonumber(x)? % tonumber(y)?)),
        "_pow" => fixed!([x, y], Number(tonumber(x)?.powf(tonumber(y)?))),
        "_floor" => fixed!([x], Number(tonumber(x)?.floor())),
        "_round" => fixed!([x], Number(tonumber(x)?.round())),
        "_sqrt" => fixed!([x], Number(tonumber(x)?.sqrt())),
        "_sin" => fixed!([x], Number(tonumber(x)?.sin())),
        "_cos" => fixed!([x], Number(tonumber(x)?.cos())),
        "_tan" => fixed!([x], Number(tonumber(x)?.tan())),
        "_asin" => fixed!([x], Number(tonumber(x)?.asin())),
        "_acos" => fixed!([x], Number(tonumber(x)?.acos())),
        "_atan" => fixed!([x], Number(tonumber(x)?.atan())),
        "_ln" => fixed!([x], Number(tonumber(x)?.ln())),
        "_exp" => fixed!([x], Number(tonumber(x)?.exp())),
        "len" => fixed!([i], {
            match i {
                List(l) => Number(l.borrow().len() as _),
                l => Number(tostr(l).chars().count() as _),
            }
        }),
        "eq" => fixed!([x, y], frombool(eq(x, y))),
        "not" => fixed!([x], frombool(tonumber(x)? == 0f64)),
        "lt" => fixed!([x, y], frombool(tonumber(x)? < tonumber(y)?)),
        "gt" => fixed!([x, y], frombool(tonumber(x)? > tonumber(y)?)),
        "lte" => fixed!([x, y], frombool(tonumber(x)? <= tonumber(y)?)),
        "gte" => fixed!([x, y], frombool(tonumber(x)? >= tonumber(y)?)),
        "or" => fixed!([x, y], {
            frombool(tonumber(x)? != 0f64 || tonumber(y)? != 0f64)
        }),
        "and" => fixed!([x, y], {
            frombool(tonumber(x)? != 0f64 && tonumber(y)? != 0f64)
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
        "_printerr" => {
            for (n, v) in args.iter().enumerate() {
                if n == args.len() - 1 {
                    eprintln!("{}", v);
                } else {
                    eprint!("{} ", v);
                }
            }
            Value::default()
        }
        "_printerrraw" => {
            for v in args.iter() {
                eprint!("{}", v)
            }
            Value::default()
        }
        "input" => fixed!([], {
            let mut buffer = String::new();
            std::io::stdin()
                .read_line(&mut buffer)
                .map_err(Error::IOError)?;
            StringVal(Rc::from(buffer))
        }),
        "substr" => fixed!([s, x, y], {
            let (start, stop) = (toindex(x)?, toindex(y)? + 1);
            StringVal(Rc::from(
                tostr(s)
                    .chars()
                    .skip(start)
                    .take(stop.saturating_sub(start))
                    .collect::<String>(),
            ))
        }),
        "_chr" => fixed!([x], {
            let num = tonumber(x)?.floor() as u32;
            StringVal(Rc::from(
                char::try_from(num)
                    .map_err(|_| Error::ChrError(num))?
                    .to_string(),
            ))
        }),
        "_ord" => fixed!([x], {
            let string = tostr(x);
            let mut iter = string.chars();
            let chr = iter
                .next()
                .ok_or_else(|| Error::OrdError(Rc::clone(&string)))?;
            if iter.next().is_some() {
                return Err(Error::OrdError(Rc::clone(&string)));
            };
            Number(chr as u32 as f64)
        }),
        "join" => {
            let mut string = String::new();
            for item in args {
                write!(&mut string, "{}", item).unwrap();
            }
            StringVal(Rc::from(string))
        }
        "list" => List(Rc::from(RefCell::from(args.to_vec()))),
        "index" => fixed!([l, i], {
            let list = tolist(l)?;
            let index = toindex(i)?;
            list.get(index)
                .ok_or_else(|| Error::IndexError(index, list.len()))?
                .clone()
        }),
        "push" => fixed!([l, v], {
            let mut borrow = tolist(l)?;
            borrow.push(v.clone());
            Value::default()
        }),
        "pop" => fixed!([l], tolist(l)?.pop().ok_or(Error::PopError)?),
        "insert" => fixed!([l, i, v], {
            let mut borrow = tolist(l)?;
            let index = borrow.len().min(toindex(i)?);
            borrow.insert(index, v.clone());
            Value::default()
        }),
        "remove" => fixed!([l, i], {
            let mut borrow = tolist(l)?;
            let len = borrow.len();
            let index = len.min(toindex(i)?);
            borrow.remove(index)
        }),
        "replace" => fixed!([l, i, v], {
            let mut borrow = tolist(l)?;
            let index = toindex(i)?;
            let len = borrow.len();
            *borrow.get_mut(index).ok_or(Error::IndexError(index, len))? = v.clone();
            Value::default()
        }),
        "_islist" => fixed!([x], frombool(matches!(x, List(_)))),
        "_clone" => fixed!([x], {
            match x {
                List(l) => List(Rc::new(RefCell::new(l.borrow().clone()))),
                other => other.clone(),
            }
        }),
        "set" => fixed!([l, r], {
            let l = tostr(l);
            if l.starts_with('%') {
                state.locals.insert(l, r.clone());
            } else {
                state.globals.insert(l, r.clone());
            }
            Value::default()
        }),
        "_get" => fixed!([v], {
            let s = tostr(v);
            let var = if s.starts_with('%') {
                state.locals.get(s.as_ref())
            } else {
                state.globals.get(s.as_ref())
            };
            var.cloned().ok_or(Error::NameError(s))?
        }),
        "_globals" => fixed!([], {
            List(Rc::new(RefCell::new(
                (state.globals.keys())
                    .map(|x| StringVal(Rc::clone(x)))
                    .collect(),
            )))
        }),
        "_locals" => fixed!([], {
            List(Rc::new(RefCell::new(
                (state.locals.keys())
                    .map(|x| StringVal(Rc::clone(x)))
                    .collect(),
            )))
        }),
        "_error" => fixed!([e], return Err(Error::UserError(tostr(e)))),
        "call" => fixed!([name], {
            run::execute_function(state, &("call ".to_string() + &tostr(name)), &[])?
        }),
        "_apply" => fixed!([n, a], {
            run::execute_function(state, tostr(n).as_ref(), tolist(a)?.as_slice())?
        }),
        "_rand" => {
            #[cfg(feature = "fastrand")]
            let val = fixed!([], Ok(Number(fastrand::f64())));
            #[cfg(not(feature = "fastrand"))]
            let val = Err(Error::RandUnavailable);
            val?
        }
        "_random" => {
            #[cfg(feature = "fastrand")]
            let val = fixed!([i, j], {
                let (i, j) = (tonumber(i)?.floor() as i64, tonumber(j)?.floor() as i64);
                Ok(Number(fastrand::i64(i..=j) as f64))
            });
            #[cfg(not(feature = "fastrand"))]
            let val = Err(Error::RandUnavailable);
            val?
        },
        "end" | "while" | "if" | "define" | "_func" => return Err(Error::MustBeTopLevel),
        name => run::execute_function(state, name, args)?,
    })
}
