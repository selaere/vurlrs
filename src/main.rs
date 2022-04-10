use std::{env, fs};

use crate::parse::do_code_blocks;

mod builtins;
mod parse;
mod run;

fn main() {
    let path = env::args().nth(1).expect("argument not given");
    let code = fs::read_to_string(path).expect("error while opening file");
    let mut parsed = parse::parse(&code);
    do_code_blocks(&mut parsed).expect("ehkgeioujhehj");
    parse::print_parsed(&parsed);
    println!("---");
    run::execute(parsed).unwrap_or_else(|x| {
        eprintln!("{}", x);
        return;
    });
}
