use std::println;

use atoy_macros::atoy_function;

use crate::vm::Args;

#[atoy_function]
pub fn println(args: Args){
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

// #[atoy_function]
// pub fn input(prompt: String) {
//     let mut s = String::new();
//     std::io::stdin().read_line(&mut s).unwrap();
//     s.to_string();
// }
