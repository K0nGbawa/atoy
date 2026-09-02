use std::{cell::RefCell, io::Write, print, println, rc::Rc};

use atoy_macros::atoy_function;

use crate::{
    parser::Table,
    vm::{Args, RuntimeError, ValueType},
};

fn try_add_fn_info(error: RuntimeError, name: &'static str) -> RuntimeError {
    match error {
        crate::vm::RuntimeError::TypeError {
            expected,
            found,
            thrower: None,
        } => crate::vm::RuntimeError::TypeError {
            expected,
            found,
            thrower: Some(name),
        },
        other => other,
    }
}
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
pub fn input(prompt: Option<String>) -> String {
    print!("{}", prompt.unwrap_or_default());
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

#[atoy_function]
pub fn r#type(value: &crate::parser::Value) -> String {
    ValueType::from(value).to_string()
}

#[atoy_function]
pub fn table() -> crate::parser::Value {
    crate::parser::Value::Table(Rc::new(RefCell::new(Table::new())))
}
