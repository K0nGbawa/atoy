#![allow(non_snake_case)]
use std::{cell::RefCell, collections::{HashMap, HashSet}, format, io::Write, print, println, rc::Rc};

use atoy_macros::atoy_function;

use crate::{
    parser::{Table, Value},
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


pub fn repr_inner(value: &Value, seen: &mut HashMap<Value, usize>) -> String {
    let len = seen.len();
    if let Some(id) = seen.get(value) {
        return format!("<Circular#{}>", id)
    } else {
        seen.insert(value.clone(), len);
    }
    match value {
        Value::Array(rc) => {
            format!("Array#{len} [{}]",  rc.borrow().iter().map(|e| repr_inner(e, seen)).collect::<Vec<_>>().join(", "))
        }
        Value::Table(rc) => {
            format!("Table#{len} {{{}}}", rc.borrow().data.iter().map(|(k, v)| format!("[{}]: {}", k, repr_inner(v, seen))).collect::<Vec<_>>().join(", "))
        }
        other => format!("{}", other)
    }
}

#[atoy_function]
pub fn repr(value: &Value) -> String {
    let mut seen = HashMap::new();
    repr_inner(value, &mut seen)
}

#[atoy_function]
pub fn r#type(value: &Value) -> String {
    ValueType::from(value).to_string()
}

#[atoy_function]
pub fn Table() -> crate::parser::Value {
    crate::parser::Value::Table(Rc::new(RefCell::new(Table::new())))
}

#[atoy_function]
pub fn Array() -> crate::parser::Value {
    crate::parser::Value::Array(Rc::new(RefCell::new(Vec::new())))
}

#[atoy_function]
pub fn Array_len(vector: Rc<RefCell<Vec<Value>>>) -> i64 {
    vector.borrow().len() as i64
} // 这里其实不安全，但是应该没人搞那么大的数组吧（

#[atoy_function]
pub fn Array_push(vector: Rc<RefCell<Vec<Value>>>, value: &Value) {
    vector.borrow_mut().push(value.clone());
}

type RRTable = Rc<RefCell<Table>>;

#[atoy_function]
pub fn setPrototypeOf(target: RRTable, prototype: RRTable) {
    target.borrow_mut().prototype = Some(prototype);
}

#[atoy_function]
pub fn clearPrototypeOf(target: RRTable) {
    target.borrow_mut().prototype = None;
}

#[atoy_function]
pub fn getPrototypeOf(target: RRTable) -> Value {
    if let Some(proto) = &target.borrow().prototype {
        Value::Table(proto.clone())
    } else {
        Value::None
    }
}

#[atoy_function]
pub fn setMetatableOf(target: RRTable, meta: RRTable) {
    target.borrow_mut().meta = Some(meta);
}

#[atoy_function]
pub fn clearMetatableOf(target: RRTable) {
    target.borrow_mut().meta = None;
}

#[atoy_function]
pub fn getMetatableOf(target: RRTable) -> Value {
    if let Some(meta) = &target.borrow().meta {
        Value::Table(meta.clone())
    } else {
        Value::None
    }
}
