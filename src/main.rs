use std::{collections::HashMap, env, fs};

mod builtins;
mod parse;
mod run;

fn main() {
    let path = env::args().nth(1).expect("argument not given");
    let code = fs::read_to_string(path).expect("error while opening file");
    let parsed = parse::parse(&code);
    parse::print_parsed(&parsed);
    println!("---");
    run::execute_lines(
        run::State {
            globals: HashMap::new(),
        },
        parsed,
    )
    .unwrap();
}
