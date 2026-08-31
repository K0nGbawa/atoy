use std::println;

use atoy_macros::atoy_function;

use crate::parser::Value;

use crate::vm::Args;

pub fn println(args: Args) -> Result<Value, String> {
    let values = args.values;
    println!(
        "{}",
        values
            .iter()
            .map(|v| format!("{}", v))
            .collect::<Vec<_>>()
            .join(" ")
    );
    Ok(Value::None)
}

#[atoy_function]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
