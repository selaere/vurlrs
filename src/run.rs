use crate::{builtins, parse};
use parse::{Command, Expr};
use std::{collections::HashMap, error::Error, fmt, rc::Rc, cell::RefCell};

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct State {
    pub(crate) globals: HashMap<Rc<str>, Value>,
    pub(crate) lineno: usize,
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Value {
    String(Rc<str>),
    List(Rc<RefCell<Vec<Value>>>),
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
    function: Rc<str>,
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
    NameError(Rc<str>),
    IsNotNumber(Value),
    IOError(std::io::Error),
    ZeroIndex,
    IndexError { index: usize, len: usize },
    PopError,
    OrdError(Rc<str>),
    ChrError(u32),
}
impl fmt::Display for RunErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValueError(num) => write!(f, "expected {} arguments", num),
            Self::NotImplemented => write!(f, "command not implemented"),
            Self::NameError(name) => write!(f, "variable [{}] is undefined", name),
            Self::IsNotNumber(value) => write!(f, "{} is not a number", value),
            Self::IOError(err) => write!(f, "io error: {}", err),
            Self::ZeroIndex => write!(f, "vurl is one-indexed, sadly"),
            Self::IndexError { index, len } => {
                write!(f, "tried to use index {} of a list of {} items", index, len)
            }
            Self::PopError => write!(f, "cannot pop from an empty list"),
            Self::OrdError(s) => write!(f, "string \"{}\" must be one character long", s),
            Self::ChrError(i) => write!(f, "{} is not a valid unicode codepoint", i),
        }
    }
}
impl Error for RunErrorKind {}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::List(v) => {
                let borrow = v.borrow();
                let mut iter = borrow.iter();
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
            let args = (args.iter())
                .map(|x| evaluate(state, x))
                .collect::<Result<Vec<Value>, _>>()?;
            builtins::builtins(state, name, &args[..]).map_err(|x| RunError {
                line: state.lineno,
                function: Rc::from(name.as_str()),
                inner: x,
            })
        }
        Expr::Literal(s) => Ok(Value::String(Rc::from(s.as_str()))),
        Expr::Variable(s) => (state.globals.get(s.as_str()).cloned()).ok_or_else(|| RunError {
            line: state.lineno,
            function: Rc::from("n/a"),
            inner: RunErrorKind::NameError(Rc::from(s.as_str())),
        }),
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
