use crate::run::{self, Function, RunErrorKind as Error, State, Value};
use std::cell::{RefCell, RefMut};
use std::fmt::Write;
use std::rc::Rc;
use std::time::SystemTime;
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
                ))
            }
        };
    }
    // for numeric commands
    macro_rules! monad {
        ($code:expr) => {
            fixed!([arg1], Number($code(tonumber(arg1)?) as f64))
        };
    }
    macro_rules! dyad {
        ($code:expr) => {
            fixed!([arg1, arg2], {
                Number($code(tonumber(arg1)?, tonumber(arg2)?) as f64)
            })
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
            name => run::execute_command(state, name, args)?,
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
        "sub" => dyad!(<f64 as std::ops::Sub>::sub),
        "div" => dyad!(<f64 as std::ops::Div>::div),
        "mod" => dyad!(<f64 as std::ops::Rem>::rem),
        "_pow" => dyad!(f64::powf),
        "_floor" => monad!(f64::floor),
        "_round" => monad!(f64::round),
        "_sqrt" => monad!(f64::sqrt),
        "_sin" => monad!(f64::sin),
        "_cos" => monad!(f64::cos),
        "_tan" => monad!(f64::tan),
        "_asin" => monad!(f64::asin),
        "_acos" => monad!(f64::acos),
        "_atan" => monad!(f64::atan),
        "_ln" => monad!(f64::ln),
        "_exp" => monad!(f64::exp),
        "len" => fixed!([i], {
            match i {
                List(l) => Number(l.borrow().len() as _),
                l => Number(tostr(l).chars().count() as _),
            }
        }),
        "eq" => fixed!([x, y], frombool(eq(x, y))),
        "not" => monad!(|x| (x == 0f64) as i64),
        "lt" => dyad!(|x, y| (x < y) as i64),
        "gt" => dyad!(|x, y| (x > y) as i64),
        "lte" => dyad!(|x, y| (x <= y) as i64),
        "gte" => dyad!(|x, y| (x >= y) as i64),
        "or" => {
            for arg in args {
                if tonumber(arg)? != 0f64 {
                    return Ok(Number(1f64));
                }
            }
            Number(0f64)
        }
        "and" => {
            for arg in args {
                if tonumber(arg)? == 0f64 {
                    return Ok(Number(0f64));
                }
            }
            Number(1f64)
        }
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
        "_islist" => fixed!([x], Number(matches!(x, List(_)) as i64 as f64)),
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
            run::execute_command(state, &("call ".to_string() + &tostr(name)), &args[1..])?
        }),
        "_apply" => fixed!([n, a], {
            run::execute_command(state, tostr(n).as_ref(), tolist(a)?.as_slice())?
        }),
        "_return" => {
            return Err(match args {
                [] => Error::Return(Value::default()),
                [x] => Error::Return(x.clone()),
                _ => Error::ValueError(1),
            })
        }
        "_time" => fixed!([], {
            Number(
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs_f64(),
            )
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
        }
        "end" | "while" | "if" | "define" | "_func" => return Err(Error::MustBeTopLevel),
        _ => return Err(Error::IsNotBuiltIn),
    })
}
