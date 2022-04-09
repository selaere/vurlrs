use std::{fmt, iter, str};

pub(crate) fn print_parsed(parsed: &[Option<Command>]) {
    for line in parsed {
        if let Some(cmd) = line {
            println!("{}", cmd);
        } else {
            println!("~");
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Expr {
    Command(Command),
    Literal(String),
    Variable(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Command {
    pub(crate) name: String,
    pub(crate) args: Vec<Expr>,
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Command(cmd) => {
                fmt::Display::fmt(cmd, f)?;
            }
            Self::Literal(s) => {
                write!(f, "\"{}\"", s.replace('"', r#"\""#))?;
            }
            Self::Variable(s) => {
                write!(f, "[{}]", s)?;
            }
        }
        Ok(())
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

pub(crate) fn parse(code: &str) -> Vec<Option<Command>> {
    code.split('\n')
        .enumerate()
        .map(|(lineno, line)| {
            let line = line.trim();
            if line == "" || line.starts_with("#") {
                None
            } else {
                Some(
                    parse_command(&mut line.trim().chars().peekable(), true)
                        .unwrap_or_else(|e| panic!("syntax error: {} at line {}", e, lineno)),
                )
            }
        })
        .collect()
}

fn parse_command(
    chars: &mut iter::Peekable<str::Chars>,
    is_top_level: bool,
) -> Result<Command, String> {
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
                        None => return Err("strings cannot span multiple lines".to_string()),
                    }
                }
                args.push(Expr::Literal(s))
            }
            Some(' ') => (),

            Some(')') if is_top_level => return Err("unexpected )".to_string()),
            Some(')') => break,

            None if is_top_level => break,
            None => return Err("unclosed parenthesis".to_string()),

            Some(fst) => {
                let mut s = String::with_capacity(chars.size_hint().0);
                s.push(fst);
                loop {
                    if let Some(' ' | ')') | None = chars.peek().map(|&x| x) {
                        break;
                    }
                    chars.next().map(|x| s.push(x));
                }
                args.push(
                    if s.bytes().next() == Some(b'[') && s.bytes().last() == Some(b']') {
                        Expr::Variable(s[1..s.len() - 1].to_owned())
                    } else {
                        Expr::Literal(s)
                    },
                )
            }
        }
    }
    if args.len() == 0 {
        return Err("empty command".to_string());
    }
    if let Expr::Literal(name) = &args.get(0).ok_or_else(|| "empty command".to_string())? {
        Ok(Command {
            name: name.to_owned(),
            args: args[1..].to_vec(),
        })
    } else {
        return Err("name must be a string".to_string());
    }
}
