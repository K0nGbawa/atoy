use std::{io::Write, print, println};

use atoy_macros::atoy_function;

use crate::vm::Args;

#[atoy_function]
pub fn println(args: Args) {
    let values = args.values;
    println!(
        "{}",
        values
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    );
}

#[atoy_function]
pub fn input(prompt: String) -> String {
    print!("{}", prompt);
    std::io::stdout().flush().unwrap();
    let mut s = String::new();
    std::io::stdin().read_line(&mut s).unwrap();
    if s.ends_with("\r\n") {
        s.truncate(s.len() - 2);
    } else if s.ends_with('\n') {
        s.truncate(s.len() - 1);
    }
    s
}
