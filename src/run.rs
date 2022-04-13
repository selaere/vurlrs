use crate::{builtins, parse};
use parse::{Command, Expr};
use std::{cell::RefCell, collections::HashMap, error::Error, fmt, rc::Rc};

#[derive(PartialEq, Debug)]
pub struct State<'a> {
    pub globals: &'a mut HashMap<Rc<str>, Value>,
    pub locals: HashMap<Rc<str>, Value>,
    pub lineno: usize,
    pub functions: &'a mut HashMap<Rc<str>, Function>,
    pub lines: &'a [Option<Command>],
}

#[derive(Clone, PartialEq, Debug)]
pub struct Function {
    pub lineno: usize,
    pub arguments: Option<Rc<[Rc<str>]>>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    String(Rc<str>),
    List(Rc<RefCell<Vec<Value>>>),
    Number(f64),
    Lineptr(usize),
}

impl Default for Value {
    fn default() -> Self {
        Self::Number(f64::NAN)
    }
}

#[derive(Debug)]
pub struct RunError {
    line: usize,
    function: Rc<str>,
    inner: RunErrorKind,
}
impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "error (line {}, command {}):\n{}",
            self.line, self.function, self.inner
        )
    }
}
impl Error for RunError {}

#[derive(Debug)]
pub enum RunErrorKind {
    Wrap(Box<RunError>),
    Return(Value),
    ValueError(usize),
    NotImplemented,
    NameError(Rc<str>),
    FuncDefined(Rc<str>),
    IsNotNumber(Value),
    IsNotList(Value),
    IOError(std::io::Error),
    ZeroIndex,
    IndexError(usize, usize),
    PopError,
    OrdError(Rc<str>),
    ChrError(u32),
}
impl fmt::Display for RunErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wrap(e) => write!(f, "{}", e),
            Self::Return(v) => write!(f, "value {} returned outside function", v),
            Self::ValueError(num) => write!(
                f,
                "expected {} argument{}",
                num,
                if *num == 1 { "" } else { "s" }
            ),
            Self::NotImplemented => write!(f, "command not implemented"),
            Self::NameError(name) => write!(f, "variable [{}] is undefined", name),
            Self::FuncDefined(name) => write!(f, "function {} is already defined", name),
            Self::IsNotNumber(value) => write!(f, "{} is not a number", value),
            Self::IsNotList(value) => write!(f, "{} is not a list", value),
            Self::IOError(err) => write!(f, "io error: {}", err),
            Self::ZeroIndex => write!(f, "vurl is one-indexed, sadly"),
            Self::IndexError(index, len) => {
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
            Value::Lineptr(lineno) => write!(f, "(line {})", lineno),
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
        Expr::Number(n) => Ok(Value::Number(*n)),
        Expr::Variable(s) => {
            let var = if s.starts_with('%') {
                state.locals.get(s.as_str())
            } else {
                state.globals.get(s.as_str())
            };
            var.cloned().ok_or_else(|| RunError {
                line: state.lineno,
                function: Rc::from("n/a"),
                inner: RunErrorKind::NameError(Rc::from(s.as_str())),
            })
        }
        Expr::Lineptr(l) => Ok(Value::Lineptr(*l)),
    }
}

pub fn execute(lines: Vec<Option<Command>>) -> Result<(), RunError> {
    let mut state = State {
        globals: &mut HashMap::new(),
        locals: HashMap::new(),
        functions: &mut HashMap::new(),
        lineno: 0,
        lines: &lines,
    };
    while state.lineno < lines.len() {
        if let Some(cmd) = &lines[state.lineno] {
            evaluate(&mut state, &Expr::Command(cmd.to_owned()))?;
        }
        state.lineno += 1;
    }
    Ok(())
}

pub fn execute_function(
    state: &mut State,
    name: &str,
    args: &[Value],
) -> Result<Value, RunErrorKind> {
    let func = state
        .functions
        .get(name)
        .ok_or(RunErrorKind::NotImplemented)?;
    let mut locals = HashMap::from([(
        Rc::from("%args"),
        Value::List(Rc::from(RefCell::from(args.to_vec()))),
    )]);
    if let Some(fargs) = &func.arguments {
        let fargs = Rc::clone(fargs);
        if fargs.len() != args.len() {
            return Err(RunErrorKind::ValueError(fargs.len()));
        }
        for (k, v) in fargs.iter().zip(args) {
            locals.insert(Rc::clone(k), v.clone());
        }
    }
    let mut state = State {
        globals: state.globals,
        locals,
        lineno: func.lineno,
        functions: state.functions,
        lines: state.lines,
    };
    loop {
        if let Some(cmd) = &state.lines[state.lineno] {
            match evaluate(&mut state, &Expr::Command(cmd.to_owned())) {
                Ok(_) => (),
                Err(RunError {
                    inner: RunErrorKind::Return(v),
                    ..
                }) => return Ok(v),
                Err(e) => return Err(RunErrorKind::Wrap(Box::new(e))),
            };
        };
        state.lineno += 1;
    }
}
