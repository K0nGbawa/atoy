use std::println;

use crate::parser::Value;

use crate::vm::Args;

pub fn println(args: Args) -> Value {
    let values = args.values;
    println!(
        "{}",
        values
            .iter()
            .map(|v| format!("{}", v))
            .collect::<Vec<_>>()
            .join(" ")
    );
    Value::None
}
