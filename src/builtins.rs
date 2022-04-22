use crate::run::{execute_command, Function, RunErrorKind as Error, State, Value};
use std::cell::{RefCell, RefMut};
use std::fmt::Write;
use std::rc::Rc;
use std::time::SystemTime;
use Value::{Lineptr, List, Number, String as StringVal};

fn frombool(boole: bool) -> Value {
    Number(boole as i32 as f64)
}

impl Value {
    /// converts the value to a number
    fn tonum(&self) -> Result<f64, Error> {
        match &self {
            StringVal(s) => s
                .parse::<f64>()
                .map_err(|_| Error::IsNotNumber(self.clone())),
            List(_) => Err(Error::IsNotNumber(self.clone())),
            Number(n) => Ok(*n),
            Lineptr(_) => panic!(),
        }
    }

    fn tostr(&self) -> Rc<str> {
        match self {
            StringVal(s) => Rc::clone(s),
            other => Rc::from(format!("{}", other).as_str()),
        }
    }

    fn toindex(&self) -> Result<usize, Error> {
        (self.tonum()?.floor() as usize)
            .checked_sub(1)
            .ok_or(Error::ZeroIndex)
    }

    fn tolist(&self) -> Result<RefMut<Vec<Value>>, Error> {
        match self {
            List(l) => Ok(l.borrow_mut()),
            _ => Err(Error::IsNotList(self.clone())),
        }
    }
}

fn eq(a: &Value, b: &Value) -> bool {
    match &[a, b] {
        [List(l), List(m)] => {
            let (l, m) = (l.borrow(), m.borrow());
            l.iter().zip(m.iter()).all(|(x, y)| eq(x, y))
        }
        [Number(x), Number(y)] => x == y,
        [x, y] => x.tostr() == y.tostr(),
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
            fixed!([arg1], Number($code(arg1.tonum()?) as f64))
        };
    }
    macro_rules! dyad {
        ($code:expr) => {
            fixed!([arg1, arg2], {
                Number($code(arg1.tonum()?, arg2.tonum()?) as f64)
            })
        };
    }

    // --- code block commands ---

    if let Some(Lineptr(lineptr)) = args.last() {
        args = &args[..args.len() - 1];
        return Ok(match name {
            "end _cmd" | "end define" => match args {
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
                if cond.tonum()? == 0f64 {
                    state.lineno = *lineptr;
                }
                Value::default()
            }),
            "_cmd" => {
                if args.len() <= 1 {
                    return Err(Error::ValueError(1));
                }
                let name = &args[0];
                let arguments = &args[1..].iter().map(Value::tostr).collect::<Vec<_>>();
                let arguments = arguments
                    .first()
                    .map_or(false, |x| !str::eq(x, "...")) // ?????
                    .then(|| Rc::from(&arguments[..]));
                if (state.functions)
                    .insert(
                        name.tostr(),
                        Function {
                            lineno: state.lineno + 1,
                            arguments,
                        },
                    )
                    .is_some()
                {
                    return Err(Error::FuncDefined(name.tostr()));
                }
                state.lineno = *lineptr;
                Value::default()
            }
            "define" => fixed!([name], {
                if (state.functions)
                    .insert(
                        Rc::from("call ".to_string() + &name.tostr()),
                        Function {
                            lineno: state.lineno + 1,
                            arguments: None,
                        },
                    )
                    .is_some()
                {
                    return Err(Error::FuncDefined(name.tostr()));
                };
                state.lineno = *lineptr;
                Value::default()
            }),
            name => execute_command(state, name, args)?,
        });
    }

    // --- normal commands ---

    Ok(match name {
        "add" => Number(
            args.iter()
                .map(Value::tonum)
                .reduce(|x, y| Ok(x? + y?))
                .unwrap_or(Ok(0f64))?,
        ),
        "mul" => Number(
            args.iter()
                .map(Value::tonum)
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
                l => Number(l.tostr().chars().count() as _),
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
                if arg.tonum()? != 0f64 {
                    return Ok(Number(1f64));
                }
            }
            Number(0f64)
        }
        "and" => {
            for arg in args {
                if arg.tonum()? == 0f64 {
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
            let (start, stop) = (x.toindex()?, y.toindex()? + 1);
            StringVal(Rc::from(
                s.tostr()
                    .chars()
                    .skip(start)
                    .take(stop.saturating_sub(start))
                    .collect::<String>(),
            ))
        }),
        "_chr" => fixed!([x], {
            let num = x.tonum()?.floor() as u32;
            StringVal(Rc::from(
                char::try_from(num)
                    .map_err(|_| Error::ChrError(num))?
                    .to_string(),
            ))
        }),
        "_ord" => fixed!([x], {
            let string = x.tostr();
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
            let list = l.tolist()?;
            let index = i.toindex()?;
            list.get(index)
                .ok_or(Error::IndexError(index, list.len()))?
                .clone()
        }),
        "push" => fixed!([l, v], {
            let mut borrow = l.tolist()?;
            borrow.push(v.clone());
            Value::default()
        }),
        "pop" => fixed!([l], l.tolist()?.pop().ok_or(Error::PopError)?),
        "insert" => fixed!([l, i, v], {
            let mut borrow = l.tolist()?;
            let index = borrow.len().min(i.toindex()?);
            borrow.insert(index, v.clone());
            Value::default()
        }),
        "remove" => fixed!([l, i], {
            let mut borrow = l.tolist()?;
            let len = borrow.len();
            let index = len.min(i.toindex()?);
            borrow.remove(index)
        }),
        "replace" => fixed!([l, i, v], {
            let mut borrow = l.tolist()?;
            let index = i.toindex()?;
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
            let l = l.tostr();
            if l.starts_with('.') {
                state.locals.insert(l, r.clone());
            } else {
                state.globals.insert(l, r.clone());
            }
            Value::default()
        }),
        "_get" => fixed!([v], {
            let s = v.tostr();
            let var = if s.starts_with('.') {
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
        "_error" => fixed!([e], return Err(Error::UserError(e.tostr()))),
        "call" => execute_command(
            state,
            &("call ".to_string() + &(args.get(0).ok_or(Error::ValueError(1))?).tostr()),
            &args[1..],
        )?,
        "_apply" => fixed!([n, a], {
            execute_command(state, n.tostr().as_ref(), a.tolist()?.as_slice())?
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
                let (i, j) = (i.tonum()?.floor() as i64, j.tonum()?.floor() as i64);
                Ok(Number(fastrand::i64(i..=j) as f64))
            });
            #[cfg(not(feature = "fastrand"))]
            let val = Err(Error::RandUnavailable);
            val?
        }
        "end" | "while" | "if" | "define" | "_cmd" => return Err(Error::MustBeTopLevel),
        _ => return Err(Error::IsNotBuiltIn),
    })
}
