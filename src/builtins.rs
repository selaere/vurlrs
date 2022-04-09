use crate::run::{State, Value};
use std::fmt::Write;

fn tonumber(expr: &Value) -> Result<f64, String> {
    match &expr {
        Value::String(s) => s.parse::<f64>().map_err(|_| format!("{s} is not a number")),
        Value::List(_) => return Err(format!("list is not a number")),
        Value::Number(n) => Ok(*n),
    }
}

/*fn tostring(expr: &Value) -> String {
    format!("{}", expr)
}*/

pub(crate) fn builtins(state: &mut State, name: &str, args: Vec<Value>) -> Result<Value, String> {
    Ok(match name {
        "add" => Value::Number(
            args.iter()
                .map(|x| tonumber(x))
                .reduce(|x, y| Ok(x? + y?))
                .unwrap_or(Ok(0f64))?, // returns 0 if argument list is empty
        ),
        "mul" => Value::Number(
            args.iter()
                .map(|x| tonumber(x))
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
            Value::Number(f64::NAN)
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
        s => return Err(format!("{} not implemented", s)),
    })
}
