use std::{fmt, iter, str};

#[allow(dead_code)]
pub fn print_parsed(parsed: &[Option<Command>]) {
    for line in parsed {
        if let Some(cmd) = line {
            println!("{}", cmd);
        } else {
            println!("~");
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Expr {
    Command(Command),
    Literal(String),
    Number(f64),
    Variable(String),
    Lineptr(usize),
}

#[derive(Clone, PartialEq, Debug)]
pub struct Command {
    pub name: String,
    pub args: Vec<Expr>,
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Command(cmd) => write!(f, "{}", cmd),
            Self::Literal(s) => write!(f, "\"{}\"", s.replace('"', r#"\""#)),
            Self::Number(n) => write!(f, "{}", n),
            Self::Variable(s) => write!(f, "[{}]", s),
            Self::Lineptr(s) => write!(f, "(line {})", s),
        }
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = self.args.iter();
        write!(f, "{}(", self.name)?;
        if let Some(x) = iter.next() {
            write!(f, "{}", x)?
        }
        for i in iter {
            write!(f, " {}", i)?;
        }
        write!(f, ")")?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Lined(usize, ParseErrorLine),
    UnclosedBlock,
    UnexpectedEnd,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorLine {
    StringEOL,
    NameIsNotString,
    UnclosedParen,
    UnexpectedParen,
    EmptyCommand,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lined(line, error) => write!(f, "error at line {}: {}", line, error),
            Self::UnclosedBlock => write!(f, "unclosed block"),
            Self::UnexpectedEnd => write!(f, "unexpected `end`"),
        }
    }
}
impl std::error::Error for ParseError {}

impl fmt::Display for ParseErrorLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Self::StringEOL => "quoted strings cannot span multiple lines",
            Self::NameIsNotString => "the name of a command must be a string, try using _apply",
            Self::UnclosedParen => "unclosed parenthesis",
            Self::UnexpectedParen => "unexpected parenthesis",
            Self::EmptyCommand => "empty command",
        };
        write!(f, "{}", str)
    }
}

pub fn parse(code: &str) -> Result<Vec<Option<Command>>, ParseError> {
    let mut stack = Vec::new();
    let mut commands = Vec::<Option<Command>>::new();
    for (lineno, line) in code.split('\n').enumerate() {
        let line = line.trim();
        if !line.is_empty() && !line.starts_with('#') {
            let mut cmd = parse_command(&mut line.trim().chars().peekable(), true)
                .map_err(|e| ParseError::Lined(lineno, e))?;
            match cmd.name.as_str() {
                "if" | "while" | "define" | "_func" => stack.push(lineno),
                "end" => {
                    let startno = stack.pop().ok_or(ParseError::UnexpectedEnd)?;
                    let startline = commands[startno].as_mut().unwrap();
                    startline.args.push(Expr::Lineptr(lineno));

                    cmd.args.push(Expr::Lineptr(startno));
                    cmd.name = cmd.name + " " + &startline.name;
                }
                _ => (),
            }
            commands.push(Some(cmd));
        } else {
            commands.push(None);
        }
    }
    if !stack.is_empty() {
        return Err(ParseError::UnclosedBlock);
    }
    Ok(commands)
}

fn parse_command(
    chars: &mut iter::Peekable<str::Chars>,
    is_top_level: bool,
) -> Result<Command, ParseErrorLine> {
    let mut args: Vec<Expr> = vec![];
    loop {
        match chars.next() {
            Some('(') => args.push(Expr::Command(parse_command(chars, false)?)),
            Some('"') => {
                let mut s = String::with_capacity(chars.size_hint().0);
                loop {
                    match chars.next() {
                        Some('"') if matches!(chars.peek(), Some(')' | ' ') | None) => break,
                        Some(chr) => s.push(chr),
                        None => return Err(ParseErrorLine::StringEOL),
                    }
                }
                args.push(Expr::Literal(s))
            }
            Some(' ') => (),

            Some(')') if is_top_level => return Err(ParseErrorLine::UnexpectedParen),
            Some(')') => break,

            None if is_top_level => break,
            None => return Err(ParseErrorLine::UnclosedParen),

            Some(fst) => {
                let mut s = String::new();
                let mut parenlevel = 0;
                s.push(fst);
                loop {
                    match chars.peek() {
                        Some('(') => parenlevel += 1,
                        Some(')') if parenlevel > 0 => parenlevel -= 1,
                        Some(' ' | ')') | None => break,
                        _ => (),
                    }
                    if let Some(x) = chars.next() {
                        s.push(x)
                    }
                }
                args.push(if s.starts_with('[') && s.ends_with(']') {
                    Expr::Variable(s[1..s.len() - 1].to_owned())
                } else if let Ok(x) = s.parse::<f64>() {
                    Expr::Number(x)
                } else {
                    Expr::Literal(s)
                })
            }
        }
    }
    if let Expr::Literal(name) = &args.get(0).ok_or(ParseErrorLine::EmptyCommand)? {
        Ok(Command {
            name: name.to_owned(),
            args: args[1..].to_vec(),
        })
    } else {
        Err(ParseErrorLine::NameIsNotString)
    }
}
