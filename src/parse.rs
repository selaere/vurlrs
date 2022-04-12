use std::{fmt, iter, str};

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

pub fn parse(code: &str) -> Vec<Option<Command>> {
    code.split('\n')
        .enumerate()
        .map(|(lineno, line)| {
            let line = line.trim();
            (!line.is_empty() && !line.starts_with('#')).then(|| {
                parse_command(&mut line.trim().chars().peekable(), true)
                    .unwrap_or_else(|e| panic!("syntax error: {} at line {}", e, lineno))
            })
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
                    if let Some(' ' | ')') | None = chars.peek().copied() {
                        break;
                    }
                    if let Some(x) = chars.next() {
                        s.push(x)
                    }
                }
                args.push(
                    if s.bytes().next() == Some(b'[') && s.bytes().last() == Some(b']') {
                        Expr::Variable(s[1..s.len() - 1].to_owned())
                    } else if let Ok(x) = s.parse::<f64>() {
                        Expr::Number(x)
                    } else {
                        Expr::Literal(s)
                    },
                )
            }
        }
    }
    if args.is_empty() {
        return Err("empty command".to_string());
    }
    if let Expr::Literal(name) = &args.get(0).ok_or_else(|| "empty command".to_string())? {
        Ok(Command {
            name: name.to_owned(),
            args: args[1..].to_vec(),
        })
    } else {
        Err("name must be a string".to_string())
    }
}

pub fn do_code_blocks(cmds: &mut Vec<Option<Command>>) -> Result<(), String> {
    let mut stack: Vec<usize> = Vec::new();
    for lineno in 0..cmds.len() {
        if let Some(cmd) = &cmds[lineno] {
            match cmd.name.as_str() {
                "if" | "while" | "define" | "_func" => stack.push(lineno),
                "end" => {
                    // ugly
                    let start = (stack.pop())
                        .ok_or_else(|| format!("unexpected ``end`` at line {}", lineno + 1))?;
                    let startline = cmds[start].as_mut().unwrap();
                    startline.args.push(Expr::Lineptr(lineno));
                    let name = startline.name.to_owned();
                    let endline = cmds[lineno].as_mut().unwrap();
                    endline.args.push(Expr::Lineptr(start));
                    endline.name = "end ".to_string() + &name;
                }
                _ => (),
            };
        }
    }
    if !stack.is_empty() {
        return Err("``end`` missing".to_string());
    }
    Ok(())
}
