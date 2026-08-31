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
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    );
    Ok(Value::None)
}

#[atoy_function(crate::builtin)]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub mod time {
    use std::time::{SystemTime, UNIX_EPOCH};

    use atoy_macros::atoy_function;
    #[atoy_function(crate::builtin::time)]
    pub fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before Unix epoch")
            .as_millis() as i64
    }
}
