use crate::{builtins, parse};
use parse::{Command, Expr};
use std::{collections::HashMap, error::Error, fmt};

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct State {
    pub(crate) globals: HashMap<String, Value>,
    pub(crate) lineno: usize,
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Value {
    String(String),
    List(Vec<Value>),
    Number(f64),
    Quoted(Expr),
}

impl Default for Value {
    fn default() -> Self {
        Self::Number(f64::NAN)
    }
}

#[derive(Debug)]
pub(crate) struct RunError {
    line: usize,
    function: String,
    inner: RunErrorKind,
}
impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "runtime error (line {}, command {}): {}",
            self.line, self.function, self.inner
        )
    }
}
impl Error for RunError {}

#[derive(Debug)]
pub(crate) enum RunErrorKind {
    ValueError(i32),
    NotImplemented,
    IsNotNumber(Value),
    IOError(std::io::Error),
    IndexError { index: usize, len: usize },
    OrdError(String),
    ChrError(u32),
}
impl fmt::Display for RunErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValueError(num) => write!(f, "expected {} arguments", num),
            Self::NotImplemented => write!(f, "command not implemented"),
            Self::IsNotNumber(value) => write!(f, "{} is not a number", value),
            Self::IOError(err) => write!(f, "io error: {}", err),
            Self::IndexError { index, len } => {
                write!(f, "tried to get index {} of a list of {} items", index, len)
            }
            Self::OrdError(s) => write!(f, "string \"{}\" must be one character long", s),
            Self::ChrError(i) => write!(f, "{} is not a valid unicode codepoint", i)
        }
    }
}
impl Error for RunErrorKind {}

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
            Value::Quoted(expr) => write!(f, "'{}", expr),
        }
    }
}

fn evaluate(state: &mut State, expr: &Expr) -> Result<Value, RunError> {
    match expr {
        Expr::Command(Command { name, args }) => {
            let args = args
                .iter()
                .map(|x| evaluate(state, x))
                .collect::<Result<Vec<Value>, _>>()?;
            builtins::builtins(state, &name, &args[..]).map_err(|x| RunError {
                line: state.lineno,
                function: name.to_string(),
                inner: x,
            })
        }
        Expr::Literal(s) => Ok(Value::String(s.to_owned())),
        Expr::Variable(s) => Ok(state.globals[s].clone()),
        expr => Ok(Value::Quoted(expr.clone())),
    }
}

pub(crate) fn execute(lines: Vec<Option<Command>>) -> Result<(), RunError> {
    let mut state = State {
        globals: HashMap::new(),
        lineno: 0,
    };
    while state.lineno < lines.len() {
        if let Some(cmd) = &lines[state.lineno] {
            evaluate(&mut state, &Expr::Command(cmd.to_owned()))?;
        }
        state.lineno += 1;
    }
    Ok(())
}
