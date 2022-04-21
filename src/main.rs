use std::collections::HashMap;

mod builtins;
mod parse;
mod run;

fn main() {
    if let Some(path) = std::env::args().nth(1) {
        let code = std::fs::read_to_string(path).expect("error while opening file");
        let parsed = parse::parse(&code).expect("parsing error");
        // parse::print_parsed(&parsed);
        // println!("---");
        run::execute(&parsed).unwrap_or_else(|x| {
            eprintln!("{}", x);
        });
    } else {
        repl();
    }
}

fn repl() {
    let stdin = std::io::stdin();
    println!("welcome to vurlrs repl. do `quit` to quit.\nnote: you cannot use code blocks yet");
    let lines = Vec::new();
    let mut globals = HashMap::new();
    let mut locals = HashMap::new();
    let mut functions = HashMap::new();
    loop {
        print!(">>> ");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let mut buf = String::new();
        stdin.read_line(&mut buf).expect("error reading from stdin");
        let line = buf.trim();
        if line.starts_with('[') && line.ends_with(']') && !line.contains(' ') {
            buf = String::from("print ") + line;
        }
        match parse::parse_line(&buf) {
            Err(x) => {
                eprintln!("parsing error: {}", x);
                continue;
            }
            Ok(None) => (),
            Ok(Some(cmd)) => {
                if cmd.name == "quit" {
                    println!("bye");
                    return;
                }
                // right now this is a bit useless. when we actually handle function definitions it
                // will be necessary to keep the lines in `lines`
                let mut state = run::State {
                    globals: &mut globals,
                    locals: &mut locals,
                    functions: &mut functions,
                    lineno: lines.len(),
                    lines: &lines,
                };
                match run::evaluate(&mut state, &parse::Expr::Command(cmd.to_owned())) {
                    Err(e) => eprintln!("error: {}", e),
                    Ok(run::Value::String(x)) if x.is_empty() => (),
                    Ok(val) => println!("{}", val),
                }
            }
        }
    }
}
