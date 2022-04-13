use std::{env, fs};


mod builtins;
mod parse;
mod run;

fn main() {
    let path = env::args().nth(1).expect("argument not given");
    let code = fs::read_to_string(path).expect("error while opening file");
    let parsed = parse::parse(&code).expect("parsing error");
    parse::print_parsed(&parsed);
    println!("---");
    run::execute(parsed).unwrap_or_else(|x| {
        eprintln!("{}", x);
    });
}
