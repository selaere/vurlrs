use std::{collections::HashMap, fmt};

use crate::{parse, builtins};
use parse::{Command, Expr};

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct State {
    pub(crate) globals: HashMap<String, Value>,
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Value {
    String(String),
    List(Vec<Value>),
    Number(f64),
}

impl Default for Value {
    fn default() -> Self {
        Self::Number(f64::NAN)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::List(v) => {
                let mut iter = v.iter();
                write!(f, "(")?;
                if let Some(x) = iter.next() {
                    write!(f, "{}", x)?
                }
                for i in iter {
                    write!(f, ",{}", i)?;
                }
                write!(f, ")")?;
                Ok(())
            }
            Value::Number(s) => write!(f, "{}", s),
        }
    }
}

fn evaluate(state: &mut State, expr: &Expr) -> Result<Value, String> {
    match expr {
        Expr::Command(Command { name, args }) => {
            let args = args
                .iter()
                .map(|x| evaluate(state, x))
                .collect::<Result<_, _>>()?;
            builtins::builtins(state, &name, args)
        }
        Expr::Literal(s) => Ok(Value::String(s.to_owned())),
        Expr::Variable(s) => Ok(state.globals[s].clone()),
    }
}

pub(crate) fn execute_lines(mut state: State, lines: Vec<Option<Command>>) -> Result<(), String> {
    let mut lineno = 0;
    while lineno < lines.len() {
        if let Some(cmd) = &lines[lineno] {
            evaluate(&mut state, &Expr::Command(cmd.to_owned()))?;
        }
        lineno += 1;
    }
    Ok(())
}
